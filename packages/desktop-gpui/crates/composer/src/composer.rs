//! The composer view: a multiline input, image chips, mode and model pills, Send, and Stop.

use crate::attachments::{Attachment, can_send, mime_for_path};
use bachuc_client::{AgentControls, Client, ClientEvent};
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{InputEvent, Textarea, TextareaState};
use gpui_kit::component::menu::{DropdownMenu as _, PopupMenuItem};
use gpui_kit::component::{ActiveTheme as _, Disableable as _, Sizable as _, h_flex, v_flex};
use gpui_kit::*;
use std::path::PathBuf;

/// What a `Composer` tells its owner.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ComposerEvent {
    /// A prompt went to the client.
    Sent { text: String, images: usize },
}

/// The message box for one agent. It sends through the client; it never waits for the answer.
pub struct Composer {
    client: Entity<Client>,
    agent_id: String,
    input: Entity<TextareaState>,
    attachments: Vec<Attachment>,
    pasted: u32,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<ComposerEvent> for Composer {}

impl Composer {
    pub fn new(
        client: Entity<Client>,
        agent_id: String,
        window: &mut Window,
        cx: &mut Context<Composer>,
    ) -> Composer {
        let input = cx.new(|cx| {
            TextareaState::new(window, cx)
                .auto_grow(1, 8)
                .placeholder("Message")
                .submit_on_enter(true)
        });
        let subscriptions = vec![
            // Enter sends. Shift+Enter inserts a line and still emits `PressEnter`.
            cx.subscribe_in(
                &input,
                window,
                |this: &mut Composer, _, event: &InputEvent, window, cx| {
                    if enter_sends(event) {
                        this.submit(window, cx);
                    }
                },
            ),
            // Controls (modes, models, running) arrive with the agent list.
            cx.subscribe(&client, |_: &mut Composer, _, event: &ClientEvent, cx| {
                if matches!(event, ClientEvent::AgentsChanged | ClientEvent::Connected) {
                    cx.notify();
                }
            }),
        ];
        Composer {
            client,
            agent_id,
            input,
            attachments: Vec::new(),
            pasted: 0,
            _subscriptions: subscriptions,
        }
    }

    pub fn agent_id(&self) -> &str {
        &self.agent_id
    }

    pub fn focus(&self, window: &mut Window, cx: &mut App) {
        self.input.update(cx, |input, cx| input.focus(window, cx));
    }

    pub fn attachment_count(&self) -> usize {
        self.attachments.len()
    }

    fn submit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let value = self.input.read(cx).value();
        let Some(text) = outgoing_text(&value, self.attachments.len()) else {
            return;
        };
        let images: Vec<_> = self.attachments.drain(..).map(|a| a.to_json()).collect();
        let count = images.len();
        let agent_id = self.agent_id.clone();
        let outgoing = text.clone();
        self.client.update(cx, |client, cx| {
            client.send_prompt(&agent_id, outgoing, images, cx)
        });
        self.input
            .update(cx, |input, cx| input.set_value("", window, cx));
        cx.emit(ComposerEvent::Sent {
            text,
            images: count,
        });
        cx.notify();
    }

    fn stop(&mut self, cx: &mut Context<Self>) {
        let agent_id = self.agent_id.clone();
        self.client
            .update(cx, |client, cx| client.cancel_agent(&agent_id, cx));
    }

    fn pick_images(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let picked = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: true,
            prompt: Some("Attach".into()),
        });
        cx.spawn_in(window, async move |this, cx| {
            let Ok(Ok(Some(paths))) = picked.await else {
                return;
            };
            let loaded = cx
                .background_executor()
                .spawn(async move { load_images(paths) })
                .await;
            let _ = this.update(cx, |this, cx| {
                this.attachments.extend(loaded);
                cx.notify();
            });
        })
        .detach();
    }

    /// Takes images and copied image files from the clipboard. Returns whether it took the paste.
    fn paste(&mut self, item: &ClipboardItem, cx: &mut Context<Self>) -> bool {
        let mut took = false;
        for entry in item.entries() {
            match entry {
                ClipboardEntry::Image(image) => {
                    let mime = image.format.mime_type();
                    if !matches!(
                        mime,
                        "image/png" | "image/jpeg" | "image/gif" | "image/webp"
                    ) {
                        log::warn!("cannot attach a pasted {mime} image");
                        continue;
                    }
                    self.pasted += 1;
                    let name = format!("pasted-{}.{}", self.pasted, image.format.extension());
                    self.attachments
                        .push(Attachment::new(name, mime, &image.bytes));
                    took = true;
                }
                ClipboardEntry::ExternalPaths(paths) => {
                    let images = load_images(paths.paths().to_vec());
                    took |= !images.is_empty();
                    self.attachments.extend(images);
                }
                ClipboardEntry::String(_) => {}
            }
        }
        if took {
            cx.notify();
        }
        took
    }

    fn controls(&self, cx: &App) -> Option<AgentControls> {
        self.client.read(cx).agent_controls(&self.agent_id)
    }

    fn mode_pill(&self, controls: Option<&AgentControls>) -> impl IntoElement + use<> {
        let (modes, current) = controls
            .map(|c| (c.modes.clone(), c.current_mode.clone()))
            .unwrap_or_default();
        let label = pill_label(&modes, current.as_deref(), "Mode");
        let client = self.client.clone();
        let agent_id = self.agent_id.clone();
        pill("composer-mode", &label, "Mode", modes.is_empty()).dropdown_menu_with_anchor(
            Anchor::BottomLeft,
            move |mut menu, _, _| {
                for (id, name) in &modes {
                    let (client, agent_id, id) = (client.clone(), agent_id.clone(), id.clone());
                    menu = menu.item(
                        PopupMenuItem::new(name.clone())
                            .checked(current.as_deref() == Some(id.as_str()))
                            .on_click(move |_, _, cx| {
                                client.update(cx, |c, cx| c.set_mode(&agent_id, &id, cx))
                            }),
                    );
                }
                menu
            },
        )
    }

    fn model_pill(&self, controls: Option<&AgentControls>) -> impl IntoElement + use<> {
        let (models, current) = controls
            .map(|c| (c.models.clone(), c.current_model.clone()))
            .unwrap_or_default();
        let label = pill_label(&models, current.as_deref(), "Model");
        let client = self.client.clone();
        let agent_id = self.agent_id.clone();
        pill("composer-model", &label, "Model", models.is_empty()).dropdown_menu_with_anchor(
            Anchor::BottomLeft,
            move |menu, _, _| {
                let mut menu = menu.scrollable(true).max_h(px(320.));
                for (id, name) in &models {
                    let (client, agent_id, id) = (client.clone(), agent_id.clone(), id.clone());
                    menu = menu.item(
                        PopupMenuItem::new(name.clone())
                            .checked(current.as_deref() == Some(id.as_str()))
                            .on_click(move |_, _, cx| {
                                client.update(cx, |c, cx| c.set_model(&agent_id, &id, cx))
                            }),
                    );
                }
                menu
            },
        )
    }
}

/// Enter sends and Shift+Enter does not. The textarea inserts the new line itself in the
/// Shift case, because it runs with `submit_on_enter`; it still emits `PressEnter` then.
fn enter_sends(event: &InputEvent) -> bool {
    matches!(event, InputEvent::PressEnter { shift: false, .. })
}

/// The text to send, or `None` when there is nothing to send: blank text and no image.
fn outgoing_text(value: &str, images: usize) -> Option<String> {
    can_send(value, images).then(|| value.trim_end().to_string())
}

/// The label of the current choice, or the id when the list lacks it, or `fallback`.
fn pill_label(choices: &[(String, String)], current: Option<&str>, fallback: &str) -> String {
    match current {
        Some(id) => choices
            .iter()
            .find(|(choice, _)| choice == id)
            .map(|(_, label)| label.clone())
            .unwrap_or_else(|| id.to_string()),
        None => fallback.to_string(),
    }
}

fn pill(id: &'static str, label: &str, kind: &str, disabled: bool) -> Button {
    Button::new(id)
        .ghost()
        .xsmall()
        .label(label.to_string())
        .accessibility_id(id)
        .accessibility_label(format!("{kind}: {label}"))
        .disabled(disabled)
}

fn load_images(paths: Vec<PathBuf>) -> Vec<Attachment> {
    paths
        .iter()
        .filter(|path| mime_for_path(path).is_some())
        .filter_map(|path| Attachment::from_path(path))
        .collect()
}

impl Render for Composer {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let controls = self.controls(cx);
        let running = controls.as_ref().is_some_and(|c| c.running);
        let sendable = can_send(&self.input.read(cx).value(), self.attachments.len());
        let theme = cx.theme();
        let weak = cx.weak_entity();

        let chips = (!self.attachments.is_empty()).then(|| {
            h_flex()
                .flex_wrap()
                .gap_1()
                .children(self.attachments.iter().enumerate().map(|(index, a)| {
                    h_flex()
                        .id(("composer-chip", index))
                        .gap_1()
                        .pl_2()
                        .rounded_md()
                        .border_1()
                        .border_color(theme.border)
                        .text_xs()
                        .child(a.chip_label())
                        .child(
                            Button::new(("composer-chip-remove", index))
                                .ghost()
                                .xsmall()
                                .label("x")
                                .accessibility_label(format!("Remove {}", a.name))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    if index < this.attachments.len() {
                                        this.attachments.remove(index);
                                        cx.notify();
                                    }
                                })),
                        )
                }))
        });

        v_flex()
            .w_full()
            .gap_2()
            .p_2()
            .border_t_1()
            .border_color(theme.border)
            .children(chips)
            .child(
                Textarea::new(&self.input)
                    .w_full()
                    .accessibility_id("composer-input")
                    .aria_label("Message")
                    .on_paste(move |item, _, cx| {
                        weak.update(cx, |this, cx| this.paste(item, cx))
                            .unwrap_or(false)
                    }),
            )
            .child(
                h_flex()
                    .gap_1()
                    .child(
                        Button::new("composer-attach")
                            .ghost()
                            .xsmall()
                            .label("Attach")
                            .accessibility_label("Attach images")
                            .on_click(
                                cx.listener(|this, _, window, cx| this.pick_images(window, cx)),
                            ),
                    )
                    .child(self.mode_pill(controls.as_ref()))
                    .child(self.model_pill(controls.as_ref()))
                    .child(div().flex_1())
                    .child(
                        Button::new("composer-stop")
                            .xsmall()
                            .outline()
                            .label("Stop")
                            .accessibility_label("Stop")
                            .disabled(!running)
                            .on_click(cx.listener(|this, _, _, cx| this.stop(cx))),
                    )
                    .child(
                        Button::new("composer-send")
                            .xsmall()
                            .primary()
                            .label("Send")
                            .accessibility_label("Send")
                            .disabled(!sendable)
                            .on_click(cx.listener(|this, _, window, cx| this.submit(window, cx))),
                    ),
            )
    }
}

#[cfg(test)]
mod tests {
    // Named imports: `use super::*` would bring in GPUI's `test` macro over `#[test]`.
    use super::{enter_sends, outgoing_text, pill_label};
    use gpui_kit::component::input::InputEvent;

    fn press(secondary: bool, shift: bool) -> InputEvent {
        InputEvent::PressEnter { secondary, shift }
    }

    #[test]
    fn enter_sends_and_shift_enter_does_not() {
        assert!(enter_sends(&press(false, false)));
        assert!(enter_sends(&press(true, false)));
        assert!(!enter_sends(&press(false, true)));
        assert!(!enter_sends(&InputEvent::Change));
    }

    #[test]
    fn empty_text_with_no_image_sends_nothing() {
        assert_eq!(outgoing_text("", 0), None);
        assert_eq!(outgoing_text(" \n\t", 0), None);
    }

    #[test]
    fn text_keeps_its_lines_and_loses_trailing_space() {
        assert_eq!(outgoing_text("one\ntwo\n", 0), Some("one\ntwo".into()));
        assert_eq!(outgoing_text("  indented", 0), Some("  indented".into()));
    }

    #[test]
    fn an_image_alone_sends_empty_text() {
        assert_eq!(outgoing_text("", 1), Some(String::new()));
        assert_eq!(outgoing_text("  ", 2), Some(String::new()));
    }

    #[test]
    fn pill_label_prefers_the_choice_label() {
        let choices = vec![("a".to_string(), "Alpha".to_string())];
        assert_eq!(pill_label(&choices, Some("a"), "Mode"), "Alpha");
        assert_eq!(pill_label(&choices, Some("b"), "Mode"), "b");
        assert_eq!(pill_label(&choices, None, "Mode"), "Mode");
    }
}
