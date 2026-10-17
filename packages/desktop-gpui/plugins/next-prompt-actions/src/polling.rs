//! When the panel reads `prompts.inspect`, and when an agent is busy (`shared/polling.ts`).

use bachuc_client::AttentionReason;
use std::time::Duration;

pub(crate) const ACTIVE_POLL: Duration = Duration::from_millis(2500);
pub(crate) const JEV_REVIEWING: &str = "Jev reviewing...";

/// Only a Jev review changes server state without agent activity; everything else reads again
/// on that activity.
pub(crate) fn inspection_interval(note: Option<&str>) -> Option<Duration> {
    (note == Some(JEV_REVIEWING)).then_some(ACTIVE_POLL)
}

/// A closed runtime is at rest: Send resumes it.
pub(crate) fn agent_busy(status: Option<&str>, attention: Option<AttentionReason>) -> bool {
    let Some(status) = status else {
        return false;
    };
    !matches!(status, "idle" | "closed") || attention == Some(AttentionReason::Permission)
}

/// The agent activity that keys an inspect read: the status, and the last activity only while
/// the agent is not running, because streaming bumps it constantly.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Activity {
    pub status: String,
    pub last_activity_at: Option<String>,
}

pub(crate) fn inspection_activity(status: &str, last_activity_at: Option<&str>) -> Activity {
    Activity {
        status: status.to_string(),
        last_activity_at: last_activity_at
            .filter(|_| status != "running")
            .map(str::to_string),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ACTIVE_POLL, Activity, JEV_REVIEWING, agent_busy, inspection_activity, inspection_interval,
    };
    use bachuc_client::AttentionReason;
    use std::time::Duration;

    #[test]
    fn only_a_running_jev_review_polls() {
        assert_eq!(inspection_interval(Some("")), None);
        assert_eq!(inspection_interval(None), None);
        assert_eq!(inspection_interval(Some(JEV_REVIEWING)), Some(ACTIVE_POLL));
        assert_eq!(ACTIVE_POLL, Duration::from_millis(2500));
    }

    #[test]
    fn an_agent_is_busy_while_it_is_not_at_rest_or_waits_for_a_permission() {
        assert!(!agent_busy(Some("idle"), None));
        assert!(!agent_busy(Some("idle"), Some(AttentionReason::Finished)));
        assert!(agent_busy(Some("idle"), Some(AttentionReason::Permission)));
        assert!(agent_busy(Some("running"), None));
        assert!(agent_busy(Some("initializing"), None));
        // Send resumes a closed thread, so a closed runtime is at rest.
        assert!(!agent_busy(Some("closed"), None));
        assert!(!agent_busy(None, None));
    }

    #[test]
    fn activity_tracks_status_and_the_last_activity_only_while_the_agent_is_not_running() {
        let at = "2026-10-09T06:00:00.000Z";
        assert_eq!(
            inspection_activity("idle", Some(at)),
            Activity {
                status: "idle".into(),
                last_activity_at: Some(at.into()),
            }
        );
        // Streaming bumps the last activity constantly; the turn end refreshes the snapshot.
        assert_eq!(
            inspection_activity("running", Some(at)),
            Activity {
                status: "running".into(),
                last_activity_at: None,
            }
        );
    }
}
