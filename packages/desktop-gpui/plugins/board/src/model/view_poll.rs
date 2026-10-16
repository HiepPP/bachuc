//! The poll of a view that loads its own data (Timeline and Recaps): one request per interval,
//! only while the view shows and the host is online, and one request in flight at most.

use super::poll::poll_due;

const MINUTE_MS: i64 = 60_000;

#[derive(Debug, Default)]
pub(crate) struct ViewPoll {
    in_flight: bool,
    last_sent: Option<i64>,
}

impl ViewPoll {
    pub(crate) fn due(&self, now: i64, interval: i64, shown: bool, online: bool) -> bool {
        shown && online && poll_due(now, self.last_sent, self.in_flight, interval)
    }

    pub(crate) fn begin(&mut self, now: i64) {
        self.in_flight = true;
        self.last_sent = Some(now);
    }

    pub(crate) fn finish(&mut self) {
        self.in_flight = false;
    }

    pub(crate) fn in_flight(&self) -> bool {
        self.in_flight
    }

    /// Milliseconds until the next poll is due; 0 when it is due now.
    pub(crate) fn wait(&self, now: i64, interval: i64) -> i64 {
        self.last_sent
            .map_or(0, |sent| (sent + interval - now).max(0))
    }
}

/// The minute a time falls in, as `Math.floor(now / 60_000)`.
pub(crate) fn minute_of(now: i64) -> i64 {
    now.div_euclid(MINUTE_MS)
}

/// Milliseconds until the next minute starts, so the now line moves once a minute.
pub(crate) fn until_next_minute(now: i64) -> i64 {
    MINUTE_MS - now.rem_euclid(MINUTE_MS)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::recaps::RECAPS_POLL_MS;
    use crate::model::timeline::TIMELINE_POLL_MS;

    const NOW: i64 = 1_000_000;

    #[test]
    fn a_view_polls_only_while_it_shows_and_the_host_is_online() {
        let poll = ViewPoll::default();
        assert!(poll.due(NOW, TIMELINE_POLL_MS, true, true));
        assert!(!poll.due(NOW, TIMELINE_POLL_MS, false, true));
        assert!(!poll.due(NOW, TIMELINE_POLL_MS, true, false));
    }

    #[test]
    fn one_request_in_flight_and_one_per_interval() {
        let mut poll = ViewPoll::default();
        poll.begin(NOW);
        assert!(poll.in_flight());
        // Never a second request while one is in flight, even when the interval passed.
        assert!(!poll.due(NOW + TIMELINE_POLL_MS * 3, TIMELINE_POLL_MS, true, true));
        poll.finish();
        assert!(!poll.due(NOW + 14_999, TIMELINE_POLL_MS, true, true));
        assert_eq!(poll.wait(NOW + 14_999, TIMELINE_POLL_MS), 1);
        assert!(poll.due(NOW + 15_000, TIMELINE_POLL_MS, true, true));
        assert_eq!(poll.wait(NOW + 15_000, TIMELINE_POLL_MS), 0);
        // Recaps wait 30 s.
        assert!(!poll.due(NOW + 29_999, RECAPS_POLL_MS, true, true));
        assert!(poll.due(NOW + 30_000, RECAPS_POLL_MS, true, true));
    }

    #[test]
    fn the_minute_clock_moves_on_the_minute() {
        assert_eq!(minute_of(119_999), 1);
        assert_eq!(minute_of(120_000), 2);
        assert_eq!(until_next_minute(120_000), 60_000);
        assert_eq!(until_next_minute(179_999), 1);
    }
}
