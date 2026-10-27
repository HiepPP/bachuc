//! The unread thread queue and the back history behind Mod+J and Mod+Shift+J (ADR-0070). Both
//! are pure; the shell fills them from `Client`.

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
}

/// The agent ids to visit, oldest first: flagged, finished or failed, not a subagent, and not
/// the viewed agent. An entry with no timestamp sorts last; ties sort by agent id.
pub fn unread_queue(entries: &[QueueEntry], viewed: Option<&str>) -> Vec<String> {
    let mut queued: Vec<&QueueEntry> = entries
        .iter()
        .filter(|entry| {
            entry.requires_attention
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
}
