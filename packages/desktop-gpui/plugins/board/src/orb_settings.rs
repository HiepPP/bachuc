//! The "Thinking orb" settings screen, as `OrbSettingsScreen` of `client/orb-settings.tsx`,
//! built from the plugin host's settings rows.

use crate::model::OrbState;
use crate::model::settings::orb_settings;
use bachuc_plugin_host::ui::{RowText, action_row, card, note, section, select_row, switch_row};
use bachuc_plugin_host::{PluginSettings, SelectOption, SettingsState};
use gpui_kit::*;
use serde_json::Value;

const TITLE: &str = "Thinking orb";

/// The Animation select: one option per state, labeled with the capitalized id.
pub(crate) fn state_options() -> Vec<SelectOption> {
    OrbState::ALL
        .into_iter()
        .map(|state| {
            let id = state.id();
            let mut label = id[..1].to_uppercase();
            label.push_str(&id[1..]);
            SelectOption {
                id: id.into(),
                label: label.into(),
                description: None,
            }
        })
        .collect()
}

/// The Avatar opacity select: 100% down to 20% in steps of 10.
pub(crate) fn opacity_options() -> Vec<SelectOption> {
    (2..=10)
        .rev()
        .map(|step| {
            let value = step * 10;
            SelectOption {
                id: value.to_string().into(),
                label: format!("{value}%").into(),
                description: None,
            }
        })
        .collect()
}

/// `{ ...values, ...change }`: the stored values with one key replaced.
pub(crate) fn with_value(values: &Value, key: &str, value: Value) -> Value {
    let mut next = values.as_object().cloned().unwrap_or_default();
    next.insert(key.to_string(), value);
    Value::Object(next)
}

pub(crate) struct OrbSettingsView {
    settings: Entity<PluginSettings>,
    _observe: Subscription,
}

impl OrbSettingsView {
    pub(crate) fn new(settings: Entity<PluginSettings>, cx: &mut Context<Self>) -> Self {
        let observe = cx.observe(&settings, |_, _, cx| cx.notify());
        Self {
            settings,
            _observe: observe,
        }
    }
}

/// Saves one key against the revision on screen.
fn save(settings: &Entity<PluginSettings>, key: &'static str, value: Value, cx: &mut App) {
    settings.update(cx, |settings, cx| {
        if let SettingsState::Ready { values, revision } = settings.state().clone() {
            settings
                .save(with_value(&values, key, value), revision, cx)
                .detach();
        }
    });
}

impl Render for OrbSettingsView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (state, saving, save_error) = {
            let settings = self.settings.read(cx);
            (
                settings.state().clone(),
                settings.saving(),
                settings
                    .save_error()
                    .map(|error| SharedString::from(error.to_string())),
            )
        };
        let settings = self.settings.clone();
        let body = match state {
            SettingsState::Loading => note("Loading thinking orb settings…", cx),
            SettingsState::Error(error) => note(
                format!("Thinking orb settings are unavailable: {error}"),
                cx,
            ),
            SettingsState::Invalid { error, .. } => section(
                TITLE,
                card(
                    vec![action_row(
                        "board-orb-reset",
                        RowText {
                            label: "Stored settings are invalid".into(),
                            hint: Some(error.into()),
                            error: save_error,
                        },
                        "Reset to defaults",
                        saving,
                        move |_, cx| {
                            settings.update(cx, |settings, cx| settings.reset(cx).detach());
                        },
                        cx,
                    )],
                    cx,
                ),
                cx,
            ),
            SettingsState::Ready { values, .. } => {
                let orb = orb_settings(&values);
                let off = saving || !orb.enabled;
                let (on_switch, on_state, on_opacity) =
                    (settings.clone(), settings.clone(), settings);
                section(
                    TITLE,
                    card(
                        vec![
                            switch_row(
                                "board-orb-enabled",
                                RowText {
                                    label: "Show thinking orb".into(),
                                    hint: Some(
                                        "Running Board cards on desktop and web. Mobile keeps \
                                         the spinner."
                                            .into(),
                                    ),
                                    error: None,
                                },
                                orb.enabled,
                                saving,
                                move |enabled, _, cx| {
                                    save(&on_switch, "enabled", Value::Bool(enabled), cx);
                                },
                                cx,
                            ),
                            select_row(
                                "board-orb-state",
                                RowText {
                                    label: "Animation".into(),
                                    hint: Some("The avatar edge follows the orb outline.".into()),
                                    error: None,
                                },
                                state_options(),
                                orb.state.id().into(),
                                off,
                                move |state, _, cx| {
                                    save(&on_state, "state", Value::from(state), cx);
                                },
                                cx,
                            ),
                            select_row(
                                "board-orb-opacity",
                                RowText {
                                    label: "Avatar opacity".into(),
                                    hint: Some("Lower values make the orb stand out.".into()),
                                    error: save_error,
                                },
                                opacity_options(),
                                orb.avatar_opacity.to_string().into(),
                                off,
                                move |value, _, cx| {
                                    if let Ok(value) = value.parse::<u8>() {
                                        save(&on_opacity, "avatarOpacity", Value::from(value), cx);
                                    }
                                },
                                cx,
                            ),
                        ],
                        cx,
                    ),
                    cx,
                )
            }
        };
        div().w_full().child(body)
    }
}

#[cfg(test)]
mod tests {
    // Not a glob: `gpui_kit::*` brings GPUI's own `test` attribute.
    use super::{opacity_options, state_options, with_value};
    use serde_json::json;

    #[test]
    fn the_selects_offer_the_seven_states_and_nine_opacities() {
        let states: Vec<(String, String)> = state_options()
            .into_iter()
            .map(|option| (option.id.to_string(), option.label.to_string()))
            .collect();
        assert_eq!(states.len(), 7);
        assert_eq!(states[0], ("solving".into(), "Solving".into()));
        assert_eq!(states[6], ("connecting".into(), "Connecting".into()));
        let opacities: Vec<String> = opacity_options()
            .into_iter()
            .map(|option| option.label.to_string())
            .collect();
        assert_eq!(
            opacities,
            [
                "100%", "90%", "80%", "70%", "60%", "50%", "40%", "30%", "20%"
            ]
        );
        assert_eq!(opacity_options()[8].id, "20");
    }

    #[test]
    fn a_change_keeps_the_other_values() {
        let values = json!({"enabled": true, "state": "solving", "avatarOpacity": 100});
        assert_eq!(
            with_value(&values, "avatarOpacity", json!(50)),
            json!({"enabled": true, "state": "solving", "avatarOpacity": 50})
        );
        assert_eq!(
            with_value(&json!(null), "enabled", json!(false)),
            json!({"enabled": false})
        );
    }
}
