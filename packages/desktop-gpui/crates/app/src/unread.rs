//! The unread thread queue and the back history behind Mod+J and Mod+Shift+J (ADR-0070,
//! ADR-0072). Both are pure; the shell fills them from `Client` and the Board snapshot.

use std::collections::{HashMap, HashSet};

use bachuc_client::AttentionReason;

/// One agent as the queue sees it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueueEntry {
    pub agent_id: String,
    pub requires_attention: bool,
    pub reason: Option<AttentionReason>,
    pub is_subagent: bool,
    /// The daemon `attentionTimestamp`, else the agent's `updatedAt`. ISO strings in one
    /// format, so they compare as text.
    pub attention_at: Option<String>,
    /// The agent is a Board run with a status other than `running`.
    pub on_board: bool,
    /// The Board run's `endedAt`, which orders the read cards.
    pub ended_at: Option<String>,
}

/// The non-running Board runs of a `board.snapshot` answer: agent id to `endedAt`. A run that
/// does not decode is skipped.
pub fn board_finished_runs(snapshot: &serde_json::Value) -> HashMap<String, Option<String>> {
    let Some(runs) = snapshot.get("runs").and_then(|runs| runs.as_array()) else {
        return HashMap::new();
    };
    runs.iter()
        .filter_map(|run| {
            let id = run.get("agentId")?.as_str()?;
            let status = run.get("status")?.as_str()?;
            let ended_at = run.get("endedAt").and_then(|at| at.as_str());
            (status != "running").then(|| (id.to_string(), ended_at.map(String::from)))
        })
        .collect()
}

/// The agent ids of the Board runs whose status is not `running`.
pub fn board_finished_ids(snapshot: &serde_json::Value) -> HashSet<String> {
    board_finished_runs(snapshot).into_keys().collect()
}

/// The agent ids to visit, oldest first: on the Board, flagged, finished or failed, not a
/// subagent, and not the viewed agent. An entry with no timestamp sorts last; ties sort by
/// agent id.
pub fn unread_queue(entries: &[QueueEntry], viewed: Option<&str>) -> Vec<String> {
    let mut queued: Vec<&QueueEntry> = entries
        .iter()
        .filter(|entry| {
            entry.on_board
                && entry.requires_attention
                && matches!(
                    entry.reason,
                    Some(AttentionReason::Finished | AttentionReason::Error)
                )
                && !entry.is_subagent
                && Some(entry.agent_id.as_str()) != viewed
        })
        .collect();
    queued.sort_by(|a, b| {
        // `false < true`, so an entry with a timestamp comes before one without.
        (a.attention_at.is_none(), &a.attention_at, &a.agent_id).cmp(&(
            b.attention_at.is_none(),
            &b.attention_at,
            &b.agent_id,
        ))
    });
    queued.into_iter().map(|e| e.agent_id.clone()).collect()
}

/// The Mod+J cycle (ADR-0073): every non-running Board card that is not a subagent and has no
/// permission request. Unread cards come first, oldest first. Read cards follow, latest
/// `endedAt` first, a card with none last; ties sort by agent id. The rows start after the
/// viewed card and wrap; the viewed card is not a row.
pub fn switch_cycle(entries: &[QueueEntry], viewed: Option<&str>) -> Vec<String> {
    let is_unread = |entry: &QueueEntry| {
        entry.requires_attention
            && matches!(
                entry.reason,
                Some(AttentionReason::Finished | AttentionReason::Error)
            )
    };
    let (mut unread, mut read): (Vec<&QueueEntry>, Vec<&QueueEntry>) = entries
        .iter()
        .filter(|entry| {
            entry.on_board
                && !entry.is_subagent
                && entry.reason != Some(AttentionReason::Permission)
        })
        .partition(|entry| is_unread(entry));
    unread.sort_by(|a, b| {
        (a.attention_at.is_none(), &a.attention_at, &a.agent_id).cmp(&(
            b.attention_at.is_none(),
            &b.attention_at,
            &b.agent_id,
        ))
    });
    read.sort_by(|a, b| {
        (a.ended_at.is_none(), b.ended_at.as_ref(), &a.agent_id).cmp(&(
            b.ended_at.is_none(),
            a.ended_at.as_ref(),
            &b.agent_id,
        ))
    });
    // Unread cards always lead. Only the read cards rotate, so a just-read card does not push
    // the waiting unread cards to the end.
    let mut read: Vec<String> = read.into_iter().map(|e| e.agent_id.clone()).collect();
    if let Some(at) = read.iter().position(|id| Some(id.as_str()) == viewed) {
        read.rotate_left(at + 1);
        read.pop();
    }
    unread
        .into_iter()
        .map(|entry| entry.agent_id.clone())
        .filter(|id| Some(id.as_str()) != viewed)
        .chain(read)
        .collect()
}

/// The last two threads the user viewed. A page visit is not a view.
#[derive(Clone, Debug, Default)]
pub struct ViewHistory {
    last: Option<String>,
    before: Option<String>,
}

impl ViewHistory {
    /// Records the thread on screen; `None` is a page and changes nothing.
    pub fn visit(&mut self, viewed: Option<&str>) {
        let Some(id) = viewed else { return };
        if self.last.as_deref() == Some(id) {
            return;
        }
        self.before = self.last.replace(id.to_string());
    }

    /// The most recent viewed thread that is not `current`.
    pub fn back(&self, current: Option<&str>) -> Option<String> {
        if self.last.as_deref() != current {
            self.last.clone()
        } else {
            self.before.clone()
        }
    }
}

/// Milliseconds since the epoch of `YYYY-MM-DDTHH:MM:SS[.fff]Z`, the form the daemon writes.
/// None for any other form, so the row shows no age.
pub fn iso_to_ms(text: &str) -> Option<i64> {
    let (date, time) = text.strip_suffix('Z')?.split_once('T')?;
    let mut date = date.split('-').map(|part| part.parse::<i64>().ok());
    let (year, month, day) = (date.next()??, date.next()??, date.next()??);
    let (clock, fraction) = time.split_once('.').unwrap_or((time, ""));
    let mut clock = clock.split(':').map(|part| part.parse::<i64>().ok());
    let (hour, minute, second) = (clock.next()??, clock.next()??, clock.next()??);
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) || hour > 23 || minute > 59 {
        return None;
    }
    let millis: i64 = format!("{fraction:0<3}").get(..3)?.parse().ok()?;
    // Days from civil, Howard Hinnant's algorithm.
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let year_of_era = y - era * 400;
    let day_of_year = (153 * ((month + 9) % 12) + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    let days = era * 146_097 + day_of_era - 719_468;
    Some(((days * 24 + hour) * 60 + minute) * 60_000 + second * 1000 + millis)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bachuc_client::AttentionReason;

    fn entry(id: &str, reason: Option<AttentionReason>, at: Option<&str>) -> QueueEntry {
        QueueEntry {
            agent_id: id.into(),
            requires_attention: true,
            reason,
            is_subagent: false,
            attention_at: at.map(String::from),
            on_board: true,
            ended_at: None,
        }
    }

    #[test]
    fn the_oldest_finished_or_error_thread_comes_first() {
        let entries = [
            entry(
                "new",
                Some(AttentionReason::Finished),
                Some("2026-10-27T05:00:00.000Z"),
            ),
            entry(
                "old",
                Some(AttentionReason::Error),
                Some("2026-10-27T04:00:00.000Z"),
            ),
            entry("none", Some(AttentionReason::Finished), None),
        ];
        assert_eq!(unread_queue(&entries, None), ["old", "new", "none"]);
    }

    #[test]
    fn permission_subagents_read_threads_and_the_viewed_thread_stay_out() {
        let mut sub = entry("sub", Some(AttentionReason::Finished), Some("1"));
        sub.is_subagent = true;
        let mut read = entry("read", Some(AttentionReason::Finished), Some("2"));
        read.requires_attention = false;
        let entries = [
            entry("perm", Some(AttentionReason::Permission), Some("0")),
            sub,
            read,
            entry("viewed", Some(AttentionReason::Finished), Some("3")),
            entry("keep", Some(AttentionReason::Finished), Some("4")),
            entry("no-reason", None, Some("5")),
        ];
        assert_eq!(unread_queue(&entries, Some("viewed")), ["keep"]);
    }

    #[test]
    fn equal_timestamps_sort_by_agent_id() {
        let entries = [
            entry("b", Some(AttentionReason::Finished), Some("1")),
            entry("a", Some(AttentionReason::Finished), Some("1")),
        ];
        assert_eq!(unread_queue(&entries, None), ["a", "b"]);
    }

    #[test]
    fn no_entries_give_an_empty_queue() {
        assert!(unread_queue(&[], None).is_empty());
    }

    #[test]
    fn back_toggles_between_two_threads() {
        let mut history = ViewHistory::default();
        history.visit(Some("a"));
        history.visit(Some("b"));
        assert_eq!(history.back(Some("b")).as_deref(), Some("a"));
        history.visit(Some("a"));
        assert_eq!(history.back(Some("a")).as_deref(), Some("b"));
    }

    #[test]
    fn a_page_visit_keeps_the_last_thread() {
        let mut history = ViewHistory::default();
        history.visit(Some("a"));
        history.visit(Some("b"));
        history.visit(None);
        assert_eq!(history.back(None).as_deref(), Some("b"));
    }

    #[test]
    fn no_history_gives_no_back() {
        let mut history = ViewHistory::default();
        assert_eq!(history.back(None), None);
        history.visit(Some("a"));
        assert_eq!(history.back(Some("a")), None);
    }

    #[test]
    fn board_finished_ids_keep_non_running_runs_and_skip_bad_rows() {
        let snapshot = serde_json::json!({
            "observingSince": "2026-10-27T00:00:00.000Z",
            "runs": [
                {"agentId": "done", "status": "completed"},
                {"agentId": "failed", "status": "failed"},
                {"agentId": "busy", "status": "running"},
                {"status": "completed"},
                "bad"
            ]
        });
        let ids = board_finished_ids(&snapshot);
        assert!(ids.contains("done") && ids.contains("failed"));
        assert!(!ids.contains("busy"));
        assert_eq!(ids.len(), 2);
        assert!(board_finished_ids(&serde_json::json!({})).is_empty());
    }

    #[test]
    fn an_unread_agent_off_the_board_stays_out() {
        let mut off = entry("off", Some(AttentionReason::Finished), Some("1"));
        off.on_board = false;
        let entries = [off, entry("on", Some(AttentionReason::Finished), Some("2"))];
        assert_eq!(unread_queue(&entries, None), ["on"]);
    }

    fn card(id: &str, unread: bool, at: Option<&str>, ended: Option<&str>) -> QueueEntry {
        QueueEntry {
            agent_id: id.into(),
            requires_attention: unread,
            reason: unread.then_some(AttentionReason::Finished),
            is_subagent: false,
            attention_at: at.map(String::from),
            on_board: true,
            ended_at: ended.map(String::from),
        }
    }

    #[test]
    fn the_cycle_puts_unread_oldest_first_then_read_latest_first() {
        let entries = [
            card("r-old", false, None, Some("1")),
            card("u-new", true, Some("5"), Some("5")),
            card("r-new", false, None, Some("3")),
            card("u-old", true, Some("4"), Some("4")),
            card("r-none", false, None, None),
        ];
        assert_eq!(
            switch_cycle(&entries, None),
            ["u-old", "u-new", "r-new", "r-old", "r-none"]
        );
    }

    #[test]
    fn the_cycle_starts_after_the_viewed_card_and_wraps() {
        let entries = [
            card("a", false, None, Some("3")),
            card("b", false, None, Some("2")),
            card("c", false, None, Some("1")),
        ];
        assert_eq!(switch_cycle(&entries, Some("b")), ["c", "a"]);
        assert_eq!(switch_cycle(&entries, Some("c")), ["a", "b"]);
        assert_eq!(switch_cycle(&entries, Some("other")), ["a", "b", "c"]);
    }

    #[test]
    fn the_cycle_leaves_out_permission_subagents_and_off_board_agents() {
        let mut sub = card("sub", false, None, Some("1"));
        sub.is_subagent = true;
        let mut off = card("off", false, None, Some("1"));
        off.on_board = false;
        let mut perm = card("perm", true, Some("1"), Some("1"));
        perm.reason = Some(AttentionReason::Permission);
        let entries = [sub, off, perm, card("keep", false, None, Some("1"))];
        assert_eq!(switch_cycle(&entries, None), ["keep"]);
    }

    #[test]
    fn unread_cards_lead_even_after_the_viewed_card_turns_read() {
        let entries = [
            card("u2", true, Some("2"), Some("2")),
            card("u1-now-read", false, None, Some("9")),
            card("r", false, None, Some("1")),
        ];
        assert_eq!(switch_cycle(&entries, Some("u1-now-read")), ["u2", "r"]);
    }

    #[test]
    fn only_the_viewed_card_gives_an_empty_cycle() {
        let entries = [card("a", false, None, Some("1"))];
        assert!(switch_cycle(&entries, Some("a")).is_empty());
    }

    #[test]
    fn board_finished_runs_keep_ended_at() {
        let snapshot = serde_json::json!({"runs": [
            {"agentId": "a", "status": "completed", "endedAt": "2026-10-27T01:00:00.000Z"},
            {"agentId": "b", "status": "failed"},
            {"agentId": "c", "status": "running", "endedAt": "x"}
        ]});
        let runs = board_finished_runs(&snapshot);
        assert_eq!(runs.len(), 2);
        assert_eq!(runs["a"].as_deref(), Some("2026-10-27T01:00:00.000Z"));
        assert_eq!(runs["b"], None);
    }
}
