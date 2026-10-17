//! The "Prompt translate" settings screen, as `TranslateSettingsScreen` of `client/settings.tsx`.
//! It is built only from `bachuc_plugin_host::ui`.

use crate::settings::{ChineseScript, Mode, Provider, RewriteMode, Settings};
use bachuc_plugin_host::ui::{
    RowText, action_row, card, input_row, note, section, select_row, switch_row,
};
use bachuc_plugin_host::{PluginSettings, SelectOption, SettingsState};
use gpui_kit::component::input::{InputEvent, InputState};
use gpui_kit::component::v_flex;
use gpui_kit::*;

const SECTION: &str = "Prompt translate";

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
        SettingsState::Loading => Screen::Note("Loading prompt translate settings…".into()),
        SettingsState::Error(error) => Screen::Note(format!(
            "Prompt translate settings are unavailable: {error}"
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

/// The two models the inputs stand for (translate, enhance).
pub(crate) fn model_drafts(stored: &Settings, translate: &str, enhance: &str) -> (String, String) {
    (
        draft_model(translate, &stored.translate_model),
        draft_model(enhance, &stored.enhance_model),
    )
}

/// `Save models` is enabled only when a trimmed draft differs from the stored value.
pub(crate) fn models_dirty(stored: &Settings, translate: &str, enhance: &str) -> bool {
    let (translate, enhance) = model_drafts(stored, translate, enhance);
    translate != stored.translate_model || enhance != stored.enhance_model
}

fn options(choices: &[(&'static str, &'static str)]) -> Vec<SelectOption> {
    choices
        .iter()
        .map(|(id, label)| SelectOption {
            id: (*id).into(),
            label: (*label).into(),
            description: None,
        })
        .collect()
}

fn mode_choices() -> Vec<SelectOption> {
    Mode::ALL
        .into_iter()
        .map(|mode| SelectOption {
            id: mode.value().into(),
            label: mode.label().into(),
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

/// The change handler of a switch row: saves the record with `change` applied.
fn saver(
    settings: &Entity<PluginSettings>,
    change: impl Fn(&mut Settings, bool) + 'static,
) -> impl Fn(bool, &mut Window, &mut App) + 'static {
    let settings = settings.clone();
    move |on, _, cx| {
        if let Some(save) = save_with(&settings, |next| change(next, on), cx) {
            save.detach();
        }
    }
}

/// The change handler of a select row.
fn select_saver(
    settings: &Entity<PluginSettings>,
    change: impl Fn(&mut Settings, &str) + 'static,
) -> impl Fn(&str, &mut Window, &mut App) + 'static {
    let settings = settings.clone();
    move |value, _, cx| {
        if let Some(save) = save_with(&settings, |next| change(next, value), cx) {
            save.detach();
        }
    }
}

pub(crate) struct PromptTranslateSettingsView {
    settings: Entity<PluginSettings>,
    translate_model: Entity<InputState>,
    enhance_model: Entity<InputState>,
    /// The inputs take the stored models once, when the settings first load.
    seeded: bool,
    _subscriptions: Vec<Subscription>,
}

impl PromptTranslateSettingsView {
    pub(crate) fn new(
        settings: Entity<PluginSettings>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let translate_model = cx.new(|cx| InputState::new(window, cx));
        let enhance_model = cx.new(|cx| InputState::new(window, cx));
        // Typing changes whether `Save models` is enabled.
        let mut subscriptions = Vec::new();
        for input in [&translate_model, &enhance_model] {
            subscriptions.push(cx.subscribe_in(
                input,
                window,
                |_, _, event: &InputEvent, _, cx| {
                    if matches!(event, InputEvent::Change) {
                        cx.notify();
                    }
                },
            ));
        }
        subscriptions.push(cx.observe_in(&settings, window, |this, _, window, cx| {
            this.seed(window, cx);
            cx.notify();
        }));
        let mut view = Self {
            settings,
            translate_model,
            enhance_model,
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
        if self.seeded {
            return;
        }
        if self.stored(cx).is_some() {
            self.seeded = true;
            self.show_stored_models(window, cx);
        }
    }

    fn show_stored_models(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(stored) = self.stored(cx) else {
            return;
        };
        self.translate_model.update(cx, |input, cx| {
            input.set_value(stored.translate_model, window, cx)
        });
        self.enhance_model.update(cx, |input, cx| {
            input.set_value(stored.enhance_model, window, cx)
        });
    }

    fn model_texts(&self, cx: &App) -> (String, String) {
        (
            self.translate_model.read(cx).value().to_string(),
            self.enhance_model.read(cx).value().to_string(),
        )
    }

    /// `Save models`. A successful save shows the saved models in the inputs, which clears the
    /// drafts.
    fn save_models(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(stored) = self.stored(cx) else {
            return;
        };
        let (translate, enhance) = self.model_texts(cx);
        let (translate, enhance) = model_drafts(&stored, &translate, &enhance);
        let Some(save) = save_with(
            &self.settings,
            |next| {
                next.translate_model = translate;
                next.enhance_model = enhance;
            },
            cx,
        ) else {
            return;
        };
        cx.spawn_in(window, async move |this, cx| {
            if save.await {
                let _ = this.update_in(cx, |this, window, cx| this.show_stored_models(window, cx));
            }
        })
        .detach();
    }
}

impl Render for PromptTranslateSettingsView {
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
                        "prompt-translate-reset",
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
                let (translate_text, enhance_text) = self.model_texts(cx);
                let dirty = models_dirty(&values, &translate_text, &enhance_text);
                let view = cx.entity();
                v_flex()
                    .w_full()
                    .child(section(
                        SECTION,
                        card(
                            vec![
                                switch_row(
                                    "prompt-translate-translate",
                                    RowText {
                                        label: "Translate Vietnamese prompts".into(),
                                        hint: Some(
                                            "Show an English translation under each new Vietnamese prompt."
                                                .into(),
                                        ),
                                        error: save_error.map(Into::into),
                                    },
                                    values.translate,
                                    disabled,
                                    saver(&settings, |next, on| next.translate = on),
                                    cx,
                                ),
                                switch_row(
                                    "prompt-translate-enhance",
                                    RowText {
                                        label: "Rewrite drafts as clear English".into(),
                                        hint: Some(
                                            "Default for composers that have not chosen yet. Change it per conversation in the composer's Prompt pill."
                                                .into(),
                                        ),
                                        error: None,
                                    },
                                    values.enhance_shortcut,
                                    disabled,
                                    saver(&settings, |next, on| next.enhance_shortcut = on),
                                    cx,
                                ),
                                select_row(
                                    "prompt-translate-rewrite-mode",
                                    RowText {
                                        label: "Rewrite mode".into(),
                                        hint: Some(
                                            "Automatic: Enter and the send button rewrite; Cmd+Enter sends your draft as typed. Cmd+Enter: only Cmd+Enter rewrites; Enter and the send button send your draft as typed."
                                                .into(),
                                        ),
                                        error: None,
                                    },
                                    options(&[("automatic", "Automatic"), ("cmd-enter", "Cmd+Enter")]),
                                    values.rewrite_mode.value().into(),
                                    disabled || !values.enhance_shortcut,
                                    select_saver(&settings, |next, value| {
                                        if let Some(mode) = RewriteMode::parse(value) {
                                            next.rewrite_mode = mode;
                                        }
                                    }),
                                    cx,
                                ),
                            ],
                            cx,
                        ),
                        cx,
                    ))
                    .child(section(
                        "Reply language",
                        card(
                            vec![
                                switch_row(
                                    "prompt-translate-match-language",
                                    RowText {
                                        label: "Match original language".into(),
                                        hint: Some(
                                            "Your language choice comes first; otherwise follow active modes, then Vietnamese for Vietnamese drafts."
                                                .into(),
                                        ),
                                        error: None,
                                    },
                                    values.match_reply_language,
                                    disabled,
                                    saver(&settings, |next, on| next.match_reply_language = on),
                                    cx,
                                ),
                                select_row(
                                    "prompt-translate-caveman-mode",
                                    RowText {
                                        label: "New-thread Caveman mode".into(),
                                        hint: Some(
                                            "New agents start here. Each agent keeps the mode chosen in its composer's Prompt pill."
                                                .into(),
                                        ),
                                        error: None,
                                    },
                                    mode_choices(),
                                    values.caveman_mode.value().into(),
                                    disabled,
                                    select_saver(&settings, |next, value| {
                                        if let Some(mode) = Mode::parse(value) {
                                            next.caveman_mode = mode;
                                        }
                                    }),
                                    cx,
                                ),
                                select_row(
                                    "prompt-translate-chinese-script",
                                    RowText {
                                        label: "Chinese script".into(),
                                        hint: Some(
                                            "For Wenyan modes; explicit requests in your draft take priority."
                                                .into(),
                                        ),
                                        error: None,
                                    },
                                    options(&[
                                        ("skill-default", "Skill default"),
                                        ("simplified", "Simplified Chinese"),
                                    ]),
                                    values.chinese_script.value().into(),
                                    disabled,
                                    select_saver(&settings, |next, value| {
                                        if let Some(script) = ChineseScript::parse(value) {
                                            next.chinese_script = script;
                                        }
                                    }),
                                    cx,
                                ),
                            ],
                            cx,
                        ),
                        cx,
                    ))
                    .child(section(
                        "Models",
                        card(
                            vec![
                                select_row(
                                    "prompt-translate-provider",
                                    RowText {
                                        label: "Provider".into(),
                                        hint: Some(
                                            "Prompts are sent to this third-party service.".into(),
                                        ),
                                        error: None,
                                    },
                                    options(&[
                                        ("openrouter", "OpenRouter"),
                                        ("vercel", "Vercel AI Gateway"),
                                    ]),
                                    values.provider.value().into(),
                                    disabled,
                                    select_saver(&settings, |next, value| {
                                        if let Some(provider) = Provider::parse(value) {
                                            next.provider = provider;
                                        }
                                    }),
                                    cx,
                                ),
                                input_row(
                                    RowText {
                                        label: "Translate model".into(),
                                        ..Default::default()
                                    },
                                    &self.translate_model,
                                    false,
                                    cx,
                                ),
                                input_row(
                                    RowText {
                                        label: "Enhance model".into(),
                                        ..Default::default()
                                    },
                                    &self.enhance_model,
                                    false,
                                    cx,
                                ),
                                action_row(
                                    "prompt-translate-save-models",
                                    RowText {
                                        label: "Save models".into(),
                                        hint: Some(
                                            "Use model IDs exactly as the provider lists them."
                                                .into(),
                                        ),
                                        error: None,
                                    },
                                    "Save",
                                    disabled || !dirty,
                                    move |window, cx| {
                                        view.update(cx, |this, cx| this.save_models(window, cx));
                                    },
                                    cx,
                                ),
                            ],
                            cx,
                        ),
                        cx,
                    ))
                    .into_any_element()
            }
        };
        div().w_full().child(body)
    }
}

#[cfg(test)]
mod tests {
    // Not `super::*`: the glob brings in gpui's `test` macro, which shadows `#[test]`.
    use super::{Screen, models_dirty, screen};
    use crate::settings::Settings;
    use bachuc_plugin_host::SettingsState;
    use serde_json::json;

    #[test]
    fn the_screen_follows_the_settings_state() {
        assert_eq!(
            screen(&SettingsState::Loading, false, None),
            Screen::Note("Loading prompt translate settings…".into())
        );
        assert_eq!(
            screen(&SettingsState::Error("offline".into()), false, None),
            Screen::Note("Prompt translate settings are unavailable: offline".into())
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
            values: json!({ "translate": false }),
            revision: "r".into(),
        };
        assert_eq!(
            screen(&ready, false, None),
            Screen::Ready {
                values: Settings {
                    translate: false,
                    ..Settings::default()
                },
                save_error: None,
                disabled: false,
            }
        );
    }

    #[test]
    fn save_models_needs_a_trimmed_draft_that_differs() {
        let stored = Settings::default();
        let model = stored.translate_model.clone();
        assert!(!models_dirty(&stored, &model, &model));
        // An empty or blank draft stands for the stored model.
        assert!(!models_dirty(&stored, "", "   "));
        assert!(!models_dirty(&stored, &format!("  {model} "), &model));
        assert!(models_dirty(&stored, "other/model", &model));
        assert!(models_dirty(&stored, &model, " other/model "));
    }
}
