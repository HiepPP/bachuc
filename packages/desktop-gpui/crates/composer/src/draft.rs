//! The draft composer of the New workspace screen (`new-workspace-screen.tsx` lines
//! 2424-2455 and `DraftAgentControls`): a composer with no agent yet. It picks the provider,
//! model, mode, thinking, and features, submits with an empty text too, and keeps its text
//! locked while the owner creates the workspace.

use bachuc_client::{AgentFeature, AgentFeatureKind, DraftConfig, ProviderSummary};
use bachuc_plugin_host::ComposerDraft;
use serde_json::Value;
use std::collections::BTreeMap;

/// `NEW_WORKSPACE_INPUT_LINES`.
pub const NEW_WORKSPACE_INPUT_LINES: usize = 5;

/// The submit button glyph: the arrow, or `CornerDownLeft` for a draft.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SubmitIcon {
    #[default]
    Arrow,
    Return,
}

/// How a draft composer behaves.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DraftOptions {
    pub cwd: Option<String>,
    pub min_lines: usize,
    pub submit_icon: SubmitIcon,
    pub allow_empty_submit: bool,
}

impl Default for DraftOptions {
    fn default() -> Self {
        Self {
            cwd: None,
            min_lines: NEW_WORKSPACE_INPUT_LINES,
            submit_icon: SubmitIcon::Return,
            allow_empty_submit: true,
        }
    }
}

/// What the owner needs to create the workspace and its first agent.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DraftSubmission {
    pub text: String,
    pub images: Vec<Value>,
    pub attachments: Vec<Value>,
    /// `None` without a `cwd`.
    pub config: Option<DraftConfig>,
    pub feature_values: BTreeMap<String, Value>,
    pub environment: BTreeMap<String, String>,
}

impl DraftSubmission {
    /// The submission of a draft: the feature values as the daemon takes them, and the
    /// plugin draft values as the first agent's environment.
    pub fn build(
        text: String,
        images: Vec<Value>,
        attachments: Vec<Value>,
        config: Option<DraftConfig>,
        features: &[AgentFeature],
        plugin_draft: &ComposerDraft,
    ) -> Self {
        Self {
            text,
            images,
            attachments,
            config,
            feature_values: features
                .iter()
                .map(|feature| (feature.id.clone(), feature_value(feature)))
                .collect(),
            environment: plugin_draft.environment(),
        }
    }
}

/// The input is at least `lines` lines tall: 5 lines of 21 px is 105 px.
pub fn min_input_height(lines: usize, line_height: f32) -> f32 {
    line_height * lines.max(1) as f32
}

/// The agent settings of the draft; `None` without a provider or a `cwd`.
pub fn draft_config(
    provider: Option<&str>,
    cwd: Option<&str>,
    model: Option<&str>,
    mode_id: Option<&str>,
    thinking_option_id: Option<&str>,
) -> Option<DraftConfig> {
    Some(DraftConfig {
        provider: provider?.to_string(),
        cwd: cwd?.to_string(),
        model: model.map(String::from),
        mode_id: mode_id.map(String::from),
        thinking_option_id: thinking_option_id.map(String::from),
    })
}

/// The first available provider is the default.
pub fn default_provider(providers: &[ProviderSummary]) -> Option<String> {
    providers
        .iter()
        .find(|provider| provider.available)
        .map(|provider| provider.id.clone())
}

/// The model menu: every available provider's models, grouped by provider.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelGroup {
    pub provider: String,
    pub label: String,
    pub models: Vec<(String, String)>,
}

pub fn model_groups(providers: &[ProviderSummary]) -> Vec<ModelGroup> {
    providers
        .iter()
        .filter(|provider| provider.available)
        .map(|provider| ModelGroup {
            provider: provider.id.clone(),
            label: provider.label.clone(),
            models: provider.models.clone(),
        })
        .collect()
}

/// The id of a menu row: the provider, a slash, and the model. Provider ids hold no slash.
pub fn model_key(provider: &str, model: &str) -> String {
    format!("{provider}/{model}")
}

pub fn split_model_key(key: &str) -> Option<(&str, &str)> {
    key.split_once('/')
}

/// The draft requests carry a generation: an answer to an older request is dropped.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Generation(u64);

impl Generation {
    /// Starts a request and returns its generation.
    pub fn next(&mut self) -> u64 {
        self.0 += 1;
        self.0
    }

    /// Whether an answer to request `sent` is still the newest.
    pub fn accepts(&self, sent: u64) -> bool {
        sent == self.0
    }
}

/// The value a draft feature starts with, as `set_agent_feature` would send it.
pub fn feature_value(feature: &AgentFeature) -> Value {
    match &feature.kind {
        AgentFeatureKind::Toggle { value } => Value::Bool(*value),
        AgentFeatureKind::Select { value, .. } => value.clone().map_or(Value::Null, Value::String),
    }
}

/// Applies a toggle or select to the local draft features, as the daemon would.
pub fn apply_feature(features: &mut [AgentFeature], feature_id: &str, value: &Value) {
    let Some(feature) = features.iter_mut().find(|feature| feature.id == feature_id) else {
        return;
    };
    match (&mut feature.kind, value) {
        (AgentFeatureKind::Toggle { value: current }, Value::Bool(next)) => *current = *next,
        (AgentFeatureKind::Select { value: current, .. }, Value::String(next)) => {
            *current = Some(next.clone())
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn provider(id: &str, available: bool) -> ProviderSummary {
        ProviderSummary {
            id: id.into(),
            label: id.to_uppercase(),
            available,
            models: vec![(format!("{id}-1"), format!("{id} one"))],
        }
    }

    #[test]
    fn a_draft_starts_five_lines_tall() {
        let options = DraftOptions::default();
        assert_eq!(options.min_lines, 5);
        assert_eq!(options.submit_icon, SubmitIcon::Return);
        assert!(options.allow_empty_submit);
        // 21 + 4 x 21.
        assert_eq!(min_input_height(options.min_lines, 21.), 105.);
    }

    #[test]
    fn no_cwd_gives_no_config() {
        assert_eq!(draft_config(Some("claude"), None, None, None, None), None);
        assert_eq!(draft_config(None, Some("/repo"), None, None, None), None);
        assert_eq!(
            draft_config(
                Some("claude"),
                Some("/repo"),
                Some("opus"),
                Some("plan"),
                None
            ),
            Some(DraftConfig {
                provider: "claude".into(),
                cwd: "/repo".into(),
                model: Some("opus".into()),
                mode_id: Some("plan".into()),
                thinking_option_id: None,
            })
        );
    }

    #[test]
    fn the_first_available_provider_is_the_default_and_groups_skip_the_rest() {
        let providers = vec![
            provider("pi", false),
            provider("claude", true),
            provider("codex", true),
        ];
        assert_eq!(default_provider(&providers).as_deref(), Some("claude"));
        let groups = model_groups(&providers);
        assert_eq!(
            groups
                .iter()
                .map(|g| g.provider.as_str())
                .collect::<Vec<_>>(),
            ["claude", "codex"]
        );
        assert_eq!(
            split_model_key(&model_key("claude", "a/b")),
            Some(("claude", "a/b"))
        );
    }

    #[test]
    fn an_answer_after_a_newer_request_is_dropped() {
        let mut generation = Generation::default();
        let features_request = generation.next();
        // The user picks another provider before the answer arrives.
        let provider_pick = generation.next();
        assert!(!generation.accepts(features_request));
        assert!(generation.accepts(provider_pick));
    }

    #[test]
    fn draft_features_change_locally() {
        let mut features = vec![AgentFeature {
            id: "fast".into(),
            label: "Fast".into(),
            description: None,
            tooltip: None,
            icon: None,
            kind: AgentFeatureKind::Toggle { value: false },
        }];
        assert_eq!(feature_value(&features[0]), Value::Bool(false));
        apply_feature(&mut features, "fast", &Value::Bool(true));
        assert_eq!(feature_value(&features[0]), Value::Bool(true));
        apply_feature(&mut features, "missing", &Value::Bool(false));
    }

    #[test]
    fn the_submission_takes_the_environment_from_the_draft() {
        let mut plugin_draft = ComposerDraft::default();
        plugin_draft.set("routes", serde_json::json!({"mode": "fast"}));
        let features = vec![AgentFeature {
            id: "fast".into(),
            label: "Fast".into(),
            description: None,
            tooltip: None,
            icon: None,
            kind: AgentFeatureKind::Toggle { value: true },
        }];
        let submission = DraftSubmission::build(
            String::new(),
            Vec::new(),
            Vec::new(),
            None,
            &features,
            &plugin_draft,
        );
        assert_eq!(submission.environment, plugin_draft.environment());
        assert_eq!(
            submission
                .environment
                .get("PASEO_PLUGIN_COMPOSER_routes")
                .map(String::as_str),
            Some(r#"{"mode":"fast"}"#)
        );
        assert_eq!(
            submission.feature_values.get("fast"),
            Some(&Value::Bool(true))
        );
        // No `cwd`: no config, and an empty text still submits.
        assert_eq!(submission.config, None);
        assert_eq!(submission.text, "");
    }
}
