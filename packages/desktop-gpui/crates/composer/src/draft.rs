//! The draft composer of the New workspace screen (`new-workspace-screen.tsx` lines
//! 2424-2455 and `DraftAgentControls`): a composer with no agent yet. It picks the provider,
//! model, mode, thinking, and features, submits with an empty text too, and keeps its text
//! locked while the owner creates the workspace.

use bachuc_client::{AgentFeature, AgentFeatureKind, DraftConfig, ProviderSummary};
use bachuc_plugin_host::ComposerDraft;
use bachuc_settings::agent_defaults::AgentDefaults;
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

/// The source agent settings a draft starts from, as `buildForkDraftSetup` in
/// `use-fork-agent.ts`. `features` is a JSON object of feature values by feature id.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DraftSeed {
    pub provider: Option<String>,
    pub model: Option<String>,
    pub mode: Option<String>,
    pub thinking_option_id: Option<String>,
    pub features: Option<Value>,
}

/// The provider and model a seed sets. A seed with no provider keeps `current`. `None` when
/// that provider is not available: the other fields belong to it, so the whole seed is
/// ignored. A model the provider does not list is dropped.
pub fn seed_provider_model(
    seed: &DraftSeed,
    current: Option<&str>,
    providers: &[ProviderSummary],
) -> Option<(String, Option<String>)> {
    let id = seed.provider.as_deref().or(current)?;
    let provider = providers
        .iter()
        .find(|provider| provider.available && provider.id == id)?;
    Some((
        provider.id.clone(),
        offered(seed.model.as_deref(), &provider.models),
    ))
}

/// `wanted` when `choices` holds it: a seeded mode or thinking option the provider does not
/// offer is ignored.
pub fn offered(wanted: Option<&str>, choices: &[(String, String)]) -> Option<String> {
    let wanted = wanted?;
    choices
        .iter()
        .any(|(id, _)| id == wanted)
        .then(|| wanted.to_string())
}

/// The seeded values the draft's features take: a known feature id and a value of its type.
/// A select value must be one of the feature's options.
pub fn seed_feature_values(
    values: Option<&Value>,
    features: &[AgentFeature],
) -> Vec<(String, Value)> {
    let Some(Value::Object(values)) = values else {
        return Vec::new();
    };
    features
        .iter()
        .filter_map(|feature| {
            let value = values.get(&feature.id)?;
            let fits = match (&feature.kind, value) {
                (AgentFeatureKind::Toggle { .. }, Value::Bool(_)) => true,
                (AgentFeatureKind::Select { options, .. }, Value::String(value)) => {
                    options.iter().any(|option| option.id == *value)
                }
                _ => false,
            };
            fits.then(|| (feature.id.clone(), value.clone()))
        })
        .collect()
}

/// The attachments a prompt sends: the raw attachments as given, then the plugin chips.
pub fn outgoing_attachments(raw: &[Value], chips: Vec<Value>) -> Vec<Value> {
    raw.iter().cloned().chain(chips).collect()
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

/// The fields a new draft starts with. `mode` waits for the provider's mode list, as a seeded
/// mode does.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DraftStart {
    pub provider: Option<String>,
    pub model: Option<String>,
    pub thinking_id: Option<String>,
    pub mode: Option<String>,
}

/// Where a new draft starts. The stored defaults apply when their provider is available and
/// the host offers the model and thinking option; each other field falls back to the first
/// available provider, its default model, and no thinking option or mode. The thinking
/// options and mode belong to the stored provider, so they never apply to a fallback provider.
pub fn draft_start(
    defaults: &AgentDefaults,
    providers: &[ProviderSummary],
    default_model: impl Fn(&str) -> Option<String>,
    thinking_options: impl Fn(&str, &str) -> Vec<(String, String)>,
) -> DraftStart {
    let stored = defaults.provider.as_deref().and_then(|id| {
        providers
            .iter()
            .find(|provider| provider.available && provider.id == id)
    });
    let Some(provider) = stored
        .map(|provider| provider.id.clone())
        .or_else(|| default_provider(providers))
    else {
        return DraftStart::default();
    };
    let model = stored
        .and_then(|stored| offered(defaults.model.as_deref(), &stored.models))
        .or_else(|| default_model(&provider));
    let thinking_id = stored.zip(model.as_deref()).and_then(|(_, model)| {
        offered(
            defaults.thinking_option_id.as_deref(),
            &thinking_options(&provider, model),
        )
    });
    DraftStart {
        mode: stored.and(defaults.mode_id.clone()),
        provider: Some(provider),
        model,
        thinking_id,
    }
}

/// The draft a seed makes from `current`: the seed replaces every field, the stored defaults
/// too. `None` when the seed's provider is not available, so the draft keeps `current`.
pub fn seeded_start(
    seed: &DraftSeed,
    current: &DraftStart,
    providers: &[ProviderSummary],
    default_model: impl Fn(&str) -> Option<String>,
    thinking_options: impl Fn(&str, &str) -> Vec<(String, String)>,
) -> Option<DraftStart> {
    let (provider, model) = seed_provider_model(seed, current.provider.as_deref(), providers)?;
    let model = model.or_else(|| default_model(&provider));
    let thinking_id = model.as_deref().and_then(|model| {
        offered(
            seed.thinking_option_id.as_deref(),
            &thinking_options(&provider, model),
        )
    });
    Some(DraftStart {
        provider: Some(provider),
        model,
        thinking_id,
        mode: seed.mode.clone(),
    })
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

    fn select(id: &str, options: &[&str]) -> AgentFeature {
        AgentFeature {
            id: id.into(),
            label: id.into(),
            description: None,
            tooltip: None,
            icon: None,
            kind: AgentFeatureKind::Select {
                value: None,
                options: options
                    .iter()
                    .map(|option| bachuc_client::AgentFeatureOption {
                        id: (*option).into(),
                        label: (*option).into(),
                        description: None,
                        is_default: None,
                        metadata: serde_json::Map::new(),
                    })
                    .collect(),
            },
        }
    }

    #[test]
    fn a_seed_field_the_provider_does_not_offer_is_ignored() {
        let providers = vec![provider("pi", false), provider("claude", true)];
        let seed = |provider: Option<&str>, model: &str| DraftSeed {
            provider: provider.map(String::from),
            model: Some(model.into()),
            ..DraftSeed::default()
        };
        assert_eq!(
            seed_provider_model(&seed(Some("claude"), "claude-1"), None, &providers),
            Some(("claude".into(), Some("claude-1".into())))
        );
        // An unknown model is dropped; the provider still applies.
        assert_eq!(
            seed_provider_model(&seed(Some("claude"), "gone"), Some("pi"), &providers),
            Some(("claude".into(), None))
        );
        // An unavailable or unknown provider ignores the whole seed.
        assert_eq!(
            seed_provider_model(&seed(Some("pi"), "pi-1"), None, &providers),
            None
        );
        assert_eq!(
            seed_provider_model(&seed(Some("x"), "x-1"), None, &providers),
            None
        );
        // No provider in the seed: the draft's provider takes the model.
        assert_eq!(
            seed_provider_model(&seed(None, "claude-1"), Some("claude"), &providers),
            Some(("claude".into(), Some("claude-1".into())))
        );
        assert_eq!(
            seed_provider_model(&seed(None, "claude-1"), None, &providers),
            None
        );

        let modes = vec![("plan".to_string(), "Plan".to_string())];
        assert_eq!(offered(Some("plan"), &modes).as_deref(), Some("plan"));
        assert_eq!(offered(Some("yolo"), &modes), None);
        assert_eq!(offered(None, &modes), None);

        let features = vec![
            AgentFeature {
                id: "fast".into(),
                label: "Fast".into(),
                description: None,
                tooltip: None,
                icon: None,
                kind: AgentFeatureKind::Toggle { value: false },
            },
            select("effort", &["low", "high"]),
        ];
        let values = serde_json::json!({
            "fast": true,
            "effort": "max",
            "gone": true,
        });
        assert_eq!(
            seed_feature_values(Some(&values), &features),
            [("fast".to_string(), Value::Bool(true))]
        );
        let values = serde_json::json!({"fast": "yes", "effort": "high"});
        assert_eq!(
            seed_feature_values(Some(&values), &features),
            [("effort".to_string(), Value::String("high".into()))]
        );
        assert!(seed_feature_values(None, &features).is_empty());
        assert!(seed_feature_values(Some(&Value::Null), &features).is_empty());
    }

    #[test]
    fn raw_attachments_go_out_unchanged_before_the_chips() {
        let history = serde_json::json!({"type": "text", "contextKind": "chat_history"});
        let chip = serde_json::json!({"type": "text", "title": "#1 Bug"});
        assert_eq!(
            outgoing_attachments(std::slice::from_ref(&history), vec![chip.clone()]),
            [history, chip.clone()]
        );
        assert_eq!(outgoing_attachments(&[], vec![chip.clone()]), [chip]);
    }

    /// Tripwire: a draft with no seed and no raw attachment sends this exact payload.
    #[test]
    fn a_plain_draft_payload_is_unchanged() {
        let mut plugin_draft = ComposerDraft::default();
        plugin_draft.set("routes", serde_json::json!("fast"));
        let features = vec![AgentFeature {
            id: "fast".into(),
            label: "Fast".into(),
            description: None,
            tooltip: None,
            icon: None,
            kind: AgentFeatureKind::Toggle { value: true },
        }];
        let image = serde_json::json!({"data": "aGk=", "mimeType": "image/png"});
        let chip = serde_json::json!({"type": "text", "title": "#1 Bug", "text": "body"});
        let config = draft_config(Some("claude"), Some("/repo"), Some("opus"), None, None);
        let submission = DraftSubmission::build(
            "fix it".into(),
            vec![image.clone()],
            vec![chip.clone()],
            config.clone(),
            &features,
            &plugin_draft,
        );
        assert_eq!(
            submission,
            DraftSubmission {
                text: "fix it".into(),
                images: vec![image],
                attachments: vec![chip],
                config,
                feature_values: BTreeMap::from([("fast".to_string(), Value::Bool(true))]),
                environment: BTreeMap::from([(
                    "PASEO_PLUGIN_COMPOSER_routes".to_string(),
                    r#""fast""#.to_string()
                )]),
            }
        );
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

    fn hiep_defaults() -> AgentDefaults {
        AgentDefaults {
            provider: Some("claude".into()),
            model: Some("claude-opus-5-5".into()),
            thinking_option_id: Some("high".into()),
            mode_id: Some("bypassPermissions".into()),
        }
    }

    fn host() -> Vec<ProviderSummary> {
        vec![
            provider("codex", true),
            ProviderSummary {
                id: "claude".into(),
                label: "Claude".into(),
                available: true,
                models: vec![
                    ("claude-sonnet-5-5".into(), "Sonnet 5.5".into()),
                    ("claude-opus-5-5".into(), "Opus 5.5".into()),
                ],
            },
        ]
    }

    fn start(defaults: &AgentDefaults, providers: &[ProviderSummary]) -> DraftStart {
        draft_start(
            defaults,
            providers,
            |provider| Some(format!("{provider}-default")),
            |provider, model| {
                if provider == "claude" && model == "claude-opus-5-5" {
                    vec![
                        ("medium".into(), "Medium".into()),
                        ("high".into(), "High".into()),
                    ]
                } else {
                    Vec::new()
                }
            },
        )
    }

    #[test]
    fn stored_defaults_apply_when_the_host_offers_them() {
        assert_eq!(
            start(&hiep_defaults(), &host()),
            DraftStart {
                provider: Some("claude".into()),
                model: Some("claude-opus-5-5".into()),
                thinking_id: Some("high".into()),
                mode: Some("bypassPermissions".into()),
            }
        );
    }

    #[test]
    fn each_field_falls_back_when_the_host_does_not_offer_it() {
        // An unknown model takes the provider default; the thinking option of the stored model
        // is then not offered.
        let defaults = AgentDefaults {
            model: Some("claude-gone".into()),
            ..hiep_defaults()
        };
        assert_eq!(
            start(&defaults, &host()),
            DraftStart {
                provider: Some("claude".into()),
                model: Some("claude-default".into()),
                thinking_id: None,
                mode: Some("bypassPermissions".into()),
            }
        );
        // An unknown thinking option is dropped alone.
        let defaults = AgentDefaults {
            thinking_option_id: Some("ultra".into()),
            ..hiep_defaults()
        };
        let started = start(&defaults, &host());
        assert_eq!(started.model.as_deref(), Some("claude-opus-5-5"));
        assert_eq!(started.thinking_id, None);
        // An unavailable provider drops every stored field.
        let mut providers = host();
        providers[1].available = false;
        assert_eq!(
            start(&hiep_defaults(), &providers),
            DraftStart {
                provider: Some("codex".into()),
                model: Some("codex-default".into()),
                thinking_id: None,
                mode: None,
            }
        );
    }

    #[test]
    fn unset_defaults_start_as_before() {
        for defaults in [
            AgentDefaults::default(),
            AgentDefaults {
                provider: None,
                ..hiep_defaults()
            },
        ] {
            assert_eq!(
                start(&defaults, &host()),
                DraftStart {
                    provider: default_provider(&host()),
                    model: Some("codex-default".into()),
                    thinking_id: None,
                    mode: None,
                }
            );
        }
        assert_eq!(start(&hiep_defaults(), &[]), DraftStart::default());
    }

    #[test]
    fn a_seed_wins_over_the_stored_defaults() {
        let thinking = |provider: &str, model: &str| {
            if provider == "claude" && model == "claude-opus-5-5" {
                vec![
                    ("medium".into(), "Medium".into()),
                    ("high".into(), "High".into()),
                ]
            } else {
                Vec::new()
            }
        };
        let current = start(&hiep_defaults(), &host());
        let seed = DraftSeed {
            provider: Some("claude".into()),
            model: Some("claude-sonnet-5-5".into()),
            mode: Some("plan".into()),
            ..DraftSeed::default()
        };
        assert_eq!(
            seeded_start(&seed, &current, &host(), |_| None, thinking),
            Some(DraftStart {
                provider: Some("claude".into()),
                model: Some("claude-sonnet-5-5".into()),
                thinking_id: None,
                mode: Some("plan".into()),
            })
        );
        // A seed with the same model and no thinking or mode clears the stored ones.
        let seed = DraftSeed {
            model: Some("claude-opus-5-5".into()),
            ..DraftSeed::default()
        };
        let seeded = seeded_start(&seed, &current, &host(), |_| None, thinking).unwrap();
        assert_eq!(seeded.thinking_id, None);
        assert_eq!(seeded.mode, None);
        // A seed whose provider is not available keeps the stored start.
        let seed = DraftSeed {
            provider: Some("gone".into()),
            ..DraftSeed::default()
        };
        assert_eq!(
            seeded_start(&seed, &current, &host(), |_| None, thinking),
            None
        );
    }
}
