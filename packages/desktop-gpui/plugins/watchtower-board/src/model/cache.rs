//! The load rules of the store, without GPUI: when a board or overview loads again, and which
//! answer counts. Electron reads them through react-query with `staleTime: 30_000`.

use super::Loaded;
use std::collections::HashMap;
use std::hash::Hash;

/// The pill reuses a cached board younger than this.
pub(crate) const STALE_MS: i64 = 30_000;

/// Whether a value loaded at `loaded_at_ms` is still fresh at `now_ms`.
pub(crate) fn cache_fresh(loaded_at_ms: i64, now_ms: i64) -> bool {
    now_ms - loaded_at_ms < STALE_MS
}

/// Why a caller wants a value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Want {
    /// Load only when nothing is cached: the page and the panel load once.
    Missing,
    /// Load when nothing is cached or the value is stale: the active workspace for the pill.
    Fresh,
    /// Refresh and reconnects: load again, and drop the answer in flight.
    Force,
}

struct Entry<T> {
    value: Loaded<T>,
    /// When the last answer arrived.
    loaded_at: Option<i64>,
    /// The generation of the request in flight.
    in_flight: Option<u64>,
}

/// Cached values by key, with at most one request in flight per key.
pub(crate) struct Cache<K, T> {
    entries: HashMap<K, Entry<T>>,
    generation: u64,
}

impl<K: Eq + Hash + Clone, T> Default for Cache<K, T> {
    fn default() -> Self {
        Self {
            entries: HashMap::new(),
            generation: 0,
        }
    }
}

impl<K: Eq + Hash + Clone, T: Clone> Cache<K, T> {
    /// The cached value; `Loading` until the first answer.
    pub(crate) fn get(&self, key: &K) -> Loaded<T> {
        self.entries
            .get(key)
            .map_or(Loaded::Loading, |entry| entry.value.clone())
    }

    pub(crate) fn is_fetching(&self, key: &K) -> bool {
        self.entries
            .get(key)
            .is_some_and(|entry| entry.in_flight.is_some())
    }

    /// Starts a request when `want` needs one, and returns its generation. A value keeps showing
    /// while it loads again.
    pub(crate) fn begin(&mut self, key: &K, want: Want, now_ms: i64) -> Option<u64> {
        if let Some(entry) = self.entries.get(key) {
            let needed = match want {
                Want::Missing => false,
                Want::Fresh => {
                    entry.in_flight.is_none()
                        && !entry.loaded_at.is_some_and(|at| cache_fresh(at, now_ms))
                }
                Want::Force => true,
            };
            // A request in flight already answers `Missing` and `Fresh`.
            if !needed || (want != Want::Force && entry.in_flight.is_some()) {
                return None;
            }
        }
        self.generation += 1;
        let generation = self.generation;
        let entry = self.entries.entry(key.clone()).or_insert(Entry {
            value: Loaded::Loading,
            loaded_at: None,
            in_flight: None,
        });
        entry.in_flight = Some(generation);
        Some(generation)
    }

    /// Stores an answer. Returns false for an answer that a newer request replaced.
    pub(crate) fn finish(
        &mut self,
        key: &K,
        generation: u64,
        answer: anyhow::Result<T>,
        now_ms: i64,
    ) -> bool {
        let Some(entry) = self.entries.get_mut(key) else {
            return false;
        };
        if entry.in_flight != Some(generation) {
            return false;
        }
        entry.in_flight = None;
        entry.loaded_at = Some(now_ms);
        entry.value = match answer {
            Ok(value) => Loaded::Ready(value),
            Err(error) => Loaded::Error(format!("{error:#}")),
        };
        true
    }

    /// Every cached key, for a reload after a reconnect.
    pub(crate) fn keys(&self) -> Vec<K> {
        self.entries.keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_fresh_is_fresh_under_30_000_ms_and_stale_at_30_000_ms() {
        assert!(cache_fresh(1_000, 1_000));
        assert!(cache_fresh(1_000, 30_999));
        assert!(!cache_fresh(1_000, 31_000));
        assert!(!cache_fresh(1_000, 90_000));
    }

    #[test]
    fn no_second_request_while_one_is_in_flight() {
        let mut cache: Cache<String, u32> = Cache::default();
        let key = "w1".to_string();
        let first = cache.begin(&key, Want::Fresh, 0);
        assert!(first.is_some());
        assert!(cache.is_fetching(&key));
        assert_eq!(cache.begin(&key, Want::Fresh, 10), None);
        assert_eq!(cache.begin(&key, Want::Missing, 10), None);
        assert_eq!(
            cache.begin(&key, Want::Fresh, 60_000),
            None,
            "still in flight"
        );
        assert!(cache.finish(&key, first.unwrap(), Ok(7), 100));
        assert_eq!(cache.get(&key), Loaded::Ready(7));
        assert!(!cache.is_fetching(&key));
    }

    #[test]
    fn a_fresh_value_is_reused_and_a_stale_one_loads_again() {
        let mut cache: Cache<String, u32> = Cache::default();
        let key = "w1".to_string();
        let first = cache.begin(&key, Want::Fresh, 0).unwrap();
        cache.finish(&key, first, Ok(1), 1_000);
        assert_eq!(cache.begin(&key, Want::Fresh, 30_999), None);
        assert_eq!(
            cache.begin(&key, Want::Missing, 90_000),
            None,
            "the page loads once"
        );
        let second = cache.begin(&key, Want::Fresh, 31_000);
        assert!(second.is_some());
        // The old value shows while the new one loads.
        assert_eq!(cache.get(&key), Loaded::Ready(1));
    }

    #[test]
    fn a_forced_load_replaces_the_answer_in_flight() {
        let mut cache: Cache<String, u32> = Cache::default();
        let key = "w1".to_string();
        let first = cache.begin(&key, Want::Missing, 0).unwrap();
        let second = cache.begin(&key, Want::Force, 5).unwrap();
        assert!(!cache.finish(&key, first, Ok(1), 10), "replaced");
        assert_eq!(cache.get(&key), Loaded::Loading);
        assert!(cache.finish(&key, second, Ok(2), 20));
        assert_eq!(cache.get(&key), Loaded::Ready(2));
    }

    #[test]
    fn an_error_is_kept_with_its_message() {
        let mut cache: Cache<String, u32> = Cache::default();
        let key = "w1".to_string();
        let first = cache.begin(&key, Want::Missing, 0).unwrap();
        cache.finish(
            &key,
            first,
            Err(anyhow::anyhow!("Workspace is unavailable")),
            1,
        );
        assert_eq!(
            cache.get(&key),
            Loaded::Error("Workspace is unavailable".into())
        );
    }

    /// The anti-goal tripwire: 100 redraws of one workspace's composer over 65 s, each asking
    /// for a fresh board and overview as the store does for the active workspace, send at most
    /// one request of each per 30,000 ms.
    #[test]
    fn one_hundred_redraws_send_at_most_one_request_per_30_000_ms() {
        let mut boards: Cache<String, u32> = Cache::default();
        let mut overviews: Cache<String, u32> = Cache::default();
        let key = "w1".to_string();
        let mut sent = Vec::new();
        for redraw in 0..100i64 {
            let now = redraw * 650;
            for (name, cache) in [("board", &mut boards), ("overview", &mut overviews)] {
                if let Some(generation) = cache.begin(&key, Want::Fresh, now) {
                    sent.push((name, now));
                    // The answer arrives 200 ms later, before the next redraw.
                    cache.finish(&key, generation, Ok(1), now + 200);
                }
            }
        }
        let times = |name| -> Vec<i64> {
            sent.iter()
                .filter(|(sent_name, _)| *sent_name == name)
                .map(|(_, at)| *at)
                .collect()
        };
        // Each answer is stale 30,000 ms after it arrived: at 30,200 and 60,750.
        assert_eq!(times("board"), [0, 30_550, 61_100]);
        assert_eq!(times("overview"), [0, 30_550, 61_100]);
        for pair in times("board").windows(2) {
            assert!(pair[1] - pair[0] >= STALE_MS);
        }
    }
}
