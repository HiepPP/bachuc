//! The pure parts of the provider feature controls: which icon, color, label, and request a
//! feature gets. `agent_controls/mod.rs` draws them.

use bachuc_client::{AgentFeature, AgentFeatureKind};
use bachuc_icons::IconName;
use serde_json::{Value, json};
use std::collections::HashSet;

/// `FAST_MODE_FEATURE_ID`, `PLAN_MODE_FEATURE_ID`, and the `auto_accept` id of
/// `getFeatureHighlightColor` in `packages/app/src/composer/agent-controls/utils.ts`.
const FAST_MODE: &str = "fast_mode";
const PLAN_MODE: &str = "plan_mode";
const AUTO_ACCEPT: &str = "auto_accept";

/// `getAgentFeatureIcon`: the icon for a feature `icon` name, `Settings2` for an unknown one.
pub fn feature_icon(name: Option<&str>) -> IconName {
    match name {
        Some("list-todo") => IconName::ListTodo,
        Some("shield-check") => IconName::ShieldCheck,
        Some("zap") => IconName::Zap,
        _ => IconName::Settings2,
    }
}

/// `getFeatureTooltip`: the tooltip, else the label.
pub fn feature_tooltip(feature: &AgentFeature) -> &str {
    feature.tooltip.as_deref().unwrap_or(&feature.label)
}

/// `getFeatureHighlightColor`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Highlight {
    Yellow,
    Green,
    Blue,
    Default,
}

pub fn feature_highlight(feature_id: &str) -> Highlight {
    match feature_id {
        FAST_MODE => Highlight::Yellow,
        AUTO_ACCEPT => Highlight::Green,
        PLAN_MODE => Highlight::Blue,
        _ => Highlight::Default,
    }
}

/// The highlight of a toggle while it is on; an off toggle and a select draw muted.
pub fn active_highlight(feature: &AgentFeature) -> Highlight {
    match feature.kind {
        AgentFeatureKind::Toggle { value: true } => feature_highlight(&feature.id),
        _ => Highlight::Default,
    }
}

/// The selected option id of a select. A null value, or an id the options lack, gives `None`,
/// so the pill shows the feature label as Electron's `selectedOption?.label ?? feature.label`.
pub fn select_current(feature: &AgentFeature) -> Option<String> {
    match &feature.kind {
        AgentFeatureKind::Select { value, options } => value
            .as_deref()
            .filter(|id| options.iter().any(|option| option.id == *id))
            .map(String::from),
        AgentFeatureKind::Toggle { .. } => None,
    }
}

/// The `(id, label)` options of a select feature.
pub fn select_choices(feature: &AgentFeature) -> Vec<(String, String)> {
    match &feature.kind {
        AgentFeatureKind::Select { options, .. } => options
            .iter()
            .map(|option| (option.id.clone(), option.label.clone()))
            .collect(),
        AgentFeatureKind::Toggle { .. } => Vec::new(),
    }
}

/// The request a click on a toggle sends: its id and the opposite of the value the daemon
/// reports. `None` for an unknown id or a select. After a failed request the daemon list is
/// unchanged, so the next click sends the same payload and the pill keeps the daemon value.
pub fn toggle_request(features: &[AgentFeature], feature_id: &str) -> Option<(String, Value)> {
    let feature = features.iter().find(|feature| feature.id == feature_id)?;
    match feature.kind {
        AgentFeatureKind::Toggle { value } => Some((feature.id.clone(), json!(!value))),
        AgentFeatureKind::Select { .. } => None,
    }
}

/// The request a pick in a select sends: the option id. `None` for an unknown feature, a toggle,
/// or an option the select lacks.
pub fn select_request(
    features: &[AgentFeature],
    feature_id: &str,
    option_id: &str,
) -> Option<(String, Value)> {
    let feature = features.iter().find(|feature| feature.id == feature_id)?;
    match &feature.kind {
        AgentFeatureKind::Select { options, .. }
            if options.iter().any(|option| option.id == option_id) =>
        {
            Some((feature.id.clone(), json!(option_id)))
        }
        _ => None,
    }
}

/// The features whose request has no answer yet. A click on one of them sends nothing, so one
/// click is one `set_agent_feature` request.
#[derive(Default)]
pub struct FeatureRequests {
    pending: HashSet<String>,
}

impl FeatureRequests {
    /// Marks the feature as sent. Returns false when a request for it is already in flight.
    pub fn begin(&mut self, feature_id: &str) -> bool {
        self.pending.insert(feature_id.to_string())
    }

    pub fn finish(&mut self, feature_id: &str) {
        self.pending.remove(feature_id);
    }

    pub fn is_pending(&self, feature_id: &str) -> bool {
        self.pending.contains(feature_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bachuc_client::AgentFeatureOption;

    fn toggle(id: &str, value: bool) -> AgentFeature {
        AgentFeature {
            id: id.into(),
            label: id.to_uppercase(),
            description: None,
            tooltip: None,
            icon: None,
            kind: AgentFeatureKind::Toggle { value },
        }
    }

    fn option(id: &str, label: &str) -> AgentFeatureOption {
        AgentFeatureOption {
            id: id.into(),
            label: label.into(),
            description: None,
            is_default: None,
            metadata: Default::default(),
        }
    }

    fn select(id: &str, value: Option<&str>) -> AgentFeature {
        AgentFeature {
            id: id.into(),
            label: "Effort".into(),
            description: None,
            tooltip: Some("Pick an effort".into()),
            icon: Some("unknown-icon".into()),
            kind: AgentFeatureKind::Select {
                value: value.map(String::from),
                options: vec![option("low", "Low"), option("high", "High")],
            },
        }
    }

    #[test]
    fn known_icon_names_map_and_unknown_names_fall_back() {
        assert_eq!(feature_icon(Some("zap")), IconName::Zap);
        assert_eq!(feature_icon(Some("list-todo")), IconName::ListTodo);
        assert_eq!(feature_icon(Some("shield-check")), IconName::ShieldCheck);
        assert_eq!(feature_icon(Some("rocket")), IconName::Settings2);
        assert_eq!(feature_icon(None), IconName::Settings2);
    }

    #[test]
    fn the_tooltip_falls_back_to_the_label() {
        assert_eq!(feature_tooltip(&select("effort", None)), "Pick an effort");
        assert_eq!(feature_tooltip(&toggle("a", false)), "A");
    }

    #[test]
    fn highlight_colors_follow_the_feature_id_and_the_value() {
        assert_eq!(feature_highlight("fast_mode"), Highlight::Yellow);
        assert_eq!(feature_highlight("auto_accept"), Highlight::Green);
        assert_eq!(feature_highlight("plan_mode"), Highlight::Blue);
        assert_eq!(feature_highlight("other"), Highlight::Default);
        assert_eq!(
            active_highlight(&toggle("fast_mode", true)),
            Highlight::Yellow
        );
        assert_eq!(
            active_highlight(&toggle("fast_mode", false)),
            Highlight::Default
        );
        assert_eq!(
            active_highlight(&select("fast_mode", Some("low"))),
            Highlight::Default
        );
    }

    #[test]
    fn features_keep_the_order_the_daemon_sends() {
        let features = vec![
            toggle("plan_mode", false),
            select("effort", None),
            toggle("fast_mode", true),
        ];
        let ids: Vec<_> = features.iter().map(|f| f.id.as_str()).collect();
        assert_eq!(ids, ["plan_mode", "effort", "fast_mode"]);
        // A request reads the list by id, so the order never changes what a click sends.
        assert_eq!(
            toggle_request(&features, "fast_mode"),
            Some(("fast_mode".into(), json!(false)))
        );
        assert_eq!(
            toggle_request(&features, "plan_mode"),
            Some(("plan_mode".into(), json!(true)))
        );
    }

    #[test]
    fn a_toggle_request_sends_the_opposite_bool() {
        let features = vec![toggle("fast_mode", false), select("effort", None)];
        assert_eq!(
            toggle_request(&features, "fast_mode"),
            Some(("fast_mode".into(), json!(true)))
        );
        assert_eq!(toggle_request(&features, "effort"), None);
        assert_eq!(toggle_request(&features, "missing"), None);
    }

    #[test]
    fn a_failed_request_keeps_the_daemon_value() {
        let mut requests = FeatureRequests::default();
        let daemon = vec![toggle("fast_mode", false)];
        assert!(requests.begin("fast_mode"));
        // The daemon refused: nothing changed the list, so the pill still reads `false` and the
        // next click sends `true` again, not `false` from a value we never saw.
        requests.finish("fast_mode");
        assert!(!requests.is_pending("fast_mode"));
        assert_eq!(
            active_highlight(&daemon[0]),
            Highlight::Default,
            "the pill shows the daemon value"
        );
        assert_eq!(
            toggle_request(&daemon, "fast_mode"),
            Some(("fast_mode".into(), json!(true)))
        );
    }

    #[test]
    fn one_click_sends_one_request() {
        let mut requests = FeatureRequests::default();
        assert!(requests.begin("fast_mode"));
        assert!(!requests.begin("fast_mode"), "a second click is dropped");
        assert!(requests.begin("plan_mode"), "other features are free");
        requests.finish("fast_mode");
        assert!(requests.begin("fast_mode"));
    }

    #[test]
    fn a_select_with_a_null_value_shows_its_label() {
        let feature = select("effort", None);
        assert_eq!(select_current(&feature), None);
        assert_eq!(select_current(&select("effort", Some("max"))), None);
        assert_eq!(
            select_choices(&feature),
            vec![
                ("low".to_string(), "Low".to_string()),
                ("high".to_string(), "High".to_string())
            ]
        );
    }

    #[test]
    fn a_select_option_change_sends_the_option_id() {
        let features = vec![select("effort", Some("low"))];
        assert_eq!(select_current(&features[0]).as_deref(), Some("low"));
        assert_eq!(
            select_request(&features, "effort", "high"),
            Some(("effort".into(), json!("high")))
        );
        assert_eq!(select_request(&features, "effort", "max"), None);
        assert_eq!(select_request(&features, "fast_mode", "high"), None);
        let toggles = vec![toggle("fast_mode", false)];
        assert_eq!(select_request(&toggles, "fast_mode", "high"), None);
    }
}
