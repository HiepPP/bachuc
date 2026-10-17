//! The Caveman mode of each composer, as `client/prefs.ts` keeps it: an agent composer reads
//! and writes the mode through `translate.mode.*`; a New workspace draft keeps it in its plugin
//! value and falls back to the settings `cavemanMode`.
//!
//! A pill's `state` closure gets only `&App`, so the store keeps the modes in a `RefCell` and
//! starts a read the first time an agent is asked for. At most one read runs per agent between
//! reconnects, and a failed read starts no read loop (`failed_mode_read_no_loop`).

use crate::PLUGIN_ID;
use crate::rpc::{MODE_READ, MODE_WRITE, mode_read_input, mode_write_input, parse_mode};
use crate::settings::Mode;
use crate::shared::Shared;
use bachuc_plugin_host::{ComposerTarget, PillBehavior, PillState, SelectOption};
use gpui_kit::App;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

/// The modes and bookkeeping, without any `App`. The key is an agent id.
#[derive(Default)]
pub(crate) struct Modes {
    known: BTreeMap<String, Mode>,
    in_flight: BTreeSet<String>,
    /// Agents whose read failed. They wait for a reconnect, so one failure starts no read loop.
    failed: BTreeSet<String>,
}

impl Modes {
    pub(crate) fn mode(&self, key: &str) -> Option<Mode> {
        self.known.get(key).copied()
    }

    /// A pill asks for `key`. True when the caller starts the read: the mode is unknown, no read
    /// runs for it, and its last read did not fail.
    pub(crate) fn ask(&mut self, key: &str) -> bool {
        !self.known.contains_key(key)
            && !self.failed.contains(key)
            && self.in_flight.insert(key.to_string())
    }

    /// Keeps an answer of a read or a write. True when the mode differs from the last one,
    /// which is when pills redraw (`publish`, `prefs.ts:15`).
    pub(crate) fn publish(&mut self, key: &str, mode: Mode) -> bool {
        self.in_flight.remove(key);
        self.known.insert(key.to_string(), mode) != Some(mode)
    }

    /// A failed read clears the in-flight mark. The mode stays unknown.
    pub(crate) fn read_failed(&mut self, key: &str) {
        self.in_flight.remove(key);
        self.failed.insert(key.to_string());
    }

    /// The optimistic part of a save. False while the mode is unknown (`save`, `prefs.ts:56`),
    /// when nothing is saved.
    pub(crate) fn begin_save(&mut self, key: &str, mode: Mode) -> bool {
        match self.known.get_mut(key) {
            Some(known) => {
                *known = mode;
                true
            }
            None => false,
        }
    }

    /// After a reconnect every agent reads again.
    pub(crate) fn clear(&mut self) {
        *self = Self::default();
    }
}

/// The pill label: the mode's label, or `Default` while the mode is unknown.
pub(crate) fn pill_label(mode: Option<Mode>) -> &'static str {
    mode.map_or("Default", Mode::label)
}

/// A draft composer's mode: its saved `{mode}`, else the settings `cavemanMode` once they load
/// (`prefs.ts:44`).
pub(crate) fn draft_mode(saved: Option<&Value>, settings_default: Option<Mode>) -> Option<Mode> {
    saved
        .and_then(|saved| saved.get("mode"))
        .and_then(Value::as_str)
        .and_then(Mode::parse)
        .or(settings_default)
}

pub(crate) fn draft_payload(mode: Mode) -> Value {
    json!({ "mode": mode.value() })
}

pub(crate) fn mode_options() -> Vec<SelectOption> {
    Mode::ALL
        .into_iter()
        .map(|mode| SelectOption {
            id: mode.value().into(),
            label: mode.label().into(),
            description: None,
        })
        .collect()
}

impl Shared {
    /// The mode of a composer, `None` while unknown. An agent whose mode is missing starts one
    /// read.
    fn current_mode(self: &Rc<Self>, target: &ComposerTarget, cx: &App) -> Option<Mode> {
        if let Some(draft) = &target.draft {
            let default = self
                .ready_settings(cx)
                .map(|settings| settings.caveman_mode);
            return draft_mode(draft.read(cx).get(PLUGIN_ID), default);
        }
        let agent_id = target.agent_id.as_deref()?;
        if self.modes.borrow_mut().ask(agent_id) {
            self.spawn_read(agent_id.to_string(), cx);
        }
        self.modes.borrow().mode(agent_id)
    }

    /// The pill of a composer, `None` for a target that is neither an agent nor a draft.
    pub(crate) fn pill_state(
        self: &Rc<Self>,
        target: &ComposerTarget,
        cx: &App,
    ) -> Option<PillState> {
        if target.draft.is_none() && target.agent_id.is_none() {
            return None;
        }
        let mode = self.current_mode(target, cx);
        let shared = self.clone();
        Some(PillState {
            title: "Caveman mode".into(),
            label: Some(pill_label(mode).into()),
            icon: None,
            color: None,
            disabled: false,
            behavior: PillBehavior::Select {
                options: mode_options(),
                value: mode.map(|mode| mode.value().into()),
                on_select: Rc::new(move |target, value, cx| shared.pick(target, value, cx)),
            },
        })
    }

    fn spawn_read(self: &Rc<Self>, agent_id: String, cx: &App) {
        let weak = Rc::downgrade(self);
        cx.spawn(async move |cx| {
            let Some(task) = cx.update(|cx| {
                weak.upgrade()
                    .map(|shared| shared.ctx.rpc(MODE_READ, mode_read_input(&agent_id), cx))
            }) else {
                return;
            };
            let result = task.await.and_then(|answer| parse_mode(&answer));
            cx.update(|cx| {
                let Some(shared) = weak.upgrade() else { return };
                let changed = match result {
                    Ok(mode) => shared.modes.borrow_mut().publish(&agent_id, mode),
                    Err(error) => {
                        log::debug!("translate.mode.read failed: {error:#}");
                        shared.modes.borrow_mut().read_failed(&agent_id);
                        false
                    }
                };
                if changed {
                    shared.ctx.notify(cx);
                }
            });
        })
        .detach();
    }

    /// Saves a pick: a draft stores it in its value, an agent publishes it at once and sends
    /// `translate.mode.write`.
    fn pick(self: &Rc<Self>, target: &ComposerTarget, value: &str, cx: &mut App) {
        let Some(mode) = Mode::parse(value) else {
            return;
        };
        if let Some(draft) = &target.draft {
            draft.update(cx, |draft, cx| {
                draft.set(PLUGIN_ID, draft_payload(mode));
                cx.notify();
            });
            self.ctx.notify(cx);
            return;
        }
        let Some(agent_id) = target.agent_id.clone() else {
            return;
        };
        if !self.modes.borrow_mut().begin_save(&agent_id, mode) {
            return;
        }
        self.ctx.notify(cx);
        let answer = self
            .ctx
            .rpc(MODE_WRITE, mode_write_input(&agent_id, mode), cx);
        let weak = Rc::downgrade(self);
        cx.spawn(async move |cx| {
            let result = answer.await.and_then(|answer| parse_mode(&answer));
            cx.update(|cx| {
                let Some(shared) = weak.upgrade() else { return };
                match result {
                    Ok(saved) => {
                        if shared.modes.borrow_mut().publish(&agent_id, saved) {
                            shared.ctx.notify(cx);
                        }
                    }
                    // The optimistic mode stays, as in Electron.
                    Err(error) => log::warn!("translate.mode.write failed: {error:#}"),
                }
            });
        })
        .detach();
    }
}

#[cfg(test)]
mod tests {
    use super::{Modes, draft_mode, draft_payload, mode_options, pill_label};
    use crate::settings::Mode;
    use serde_json::json;

    #[test]
    fn pill_label_rule() {
        assert_eq!(pill_label(Some(Mode::Ultra)), "Ultra");
        assert_eq!(pill_label(Mode::parse("ultra")), "Ultra");
        assert_eq!(pill_label(Some(Mode::FollowAgent)), "Default");
        assert_eq!(pill_label(Mode::parse("not-a-mode")), "Default");
        assert_eq!(pill_label(None), "Default");
        let options = mode_options();
        assert_eq!(options.len(), 7);
        assert_eq!(options[0].id.as_ref(), "follow-agent");
        assert_eq!(options[6].label.as_ref(), "Wenyan Ultra");
    }

    #[test]
    fn a_draft_reads_its_saved_mode_then_the_settings() {
        let saved = draft_payload(Mode::Lite);
        assert_eq!(draft_mode(Some(&saved), Some(Mode::Full)), Some(Mode::Lite));
        assert_eq!(draft_mode(None, Some(Mode::Full)), Some(Mode::Full));
        assert_eq!(
            draft_mode(Some(&json!({ "mode": "nope" })), Some(Mode::Full)),
            Some(Mode::Full)
        );
        assert_eq!(draft_mode(None, None), None);
    }

    #[test]
    fn read_once_per_agent() {
        let mut modes = Modes::default();
        assert_eq!((0..100).filter(|_| modes.ask("a1")).count(), 1);
        assert!(modes.ask("a2"));
        assert!(modes.publish("a1", Mode::Ultra));
        assert!(!modes.publish("a1", Mode::Ultra));
        assert!(!modes.ask("a1"));
        assert_eq!(modes.mode("a1"), Some(Mode::Ultra));
        assert!(!modes.begin_save("a3", Mode::Lite));
        assert!(modes.begin_save("a1", Mode::Lite));
        assert_eq!(modes.mode("a1"), Some(Mode::Lite));
    }

    #[test]
    fn failed_mode_read_no_loop() {
        let mut modes = Modes::default();
        assert!(modes.ask("a1"));
        modes.read_failed("a1");
        // The mark is clear, yet no second read starts however often the pill draws.
        assert_eq!((0..100).filter(|_| modes.ask("a1")).count(), 0);
        assert_eq!(modes.mode("a1"), None);
        // A reconnect lets the agent read again.
        modes.clear();
        assert!(modes.ask("a1"));
    }
}
