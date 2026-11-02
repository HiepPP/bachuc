//! The Ponytail mode of each composer: an agent composer reads and writes it through
//! `ponytail.mode.*` (`shared/contracts.ts`); a New workspace draft keeps it in its plugin value
//! (`PASEO_PLUGIN_COMPOSER_ponytail`, `shared/modes.ts`).
//!
//! A pill's `state` closure gets only `&App`, so the store keeps the modes in a `RefCell` and
//! starts a read the first time an agent is asked for. At most one read runs per agent between
//! reconnects, and a failed read starts no read loop.

use anyhow::Context as _;
use bachuc_plugin_host::SelectOption;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const MODE_READ: &str = "ponytail.mode.read";
pub(crate) const MODE_WRITE: &str = "ponytail.mode.write";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Mode {
    Off,
    Lite,
    /// The server default for an agent with no saved mode.
    #[default]
    Full,
    Ultra,
}

impl Mode {
    pub(crate) const ALL: [Mode; 4] = [Mode::Off, Mode::Lite, Mode::Full, Mode::Ultra];

    pub(crate) fn value(self) -> &'static str {
        match self {
            Mode::Off => "off",
            Mode::Lite => "lite",
            Mode::Full => "full",
            Mode::Ultra => "ultra",
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Mode::Off => "Off",
            Mode::Lite => "Lite",
            Mode::Full => "Full",
            Mode::Ultra => "Ultra",
        }
    }

    /// `None` for a value outside the four.
    pub(crate) fn parse(value: &str) -> Option<Mode> {
        Mode::ALL.into_iter().find(|mode| mode.value() == value)
    }
}

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
    /// which is when pills redraw.
    pub(crate) fn publish(&mut self, key: &str, mode: Mode) -> bool {
        self.in_flight.remove(key);
        self.known.insert(key.to_string(), mode) != Some(mode)
    }

    /// A failed read clears the in-flight mark. The mode stays unknown.
    pub(crate) fn read_failed(&mut self, key: &str) {
        self.in_flight.remove(key);
        self.failed.insert(key.to_string());
    }

    /// After a reconnect every agent reads again.
    pub(crate) fn clear(&mut self) {
        *self = Self::default();
    }
}

/// The pill label. An unknown mode shows the server default.
pub(crate) fn pill_label(mode: Option<Mode>) -> String {
    format!("Ponytail: {}", mode.unwrap_or_default().label())
}

/// A draft composer's mode: its saved `{mode}`, else the server default.
pub(crate) fn draft_mode(saved: Option<&Value>) -> Mode {
    saved
        .and_then(|saved| saved.get("mode"))
        .and_then(Value::as_str)
        .and_then(Mode::parse)
        .unwrap_or_default()
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

pub(crate) fn mode_read_input(agent_id: &str) -> Value {
    json!({ "agentId": agent_id })
}

pub(crate) fn mode_write_input(agent_id: &str, mode: Mode) -> Value {
    json!({ "agentId": agent_id, "mode": mode.value() })
}

/// `{mode}`.
pub(crate) fn parse_mode(answer: &Value) -> anyhow::Result<Mode> {
    let value = answer
        .get("mode")
        .and_then(Value::as_str)
        .context("the answer has no `mode` text")?;
    Mode::parse(value).with_context(|| format!("unknown Ponytail mode `{value}`"))
}

#[cfg(test)]
mod tests {
    use super::{Mode, Modes, draft_mode, draft_payload, mode_options, pill_label};
    use serde_json::json;

    #[test]
    fn labels_and_options() {
        assert_eq!(pill_label(Some(Mode::Ultra)), "Ponytail: Ultra");
        assert_eq!(pill_label(Mode::parse("off")), "Ponytail: Off");
        assert_eq!(pill_label(Mode::parse("nope")), "Ponytail: Full");
        assert_eq!(pill_label(None), "Ponytail: Full");
        let options = mode_options();
        assert_eq!(
            options.iter().map(|o| o.id.as_ref()).collect::<Vec<_>>(),
            ["off", "lite", "full", "ultra"]
        );
    }

    #[test]
    fn a_draft_reads_its_saved_mode_then_full() {
        assert_eq!(draft_mode(Some(&draft_payload(Mode::Off))), Mode::Off);
        assert_eq!(draft_mode(None), Mode::Full);
        assert_eq!(draft_mode(Some(&json!({ "mode": "nope" }))), Mode::Full);
    }

    #[test]
    fn read_once_per_agent_and_no_loop_after_failure() {
        let mut modes = Modes::default();
        assert_eq!((0..100).filter(|_| modes.ask("a1")).count(), 1);
        assert!(modes.publish("a1", Mode::Lite));
        assert!(!modes.publish("a1", Mode::Lite));
        assert!(modes.ask("a2"));
        modes.read_failed("a2");
        assert_eq!((0..100).filter(|_| modes.ask("a2")).count(), 0);
        modes.clear();
        assert!(modes.ask("a2"));
    }
}

#[cfg(test)]
mod rpc_tests {
    use super::{Mode, mode_read_input, mode_write_input, parse_mode};
    use serde_json::json;

    #[test]
    fn the_rpc_inputs_and_answers_follow_contracts_ts() {
        assert_eq!(mode_read_input("a1"), json!({ "agentId": "a1" }));
        assert_eq!(
            mode_write_input("a1", Mode::Ultra),
            json!({ "agentId": "a1", "mode": "ultra" })
        );
        assert_eq!(parse_mode(&json!({ "mode": "off" })).unwrap(), Mode::Off);
        assert!(parse_mode(&json!({ "mode": "nope" })).is_err());
        assert!(parse_mode(&json!({})).is_err());
    }
}
