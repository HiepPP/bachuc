//! When the store sends `board.snapshot`, and the connection state of the lane rail
//! (`client/connection.ts`). The store drives these rules with GPUI timers.

use serde_json::Value;

/// `BOARD_LIVE_WINDOW_MS`: data older than this shows as Stale.
pub(crate) const BOARD_LIVE_WINDOW_MS: i64 = 10_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ConnectionState {
    Connecting,
    Live,
    Offline,
    Stale,
    Error,
}

/// `boardConnectionState`.
pub(crate) fn connection_state(
    connected: bool,
    failed: bool,
    received_at: Option<i64>,
    now: i64,
) -> ConnectionState {
    match received_at {
        _ if !connected => ConnectionState::Offline,
        _ if failed => ConnectionState::Error,
        None => ConnectionState::Connecting,
        Some(at) if at <= 0 || now - at > BOARD_LIVE_WINDOW_MS => ConnectionState::Stale,
        Some(_) => ConnectionState::Live,
    }
}

/// A poll is due one interval after the last request started, never while one is in flight.
pub(crate) fn poll_due(now: i64, last_sent: Option<i64>, in_flight: bool, interval: i64) -> bool {
    !in_flight && last_sent.is_none_or(|sent| now - sent >= interval)
}

/// What an answer changed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Received {
    /// New data: decode it and redraw.
    Changed,
    /// The same JSON as the last answer: only the receive time moved.
    Unchanged,
    Failed,
}

/// The poll state of one store: at most one request in flight and one per interval.
#[derive(Debug, Default)]
pub(crate) struct Poller {
    connected: bool,
    in_flight: bool,
    /// A load asked for outside the cadence: after a connect or a card action.
    reload: bool,
    last_sent: Option<i64>,
    last_json: Option<Value>,
    received_at: Option<i64>,
    failed: bool,
}

impl Poller {
    /// `ClientEvent::Connected`: load now, or as soon as the request in flight ends.
    pub(crate) fn connected(&mut self) {
        self.connected = true;
        self.reload = true;
    }

    pub(crate) fn disconnected(&mut self) {
        self.connected = false;
    }

    pub(crate) fn is_connected(&self) -> bool {
        self.connected
    }

    pub(crate) fn in_flight(&self) -> bool {
        self.in_flight
    }

    /// The last answer that changed.
    pub(crate) fn last_json(&self) -> Option<&Value> {
        self.last_json.as_ref()
    }

    /// A card action changed the host state; load again without waiting for the interval.
    pub(crate) fn refresh(&mut self) {
        self.reload = true;
    }

    pub(crate) fn should_send(&self, now: i64, interval: i64) -> bool {
        self.connected
            && !self.in_flight
            && (self.reload || poll_due(now, self.last_sent, self.in_flight, interval))
    }

    pub(crate) fn begin(&mut self, now: i64) {
        self.in_flight = true;
        self.reload = false;
        self.last_sent = Some(now);
    }

    pub(crate) fn finish(&mut self, answer: anyhow::Result<Value>, now: i64) -> Received {
        self.in_flight = false;
        match answer {
            Ok(value) => {
                self.failed = false;
                self.received_at = Some(now);
                if self.last_json.as_ref() == Some(&value) {
                    Received::Unchanged
                } else {
                    self.last_json = Some(value);
                    Received::Changed
                }
            }
            Err(_) => {
                self.failed = true;
                Received::Failed
            }
        }
    }

    /// Milliseconds until the next request, or `None` when nothing will be sent until a
    /// connect, an answer, or a refresh.
    pub(crate) fn next_wake(&self, now: i64, interval: i64) -> Option<i64> {
        if !self.connected || self.in_flight {
            return None;
        }
        if self.reload {
            return Some(0);
        }
        Some(
            self.last_sent
                .map_or(0, |sent| (sent + interval - now).max(0)),
        )
    }

    pub(crate) fn connection(&self, now: i64) -> ConnectionState {
        connection_state(self.connected, self.failed, self.received_at, now)
    }

    /// When the rail turns from Live to Stale with no new answer.
    pub(crate) fn stale_at(&self) -> Option<i64> {
        self.received_at.map(|at| at + BOARD_LIVE_WINDOW_MS + 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::run::{ACTIVE_BOARD_POLL_MS, IDLE_BOARD_POLL_MS};
    use serde_json::json;

    const NOW: i64 = 100_000;

    #[test]
    fn each_connection_state() {
        assert_eq!(
            connection_state(false, false, Some(NOW), NOW),
            ConnectionState::Offline
        );
        assert_eq!(
            connection_state(false, false, None, NOW),
            ConnectionState::Offline
        );
        assert_eq!(
            connection_state(true, true, Some(NOW), NOW),
            ConnectionState::Error
        );
        assert_eq!(
            connection_state(true, false, None, NOW),
            ConnectionState::Connecting
        );
        assert_eq!(
            connection_state(true, false, Some(NOW - BOARD_LIVE_WINDOW_MS - 1), NOW),
            ConnectionState::Stale
        );
        assert_eq!(
            connection_state(true, false, Some(NOW - BOARD_LIVE_WINDOW_MS), NOW),
            ConnectionState::Live
        );
    }

    #[test]
    fn poll_due_waits_2000_ms_and_never_runs_while_in_flight() {
        let interval = ACTIVE_BOARD_POLL_MS;
        assert!(poll_due(NOW, None, false, interval));
        assert!(!poll_due(NOW, None, true, interval));
        assert!(!poll_due(NOW + 1_999, Some(NOW), false, interval));
        assert!(poll_due(NOW + 2_000, Some(NOW), false, interval));
        assert!(!poll_due(NOW + 9_000, Some(NOW), true, interval));
    }

    #[test]
    fn the_idle_cadence_is_15000_ms_with_no_running_run() {
        let mut poller = Poller::default();
        poller.connected();
        assert!(poller.should_send(NOW, IDLE_BOARD_POLL_MS));
        poller.begin(NOW);
        assert_eq!(poller.next_wake(NOW, IDLE_BOARD_POLL_MS), None);
        assert!(!poller.should_send(NOW + 20_000, IDLE_BOARD_POLL_MS));
        poller.finish(Ok(json!({"runs": []})), NOW + 100);
        assert_eq!(
            poller.next_wake(NOW + 100, IDLE_BOARD_POLL_MS),
            Some(14_900)
        );
        assert!(!poller.should_send(NOW + 14_999, IDLE_BOARD_POLL_MS));
        assert!(poller.should_send(NOW + 15_000, IDLE_BOARD_POLL_MS));
    }

    #[test]
    fn connected_reloads_at_once_and_after_a_reconnect() {
        let mut poller = Poller::default();
        assert!(!poller.should_send(NOW, ACTIVE_BOARD_POLL_MS));
        assert_eq!(poller.next_wake(NOW, ACTIVE_BOARD_POLL_MS), None);
        poller.connected();
        poller.begin(NOW);
        poller.finish(Ok(json!(1)), NOW);
        poller.disconnected();
        assert!(!poller.should_send(NOW + 60_000, ACTIVE_BOARD_POLL_MS));
        assert_eq!(poller.connection(NOW), ConnectionState::Offline);
        // The reconnect loads at once, inside the interval.
        poller.connected();
        assert_eq!(poller.next_wake(NOW + 10, ACTIVE_BOARD_POLL_MS), Some(0));
        assert!(poller.should_send(NOW + 10, ACTIVE_BOARD_POLL_MS));
        // A connect during a request waits for it: never two at once.
        poller.begin(NOW + 10);
        poller.connected();
        assert!(!poller.should_send(NOW + 20, ACTIVE_BOARD_POLL_MS));
        poller.finish(Err(anyhow::anyhow!("disconnected")), NOW + 30);
        assert!(poller.should_send(NOW + 30, ACTIVE_BOARD_POLL_MS));
    }

    #[test]
    fn an_equal_snapshot_changes_nothing_but_the_receive_time() {
        let mut poller = Poller::default();
        poller.connected();
        poller.begin(NOW);
        assert_eq!(poller.finish(Ok(json!({"a": 1})), NOW), Received::Changed);
        poller.begin(NOW + 2_000);
        assert_eq!(
            poller.finish(Ok(json!({"a": 1})), NOW + 2_100),
            Received::Unchanged
        );
        assert_eq!(
            poller.stale_at(),
            Some(NOW + 2_100 + BOARD_LIVE_WINDOW_MS + 1)
        );
        poller.begin(NOW + 4_000);
        assert_eq!(
            poller.finish(Err(anyhow::anyhow!("x")), NOW + 4_100),
            Received::Failed
        );
        assert_eq!(poller.connection(NOW + 4_100), ConnectionState::Error);
        poller.begin(NOW + 6_000);
        assert_eq!(
            poller.finish(Ok(json!({"a": 2})), NOW + 6_100),
            Received::Changed
        );
        assert_eq!(poller.connection(NOW + 6_100), ConnectionState::Live);
    }
}
