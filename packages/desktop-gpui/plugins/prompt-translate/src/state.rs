//! The current settings and `active_since` (`client/state.ts`). Adapters read them
//! synchronously; the `PluginSettings` watcher keeps them current.

use crate::settings::Settings;

/// Re-enabling translation must not translate prompts sent while it was off (`state.ts:10`).
pub(crate) fn next_active_since(was_on: bool, now_on: bool, active_since: i64, now: i64) -> i64 {
    if now_on && !was_on { now } else { active_since }
}

pub(crate) struct Current {
    pub(crate) values: Settings,
    /// Milliseconds since the Unix epoch. Starts at activation, as `Date.now()` at module load.
    pub(crate) active_since: i64,
}

impl Current {
    pub(crate) fn new(now: i64) -> Self {
        Self {
            values: Settings::default(),
            active_since: now,
        }
    }

    /// Takes new values. True when `translate` changed, which is when the transformer's answers
    /// change and the timeline must run it again.
    pub(crate) fn apply(&mut self, next: Settings, now: i64) -> bool {
        self.active_since = next_active_since(
            self.values.translate,
            next.translate,
            self.active_since,
            now,
        );
        let translate_changed = self.values.translate != next.translate;
        self.values = next;
        translate_changed
    }
}

pub(crate) fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

#[cfg(test)]
mod tests {
    use super::Current;
    use crate::settings::Settings;

    #[test]
    fn switching_translation_back_on_moves_active_since_forward_and_notifies() {
        let mut current = Current::new(100);
        let off = Settings {
            translate: false,
            ..current.values.clone()
        };
        assert!(current.apply(off.clone(), 150));
        // Off keeps the old value: only turning on moves it.
        assert_eq!(current.active_since, 100);
        let on = Settings {
            translate: true,
            ..off
        };
        assert!(current.apply(on.clone(), 200));
        assert_eq!(current.active_since, 200);
        // On then on: it stays, and nothing about the transformer changed.
        assert!(!current.apply(on, 300));
        assert_eq!(current.active_since, 200);
    }
}
