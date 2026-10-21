//! The client-only "turn pending" overlay: a run whose agent got a prompt the daemon has not
//! confirmed shows as Running, until a `board.snapshot` answer confirms or the mark ends.

use super::run::{BoardSnapshot, Run, RunStatus};
use super::text::time_value;
use chrono::SecondsFormat;

/// A marked run shows as Running while it is not running and did not end after the mark. An end
/// at the mark time is the old turn; an end that does not parse keeps the overlay, and the mark
/// limit bounds that case.
pub(crate) fn shows_pending(run: &Run, marked_at: i64) -> bool {
    !run.running() && time_value(run.ended_at.as_deref()).is_none_or(|ended| ended <= marked_at)
}

/// The snapshot as shown: each run whose agent has a mark and `shows_pending` becomes Running,
/// with no end and a start at the mark time. It adds and removes no run.
pub(crate) fn apply_pending(
    snapshot: &BoardSnapshot,
    mark: impl Fn(&str) -> Option<i64>,
) -> BoardSnapshot {
    let mut shown = snapshot.clone();
    for run in &mut shown.runs {
        let Some(marked_at) = mark(&run.agent_id).filter(|at| shows_pending(run, *at)) else {
            continue;
        };
        let Some(started) = chrono::DateTime::from_timestamp_millis(marked_at) else {
            continue;
        };
        run.status = RunStatus::Running;
        run.ended_at = None;
        run.started_at = Some(started.to_rfc3339_opts(SecondsFormat::Millis, true));
    }
    shown
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::run::tests::card;

    fn mark() -> i64 {
        chrono::DateTime::parse_from_rfc3339("2026-10-21T00:00:00Z")
            .unwrap()
            .timestamp_millis()
    }

    fn finished(agent_id: &str, ended_at: &str) -> Run {
        let mut run = card(agent_id, "app", false);
        run.agent_id = agent_id.into();
        run.status = RunStatus::Completed;
        run.ended_at = Some(ended_at.into());
        run
    }

    fn snapshot(runs: Vec<Run>) -> BoardSnapshot {
        BoardSnapshot {
            runs,
            observing_since: "2026-10-01T00:00:00Z".into(),
        }
    }

    #[test]
    fn a_marked_finished_run_shows_as_running() {
        let input = snapshot(vec![
            finished("a", "2026-10-20T23:59:00Z"),
            finished("b", "2026-10-20T23:59:00Z"),
        ]);
        let shown = apply_pending(&input, |id| (id == "a").then(mark));
        assert_eq!(shown.runs[0].status, RunStatus::Running);
        assert_eq!(shown.runs[0].ended_at, None);
        let started = shown.runs[0].started_at.as_deref().unwrap();
        assert_eq!(
            chrono::DateTime::parse_from_rfc3339(started)
                .unwrap()
                .timestamp_millis(),
            mark()
        );
        assert_eq!(shown.runs[1], input.runs[1]);
    }

    #[test]
    fn a_failed_run_moves_too() {
        let mut run = finished("a", "2026-10-20T23:59:00Z");
        run.status = RunStatus::Failed;
        assert!(shows_pending(&run, mark()));
    }

    #[test]
    fn a_run_that_ended_after_the_mark_stays_finished() {
        let run = finished("a", "2026-10-21T00:00:05Z");
        assert!(!shows_pending(&run, mark()));
    }

    #[test]
    fn a_run_that_ended_at_the_mark_still_moves() {
        let run = finished("a", "2026-10-21T00:00:00Z");
        assert!(shows_pending(&run, mark()));
    }

    #[test]
    fn a_running_run_needs_no_overlay() {
        let mut run = finished("a", "2026-10-20T23:59:00Z");
        run.status = RunStatus::Running;
        run.ended_at = None;
        assert!(!shows_pending(&run, mark()));
    }

    #[test]
    fn an_unreadable_end_time_keeps_the_overlay() {
        assert!(shows_pending(&finished("a", "not a time"), mark()));
    }

    #[test]
    fn a_mark_without_a_run_adds_no_card() {
        let input = snapshot(vec![finished("a", "2026-10-20T23:59:00Z")]);
        let shown = apply_pending(&input, |id| (id == "missing").then(mark));
        assert_eq!(shown, input);
    }

    #[test]
    fn no_mark_returns_an_equal_snapshot() {
        let input = snapshot(vec![finished("a", "2026-10-20T23:59:00Z")]);
        assert_eq!(apply_pending(&input, |_| None), input);
    }
}
