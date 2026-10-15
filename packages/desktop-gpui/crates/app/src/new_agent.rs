//! The new-agent dialog: a provider, a model, a folder, and a first prompt.

use crate::actions::{CloseDialogAction, DIALOG_CONTEXT};
use crate::shell::Shell;
use bachuc_client::ProviderSummary;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{Input, InputEvent, InputState, Textarea, TextareaState};
use gpui_kit::component::{
    ActiveTheme as _, Disableable as _, Selectable as _, Sizable as _, h_flex, v_flex,
};
use gpui_kit::*;

/// The state of the open dialog.
pub struct NewAgent {
    /// The chosen provider id. None picks the first available one.
    pub provider: Option<String>,
    /// The chosen model id. None uses the provider default.
    pub model: Option<String>,
    pub folder: Entity<InputState>,
    pub prompt: Entity<TextareaState>,
    pub error: Option<String>,
    pub creating: bool,
    _subscription: Subscription,
}

impl NewAgent {
    pub fn new(default_folder: String, window: &mut Window, cx: &mut Context<Shell>) -> Self {
        let folder = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Folder")
                .default_value(default_folder)
        });
        let prompt = cx.new(|cx| {
            TextareaState::new(window, cx)
                .auto_grow(3, 8)
                .placeholder("Prompt")
                .submit_on_enter(true)
        });
        // Enter sends the prompt; Shift+Enter inserts a line and still emits `PressEnter`.
        let subscription = cx.subscribe_in(
            &prompt,
            window,
            |shell: &mut Shell, _, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::PressEnter { shift: false, .. }) {
                    shell.create_agent(window, cx);
                }
            },
        );
        prompt.update(cx, |prompt, cx| prompt.focus(window, cx));
        NewAgent {
            provider: None,
            model: None,
            folder,
            prompt,
            error: None,
            creating: false,
            _subscription: subscription,
        }
    }

    pub fn provider<'a>(&self, providers: &'a [ProviderSummary]) -> Option<&'a ProviderSummary> {
        pick_provider(self.provider.as_deref(), providers)
    }

    pub fn model<'a>(&'a self, provider: &ProviderSummary) -> Option<&'a str> {
        pick_model(self.model.as_deref(), provider)
    }
}

/// The chosen provider, or the first available one. An unavailable provider is never picked.
pub fn pick_provider<'a>(
    chosen: Option<&str>,
    providers: &'a [ProviderSummary],
) -> Option<&'a ProviderSummary> {
    match chosen {
        Some(id) => providers.iter().find(|provider| provider.id == id),
        None => providers.iter().find(|provider| provider.available),
    }
    .filter(|provider| provider.available)
}

/// The chosen model id when the provider has it; None means the provider default.
pub fn pick_model<'a>(chosen: Option<&'a str>, provider: &ProviderSummary) -> Option<&'a str> {
    let id = chosen?;
    provider
        .models
        .iter()
        .any(|(model, _)| model == id)
        .then_some(id)
}

/// True when the dialog has everything `create_agent` needs.
pub fn can_create(
    provider: Option<&ProviderSummary>,
    folder: &str,
    prompt: &str,
    creating: bool,
) -> bool {
    provider.is_some() && !folder.trim().is_empty() && !prompt.trim().is_empty() && !creating
}

impl Shell {
    pub(crate) fn render_new_agent(&mut self, cx: &mut Context<Self>) -> Option<AnyElement> {
        // The shell calls this on every frame, so it returns before it copies anything.
        self.new_agent.as_ref()?;
        let theme = cx.theme().clone();
        let providers: Vec<ProviderSummary> = self.client.read(cx).providers().to_vec();
        let dialog = self.new_agent.as_ref()?;
        let provider = dialog.provider(&providers).cloned();
        let model = provider
            .as_ref()
            .and_then(|provider| dialog.model(provider).map(str::to_string));
        let folder_state = dialog.folder.clone();
        let prompt_state = dialog.prompt.clone();
        let error = dialog.error.clone();
        let creating = dialog.creating;
        let folder = folder_state.read(cx).value();
        let prompt = prompt_state.read(cx).value();
        let ready = can_create(provider.as_ref(), &folder, &prompt, creating);

        let provider_buttons = providers.iter().enumerate().map(|(ix, entry)| {
            let id = entry.id.clone();
            let chosen = provider.as_ref().is_some_and(|p| p.id == entry.id);
            Button::new(("new-agent-provider", ix))
                .small()
                .outline()
                .label(entry.label.clone())
                .accessibility_label(if entry.available {
                    format!("Provider {}", entry.label)
                } else {
                    format!("Provider {}, unavailable", entry.label)
                })
                .selected(chosen)
                .disabled(!entry.available)
                .on_click(cx.listener(move |this, _, _, cx| {
                    if let Some(dialog) = this.new_agent.as_mut() {
                        dialog.provider = Some(id.clone());
                        dialog.model = None;
                        cx.notify();
                    }
                }))
        });

        let model_buttons = provider.iter().flat_map(|provider| {
            let default = std::iter::once((None, "Default".to_string()));
            let models = provider
                .models
                .iter()
                .map(|(id, label)| (Some(id.clone()), label.clone()));
            default.chain(models).collect::<Vec<_>>()
        });
        let model_buttons = model_buttons.enumerate().map(|(ix, (id, label))| {
            let chosen = id == model;
            Button::new(("new-agent-model", ix))
                .small()
                .outline()
                .label(label.clone())
                .accessibility_label(format!("Model {label}"))
                .selected(chosen)
                .on_click(cx.listener(move |this, _, _, cx| {
                    if let Some(dialog) = this.new_agent.as_mut() {
                        dialog.model = id.clone();
                        cx.notify();
                    }
                }))
        });

        let field = |label: &'static str| {
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(label)
        };

        let card = v_flex()
            .id("new-agent-dialog")
            .role(Role::Dialog)
            .aria_label("New agent")
            .key_context(DIALOG_CONTEXT)
            .w(px(520.))
            .max_h(relative(0.9))
            .gap_3()
            .p_4()
            .rounded_lg()
            .border_1()
            .border_color(theme.border)
            .bg(theme.popover)
            .text_color(theme.popover_foreground)
            .on_action(cx.listener(|this, _: &CloseDialogAction, _, cx| this.close_new_agent(cx)))
            .child(div().font_weight(FontWeight::SEMIBOLD).child("New agent"))
            .child(field("Provider"))
            .child(
                h_flex()
                    .id("new-agent-providers")
                    .role(Role::Group)
                    .aria_label("Providers")
                    .flex_wrap()
                    .gap_1()
                    .children(provider_buttons),
            )
            .child(field("Model"))
            .child(
                h_flex()
                    .id("new-agent-models")
                    .role(Role::Group)
                    .aria_label("Models")
                    .flex_wrap()
                    .gap_1()
                    .max_h(px(140.))
                    .overflow_y_scroll()
                    .children(model_buttons),
            )
            .child(field("Folder"))
            .child(
                h_flex()
                    .gap_1()
                    .child(
                        div().flex_1().child(
                            Input::new(&folder_state)
                                .accessibility_id("new-agent-folder")
                                .aria_label("Folder"),
                        ),
                    )
                    .child(
                        Button::new("new-agent-browse")
                            .small()
                            .outline()
                            .label("Browse")
                            .accessibility_label("Browse for a folder")
                            .on_click(
                                cx.listener(|this, _, window, cx| this.pick_folder(window, cx)),
                            ),
                    ),
            )
            .child(field("Prompt"))
            .child(
                Textarea::new(&prompt_state)
                    .w_full()
                    .accessibility_id("new-agent-prompt")
                    .aria_label("Prompt"),
            )
            .children(error.map(|message| {
                div()
                    .id("new-agent-error")
                    .role(Role::Alert)
                    .aria_label(format!("Error: {message}"))
                    .text_sm()
                    .text_color(theme.danger)
                    .child(message)
            }))
            .child(
                h_flex()
                    .justify_end()
                    .gap_2()
                    .child(
                        Button::new("new-agent-cancel")
                            .outline()
                            .label("Cancel")
                            .accessibility_label("Cancel new agent")
                            .on_click(cx.listener(|this, _, _, cx| this.close_new_agent(cx))),
                    )
                    .child(
                        Button::new("new-agent-create")
                            .primary()
                            .label(if creating { "Creating..." } else { "Create" })
                            .accessibility_label("Create agent")
                            .disabled(!ready)
                            .on_click(
                                cx.listener(|this, _, window, cx| this.create_agent(window, cx)),
                            ),
                    ),
            );

        Some(
            div()
                .id("new-agent-backdrop")
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .bg(theme.overlay)
                .occlude()
                .child(card)
                .into_any_element(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{can_create, pick_model, pick_provider};
    use bachuc_client::ProviderSummary;

    fn provider(id: &str, available: bool) -> ProviderSummary {
        ProviderSummary {
            id: id.into(),
            label: id.into(),
            available,
            models: vec![("m1".into(), "Model one".into())],
        }
    }

    #[test]
    fn create_needs_a_provider_a_folder_and_a_prompt() {
        let mock = provider("mock", true);
        assert!(can_create(Some(&mock), "/tmp", "hi", false));
        assert!(!can_create(None, "/tmp", "hi", false));
        assert!(!can_create(Some(&mock), " ", "hi", false));
        assert!(!can_create(Some(&mock), "/tmp", "\n", false));
        assert!(!can_create(Some(&mock), "/tmp", "hi", true));
    }

    #[test]
    fn an_unavailable_provider_is_never_chosen() {
        let providers = [provider("claude", false), provider("mock", true)];
        assert_eq!(
            pick_provider(None, &providers).map(|p| p.id.as_str()),
            Some("mock")
        );
        assert!(pick_provider(Some("claude"), &providers).is_none());
        assert_eq!(
            pick_provider(Some("mock"), &providers).map(|p| p.id.as_str()),
            Some("mock")
        );
        assert!(pick_provider(None, &[provider("claude", false)]).is_none());
    }

    #[test]
    fn a_model_the_provider_lacks_falls_back_to_the_default() {
        let mock = provider("mock", true);
        assert_eq!(pick_model(Some("m1"), &mock), Some("m1"));
        assert_eq!(pick_model(Some("other"), &mock), None);
        assert_eq!(pick_model(None, &mock), None);
    }
}
