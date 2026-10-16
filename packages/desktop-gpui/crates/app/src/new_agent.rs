//! The new-agent draft tab: the Electron draft, a composer with a provider and a model choice,
//! opened from the new-tab launcher. It replaces the old new-agent dialog. In a workspace the
//! agent starts in the workspace folder; with no workspace the draft asks for a folder.

use bachuc_client::{Client, ClientEvent, ProviderSummary};
use bachuc_theme::metrics::{
    FONT_BASE, FONT_LG, FONT_SM, MAX_CONTENT_WIDTH, RADIUS_2XL, RADIUS_LG, SPACE_1, SPACE_2,
    SPACE_3, SPACE_4, WEIGHT_MEDIUM,
};
use bachuc_theme::{tokens, ui_font_size};
use gpui_kit::component::input::{Input, InputEvent, InputState, Textarea, TextareaState};
use gpui_kit::component::theme::Theme;
use gpui_kit::component::{h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

/// The draft made an agent; the owner replaces the draft tab with the agent tab.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Created {
    pub key: String,
    pub agent_id: String,
}

pub struct DraftPane {
    client: Entity<Client>,
    key: String,
    /// The workspace folder. None shows the folder field.
    cwd: Option<String>,
    folder: Entity<InputState>,
    prompt: Entity<TextareaState>,
    /// The chosen provider id. None picks the first available one.
    provider: Option<String>,
    /// The chosen model id. None uses the provider default.
    model: Option<String>,
    /// The client's provider list as last drawn, so the pane redraws only when it changes.
    providers: Vec<ProviderSummary>,
    error: Option<String>,
    creating: bool,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<Created> for DraftPane {}

impl DraftPane {
    pub fn new(
        client: Entity<Client>,
        key: String,
        cwd: Option<String>,
        window: &mut Window,
        cx: &mut Context<DraftPane>,
    ) -> DraftPane {
        let folder = cx.new(|cx| InputState::new(window, cx).placeholder("Folder"));
        let prompt = cx.new(|cx| {
            TextareaState::new(window, cx)
                .auto_grow(3, 12)
                .placeholder("Message the agent")
                .submit_on_enter(true)
        });
        let subscriptions = vec![
            // Enter sends; Shift+Enter inserts a line and still emits `PressEnter`.
            cx.subscribe_in(
                &prompt,
                window,
                |this: &mut DraftPane, _, event: &InputEvent, window, cx| match event {
                    InputEvent::PressEnter { shift: false, .. } => this.create(window, cx),
                    // The Create button follows the text.
                    InputEvent::Change => cx.notify(),
                    _ => {}
                },
            ),
            cx.subscribe(&folder, |_, _, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Change) {
                    cx.notify();
                }
            }),
            cx.subscribe(&client, |this: &mut DraftPane, client, event, cx| {
                if matches!(event, ClientEvent::AgentsChanged | ClientEvent::Connected) {
                    let providers = client.read(cx).providers();
                    if this.providers != providers {
                        this.providers = providers.to_vec();
                        cx.notify();
                    }
                }
            }),
            cx.observe_global::<Theme>(|_, cx| cx.notify()),
        ];
        let providers = client.read(cx).providers().to_vec();
        DraftPane {
            client,
            key,
            cwd,
            folder,
            prompt,
            provider: None,
            model: None,
            providers,
            error: None,
            creating: false,
            _subscriptions: subscriptions,
        }
    }

    pub fn focus(&self, window: &mut Window, cx: &mut App) {
        self.prompt
            .update(cx, |prompt, cx| prompt.focus(window, cx));
    }

    fn folder_value(&self, cx: &App) -> String {
        match &self.cwd {
            Some(cwd) => cwd.clone(),
            None => self.folder.read(cx).value().trim().to_string(),
        }
    }

    fn create(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let folder = self.folder_value(cx);
        let prompt = self.prompt.read(cx).value().trim().to_string();
        let provider = pick_provider(self.provider.as_deref(), &self.providers);
        if !can_create(provider, &folder, &prompt, self.creating) {
            return;
        }
        let Some(provider) = provider else {
            return;
        };
        let provider_id = provider.id.clone();
        let model = pick_model(self.model.as_deref(), provider).map(str::to_string);
        self.creating = true;
        self.error = None;
        let task = self.client.update(cx, |client, cx| {
            client.create_agent(&provider_id, model.as_deref(), &folder, &prompt, cx)
        });
        cx.spawn_in(window, async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |this, cx| {
                this.creating = false;
                match result {
                    Ok(agent_id) => cx.emit(Created {
                        key: this.key.clone(),
                        agent_id,
                    }),
                    Err(error) => this.error = Some(format!("{error:#}")),
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    /// A choice pill. The chosen one says so in its label, because a button's selected state
    /// does not reach VoiceOver (a11y audit).
    fn pill(
        &self,
        id: (&'static str, usize),
        text: String,
        label: String,
        chosen: bool,
        enabled: bool,
        cx: &App,
    ) -> Stateful<Div> {
        let t = tokens(cx);
        div()
            .id(id)
            .role(Role::Button)
            .aria_label(if chosen {
                format!("{label}, selected")
            } else {
                label
            })
            .aria_selected(chosen)
            .px(px(SPACE_2))
            .py(px(SPACE_1))
            .rounded(px(RADIUS_LG))
            .border_1()
            .border_color(if chosen { t.border_accent } else { t.border })
            .text_size(ui_font_size(FONT_SM, cx))
            .text_color(if chosen {
                t.foreground
            } else {
                t.foreground_muted
            })
            .when(chosen, |pill| pill.bg(t.surface2))
            .when(!enabled, |pill| pill.opacity(0.5))
            .when(enabled, |pill| {
                pill.cursor_pointer().hover(|style| style.bg(t.surface2))
            })
            .child(text)
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

/// True when the draft has everything `create` needs.
pub fn can_create(
    provider: Option<&ProviderSummary>,
    folder: &str,
    prompt: &str,
    creating: bool,
) -> bool {
    provider.is_some() && !folder.trim().is_empty() && !prompt.trim().is_empty() && !creating
}

impl Render for DraftPane {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = tokens(cx);
        let provider = pick_provider(self.provider.as_deref(), &self.providers).cloned();
        let model = provider
            .as_ref()
            .and_then(|provider| pick_model(self.model.as_deref(), provider).map(str::to_string));
        let folder = self.folder_value(cx);
        let prompt = self.prompt.read(cx).value();
        let ready = can_create(provider.as_ref(), &folder, &prompt, self.creating);

        let provider_pills: Vec<Stateful<Div>> = self
            .providers
            .iter()
            .enumerate()
            .map(|(ix, entry)| {
                let chosen = provider.as_ref().is_some_and(|p| p.id == entry.id);
                let label = if entry.available {
                    format!("Provider {}", entry.label)
                } else {
                    format!("Provider {}, unavailable", entry.label)
                };
                let id = entry.id.clone();
                self.pill(
                    ("draft-provider", ix),
                    entry.label.clone(),
                    label,
                    chosen,
                    entry.available,
                    cx,
                )
                .when(entry.available, |pill| {
                    pill.on_click(cx.listener(move |this, _, _, cx| {
                        this.provider = Some(id.clone());
                        this.model = None;
                        cx.notify();
                    }))
                })
            })
            .collect();

        let models: Vec<(Option<String>, String)> = provider
            .iter()
            .flat_map(|provider| {
                std::iter::once((None, "Default".to_string())).chain(
                    provider
                        .models
                        .iter()
                        .map(|(id, label)| (Some(id.clone()), label.clone())),
                )
            })
            .collect();
        let model_pills: Vec<Stateful<Div>> = models
            .into_iter()
            .enumerate()
            .map(|(ix, (id, label))| {
                let chosen = id == model;
                self.pill(
                    ("draft-model", ix),
                    label.clone(),
                    format!("Model {label}"),
                    chosen,
                    true,
                    cx,
                )
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.model = id.clone();
                    cx.notify();
                }))
            })
            .collect();

        let heading = |text: &'static str| {
            div()
                .text_size(ui_font_size(FONT_SM, cx))
                .text_color(t.foreground_extra_muted)
                .child(text)
        };
        let selection = match (&provider, &model) {
            (Some(provider), Some(model)) => format!("{} · {model}", provider.label),
            (Some(provider), None) => format!("{} · Default model", provider.label),
            (None, _) => "No provider available".to_string(),
        };

        let send_fg = if ready {
            t.accent_foreground
        } else {
            t.foreground_muted
        };
        let card = v_flex()
            .id("draft-composer")
            .role(Role::Group)
            .aria_label(format!("New agent, {selection}"))
            .w_full()
            .max_w(px(MAX_CONTENT_WIDTH))
            .gap(px(SPACE_3))
            .p(px(SPACE_3))
            .rounded(px(RADIUS_2XL))
            .border_1()
            .border_color(t.border)
            .bg(t.surface1)
            .when(self.cwd.is_none(), |card| {
                card.child(heading("Folder")).child(
                    Input::new(&self.folder)
                        .accessibility_id("draft-folder")
                        .aria_label("Folder"),
                )
            })
            .child(
                Textarea::new(&self.prompt)
                    .w_full()
                    .accessibility_id("draft-prompt")
                    .aria_label("Prompt"),
            )
            .child(heading("Provider"))
            .child(
                h_flex()
                    .id("draft-providers")
                    .role(Role::Group)
                    .aria_label("Providers")
                    .flex_wrap()
                    .gap(px(SPACE_1))
                    .children(provider_pills),
            )
            .child(heading("Model"))
            .child(
                h_flex()
                    .id("draft-models")
                    .role(Role::Group)
                    .aria_label("Models")
                    .flex_wrap()
                    .gap(px(SPACE_1))
                    .max_h(px(120.))
                    .overflow_y_scroll()
                    .children(model_pills),
            )
            .children(self.error.clone().map(|message| {
                div()
                    .id("draft-error")
                    .role(Role::Alert)
                    .aria_label(format!("Error: {message}"))
                    .text_size(ui_font_size(FONT_SM, cx))
                    .text_color(t.destructive)
                    .child(message)
            }))
            .child(
                h_flex()
                    .justify_between()
                    .items_center()
                    .gap(px(SPACE_2))
                    .child(
                        div()
                            .id("draft-selection")
                            .role(Role::Status)
                            .aria_label(format!("Selected: {selection}"))
                            .min_w_0()
                            .truncate()
                            .text_size(ui_font_size(FONT_SM, cx))
                            .text_color(t.foreground_muted)
                            .child(selection.clone()),
                    )
                    .child(
                        div()
                            .id("draft-create")
                            .role(Role::Button)
                            .aria_label("Create agent")
                            .flex_shrink_0()
                            .px(px(SPACE_3))
                            .py(px(SPACE_1))
                            .rounded(px(RADIUS_LG))
                            .text_size(ui_font_size(FONT_BASE, cx))
                            .text_color(send_fg)
                            .when(ready, |button| {
                                button.bg(t.accent).cursor_pointer().on_click(
                                    cx.listener(|this, _, window, cx| this.create(window, cx)),
                                )
                            })
                            .when(!ready, |button| button.bg(t.surface2))
                            .child(if self.creating {
                                "Creating..."
                            } else {
                                "Create"
                            }),
                    ),
            );

        let where_text = self.cwd.clone().unwrap_or_else(|| "Pick a folder".into());
        v_flex()
            .id("pane-draft")
            .role(Role::Group)
            .aria_label("New agent")
            .size_full()
            .p(px(SPACE_4))
            .child(
                v_flex()
                    .flex_1()
                    .min_h_0()
                    .items_center()
                    .justify_center()
                    .gap(px(SPACE_1))
                    .child(
                        div()
                            .text_size(ui_font_size(FONT_LG, cx))
                            .font_weight(FontWeight(WEIGHT_MEDIUM))
                            .text_color(t.foreground)
                            .child("New agent"),
                    )
                    .child(
                        div()
                            .text_size(ui_font_size(FONT_BASE, cx))
                            .text_color(t.foreground_muted)
                            .child(where_text),
                    ),
            )
            .child(h_flex().w_full().justify_center().child(card))
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
