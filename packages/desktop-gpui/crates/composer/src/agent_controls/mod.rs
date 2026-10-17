//! `packages/app/src/composer/agent-controls/index.tsx`: the model, thinking, mode, and feature
//! controls of the toolbar. A toggle feature is an icon button with a tooltip
//! (`DesktopFeatureItem`); in condensed and tight density the features fold into one Settings2
//! button.

pub mod control;
pub mod features;
pub mod layout;

use crate::draft::{ModelGroup, model_key};
use bachuc_client::{AgentFeature, AgentFeatureKind, ThinkingControls};
use bachuc_icons::{IconName, icon, provider_fallback_icon, provider_logo};
use bachuc_theme::metrics::{FONT_BASE, FONT_SM, ICON_SM, RADIUS_2XL, RADIUS_MD, SPACE_2};
use bachuc_theme::{tokens, ui_font_size};
use control::{GLYPH, Glyph, PillTrigger, TriggerLook};
use features::{
    FeatureRequests, feature_icon, feature_tint, feature_tooltip, select_choices, select_current,
};
use gpui_kit::component::popover::Popover;
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::component::{Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use layout::{CONTROL_SIZE, ControlPresence, Density, FeaturePresence, presentation};
use std::rc::Rc;

/// The menu row height of a choice.
const MENU_ROW_HEIGHT: f32 = 32.;

/// The model pill icon: the provider logo, or the fallback icon of `getProviderIcon`.
pub fn provider_glyph(provider: Option<&str>, color: Hsla) -> Glyph {
    let provider = provider.unwrap_or_default();
    match provider_logo(provider, color) {
        Some(image) => Glyph::Image(image),
        None => Glyph::Icon(provider_fallback_icon(provider)),
    }
}

/// The label of the current choice, or the id when the list lacks it, or `fallback`.
pub fn pill_label(choices: &[(String, String)], current: Option<&str>, fallback: &str) -> String {
    match current {
        Some(id) => choices
            .iter()
            .find(|(choice, _)| choice == id)
            .map(|(_, label)| label.clone())
            .unwrap_or_else(|| id.to_string()),
        None => fallback.to_string(),
    }
}

pub(crate) fn element_id(prefix: &str, id: &str) -> ElementId {
    ElementId::Name(format!("{prefix}-{id}").into())
}

/// One menu row: the label and a check on the current one.
pub(crate) fn menu_row(
    id: impl Into<ElementId>,
    label: String,
    selected: bool,
    cx: &App,
) -> Stateful<Div> {
    let t = tokens(cx);
    h_flex()
        .id(id)
        .role(Role::MenuItem)
        .aria_label(label.clone())
        .aria_selected(selected)
        .h(px(MENU_ROW_HEIGHT))
        .px(px(SPACE_2))
        .gap(px(SPACE_2))
        .items_center()
        .rounded(px(RADIUS_MD))
        .cursor_pointer()
        .text_size(ui_font_size(FONT_BASE, cx))
        .text_color(t.foreground)
        .hover(move |row| row.bg(t.surface2))
        .child(div().flex_1().truncate().child(label))
        .when(selected, |row| {
            row.child(
                icon(IconName::Check)
                    .with_size(px(ICON_SM))
                    .text_color(t.foreground_muted),
            )
        })
}

pub(crate) type PickFn = Rc<dyn Fn(&str, &mut App)>;

/// A pill that opens a menu of choices above itself. With no choices it only shows the label.
#[allow(clippy::too_many_arguments)]
pub fn choice_pill(
    id: ElementId,
    kind: String,
    glyph: Glyph,
    choices: Vec<(String, String)>,
    current: Option<String>,
    fallback: &str,
    tooltip: Option<String>,
    look: TriggerLook,
    on_pick: impl Fn(&str, &mut App) + 'static,
) -> AnyElement {
    let label = pill_label(&choices, current.as_deref(), fallback);
    let trigger = PillTrigger {
        id: id.clone(),
        kind: kind.clone(),
        glyph,
        label,
        tooltip,
        disabled: choices.is_empty(),
        open: false,
        look,
    };
    if choices.is_empty() {
        return trigger.into_any_element();
    }
    let choices = Rc::new(choices);
    let on_pick: PickFn = Rc::new(on_pick);
    Popover::new(id)
        .anchor(Anchor::BottomLeft)
        .p_1()
        .trigger(trigger)
        .content(move |_, _, cx| {
            let current = current.clone();
            let rows = choices.iter().enumerate().map(|(ix, (choice, name))| {
                let selected = current.as_deref() == Some(choice.as_str());
                let (choice, on_pick) = (choice.clone(), on_pick.clone());
                menu_row(("composer-menu-item", ix), name.clone(), selected, cx).on_click(
                    cx.listener(move |state, _, window, cx| {
                        on_pick(&choice, cx);
                        state.dismiss(window, cx);
                    }),
                )
            });
            v_flex()
                .id("composer-menu")
                .role(Role::Menu)
                .aria_label(format!("{kind} menu"))
                .min_w(px(200.))
                .max_h(px(320.))
                .overflow_y_scroll()
                .children(rows)
        })
        .into_any_element()
}

/// A toggle feature: an icon button with a tooltip, tinted while the feature is on.
fn feature_toggle(
    feature: &AgentFeature,
    color: Hsla,
    hover: Hsla,
    pending: bool,
    on_click: impl Fn(&mut App) + 'static,
) -> AnyElement {
    let tooltip = feature_tooltip(feature).to_string();
    h_flex()
        .id(element_id("composer-feature", &feature.id))
        .role(Role::Button)
        .aria_label(tooltip.clone())
        .h(px(CONTROL_SIZE))
        .px(px(SPACE_2))
        .flex_none()
        .items_center()
        .rounded(px(RADIUS_2XL))
        .cursor_pointer()
        .hover(move |pill| pill.bg(hover))
        .when(pending, |pill| pill.opacity(0.5))
        .tooltip(move |window, cx| Tooltip::new(tooltip.clone()).build(window, cx))
        .on_click(move |_, _, cx| on_click(cx))
        .child(
            icon(feature_icon(feature.icon.as_deref()))
                .with_size(px(GLYPH))
                .text_color(color),
        )
        .into_any_element()
}

/// What a feature press asks for. The owner sends the request.
#[derive(Clone)]
pub struct FeatureHandlers {
    pub toggle: Rc<dyn Fn(&str, &mut App)>,
    pub select: Rc<dyn Fn(&str, &str, &mut App)>,
}

/// The features in the order the daemon sends them, each its own control.
fn feature_pills(
    features: &[AgentFeature],
    requests: &FeatureRequests,
    look: TriggerLook,
    handlers: &FeatureHandlers,
    cx: &App,
) -> Vec<AnyElement> {
    let t = tokens(cx);
    features
        .iter()
        .map(|feature| match &feature.kind {
            AgentFeatureKind::Toggle { .. } => {
                let (toggle, feature_id) = (handlers.toggle.clone(), feature.id.clone());
                feature_toggle(
                    feature,
                    feature_tint(feature, &t),
                    t.surface2,
                    requests.is_pending(&feature.id),
                    move |cx| toggle(&feature_id, cx),
                )
            }
            AgentFeatureKind::Select { .. } => {
                let (select, feature_id) = (handlers.select.clone(), feature.id.clone());
                choice_pill(
                    element_id("composer-feature", &feature.id),
                    feature.label.clone(),
                    Glyph::Icon(feature_icon(feature.icon.as_deref())),
                    select_choices(feature),
                    select_current(feature),
                    &feature.label,
                    Some(feature_tooltip(feature).to_string()),
                    look,
                    move |option_id, cx| select(&feature_id, option_id, cx),
                )
            }
        })
        .collect()
}

/// The condensed features: one Settings2 button whose menu lists every feature.
fn features_menu(features: &[AgentFeature], handlers: &FeatureHandlers) -> AnyElement {
    let features = Rc::new(features.to_vec());
    let handlers = handlers.clone();
    Popover::new("agent-controls-features")
        .anchor(Anchor::BottomLeft)
        .p_1()
        .trigger(PillTrigger {
            id: "agent-controls-features-trigger".into(),
            kind: "Features".into(),
            glyph: Glyph::Icon(IconName::Settings2),
            label: "Features".into(),
            tooltip: None,
            disabled: false,
            open: false,
            look: TriggerLook {
                show_caret: false,
                icon_only: true,
            },
        })
        .content(move |_, _, cx| {
            let t = tokens(cx);
            let small = ui_font_size(FONT_SM, cx);
            let mut rows: Vec<AnyElement> = Vec::new();
            for feature in features.iter() {
                match &feature.kind {
                    AgentFeatureKind::Toggle { value } => {
                        let (toggle, feature_id) = (handlers.toggle.clone(), feature.id.clone());
                        rows.push(
                            menu_row(
                                element_id("agent-features-toggle", &feature.id),
                                feature.label.clone(),
                                *value,
                                cx,
                            )
                            .on_click(move |_, _, cx| toggle(&feature_id, cx))
                            .into_any_element(),
                        );
                    }
                    AgentFeatureKind::Select { .. } => {
                        rows.push(
                            div()
                                .px(px(SPACE_2))
                                .pt(px(SPACE_2))
                                .text_size(small)
                                .text_color(t.foreground_muted)
                                .child(feature.label.clone())
                                .into_any_element(),
                        );
                        let current = select_current(feature);
                        for (option_id, label) in select_choices(feature) {
                            let selected = current.as_deref() == Some(option_id.as_str());
                            let (select, feature_id) =
                                (handlers.select.clone(), feature.id.clone());
                            rows.push(
                                menu_row(
                                    element_id(
                                        "agent-features-option",
                                        &format!("{}-{option_id}", feature.id),
                                    ),
                                    label,
                                    selected,
                                    cx,
                                )
                                .on_click(move |_, _, cx| select(&feature_id, &option_id, cx))
                                .into_any_element(),
                            );
                        }
                    }
                }
            }
            v_flex()
                .id("agent-features-menu")
                .role(Role::Menu)
                .aria_label("Features")
                .min_w(px(200.))
                .max_h(px(320.))
                .overflow_y_scroll()
                .children(rows)
        })
        .into_any_element()
}

/// The presence the density rule reads. A select counts the label it shows.
pub fn control_presence(
    has_model: bool,
    has_thinking: bool,
    has_mode: bool,
    features: &[AgentFeature],
) -> ControlPresence {
    ControlPresence {
        has_model,
        has_thinking,
        has_mode,
        features: features
            .iter()
            .map(|feature| match &feature.kind {
                AgentFeatureKind::Toggle { .. } => FeaturePresence::Toggle,
                AgentFeatureKind::Select { .. } => {
                    let current = select_current(feature);
                    let label = select_choices(feature)
                        .into_iter()
                        .find(|(id, _)| Some(id) == current.as_ref())
                        .map(|(_, label)| label)
                        .unwrap_or_else(|| feature.label.clone());
                    FeaturePresence::Select { label }
                }
            })
            .collect(),
    }
}

/// The draft model pill: every available provider's models under a header per provider. A
/// row id is `model_key(provider, model)`, so a pick sets both.
fn grouped_model_pill(
    glyph: Glyph,
    groups: Vec<ModelGroup>,
    current: Option<String>,
    look: TriggerLook,
    on_pick: PickFn,
) -> AnyElement {
    let label = groups
        .iter()
        .flat_map(|group| {
            group
                .models
                .iter()
                .map(move |(model, label)| (model_key(&group.provider, model), label))
        })
        .find(|(key, _)| Some(key) == current.as_ref())
        .map(|(_, label)| label.clone())
        .unwrap_or_else(|| "Model".into());
    let trigger = PillTrigger {
        id: "composer-model".into(),
        kind: "Model".into(),
        glyph,
        label,
        tooltip: None,
        disabled: groups.is_empty(),
        open: false,
        look,
    };
    if groups.is_empty() {
        return trigger.into_any_element();
    }
    let groups = Rc::new(groups);
    Popover::new("composer-model")
        .anchor(Anchor::BottomLeft)
        .p_1()
        .trigger(trigger)
        .content(move |_, _, cx| {
            let t = tokens(cx);
            let small = ui_font_size(FONT_SM, cx);
            let mut rows: Vec<AnyElement> = Vec::new();
            for group in groups.iter() {
                rows.push(
                    div()
                        .px(px(SPACE_2))
                        .pt(px(SPACE_2))
                        .text_size(small)
                        .text_color(t.foreground_muted)
                        .child(group.label.clone())
                        .into_any_element(),
                );
                for (model, label) in &group.models {
                    let key = model_key(&group.provider, model);
                    let selected = current.as_ref() == Some(&key);
                    let on_pick = on_pick.clone();
                    rows.push(
                        menu_row(
                            element_id("composer-model-item", &key),
                            label.clone(),
                            selected,
                            cx,
                        )
                        .on_click(cx.listener(move |state, _, window, cx| {
                            on_pick(&key, cx);
                            state.dismiss(window, cx);
                        }))
                        .into_any_element(),
                    );
                }
            }
            v_flex()
                .id("composer-menu")
                .role(Role::Menu)
                .aria_label("Model menu")
                .min_w(px(240.))
                .max_h(px(320.))
                .overflow_y_scroll()
                .children(rows)
        })
        .into_any_element()
}

/// What the controls show.
pub struct ControlsView<'a> {
    pub model_glyph: Glyph,
    /// A draft lists every provider's models; an agent lists its own provider's.
    pub model_groups: Option<Vec<ModelGroup>>,
    pub models: Vec<(String, String)>,
    pub current_model: Option<String>,
    pub thinking: Option<ThinkingControls>,
    pub modes: Vec<(String, String)>,
    pub current_mode: Option<String>,
    pub features: &'a [AgentFeature],
    pub requests: &'a FeatureRequests,
}

/// What a control pick asks for.
#[derive(Clone)]
pub struct ControlHandlers {
    pub model: PickFn,
    pub thinking: PickFn,
    pub mode: PickFn,
    pub features: FeatureHandlers,
}

/// The controls in the Electron order: model, thinking, mode, then the features.
pub fn agent_controls(
    view: ControlsView,
    density: Density,
    handlers: &ControlHandlers,
    cx: &App,
) -> Vec<AnyElement> {
    let shown = presentation(density);
    let look = TriggerLook {
        show_caret: shown.show_carets,
        icon_only: false,
    };
    let mut pills = Vec::new();
    let model = handlers.model.clone();
    match view.model_groups {
        Some(groups) => pills.push(grouped_model_pill(
            view.model_glyph,
            groups,
            view.current_model,
            look,
            model,
        )),
        None => pills.push(choice_pill(
            "composer-model".into(),
            "Model".into(),
            view.model_glyph,
            view.models,
            view.current_model,
            "Model",
            None,
            look,
            move |id, cx| model(id, cx),
        )),
    }
    if let Some(thinking) = view.thinking {
        let pick = handlers.thinking.clone();
        pills.push(choice_pill(
            "composer-thinking".into(),
            "Thinking".into(),
            Glyph::Icon(IconName::Brain),
            thinking.options,
            thinking.current,
            "Thinking",
            None,
            look,
            move |id, cx| pick(id, cx),
        ));
    }
    // `AgentModeControl` renders nothing while the agent reports no modes (a closed agent).
    if !view.modes.is_empty() {
        let pick = handlers.mode.clone();
        pills.push(choice_pill(
            "composer-mode".into(),
            "Mode".into(),
            Glyph::Icon(IconName::ShieldCheck),
            view.modes,
            view.current_mode,
            "Mode",
            None,
            TriggerLook {
                show_caret: shown.show_carets,
                icon_only: !shown.show_mode_label,
            },
            move |id, cx| pick(id, cx),
        ));
    }
    if shown.aggregate_features {
        if !view.features.is_empty() {
            pills.push(features_menu(view.features, &handlers.features));
        }
    } else {
        pills.extend(feature_pills(
            view.features,
            view.requests,
            look,
            &handlers.features,
            cx,
        ));
    }
    pills
}

#[cfg(test)]
mod tests {
    // Named imports: `use super::*` would bring in GPUI's `test` macro over `#[test]`.
    use super::{Glyph, control_presence, pill_label, provider_glyph};
    use crate::agent_controls::layout::FeaturePresence;
    use bachuc_client::{AgentFeature, AgentFeatureKind, AgentFeatureOption};
    use bachuc_icons::IconName;
    use gpui_kit::{Hsla, Rgba};

    #[test]
    fn the_provider_logo_or_the_fallback_icon_is_drawn() {
        let color: Hsla = Rgba {
            r: 1.,
            g: 0.5,
            b: 0.,
            a: 0.5,
        }
        .into();
        assert!(matches!(
            provider_glyph(Some("claude"), color),
            Glyph::Image(_)
        ));
        assert!(matches!(
            provider_glyph(Some("codex"), color),
            Glyph::Image(_)
        ));
        assert!(matches!(
            provider_glyph(Some("custom"), color),
            Glyph::Icon(IconName::Bot)
        ));
        assert!(matches!(
            provider_glyph(Some("kiro"), color),
            Glyph::Icon(IconName::PackagePlus)
        ));
        assert!(matches!(
            provider_glyph(None, color),
            Glyph::Icon(IconName::Bot)
        ));
    }

    #[test]
    fn a_select_pill_shows_its_label_until_an_option_is_chosen() {
        let choices = vec![("low".to_string(), "Low".to_string())];
        assert_eq!(pill_label(&choices, None, "Effort"), "Effort");
        assert_eq!(pill_label(&choices, Some("low"), "Effort"), "Low");
    }

    #[test]
    fn pill_label_prefers_the_choice_label() {
        let choices = vec![("a".to_string(), "Alpha".to_string())];
        assert_eq!(pill_label(&choices, Some("a"), "Mode"), "Alpha");
        assert_eq!(pill_label(&choices, Some("b"), "Mode"), "b");
        assert_eq!(pill_label(&choices, None, "Mode"), "Mode");
    }

    #[test]
    fn a_select_feature_counts_the_label_it_shows() {
        let select = AgentFeature {
            id: "effort".into(),
            label: "Effort".into(),
            description: None,
            tooltip: None,
            icon: None,
            kind: AgentFeatureKind::Select {
                value: Some("high".into()),
                options: vec![AgentFeatureOption {
                    id: "high".into(),
                    label: "High".into(),
                    description: None,
                    is_default: None,
                    metadata: Default::default(),
                }],
            },
        };
        let toggle = AgentFeature {
            kind: AgentFeatureKind::Toggle { value: true },
            id: "fast".into(),
            ..select.clone()
        };
        let presence = control_presence(true, false, true, &[toggle, select]);
        assert_eq!(
            presence.features,
            [
                FeaturePresence::Toggle,
                FeaturePresence::Select {
                    label: "High".into()
                }
            ]
        );
    }
}
