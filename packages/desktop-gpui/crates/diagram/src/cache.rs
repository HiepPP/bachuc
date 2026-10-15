//! Render queue and result cache. It has no WebView, so tests can drive it directly.

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use gpui_kit::Image;

use crate::{DiagramKind, DiagramState};

/// One render the WebView runs. `id` matches the WebView's answer to the job.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Job {
    pub id: u64,
    pub kind: DiagramKind,
    pub source: String,
}

#[derive(Default)]
pub(crate) struct RenderCache {
    done: [HashMap<String, DiagramState>; 2],
    queue: VecDeque<(DiagramKind, String)>,
    running: Option<Job>,
    /// The running job started before `clear`, so its result is out of date.
    running_is_stale: bool,
    next_id: u64,
}

fn slot(kind: DiagramKind) -> usize {
    match kind {
        DiagramKind::Mermaid => 0,
        DiagramKind::Math => 1,
    }
}

impl RenderCache {
    /// Returns the cached result, or queues the source once and returns `Pending`.
    pub fn request(&mut self, kind: DiagramKind, source: &str) -> DiagramState {
        if let Some(state) = self.done[slot(kind)].get(source) {
            return state.clone();
        }
        let running = self
            .running
            .as_ref()
            .is_some_and(|job| job.kind == kind && job.source == source);
        let queued = self.queue.iter().any(|(k, s)| *k == kind && s == source);
        if !running && !queued {
            self.queue.push_back((kind, source.to_string()));
        }
        DiagramState::Pending
    }

    /// Starts the next queued job. Returns `None` while a job runs, so renders run one at a time.
    pub fn next_job(&mut self) -> Option<Job> {
        if self.running.is_some() {
            return None;
        }
        let (kind, source) = self.queue.pop_front()?;
        self.next_id += 1;
        let job = Job {
            id: self.next_id,
            kind,
            source,
        };
        self.running = Some(job.clone());
        Some(job)
    }

    pub fn running(&self, id: u64) -> bool {
        self.running.as_ref().is_some_and(|job| job.id == id)
    }

    /// Stores the result of the running job `id` and returns its key. A stale job goes back to the
    /// front of the queue and returns `None`, as does an id that is not running.
    pub fn finish(
        &mut self,
        id: u64,
        result: Result<Arc<Image>, String>,
    ) -> Option<(DiagramKind, String)> {
        if !self.running(id) {
            return None;
        }
        let job = self.running.take()?;
        if std::mem::take(&mut self.running_is_stale) {
            self.queue.push_front((job.kind, job.source));
            return None;
        }
        let state = match result {
            Ok(image) => DiagramState::Ready(image),
            Err(error) => DiagramState::Failed(error),
        };
        self.done[slot(job.kind)].insert(job.source.clone(), state);
        Some((job.kind, job.source))
    }

    /// Drops every result, for example after the theme changes. The running job renders again.
    pub fn clear(&mut self) {
        self.done.iter_mut().for_each(HashMap::clear);
        self.running_is_stale = self.running.is_some();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::ImageFormat;

    fn image() -> Arc<Image> {
        Arc::new(Image::from_bytes(ImageFormat::Png, vec![1, 2, 3]))
    }

    #[test]
    fn repeated_source_is_ready_from_the_cache_without_a_new_job() {
        let mut cache = RenderCache::default();
        assert!(matches!(
            cache.request(DiagramKind::Mermaid, "graph TD; A-->B"),
            DiagramState::Pending
        ));
        let job = cache.next_job().expect("job");
        let img = image();
        assert_eq!(
            cache.finish(job.id, Ok(img.clone())),
            Some((DiagramKind::Mermaid, "graph TD; A-->B".to_string()))
        );
        match cache.request(DiagramKind::Mermaid, "graph TD; A-->B") {
            DiagramState::Ready(cached) => assert!(Arc::ptr_eq(&cached, &img)),
            other => panic!("expected Ready, got {other:?}"),
        }
        assert_eq!(cache.next_job(), None);
    }

    #[test]
    fn failed_source_gives_failed_with_the_error_text_and_is_cached() {
        let mut cache = RenderCache::default();
        cache.request(DiagramKind::Mermaid, "flowchart LR\n A --> (");
        let job = cache.next_job().expect("job");
        cache.finish(job.id, Err("Parse error on line 2".into()));
        match cache.request(DiagramKind::Mermaid, "flowchart LR\n A --> (") {
            DiagramState::Failed(error) => assert_eq!(error, "Parse error on line 2"),
            other => panic!("expected Failed, got {other:?}"),
        }
        assert_eq!(cache.next_job(), None);
    }

    #[test]
    fn requests_run_one_at_a_time_and_queue_each_source_once() {
        let mut cache = RenderCache::default();
        cache.request(DiagramKind::Math, "x^2");
        cache.request(DiagramKind::Math, "x^2");
        cache.request(DiagramKind::Math, "y^2");
        let first = cache.next_job().expect("first job");
        assert_eq!(first.source, "x^2");
        assert_eq!(cache.next_job(), None, "second job waits for the first");
        cache.request(DiagramKind::Math, "x^2");
        cache.finish(first.id, Ok(image()));
        let second = cache.next_job().expect("second job");
        assert_eq!(second.source, "y^2");
        cache.finish(second.id, Ok(image()));
        assert_eq!(cache.next_job(), None, "x^2 was queued only once");
    }

    #[test]
    fn kinds_are_cached_apart() {
        let mut cache = RenderCache::default();
        cache.request(DiagramKind::Math, "A");
        let job = cache.next_job().expect("job");
        cache.finish(job.id, Ok(image()));
        assert!(matches!(
            cache.request(DiagramKind::Mermaid, "A"),
            DiagramState::Pending
        ));
    }

    #[test]
    fn unknown_or_old_ids_are_ignored() {
        let mut cache = RenderCache::default();
        cache.request(DiagramKind::Math, "x");
        let job = cache.next_job().expect("job");
        assert_eq!(cache.finish(job.id + 1, Ok(image())), None);
        assert!(cache.running(job.id));
    }

    #[test]
    fn clear_renders_the_running_job_again() {
        let mut cache = RenderCache::default();
        cache.request(DiagramKind::Math, "x");
        let job = cache.next_job().expect("job");
        cache.clear();
        assert_eq!(cache.finish(job.id, Ok(image())), None);
        let again = cache.next_job().expect("job again");
        assert_eq!(again.source, "x");
        assert!(again.id > job.id);
    }
}
