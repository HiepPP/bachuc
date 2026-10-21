//! Client-only "turn pending" marks: an agent that got a prompt the daemon has not confirmed.

use std::collections::HashMap;

/// A mark hides once it is this old, in milliseconds.
pub const PENDING_TURN_TTL_MS: i64 = 15_000;

/// Emitted by `Client` when a pending turn mark is added or cleared.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PendingTurnsChanged;

/// Mark times in UNIX milliseconds, by agent id.
#[derive(Default)]
pub(crate) struct PendingTurns {
    marks: HashMap<String, i64>,
}

impl PendingTurns {
    /// Sets the mark time for `agent_id` and drops expired marks.
    pub(crate) fn mark(&mut self, agent_id: &str, now_ms: i64) {
        self.marks
            .retain(|_, marked| now_ms.saturating_sub(*marked) < PENDING_TURN_TTL_MS);
        self.marks.insert(agent_id.to_owned(), now_ms);
    }

    /// True when a mark was removed.
    pub(crate) fn clear(&mut self, agent_id: &str) -> bool {
        self.marks.remove(agent_id).is_some()
    }

    /// The mark time, or `None` when there is no mark or it reached the limit.
    pub(crate) fn get(&self, agent_id: &str, now_ms: i64) -> Option<i64> {
        let marked = *self.marks.get(agent_id)?;
        (now_ms.saturating_sub(marked) < PENDING_TURN_TTL_MS).then_some(marked)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_000_000;

    #[test]
    fn a_mark_shows_until_the_limit() {
        let mut marks = PendingTurns::default();
        marks.mark("a", NOW);
        assert_eq!(marks.get("a", NOW), Some(NOW));
        assert_eq!(marks.get("a", NOW + PENDING_TURN_TTL_MS - 1), Some(NOW));
        assert_eq!(marks.get("a", NOW + PENDING_TURN_TTL_MS), None);
        assert_eq!(marks.get("b", NOW), None);
    }

    #[test]
    fn clear_reports_whether_a_mark_was_removed() {
        let mut marks = PendingTurns::default();
        assert!(!marks.clear("a"));
        marks.mark("a", NOW);
        assert!(marks.clear("a"));
        assert_eq!(marks.get("a", NOW), None);
    }

    #[test]
    fn a_new_mark_replaces_the_old_time() {
        let mut marks = PendingTurns::default();
        marks.mark("a", NOW);
        marks.mark("a", NOW + 5_000);
        assert_eq!(
            marks.get("a", NOW + PENDING_TURN_TTL_MS + 1),
            Some(NOW + 5_000)
        );
    }
}
