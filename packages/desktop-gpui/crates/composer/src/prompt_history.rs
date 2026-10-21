//! Prompt history of the composer (ADR-0060): Up and Down recall the prompts sent earlier in the
//! same repo, as in the Claude Code CLI. The GPUI client keeps them in `prompt-history.json`,
//! keyed by the project root, so all worktrees of one repo share one list.

use bachuc_client::ProjectNode;
use bachuc_command::data_dir;
use gpui_kit::Global;
use serde_json::{Map, Value};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::Path;
use std::rc::Rc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

const FILE: &str = "prompt-history.json";
/// The entries one key keeps; the oldest drop first.
pub const MAX_ENTRIES: usize = 200;

/// The sent prompts of each key, oldest first.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PromptHistory {
    keys: BTreeMap<String, Vec<String>>,
}

impl PromptHistory {
    /// A bad file, key, or entry is dropped: the history starts empty rather than failing.
    pub fn parse(text: &str) -> Self {
        let Ok(Value::Object(stored)) = serde_json::from_str::<Value>(text) else {
            return Self::default();
        };
        let keys = stored
            .into_iter()
            .filter_map(|(key, value)| {
                let Value::Array(items) = value else {
                    return None;
                };
                let mut list: Vec<String> = items
                    .into_iter()
                    .filter_map(|item| item.as_str().map(String::from))
                    .collect();
                let extra = list.len().saturating_sub(MAX_ENTRIES);
                list.drain(..extra);
                Some((key, list))
            })
            .collect();
        Self { keys }
    }

    pub fn to_json(&self) -> String {
        let stored: Map<String, Value> = self
            .keys
            .iter()
            .map(|(key, list)| (key.clone(), Value::from(list.clone())))
            .collect();
        Value::Object(stored).to_string()
    }

    /// Adds `text` as the newest entry of `key`. Empty text and a repeat of the newest entry
    /// change nothing. Returns whether the history changed.
    pub fn push(&mut self, key: &str, text: &str) -> bool {
        if text.trim().is_empty() {
            return false;
        }
        let list = self.keys.entry(key.to_string()).or_default();
        if list.last().is_some_and(|newest| newest == text) {
            return false;
        }
        list.push(text.to_string());
        let extra = list.len().saturating_sub(MAX_ENTRIES);
        list.drain(..extra);
        true
    }

    /// The prompts of `key`, oldest first.
    pub fn entries(&self, key: &str) -> &[String] {
        self.keys.get(key).map_or(&[], Vec::as_slice)
    }

    /// A missing or bad file gives an empty history.
    pub fn load_from(dir: &Path) -> Self {
        std::fs::read_to_string(dir.join(FILE))
            .map(|text| Self::parse(&text))
            .unwrap_or_default()
    }
}

/// Writes `json` through a temp file and a rename, so a crash leaves the old file.
fn write_file(dir: &Path, json: &str) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let temp = dir.join(format!("{FILE}.tmp"));
    std::fs::write(&temp, json)?;
    std::fs::rename(temp, dir.join(FILE))
}

/// The newest write that reached the disk. Two writes may run at once; the older one skips.
static WRITTEN: Mutex<u64> = Mutex::new(0);
static LATEST: AtomicU64 = AtomicU64::new(0);

/// The history every composer shares, read from the disk once.
#[derive(Clone)]
pub struct SharedHistory(Rc<RefCell<PromptHistory>>);

impl Global for SharedHistory {}

impl SharedHistory {
    /// The shared history; the first call reads the file.
    pub fn get(cx: &mut gpui_kit::App) -> Self {
        if let Some(shared) = cx.try_global::<SharedHistory>() {
            return shared.clone();
        }
        let shared = Self(Rc::new(RefCell::new(PromptHistory::load_from(&data_dir()))));
        cx.set_global(shared.clone());
        shared
    }

    pub fn entries(&self, key: &str) -> Vec<String> {
        self.0.borrow().entries(key).to_vec()
    }

    /// Adds the prompt and saves the file off the UI thread when the history changed.
    pub fn push(&self, key: &str, text: &str, cx: &gpui_kit::App) {
        if !self.0.borrow_mut().push(key, text) {
            return;
        }
        let json = self.0.borrow().to_json();
        let sequence = LATEST.fetch_add(1, Ordering::SeqCst) + 1;
        cx.background_executor()
            .spawn(async move {
                let Ok(mut written) = WRITTEN.lock() else {
                    return;
                };
                if *written >= sequence {
                    return;
                }
                match write_file(&data_dir(), &json) {
                    Ok(()) => *written = sequence,
                    Err(error) => log::warn!("prompt history: {error}"),
                }
            })
            .detach();
    }
}

/// Where Up and Down stand in the entries of one key.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Recall {
    /// The index of the shown entry; `None` while the owner's own text shows.
    at: Option<usize>,
    /// The owner's text from before the first Up.
    stash: String,
}

impl Recall {
    /// The next older entry, or `None` at the oldest one and with no entries. `current` is
    /// the input text; the first Up stashes it.
    pub fn up(&mut self, list: &[String], current: &str) -> Option<String> {
        if list.is_empty() {
            return None;
        }
        let next = match self.at {
            None => {
                self.stash = current.to_string();
                list.len() - 1
            }
            Some(at) => at.min(list.len()).checked_sub(1)?,
        };
        self.at = Some(next);
        Some(list[next].clone())
    }

    /// The next newer entry, then the stash after the newest. `None` when no walk is active.
    pub fn down(&mut self, list: &[String]) -> Option<String> {
        let at = self.at?;
        match list.get(at + 1) {
            Some(entry) => {
                self.at = Some(at + 1);
                Some(entry.clone())
            }
            None => {
                self.at = None;
                Some(std::mem::take(&mut self.stash))
            }
        }
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// The list a prompt of `cwd` belongs to: the root of the project that has `cwd` as a workspace
/// or as its root, else `cwd`. Paths compare without a trailing `/`.
pub fn history_key(cwd: &str, projects: &[ProjectNode]) -> String {
    let cwd = cwd.trim_end_matches('/');
    projects
        .iter()
        .find(|project| {
            project.root.trim_end_matches('/') == cwd
                || project
                    .workspaces
                    .iter()
                    .any(|workspace| workspace.cwd.trim_end_matches('/') == cwd)
        })
        .map_or(cwd, |project| project.root.trim_end_matches('/'))
        .to_string()
}

/// The cursor is a UTF-8 byte offset. `\n` is ASCII, so a byte scan never splits a char.
pub fn on_first_line(text: &str, cursor: usize) -> bool {
    let bytes = text.as_bytes();
    !bytes[..cursor.min(bytes.len())].contains(&b'\n')
}

pub fn on_last_line(text: &str, cursor: usize) -> bool {
    let bytes = text.as_bytes();
    !bytes[cursor.min(bytes.len())..].contains(&b'\n')
}

#[cfg(test)]
mod tests {
    use super::*;
    use bachuc_client::{ProjectNode, WorkspaceNode};

    fn entries(items: &[&str]) -> Vec<String> {
        items.iter().map(|item| item.to_string()).collect()
    }

    #[test]
    fn push_skips_empty_and_a_repeat_of_the_newest() {
        let mut history = PromptHistory::default();
        history.push("/repo", "first");
        history.push("/repo", "   ");
        history.push("/repo", "first");
        history.push("/repo", "second");
        assert_eq!(
            history.entries("/repo"),
            entries(&["first", "second"]).as_slice()
        );
        assert!(history.entries("/other").is_empty());
    }

    #[test]
    fn push_keeps_the_newest_two_hundred() {
        let mut history = PromptHistory::default();
        for n in 0..205 {
            history.push("/repo", &format!("p{n}"));
        }
        let kept = history.entries("/repo");
        assert_eq!(kept.len(), MAX_ENTRIES);
        assert_eq!(kept.first().map(String::as_str), Some("p5"));
        assert_eq!(kept.last().map(String::as_str), Some("p204"));
    }

    #[test]
    fn a_bad_file_gives_an_empty_history_and_json_round_trips() {
        assert!(PromptHistory::parse("not json").entries("/repo").is_empty());
        let mut history = PromptHistory::default();
        history.push("/repo", "line one\nline two");
        let back = PromptHistory::parse(&history.to_json());
        assert_eq!(
            back.entries("/repo"),
            entries(&["line one\nline two"]).as_slice()
        );
    }

    #[test]
    fn up_walks_older_and_down_returns_the_stash() {
        let list = entries(&["a", "b", "c"]);
        let mut recall = Recall::default();
        assert_eq!(recall.up(&list, "draft").as_deref(), Some("c"));
        assert_eq!(recall.up(&list, "c").as_deref(), Some("b"));
        assert_eq!(recall.up(&list, "b").as_deref(), Some("a"));
        assert_eq!(recall.up(&list, "a"), None);
        assert_eq!(recall.down(&list).as_deref(), Some("b"));
        assert_eq!(recall.down(&list).as_deref(), Some("c"));
        assert_eq!(recall.down(&list).as_deref(), Some("draft"));
        assert_eq!(recall.down(&list), None);
    }

    #[test]
    fn up_with_no_entries_does_nothing() {
        let mut recall = Recall::default();
        assert_eq!(recall.up(&[], "draft"), None);
        assert_eq!(recall.down(&[]), None);
    }

    #[test]
    fn reset_starts_a_new_walk() {
        let list = entries(&["a", "b"]);
        let mut recall = Recall::default();
        recall.up(&list, "draft");
        recall.reset();
        assert_eq!(recall.up(&list, "new draft").as_deref(), Some("b"));
        assert_eq!(recall.down(&list).as_deref(), Some("new draft"));
    }

    #[test]
    fn line_checks_use_the_cursor_offset() {
        let text = "one\ntwo\nthree";
        assert!(on_first_line(text, 2));
        assert!(!on_first_line(text, 5));
        assert!(on_last_line(text, text.len()));
        assert!(!on_last_line(text, 5));
        assert!(on_first_line("", 0) && on_last_line("", 0));
        assert!(on_first_line("é\nx", 2));
        assert!(!on_last_line("é\nx", 2));
    }

    #[test]
    fn a_worktree_cwd_keys_to_its_project_root() {
        let projects = vec![ProjectNode {
            id: "p1".into(),
            name: "bachuc".into(),
            root: "/Users/hiep/Projects/bachuc".into(),
            workspaces: vec![WorkspaceNode {
                id: "w1".into(),
                cwd: "/Users/hiep/.bachuc/worktrees/bachuc/feat-x".into(),
                ..Default::default()
            }],
        }];
        assert_eq!(
            history_key("/Users/hiep/.bachuc/worktrees/bachuc/feat-x", &projects),
            "/Users/hiep/Projects/bachuc"
        );
        assert_eq!(
            history_key("/Users/hiep/Projects/bachuc", &projects),
            "/Users/hiep/Projects/bachuc"
        );
        assert_eq!(
            history_key("/Users/hiep/Projects/bachuc/", &projects),
            "/Users/hiep/Projects/bachuc"
        );
        assert_eq!(history_key("/tmp/loose", &projects), "/tmp/loose");
    }
}
