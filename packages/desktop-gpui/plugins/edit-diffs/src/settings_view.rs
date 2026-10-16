//! The "Edit diffs" settings screen, as `DisplaySettingsScreen` of `client/settings.tsx`.

use crate::state::{Screen, screen, with_auto_expand};
use bachuc_plugin_host::ui::{RowText, action_row, card, note, section, switch_row};
use bachuc_plugin_host::{PluginSettings, SettingsState};
use gpui_kit::*;

const TITLE: &str = "Edit diffs";
const AUTO_EXPAND_HINT: &str =
    "Show each edit's diff open in the chat. When off, press the row to open it.";

pub(crate) struct DisplaySettingsView {
    settings: Entity<PluginSettings>,
    _observe: Subscription,
}

impl DisplaySettingsView {
    pub(crate) fn new(settings: Entity<PluginSettings>, cx: &mut Context<Self>) -> Self {
        let observe = cx.observe(&settings, |_, _, cx| cx.notify());
        Self {
            settings,
            _observe: observe,
        }
    }
}

impl Render for DisplaySettingsView {
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
                        "edit-diffs-reset",
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
                auto_expand,
                save_error,
                disabled,
            } => section(
                TITLE,
                card(
                    vec![switch_row(
                        "edit-diffs-auto-expand",
                        RowText {
                            label: "Auto-expand edit diffs".into(),
                            hint: Some(AUTO_EXPAND_HINT.into()),
                            error: save_error.map(Into::into),
                        },
                        auto_expand,
                        disabled,
                        move |value, _, cx| {
                            settings.update(cx, |settings, cx| {
                                // Saves against the revision on screen.
                                if let SettingsState::Ready { values, revision } =
                                    settings.state().clone()
                                {
                                    let values = with_auto_expand(&values, value);
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
