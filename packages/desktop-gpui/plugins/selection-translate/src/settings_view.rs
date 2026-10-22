//! The "Selection translate" settings screen: the provider and the model. It is built only from
//! `bachuc_plugin_host::ui`, like the prompt-translate screen.

use crate::settings::{Provider, Settings};
use bachuc_plugin_host::ui::{RowText, action_row, card, input_row, note, section, select_row};
use bachuc_plugin_host::{PluginSettings, SelectOption, SettingsState};
use gpui_kit::component::input::{InputEvent, InputState};
use gpui_kit::*;

const SECTION: &str = "Selection translate";

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
        values: Settings,
        save_error: Option<String>,
        disabled: bool,
    },
}

pub(crate) fn screen(state: &SettingsState, saving: bool, save_error: Option<&str>) -> Screen {
    let save_error = save_error.map(str::to_string);
    match state {
        SettingsState::Loading => Screen::Note("Loading selection translate settings…".into()),
        SettingsState::Error(error) => Screen::Note(format!(
            "Selection translate settings are unavailable: {error}"
        )),
        SettingsState::Invalid { error, .. } => Screen::Invalid {
            error: error.clone(),
            save_error,
            disabled: saving,
        },
        SettingsState::Ready { values, .. } => Screen::Ready {
            values: Settings::decode(values),
            save_error,
            disabled: saving,
        },
    }
}

/// The model a draft stands for: its trimmed text, or the stored model while it is empty.
fn draft_model(text: &str, stored: &str) -> String {
    let text = text.trim();
    if text.is_empty() { stored } else { text }.to_string()
}

/// `Save` is enabled only when the trimmed draft differs from the stored model.
pub(crate) fn model_dirty(stored: &Settings, draft: &str) -> bool {
    draft_model(draft, &stored.model) != stored.model
}

fn provider_options() -> Vec<SelectOption> {
    [
        (Provider::OpenRouter, "OpenRouter"),
        (Provider::Vercel, "Vercel AI Gateway"),
    ]
    .into_iter()
    .map(|(provider, label)| SelectOption {
        id: provider.value().into(),
        label: label.into(),
        description: None,
    })
    .collect()
}

/// Saves the record on screen with `change` applied, against the revision on screen. `None`
/// while nothing is loaded.
fn save_with(
    settings: &Entity<PluginSettings>,
    change: impl FnOnce(&mut Settings),
    cx: &mut App,
) -> Option<Task<bool>> {
    settings.update(cx, |settings, cx| {
        let SettingsState::Ready { values, revision } = settings.state().clone() else {
            return None;
        };
        let mut next = Settings::decode(&values);
        change(&mut next);
        Some(settings.save(next.to_value(), revision, cx))
    })
}

pub(crate) struct SelectionTranslateSettingsView {
    settings: Entity<PluginSettings>,
    model: Entity<InputState>,
    /// The input takes the stored model once, when the settings first load.
    seeded: bool,
    _subscriptions: Vec<Subscription>,
}

impl SelectionTranslateSettingsView {
    pub(crate) fn new(
        settings: Entity<PluginSettings>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let model = cx.new(|cx| InputState::new(window, cx));
        // Typing changes whether `Save` is enabled.
        let subscriptions = vec![
            cx.subscribe_in(&model, window, |_, _, event: &InputEvent, _, cx| {
                if matches!(event, InputEvent::Change) {
                    cx.notify();
                }
            }),
            cx.observe_in(&settings, window, |this, _, window, cx| {
                this.seed(window, cx);
                cx.notify();
            }),
        ];
        let mut view = Self {
            settings,
            model,
            seeded: false,
            _subscriptions: subscriptions,
        };
        view.seed(window, cx);
        view
    }

    fn stored(&self, cx: &App) -> Option<Settings> {
        match self.settings.read(cx).state() {
            SettingsState::Ready { values, .. } => Some(Settings::decode(values)),
            _ => None,
        }
    }

    fn seed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.seeded && self.stored(cx).is_some() {
            self.seeded = true;
            self.show_stored_model(window, cx);
        }
    }

    fn show_stored_model(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(stored) = self.stored(cx) {
            self.model
                .update(cx, |input, cx| input.set_value(stored.model, window, cx));
        }
    }

    /// `Save`. A successful save shows the saved model in the input, which clears the draft.
    fn save_model(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(stored) = self.stored(cx) else {
            return;
        };
        let model = draft_model(&self.model.read(cx).value(), &stored.model);
        let Some(save) = save_with(&self.settings, |next| next.model = model, cx) else {
            return;
        };
        cx.spawn_in(window, async move |this, cx| {
            if save.await {
                let _ = this.update_in(cx, |this, window, cx| this.show_stored_model(window, cx));
            }
        })
        .detach();
    }
}

impl Render for SelectionTranslateSettingsView {
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
                SECTION,
                card(
                    vec![action_row(
                        "selection-translate-reset",
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
                values,
                save_error,
                disabled,
            } => {
                let dirty = model_dirty(&values, &self.model.read(cx).value());
                let view = cx.entity();
                section(
                    SECTION,
                    card(
                        vec![
                            select_row(
                                "selection-translate-provider",
                                RowText {
                                    label: "Provider".into(),
                                    hint: Some(
                                        "Selected text is sent to this third-party service.".into(),
                                    ),
                                    error: save_error.map(Into::into),
                                },
                                provider_options(),
                                values.provider.value().into(),
                                disabled,
                                {
                                    let settings = settings.clone();
                                    move |value, _, cx| {
                                        let Some(provider) = Provider::parse(value) else {
                                            return;
                                        };
                                        if let Some(save) = save_with(
                                            &settings,
                                            |next| next.provider = provider,
                                            cx,
                                        ) {
                                            save.detach();
                                        }
                                    }
                                },
                                cx,
                            ),
                            input_row(
                                RowText {
                                    label: "Model".into(),
                                    ..Default::default()
                                },
                                &self.model,
                                false,
                                cx,
                            ),
                            action_row(
                                "selection-translate-save-model",
                                RowText {
                                    label: "Save model".into(),
                                    hint: Some(
                                        "Use the model ID exactly as the provider lists it.".into(),
                                    ),
                                    error: None,
                                },
                                "Save",
                                disabled || !dirty,
                                move |window, cx| {
                                    view.update(cx, |this, cx| this.save_model(window, cx));
                                },
                                cx,
                            ),
                        ],
                        cx,
                    ),
                    cx,
                )
                .into_any_element()
            }
        };
        div().w_full().child(body)
    }
}

#[cfg(test)]
mod tests {
    // Not `super::*`: the glob brings in gpui's `test` macro, which shadows `#[test]`.
    use super::{Screen, model_dirty, screen};
    use crate::settings::Settings;
    use bachuc_plugin_host::SettingsState;
    use serde_json::json;

    #[test]
    fn the_screen_follows_the_settings_state() {
        assert_eq!(
            screen(&SettingsState::Loading, false, None),
            Screen::Note("Loading selection translate settings…".into())
        );
        assert_eq!(
            screen(&SettingsState::Error("offline".into()), false, None),
            Screen::Note("Selection translate settings are unavailable: offline".into())
        );
        let ready = SettingsState::Ready {
            values: json!({ "model": "a/b" }),
            revision: "r".into(),
        };
        assert_eq!(
            screen(&ready, true, Some("failed")),
            Screen::Ready {
                values: Settings {
                    model: "a/b".into(),
                    ..Settings::default()
                },
                save_error: Some("failed".into()),
                disabled: true,
            }
        );
    }

    #[test]
    fn save_needs_a_trimmed_draft_that_differs() {
        let stored = Settings::default();
        assert!(!model_dirty(&stored, &stored.model));
        // An empty or blank draft stands for the stored model.
        assert!(!model_dirty(&stored, "  "));
        assert!(!model_dirty(&stored, &format!(" {} ", stored.model)));
        assert!(model_dirty(&stored, " other/model "));
    }
}
