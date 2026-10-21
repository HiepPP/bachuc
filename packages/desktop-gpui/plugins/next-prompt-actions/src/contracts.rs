//! The RPC shapes of `shared/contracts.ts`. Outputs decode leniently: unknown fields are ignored,
//! and an unknown candidate state reads as `unknown`.

use serde::{Deserialize, Serialize};

pub(crate) const HOST: &str = "prompts.host";
pub(crate) const SCOPE: &str = "prompts.scope";
pub(crate) const INSPECT: &str = "prompts.inspect";
pub(crate) const SEND: &str = "prompts.send";
pub(crate) const START: &str = "prompts.start";
pub(crate) const TOGGLE: &str = "prompts.toggle";

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct HostInfo {
    pub server_id: String,
    pub evaluator: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ScopeInput<'a> {
    pub agent_id: &'a str,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct Scope {
    pub server_id: String,
    pub agent_id: String,
    pub workspace_id: String,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct Selection {
    pub id: String,
    pub block_key: String,
    pub exclusive_groups: Vec<Vec<String>>,
    pub allowed_combinations: Vec<Vec<String>>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum CandidateState {
    Ready,
    Sending,
    Sent,
    #[default]
    #[serde(other)]
    Unknown,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct Candidate {
    pub key: String,
    pub block: String,
    pub text: String,
    pub why: Option<String>,
    pub source: String,
    pub timestamp: f64,
    /// The time of the user message the reply answers; identity uses this bound.
    pub after: Option<f64>,
    pub state: CandidateState,
    pub thread: Option<String>,
    pub suggestion: Option<bool>,
    pub goal: Option<String>,
    pub selection: Option<Selection>,
}

impl Candidate {
    /// `thread: "new"`: offered only as a separate new thread.
    pub(crate) fn thread_new(&self) -> bool {
        self.thread.as_deref() == Some("new")
    }
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct Snapshot {
    pub enabled: bool,
    pub busy: bool,
    pub note: String,
    pub warning: Option<String>,
    pub candidates: Vec<Candidate>,
}

/// `key` of `prompts.send`: one key, or several sent as one numbered message.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub(crate) enum SendKey {
    One(String),
    Many(Vec<String>),
}

impl SendKey {
    pub(crate) fn of(mut keys: Vec<String>) -> Self {
        if keys.len() == 1 {
            Self::One(keys.remove(0))
        } else {
            Self::Many(keys)
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(crate) struct SendInput {
    #[serde(flatten)]
    pub scope: Scope,
    pub key: SendKey,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(crate) struct StartInput {
    #[serde(flatten)]
    pub scope: Scope,
    pub key: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(crate) struct ToggleInput {
    #[serde(flatten)]
    pub scope: Scope,
    pub enabled: bool,
}

#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(default)]
pub(crate) struct SendOutput {
    pub sent: bool,
}

#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(default)]
pub(crate) struct StartOutput {
    #[allow(dead_code)]
    pub started: bool,
}

#[cfg(test)]
mod tests {
    use super::{CandidateState, Scope, SendInput, SendKey, Snapshot};
    use serde_json::json;

    #[test]
    fn snapshots_decode_leniently() {
        let snapshot: Snapshot = serde_json::from_value(json!({
            "enabled": true, "busy": false, "note": "", "extra": 1,
            "candidates": [
                {"key": "k", "block": "b", "text": "t", "source": "s", "timestamp": 2,
                 "after": 1, "state": "ready", "thread": "new", "more": true},
                {"key": "k2", "state": "paused"},
            ],
        }))
        .unwrap();
        assert_eq!(snapshot.candidates[0].state, CandidateState::Ready);
        assert!(snapshot.candidates[0].thread_new());
        assert_eq!(snapshot.candidates[0].after, Some(1.));
        assert_eq!(snapshot.candidates[1].state, CandidateState::Unknown);
    }

    #[test]
    fn send_input_flattens_the_scope_and_keeps_one_key_a_string() {
        let scope = Scope {
            server_id: "s".into(),
            agent_id: "a".into(),
            workspace_id: "w".into(),
        };
        let one = SendInput {
            scope: scope.clone(),
            key: SendKey::of(vec!["k".into()]),
        };
        assert_eq!(
            serde_json::to_value(one).unwrap(),
            json!({"serverId": "s", "agentId": "a", "workspaceId": "w", "key": "k"})
        );
        let many = SendInput {
            scope,
            key: SendKey::of(vec!["k1".into(), "k2".into()]),
        };
        assert_eq!(
            serde_json::to_value(many).unwrap()["key"],
            json!(["k1", "k2"])
        );
    }
}
