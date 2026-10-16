//! The "Next prompt actions" settings screen, as `SendSettingsScreen` of `client/settings.tsx`.

use bachuc_plugin_host::ui::{RowText, action_row, card, note, section, switch_row};
use bachuc_plugin_host::{PluginSettings, SettingsState};
use gpui_kit::*;
use serde_json::Value;

const TITLE: &str = "Next prompt actions";
const BACK_TO_BOARD_HINT: &str =
    "Open the Board when sending a suggestion. Requires the Board plugin.";

/// `backToBoard` of the `send` settings; the schema default is `false`.
pub(crate) fn back_to_board(values: &Value) -> bool {
    values
        .get("backToBoard")
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

/// `backToBoard` while the settings are `Ready`, else the default.
pub(crate) fn back_to_board_now(state: &SettingsState) -> bool {
    match state {
        SettingsState::Ready { values, .. } => back_to_board(values),
        _ => false,
    }
}

fn with_back_to_board(values: &Value, on: bool) -> Value {
    let mut values = match values {
        Value::Object(object) => object.clone(),
        _ => serde_json::Map::new(),
    };
    values.insert("backToBoard".into(), on.into());
    Value::Object(values)
}

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
        back_to_board: bool,
        save_error: Option<String>,
        disabled: bool,
    },
}

pub(crate) fn screen(state: &SettingsState, saving: bool, save_error: Option<&str>) -> Screen {
    let save_error = save_error.map(str::to_string);
    match state {
        SettingsState::Loading => Screen::Note("Loading next prompt settings…".into()),
        SettingsState::Error(error) => {
            Screen::Note(format!("Next prompt settings are unavailable: {error}"))
        }
        SettingsState::Invalid { error, .. } => Screen::Invalid {
            error: error.clone(),
            save_error,
            disabled: saving,
        },
        SettingsState::Ready { values, .. } => Screen::Ready {
            back_to_board: back_to_board(values),
            save_error,
            disabled: saving,
        },
    }
}

pub(crate) struct SendSettingsView {
    settings: Entity<PluginSettings>,
    _observe: Subscription,
}

impl SendSettingsView {
    pub(crate) fn new(settings: Entity<PluginSettings>, cx: &mut Context<Self>) -> Self {
        let observe = cx.observe(&settings, |_, _, cx| cx.notify());
        Self {
            settings,
            _observe: observe,
        }
    }
}

impl Render for SendSettingsView {
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
                TITLE,
                card(
                    vec![action_row(
                        "next-prompt-reset",
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
                back_to_board,
                save_error,
                disabled,
            } => section(
                TITLE,
                card(
                    vec![switch_row(
                        "next-prompt-back-to-board",
                        RowText {
                            label: "Back to Board after send".into(),
                            hint: Some(BACK_TO_BOARD_HINT.into()),
                            error: save_error.map(Into::into),
                        },
                        back_to_board,
                        disabled,
                        move |value, _, cx| {
                            settings.update(cx, |settings, cx| {
                                // Saves against the revision on screen.
                                if let SettingsState::Ready { values, revision } =
                                    settings.state().clone()
                                {
                                    let values = with_back_to_board(&values, value);
                                    settings.save(values, revision, cx).detach();
                                }
                            });
                        },
                        cx,
                    )],
                    cx,
                ),
                cx,
            ),
        };
        div().w_full().child(body)
    }
}

#[cfg(test)]
mod tests {
    // Not `super::*`: the glob brings in gpui's `test` macro, which shadows `#[test]`.
    use super::{Screen, back_to_board_now, screen, with_back_to_board};
    use bachuc_plugin_host::SettingsState;
    use serde_json::json;

    #[test]
    fn the_screen_follows_the_settings_state() {
        assert_eq!(
            screen(&SettingsState::Loading, false, None),
            Screen::Note("Loading next prompt settings…".into())
        );
        assert_eq!(
            screen(&SettingsState::Error("offline".into()), false, None),
            Screen::Note("Next prompt settings are unavailable: offline".into())
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
        let ready = |values| SettingsState::Ready {
            values,
            revision: "r".into(),
        };
        assert_eq!(
            screen(&ready(json!({})), false, None),
            Screen::Ready {
                back_to_board: false,
                save_error: None,
                disabled: false,
            }
        );
        assert!(back_to_board_now(&ready(json!({"backToBoard": true}))));
        assert!(!back_to_board_now(&SettingsState::Loading));
        assert_eq!(
            with_back_to_board(&json!({"other": 1}), true),
            json!({"other": 1, "backToBoard": true})
        );
    }
}
