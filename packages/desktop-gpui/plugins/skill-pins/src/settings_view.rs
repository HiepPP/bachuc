//! The "Skill pins" settings screen, as `SkillPinsSettingsScreen` of `client/settings.tsx`. It
//! is built only from `bachuc_plugin_host::ui`.

use crate::catalog::{CATALOG, saved_skills, set_default, skill_hint};
use bachuc_plugin_host::ui::{RowText, action_row, card, note, section, switch_row};
use bachuc_plugin_host::{PluginSettings, SettingsState};
use gpui_kit::*;
use serde_json::json;

const SECTION_INVALID: &str = "Skill pins";
const SECTION_READY: &str = "Default pins for new agents";

/// What the screen shows.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Screen {
    Note(String),
    Invalid {
        error: String,
        save_error: Option<String>,
        disabled: bool,
    },
    Ready {
        defaults: Vec<String>,
        save_error: Option<String>,
        disabled: bool,
    },
}

pub(crate) fn screen(state: &SettingsState, saving: bool, save_error: Option<&str>) -> Screen {
    let save_error = save_error.map(str::to_string);
    match state {
        SettingsState::Loading => Screen::Note("Loading skill pins settings…".into()),
        SettingsState::Error(error) => {
            Screen::Note(format!("Skill pins settings are unavailable: {error}"))
        }
        SettingsState::Invalid { error, .. } => Screen::Invalid {
            error: error.clone(),
            save_error,
            disabled: saving,
        },
        SettingsState::Ready { values, .. } => Screen::Ready {
            defaults: saved_skills(values.get("defaults")),
            save_error,
            disabled: saving,
        },
    }
}

pub(crate) struct SkillPinsSettingsView {
    settings: Entity<PluginSettings>,
    _observe: Subscription,
}

impl SkillPinsSettingsView {
    pub(crate) fn new(settings: Entity<PluginSettings>, cx: &mut Context<Self>) -> Self {
        let observe = cx.observe(&settings, |_, _, cx| cx.notify());
        Self {
            settings,
            _observe: observe,
        }
    }
}

impl Render for SkillPinsSettingsView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let shown = {
            let settings = self.settings.read(cx);
            screen(settings.state(), settings.saving(), settings.save_error())
        };
        let settings = self.settings.clone();
        let body = match shown {
            Screen::Note(text) => note(text, cx),
            Screen::Invalid {
                error,
                save_error,
                disabled,
            } => section(
                SECTION_INVALID,
                card(
                    vec![action_row(
                        "skill-pins-reset",
                        RowText {
                            label: "Stored settings are invalid".into(),
                            hint: Some(error.into()),
                            error: save_error.map(Into::into),
                        },
                        "Reset to defaults",
                        disabled,
                        move |_, cx| {
                            settings.update(cx, |settings, cx| settings.reset(cx).detach());
                        },
                        cx,
                    )],
                    cx,
                ),
                cx,
            ),
            Screen::Ready {
                defaults,
                save_error,
                disabled,
            } => {
                let rows = CATALOG
                    .iter()
                    .enumerate()
                    .map(|(index, skill)| {
                        let settings = settings.clone();
                        let defaults = defaults.clone();
                        switch_row(
                            ("skill-pins-default", index),
                            RowText {
                                label: skill.label.into(),
                                hint: Some(skill_hint(skill).into()),
                                error: save_error.clone().map(Into::into),
                            },
                            defaults.iter().any(|id| id == skill.id),
                            disabled,
                            move |on, _, cx| {
                                settings.update(cx, |settings, cx| {
                                    // Saves against the revision on screen.
                                    if let SettingsState::Ready { revision, .. } =
                                        settings.state().clone()
                                    {
                                        let next = set_default(&defaults, skill.id, on);
                                        settings
                                            .save(json!({ "defaults": next }), revision, cx)
                                            .detach();
                                    }
                                });
                            },
                            cx,
                        )
                    })
                    .collect();
                section(SECTION_READY, card(rows, cx), cx)
            }
        };
        div().w_full().child(body)
    }
}

#[cfg(test)]
mod tests {
    // Not `super::*`: the glob brings in gpui's `test` macro, which shadows `#[test]`.
    use super::{Screen, screen};
    use bachuc_plugin_host::SettingsState;
    use serde_json::json;

    #[test]
    fn the_screen_follows_the_settings_state() {
        assert_eq!(
            screen(&SettingsState::Loading, false, None),
            Screen::Note("Loading skill pins settings…".into())
        );
        assert_eq!(
            screen(&SettingsState::Error("offline".into()), false, None),
            Screen::Note("Skill pins settings are unavailable: offline".into())
        );
        assert_eq!(
            screen(
                &SettingsState::Invalid {
                    error: "bad".into(),
                    revision: "r".into()
                },
                true,
                Some("failed")
            ),
            Screen::Invalid {
                error: "bad".into(),
                save_error: Some("failed".into()),
                disabled: true,
            }
        );
        let ready = SettingsState::Ready {
            values: json!({ "defaults": ["old-skill", "sequential-thinking", "watchtower"] }),
            revision: "r".into(),
        };
        assert_eq!(
            screen(&ready, false, None),
            Screen::Ready {
                defaults: vec!["watchtower".into(), "sequential-thinking".into()],
                save_error: None,
                disabled: false,
            }
        );
    }
}
