//! The card and rail text of `client/page.tsx` (66-104, 358-381, 562-580, 1234-1242).

use super::run::{Run, RunStatus};
use super::{ConnectionState, SEPARATOR};

const SECOND: i64 = 1_000;

/// `Date.parse` of an ISO time in milliseconds; `None` when it does not parse.
pub(crate) fn time_value(value: Option<&str>) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(value.filter(|value| !value.is_empty())?)
        .ok()
        .map(|time| time.timestamp_millis())
}

/// Milliseconds now, as `Date.now()`.
pub(crate) fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

pub(crate) fn duration_label(milliseconds: Option<i64>) -> Option<String> {
    let milliseconds = milliseconds.filter(|value| *value >= 0)?;
    let seconds = milliseconds / SECOND;
    if seconds < 60 {
        return Some(format!("{seconds}s"));
    }
    let minutes = seconds / 60;
    if minutes < 60 {
        return Some(format!("{minutes}m {}s", seconds % 60));
    }
    let hours = minutes / 60;
    if hours < 24 {
        return Some(format!("{hours}h {}m", minutes % 60));
    }
    Some(format!("{}d {}h", hours / 24, hours % 24))
}

pub(crate) fn relative_label(value: Option<&str>, now: i64) -> String {
    let Some(time) = time_value(value) else {
        return "Finish time unavailable".into();
    };
    let elapsed = (now - time).max(0);
    if elapsed < 10 * SECOND {
        return "Just now".into();
    }
    format!(
        "{} ago",
        duration_label(Some(elapsed)).unwrap_or_else(|| "0s".into())
    )
}

pub(crate) fn status_label(status: RunStatus) -> &'static str {
    match status {
        RunStatus::Running => "Running",
        RunStatus::Completed => "Completed",
        RunStatus::Failed => "Failed",
        RunStatus::Cancelled => "Cancelled",
        RunStatus::Unknown => "Outcome unknown",
    }
}

/// The card's status word: attention wins over the status.
pub(crate) fn card_status(run: &Run) -> &'static str {
    if run.needs_input {
        "Needs input"
    } else {
        status_label(run.status)
    }
}

pub(crate) fn run_duration(run: &Run, now: i64) -> Option<String> {
    if run.status == RunStatus::Unknown {
        return None;
    }
    let started = time_value(run.started_at.as_deref())?;
    let ended = if run.running() {
        now
    } else {
        time_value(run.ended_at.as_deref())?
    };
    duration_label(Some(ended - started))
}

/// The time text of a top-level card: elapsed while running, else when it finished.
pub(crate) fn card_timing(run: &Run, now: i64) -> String {
    if run.running() {
        return run_duration(run, now).unwrap_or_else(|| "Timing unavailable".into());
    }
    let relative = relative_label(run.ended_at.as_deref(), now);
    if run.status != RunStatus::Unknown {
        relative
    } else if relative == "Finish time unavailable" {
        "Observation time unavailable".into()
    } else {
        format!("Observed {}", relative.to_lowercase())
    }
}

/// The meta line after the status dot of a top-level card.
pub(crate) fn card_meta(run: &Run, now: i64) -> (Option<&'static str>, String) {
    let timing = card_timing(run, now);
    if run.running() {
        return (None, timing);
    }
    let ran = run_duration(run, now)
        .map(|duration| format!("{SEPARATOR}ran {duration}"))
        .unwrap_or_default();
    (Some(card_status(run)), format!("{SEPARATOR}{timing}{ran}"))
}

/// Model and effort when known; the provider stands in until the model is read.
pub(crate) fn model_label(run: &Run) -> String {
    [
        run.model.as_deref().unwrap_or(&run.provider),
        run.effort.as_deref().unwrap_or(""),
    ]
    .into_iter()
    .filter(|part| !part.is_empty())
    .collect::<Vec<_>>()
    .join(SEPARATOR)
}

/// The accessible name of a card: `title, label[, seen|, new]` (`page.tsx:580`).
pub(crate) fn card_label(run: &Run, seen: bool, subagent: bool) -> String {
    let read = if run.running() || subagent {
        ""
    } else if seen {
        ", seen"
    } else if run.unread == Some(true) {
        ", new"
    } else {
        ""
    };
    format!("{}, {}{read}", run.title, card_status(run))
}

/// The hint of a card's open action (`page.tsx:594`).
pub(crate) fn open_hint(run: &Run, now: i64) -> String {
    let duration = run_duration(run, now)
        .map(|duration| format!(", duration {duration}"))
        .unwrap_or_default();
    format!(
        "{}, {}, {}, {}{duration}",
        run.project,
        model_label(run),
        card_status(run),
        card_timing(run, now)
    )
}

/// Remove names the subagents it also hides.
pub(crate) fn remove_label(removing: bool, count: usize) -> String {
    if removing {
        "Removing…".into()
    } else if count > 1 {
        format!("Remove all{SEPARATOR}{count}")
    } else {
        "Remove".into()
    }
}

pub(crate) fn remove_name(title: &str, count: usize) -> String {
    let others = count.saturating_sub(1);
    let hint = match others {
        0 => String::new(),
        1 => " and 1 subagent".into(),
        _ => format!(" and {others} subagents"),
    };
    format!("Remove {title}{hint} from Board")
}

pub(crate) fn subagent_count(count: usize) -> String {
    let subagents = count.saturating_sub(1);
    let noun = if count == 2 { "subagent" } else { "subagents" };
    format!("{subagents} {noun}")
}

/// The member status of a collapsed cluster.
pub(crate) fn summary_label(running: usize, needs_input: usize) -> Option<String> {
    if needs_input > 0 {
        Some(format!("{needs_input} needs input"))
    } else if running > 0 {
        Some(format!("{running} running"))
    } else {
        None
    }
}

/// Hosts usually report an mDNS name; the lane shows it without the suffix.
pub(crate) fn host_name(label: &str) -> &str {
    let cut = label.len().saturating_sub(".local".len());
    match label.get(cut..) {
        Some(suffix) if suffix.eq_ignore_ascii_case(".local") => &label[..cut],
        _ => label,
    }
}

/// A laptop glyph for names that say so; else the generic machine glyph.
pub(crate) fn is_laptop(label: &str) -> bool {
    let label = label.to_lowercase();
    ["macbook", "laptop", "notebook"]
        .iter()
        .any(|word| label.contains(word))
}

pub(crate) fn connection_label(state: ConnectionState) -> &'static str {
    match state {
        ConnectionState::Error => "Connection issue",
        ConnectionState::Offline => "Offline",
        ConnectionState::Stale => "Stale",
        ConnectionState::Connecting => "Connecting",
        ConnectionState::Live => "Live",
    }
}

/// `page.tsx:1512-1519`.
pub(crate) fn snapshot_warning(state: ConnectionState) -> Option<&'static str> {
    match state {
        ConnectionState::Error => Some("Could not refresh. Showing the last successful snapshot."),
        ConnectionState::Offline => Some("Host is offline. Showing the last successful snapshot."),
        ConnectionState::Stale => {
            Some("Updates are delayed. Showing a snapshot older than 10 seconds.")
        }
        ConnectionState::Connecting | ConnectionState::Live => None,
    }
}

/// The notice when there is no snapshot to show.
pub(crate) fn unavailable_notice(state: ConnectionState) -> &'static str {
    if state == ConnectionState::Offline {
        "Board is offline. Reconnect to the host, then retry."
    } else {
        "Board is unavailable. Check the host connection, then retry."
    }
}

pub(crate) fn column_name(title: &str, count: usize, unread: usize) -> String {
    let new = if unread > 0 {
        format!(", {unread} new")
    } else {
        String::new()
    };
    format!("{title}, {count} conversations{new}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::run::tests::card;

    const NOW: i64 = 1_790_000_000_000;

    fn iso(ms: i64) -> String {
        chrono::DateTime::from_timestamp_millis(ms)
            .unwrap()
            .to_rfc3339()
    }

    #[test]
    fn durations_step_from_seconds_to_days() {
        assert_eq!(duration_label(None), None);
        assert_eq!(duration_label(Some(-1)), None);
        assert_eq!(duration_label(Some(59_999)).unwrap(), "59s");
        assert_eq!(duration_label(Some(61_000)).unwrap(), "1m 1s");
        assert_eq!(duration_label(Some(3_660_000)).unwrap(), "1h 1m");
        assert_eq!(duration_label(Some(90_000_000)).unwrap(), "1d 1h");
    }

    #[test]
    fn relative_times_and_card_timing() {
        assert_eq!(relative_label(None, NOW), "Finish time unavailable");
        assert_eq!(relative_label(Some(&iso(NOW - 9_000)), NOW), "Just now");
        assert_eq!(relative_label(Some(&iso(NOW - 75_000)), NOW), "1m 15s ago");
        let mut run = card("r", "p", false);
        run.started_at = Some(iso(NOW - 5_000));
        assert_eq!(card_timing(&run, NOW), "5s");
        assert_eq!(card_meta(&run, NOW), (None, "5s".into()));
        run.status = RunStatus::Completed;
        run.ended_at = Some(iso(NOW - 2_000));
        assert_eq!(
            card_meta(&run, NOW),
            (Some("Completed"), " · Just now · ran 3s".into())
        );
        run.status = RunStatus::Unknown;
        assert_eq!(card_timing(&run, NOW), "Observed just now");
        run.ended_at = None;
        assert_eq!(card_timing(&run, NOW), "Observation time unavailable");
    }

    #[test]
    fn the_card_label_names_title_status_and_read_state() {
        let mut run = card("r", "p", false);
        run.title = "Fix it".into();
        assert_eq!(card_label(&run, false, false), "Fix it, Running");
        run.status = RunStatus::Completed;
        run.unread = Some(true);
        assert_eq!(card_label(&run, false, false), "Fix it, Completed, new");
        assert_eq!(card_label(&run, true, false), "Fix it, Completed, seen");
        assert_eq!(card_label(&run, true, true), "Fix it, Completed");
        run.needs_input = true;
        assert_eq!(card_label(&run, false, true), "Fix it, Needs input");
    }

    #[test]
    fn model_falls_back_to_the_provider_and_joins_effort() {
        let mut run = card("r", "p", false);
        assert_eq!(model_label(&run), "codex");
        run.model = Some("Opus 5.5".into());
        run.effort = Some("High".into());
        assert_eq!(model_label(&run), "Opus 5.5 · High");
    }

    #[test]
    fn remove_announces_the_cascade() {
        assert_eq!(remove_label(false, 3), "Remove all · 3");
        assert_eq!(remove_label(false, 1), "Remove");
        assert_eq!(remove_label(true, 3), "Removing…");
        assert_eq!(
            remove_name("Parent", 3),
            "Remove Parent and 2 subagents from Board"
        );
        assert_eq!(
            remove_name("Child", 2),
            "Remove Child and 1 subagent from Board"
        );
        assert_eq!(remove_name("Leaf", 1), "Remove Leaf from Board");
        assert_eq!(subagent_count(2), "1 subagent");
        assert_eq!(subagent_count(4), "3 subagents");
        assert_eq!(summary_label(2, 1).unwrap(), "1 needs input");
        assert_eq!(summary_label(2, 0).unwrap(), "2 running");
        assert_eq!(summary_label(0, 0), None);
    }

    #[test]
    fn the_rail_names_the_host_and_its_device() {
        assert_eq!(host_name("Hieps-MacBook-Pro.local"), "Hieps-MacBook-Pro");
        assert_eq!(host_name("studio.LOCAL"), "studio");
        assert_eq!(host_name("live"), "live");
        assert!(is_laptop("Hieps-MacBook-Pro"));
        assert!(!is_laptop("macmini"));
    }

    #[test]
    fn warnings_follow_the_connection_state() {
        assert_eq!(snapshot_warning(ConnectionState::Live), None);
        assert!(
            snapshot_warning(ConnectionState::Stale)
                .unwrap()
                .contains("older than 10 seconds")
        );
        assert_eq!(connection_label(ConnectionState::Error), "Connection issue");
        assert_eq!(
            column_name("Just finished", 4, 2),
            "Just finished, 4 conversations, 2 new"
        );
    }
}
