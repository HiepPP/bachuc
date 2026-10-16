//! The composer view: the Electron message box. It has a multiline input, image pills, a slash
//! command menu, mode, model, and thinking pills, the queued prompts, and Send or Stop.

use crate::actions::{
    PrimaryAction, SendPlan, line_height, max_input_rows, primary_action, send_plan,
};
use crate::attachments::{Attachment, can_send, mime_for_path};
use crate::slash::{
    ClientCommand, SlashItem, arrow_delta, filter_items, insert_text, menu_items,
    resolve_client_command, slash_query, step,
};
use bachuc_client::{
    AgentControls, Client, ClientEvent, QueuedPrompt, SlashCommand, ThinkingControls,
};
use bachuc_icons::{IconName, icon};
use bachuc_theme::metrics::{
    FONT_BASE, FONT_CONTENT, FONT_SM, ICON_SM, RADIUS_2XL, RADIUS_LG, RADIUS_MD, SPACE_1,
    SPACE_1_5, SPACE_2, SPACE_3,
};
use bachuc_theme::{ShadowSize, shadow, tokens, ui_font_size};
use gpui_kit::component::input::{InputEvent, Position, Textarea, TextareaState};
use gpui_kit::component::popover::Popover;
use gpui_kit::component::theme::Theme;
use gpui_kit::component::{Selectable, Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

/// Size of the round toolbar buttons (attach, send, stop) and of the pills.
const CONTROL: f32 = 28.;
/// Electron draws the stop button with `palette.red[600]` in every theme. The theme crate carries
/// that value only as the light terminal red (`ansi[1]`), which is a different role.
const STOP_RED: u32 = 0xdc2626;
/// `composer.placeholders.desktop` in `packages/app/src/i18n/resources/en.ts`.
const PLACEHOLDER: &str = "Message the agent, tag @files, or use /commands and /skills";
/// The model pill icon size: the toolbar glyph size.
const GLYPH: f32 = 14.;
/// The slash list height cap: the `Autocomplete` default `maxHeight`.
const SLASH_MAX_HEIGHT: f32 = 220.;
/// Electron's `ClaudeIcon`, a filled logo. `bachuc_icons` carries only lucide stroke icons, so
/// the composer draws it as an SVG image with the fill color written in.
const CLAUDE_LOGO: &str = include_str!("../assets/claude.svg");

/// What a `Composer` tells its owner.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ComposerEvent {
    /// A prompt went to the client.
    Sent { text: String, images: usize },
    /// `/clear` archived the agent; the owner opens a fresh draft in its place.
    ReplaceWithDraft,
}

/// What the composer draws from the client. The composer redraws only when this changes.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Watched {
    controls: Option<AgentControls>,
    thinking: Option<ThinkingControls>,
    queued: Vec<QueuedPrompt>,
    supports_queue: bool,
    provider: Option<String>,
}

impl Watched {
    fn read(client: &Client, agent_id: &str) -> Self {
        Self {
            provider: client
                .agents()
                .iter()
                .find(|agent| agent.id == agent_id)
                .map(|agent| agent.provider.clone()),
            controls: client.agent_controls(agent_id),
            thinking: client.agent_thinking(agent_id),
            queued: client.queued_prompts(agent_id),
            supports_queue: client.supports_queue(),
        }
    }

    fn running(&self) -> bool {
        self.controls.as_ref().is_some_and(|c| c.running)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CommandsState {
    NotLoaded,
    Loading,
    Loaded,
    /// The agent cannot list commands (a mock agent, or no session yet). The next `/` tries again.
    Failed,
}

/// The message box for one agent. It sends through the client; it never waits for the answer.
pub struct Composer {
    client: Entity<Client>,
    agent_id: String,
    input: Entity<TextareaState>,
    attachments: Vec<Attachment>,
    pasted: u32,
    watched: Watched,
    commands: Vec<SlashCommand>,
    commands_state: CommandsState,
    /// The highlighted row of the slash menu.
    slash_active: usize,
    /// Escape closed the menu for the text being typed.
    slash_dismissed: bool,
    slash_scroll: ScrollHandle,
    /// The provider logo drawn in the last fill color; rebuilt when the theme changes it.
    logo: Option<(Hsla, Arc<Image>)>,
    /// The row limit the input was last given, from the window height.
    max_rows: usize,
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
        let max_rows = max_input_rows(f32::from(window.viewport_size().height), FONT_CONTENT);
        let input = cx.new(|cx| {
            TextareaState::new(window, cx)
                .auto_grow(1, max_rows)
                .placeholder(PLACEHOLDER)
                .submit_on_enter(true)
        });
        let watched = Watched::read(client.read(cx), &agent_id);
        let weak = cx.weak_entity();
        let subscriptions = vec![
            cx.subscribe_in(
                &input,
                window,
                |this: &mut Composer, _, event: &InputEvent, window, cx| match event {
                    InputEvent::Change => this.text_changed(cx),
                    InputEvent::PressEnter { secondary, shift } => {
                        this.enter(*secondary, *shift, window, cx)
                    }
                    _ => {}
                },
            ),
            // Redraw only when what the composer shows changed: not on every agent update.
            cx.subscribe(
                &client,
                |this: &mut Composer, client, event: &ClientEvent, cx| {
                    if matches!(event, ClientEvent::AgentsChanged | ClientEvent::Connected) {
                        let now = Watched::read(client.read(cx), &this.agent_id);
                        if now != this.watched {
                            this.watched = now;
                            cx.notify();
                        }
                    }
                },
            ),
            cx.observe_global::<Theme>(|_, cx| cx.notify()),
            // The textarea takes Up, Down, and Escape for itself, so the slash menu reads them
            // before dispatch, and only while the menu is open and the input has focus.
            cx.intercept_keystrokes(move |event, window, cx| {
                let modifiers = &event.keystroke.modifiers;
                if modifiers.platform || modifiers.control || modifiers.alt || modifiers.shift {
                    return;
                }
                let key = event.keystroke.key.clone();
                let handled = weak
                    .update(cx, |this, cx| this.slash_key(&key, window, cx))
                    .unwrap_or(false);
                if handled {
                    cx.stop_propagation();
                }
            }),
        ];
        Composer {
            client,
            agent_id,
            input,
            attachments: Vec::new(),
            pasted: 0,
            watched,
            commands: Vec::new(),
            commands_state: CommandsState::NotLoaded,
            slash_active: 0,
            slash_dismissed: false,
            slash_scroll: ScrollHandle::new(),
            logo: None,
            max_rows,
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

    fn value(&self, cx: &App) -> String {
        self.input.read(cx).value().to_string()
    }

    fn text_changed(&mut self, cx: &mut Context<Self>) {
        let value = self.value(cx);
        if slash_query(&value).is_some() {
            self.load_commands(cx);
        } else {
            self.slash_dismissed = false;
            if self.commands_state == CommandsState::Failed {
                self.commands_state = CommandsState::NotLoaded;
            }
        }
        // The best match sits at the bottom, next to the input, as Electron pins the list there.
        self.slash_active = 0;
        self.slash_scroll.scroll_to_bottom();
        cx.notify();
    }

    /// Asks the daemon for the agent's commands the first time a `/` is typed.
    fn load_commands(&mut self, cx: &mut Context<Self>) {
        if self.commands_state != CommandsState::NotLoaded {
            return;
        }
        self.commands_state = CommandsState::Loading;
        let agent_id = self.agent_id.clone();
        let task = self
            .client
            .update(cx, |client, cx| client.list_commands(&agent_id, cx));
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |this, cx| {
                match result {
                    Ok(commands) => {
                        this.commands = commands;
                        this.commands_state = CommandsState::Loaded;
                    }
                    Err(error) => {
                        log::debug!("no slash commands for {}: {error}", this.agent_id);
                        this.commands_state = CommandsState::Failed;
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// The menu rows for the text in the input; empty when the menu is closed.
    fn slash_matches(&self, cx: &App) -> Vec<SlashItem> {
        if self.slash_dismissed {
            return Vec::new();
        }
        let value = self.value(cx);
        let Some(query) = slash_query(&value) else {
            return Vec::new();
        };
        let items = menu_items(&self.commands);
        filter_items(&items, query).into_iter().cloned().collect()
    }

    /// Up, Down, Tab, and Escape while the menu is open. Returns whether it took the key.
    fn slash_key(&mut self, key: &str, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if !self.input.read(cx).focus_handle(cx).is_focused(window) {
            return false;
        }
        let matches = self.slash_matches(cx);
        if matches.is_empty() {
            return false;
        }
        let active = self.slash_active.min(matches.len() - 1);
        if let Some(delta) = arrow_delta(key) {
            self.slash_active = step(active, matches.len(), delta);
            // Rows draw worst first, so the match at `slash_active` is row `len - 1 - active`.
            self.slash_scroll
                .scroll_to_item(matches.len() - 1 - self.slash_active);
            cx.notify();
            return true;
        }
        match key {
            "tab" => self.pick(&matches[active], window, cx),
            "escape" => self.slash_dismissed = true,
            _ => return false,
        }
        cx.notify();
        true
    }

    /// Puts the command in the input and puts the caret after it.
    fn pick(&mut self, item: &SlashItem, window: &mut Window, cx: &mut Context<Self>) {
        let text = insert_text(item);
        let end = text.encode_utf16().count() as u32;
        self.input.update(cx, |input, cx| {
            input.set_value(text, window, cx);
            input.set_cursor_position(
                Position {
                    line: 0,
                    character: end,
                },
                window,
                cx,
            );
        });
        self.slash_active = 0;
        cx.notify();
    }

    /// Enter picks a menu row, or sends. Cmd+Enter (`secondary`) queues during a run. The
    /// textarea inserts the new line itself for Shift+Enter, and still reports it here.
    fn enter(&mut self, secondary: bool, shift: bool, window: &mut Window, cx: &mut Context<Self>) {
        if shift {
            return;
        }
        let matches = self.slash_matches(cx);
        if !matches.is_empty() {
            let item = &matches[self.slash_active.min(matches.len() - 1)];
            // A client command that is already complete sends; picking it again would loop.
            if self.value(cx) != insert_text(item) {
                self.pick(item, window, cx);
                return;
            }
        }
        self.submit(secondary, window, cx);
    }

    fn submit(&mut self, queue: bool, window: &mut Window, cx: &mut Context<Self>) {
        let value = self.value(cx);
        if let Some(command) = resolve_client_command(&value, self.attachments.len()) {
            self.run_client_command(command, window, cx);
            return;
        }
        let Some(text) = outgoing_text(&value, self.attachments.len()) else {
            return;
        };
        let images: Vec<_> = self.attachments.drain(..).map(|a| a.to_json()).collect();
        let count = images.len();
        let agent_id = self.agent_id.clone();
        let outgoing = text.clone();
        let (running, supports_queue) = {
            let client = self.client.read(cx);
            let running = client.agent_controls(&agent_id).is_some_and(|c| c.running);
            (running, client.supports_queue())
        };
        let plan = send_plan(running, queue, supports_queue);
        self.client.update(cx, |client, cx| match plan {
            SendPlan::Send(behavior) => {
                client.send_prompt_with(&agent_id, outgoing, images, behavior, cx)
            }
            SendPlan::Queue => client.enqueue_prompt(&agent_id, outgoing, images, cx),
        });
        self.clear_input(window, cx);
        cx.emit(ComposerEvent::Sent {
            text,
            images: count,
        });
    }

    fn run_client_command(
        &mut self,
        command: ClientCommand,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let agent_id = self.agent_id.clone();
        self.client
            .update(cx, |client, cx| client.archive_agent(&agent_id, cx));
        self.clear_input(window, cx);
        if command == ClientCommand::Clear {
            cx.emit(ComposerEvent::ReplaceWithDraft);
        }
    }

    fn clear_input(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.input
            .update(cx, |input, cx| input.set_value("", window, cx));
        self.slash_dismissed = false;
        self.slash_active = 0;
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

    /// The model pill icon: the provider logo, or `Bot` as Electron's `getProviderIcon` falls
    /// back to. Only the Claude logo is carried here.
    fn model_glyph(&mut self, color: Hsla) -> Glyph {
        if self.watched.provider.as_deref() != Some("claude") {
            return Glyph::Icon(IconName::Bot);
        }
        match &self.logo {
            Some((drawn, image)) if *drawn == color => Glyph::Image(image.clone()),
            _ => {
                let image = Arc::new(Image::from_bytes(
                    ImageFormat::Svg,
                    filled_svg(CLAUDE_LOGO, color).into_bytes(),
                ));
                self.logo = Some((color, image.clone()));
                Glyph::Image(image)
            }
        }
    }

    /// The pills, in the Electron order: model, thinking, mode.
    fn pills(&self, model_glyph: Glyph) -> Vec<AnyElement> {
        let controls = self.watched.controls.clone().unwrap_or(AgentControls {
            modes: Vec::new(),
            current_mode: None,
            models: Vec::new(),
            current_model: None,
            running: false,
        });
        let mut pills = Vec::new();

        let (client, agent_id) = (self.client.clone(), self.agent_id.clone());
        pills.push(choice_pill(
            "composer-model",
            "Model",
            model_glyph,
            controls.models,
            controls.current_model,
            "Model",
            move |id, cx| client.update(cx, |c, cx| c.set_model(&agent_id, id, cx)),
        ));

        if let Some(thinking) = self.watched.thinking.clone() {
            let (client, agent_id) = (self.client.clone(), self.agent_id.clone());
            pills.push(choice_pill(
                "composer-thinking",
                "Thinking",
                Glyph::Icon(IconName::Brain),
                thinking.options,
                thinking.current,
                "Thinking",
                move |id, cx| client.update(cx, |c, cx| c.set_thinking(&agent_id, id, cx)),
            ));
        }

        // `AgentModeControl` renders nothing while the agent reports no modes (a closed agent).
        if !controls.modes.is_empty() {
            let (client, agent_id) = (self.client.clone(), self.agent_id.clone());
            pills.push(choice_pill(
                "composer-mode",
                "Mode",
                Glyph::Icon(IconName::ShieldCheck),
                controls.modes,
                controls.current_mode,
                "Mode",
                move |id, cx| client.update(cx, |c, cx| c.set_mode(&agent_id, id, cx)),
            ));
        }
        pills
    }
}

/// The SVG with its `{FILL}` and `{OPACITY}` placeholders set to `color`.
fn filled_svg(template: &str, color: Hsla) -> String {
    let rgba = Rgba::from(color);
    let channel = |value: f32| (value.clamp(0., 1.) * 255.).round() as u8;
    template
        .replace(
            "{FILL}",
            &format!(
                "#{:02x}{:02x}{:02x}",
                channel(rgba.r),
                channel(rgba.g),
                channel(rgba.b)
            ),
        )
        .replace("{OPACITY}", &format!("{:.3}", rgba.a.clamp(0., 1.)))
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

fn load_images(paths: Vec<PathBuf>) -> Vec<Attachment> {
    paths
        .iter()
        .filter(|path| mime_for_path(path).is_some())
        .filter_map(|path| Attachment::from_path(path))
        .collect()
}

/// A round icon button with a role and a label.
fn round_button(
    id: impl Into<ElementId>,
    glyph: IconName,
    label: &'static str,
    size: f32,
    background: Option<Hsla>,
    foreground: Hsla,
    hover: Hsla,
) -> Stateful<Div> {
    div()
        .id(id)
        .role(Role::Button)
        .aria_label(label)
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .size(px(size))
        .rounded_full()
        .cursor_pointer()
        .when_some(background, |button, color| button.bg(color))
        .hover(move |button| button.bg(hover))
        .child(icon(glyph).text_color(foreground))
}

/// The icon of a pill: a lucide icon tinted by the pill, or an image drawn in its own color.
#[derive(Clone)]
enum Glyph {
    Icon(IconName),
    Image(Arc<Image>),
}

/// The trigger of a pill: an icon and the current value, 28 px high and fully round.
#[derive(IntoElement)]
struct PillTrigger {
    id: ElementId,
    kind: &'static str,
    glyph: Glyph,
    label: String,
    disabled: bool,
    open: bool,
}

impl Selectable for PillTrigger {
    fn selected(mut self, selected: bool) -> Self {
        self.open = selected;
        self
    }

    fn is_selected(&self) -> bool {
        self.open
    }
}

impl RenderOnce for PillTrigger {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = tokens(cx);
        let size = ui_font_size(FONT_SM, cx);
        let disabled = self.disabled;
        h_flex()
            .id(self.id)
            .role(Role::Button)
            .aria_label(format!("{}: {}", self.kind, self.label))
            .h(px(CONTROL))
            .px(px(SPACE_2))
            .gap(px(SPACE_1))
            .flex_none()
            .items_center()
            .rounded(px(RADIUS_2XL))
            .text_size(size)
            .text_color(t.foreground_muted)
            .when(self.open, |pill| pill.bg(t.surface2))
            .when(disabled, |pill| pill.opacity(0.5))
            .when(!disabled, |pill| {
                pill.cursor_pointer().hover(|pill| pill.bg(t.surface2))
            })
            .child(match self.glyph {
                Glyph::Icon(name) => icon(name)
                    .with_size(px(GLYPH))
                    .text_color(t.foreground_muted)
                    .into_any_element(),
                Glyph::Image(image) => img(image).flex_none().size(px(GLYPH)).into_any_element(),
            })
            .child(div().truncate().max_w(px(160.)).child(self.label))
    }
}

type PickFn = Rc<dyn Fn(&str, &mut App)>;

/// A pill that opens a menu of choices above itself. With no choices it only shows the label.
fn choice_pill(
    id: &'static str,
    kind: &'static str,
    glyph: Glyph,
    choices: Vec<(String, String)>,
    current: Option<String>,
    fallback: &str,
    on_pick: impl Fn(&str, &mut App) + 'static,
) -> AnyElement {
    let label = pill_label(&choices, current.as_deref(), fallback);
    let trigger = PillTrigger {
        id: id.into(),
        kind,
        glyph,
        label,
        disabled: choices.is_empty(),
        open: false,
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
            let t = tokens(cx);
            let size = ui_font_size(FONT_BASE, cx);
            let current = current.clone();
            let rows = choices.iter().enumerate().map(|(ix, (choice, name))| {
                let selected = current.as_deref() == Some(choice.as_str());
                let (choice, on_pick) = (choice.clone(), on_pick.clone());
                h_flex()
                    .id(("composer-menu-item", ix))
                    .role(Role::MenuItem)
                    .aria_label(name.clone())
                    .aria_selected(selected)
                    .h(px(32.))
                    .px(px(SPACE_2))
                    .gap(px(SPACE_2))
                    .items_center()
                    .rounded(px(RADIUS_MD))
                    .cursor_pointer()
                    .text_size(size)
                    .text_color(t.foreground)
                    .hover(|row| row.bg(t.surface2))
                    .on_click(cx.listener(move |state, _, window, cx| {
                        on_pick(&choice, cx);
                        state.dismiss(window, cx);
                    }))
                    .child(div().flex_1().truncate().child(name.clone()))
                    .when(selected, |row| {
                        row.child(
                            icon(IconName::Check)
                                .with_size(px(ICON_SM))
                                .text_color(t.foreground_muted),
                        )
                    })
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

impl Composer {
    fn slash_menu(
        &self,
        matches: &[SlashItem],
        cx: &mut Context<Self>,
    ) -> Option<impl IntoElement + use<>> {
        if matches.is_empty() {
            return None;
        }
        let t = tokens(cx);
        let size = ui_font_size(FONT_BASE, cx);
        let small = ui_font_size(FONT_SM, cx);
        let active = self.slash_active.min(matches.len() - 1);
        // `matches` is best first. Electron draws it reversed, so the best match sits at the
        // bottom next to the input, and selects it.
        let rows = matches.iter().enumerate().rev().map(|(ix, item)| {
            let picked = item.clone();
            h_flex()
                .id(("composer-slash-item", ix))
                .role(Role::MenuItem)
                .aria_label(format!("/{}, {}", item.name, item.description))
                .aria_selected(ix == active)
                .min_h(px(36.))
                .px(px(SPACE_3))
                .py(px(SPACE_2))
                .gap(px(SPACE_2))
                .items_center()
                .cursor_pointer()
                .text_size(size)
                .when(ix == active, |row| row.bg(t.surface2))
                .hover(|row| row.bg(t.surface2))
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.pick(&picked, window, cx);
                    this.focus(window, cx);
                }))
                .child(
                    div()
                        .flex_none()
                        .text_color(t.foreground)
                        .child(format!("/{}", item.name)),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .text_size(small)
                        .text_color(t.foreground_muted)
                        .child(item.description.clone()),
                )
        });
        // The `detailCard` of `Autocomplete`: the selected command in full, in its own box.
        let selected = &matches[active];
        let detail = (!selected.description.is_empty()).then(|| {
            v_flex()
                .gap(px(SPACE_1))
                .px(px(SPACE_3))
                .py(px(SPACE_3))
                .bg(t.surface1)
                .border_1()
                .border_color(t.border_accent)
                .rounded(px(RADIUS_LG))
                .shadow(vec![shadow(ShadowSize::Md, cx)])
                .occlude()
                .child(
                    div()
                        .text_size(size)
                        .text_color(t.foreground)
                        .child(format!("/{}", selected.name)),
                )
                .child(
                    div()
                        .text_size(small)
                        .text_color(t.foreground_muted)
                        .child(selected.description.clone()),
                )
        });
        // Above the surface, as an overlay: it takes no height from the layout.
        Some(
            v_flex()
                .absolute()
                .bottom_full()
                .left_0()
                .right_0()
                .pb(px(SPACE_1))
                .gap(px(SPACE_1))
                .children(detail)
                .child(
                    div()
                        .bg(t.surface1)
                        .border_1()
                        .border_color(t.border_accent)
                        .rounded(px(RADIUS_LG))
                        .shadow(vec![shadow(ShadowSize::Md, cx)])
                        .overflow_hidden()
                        .occlude()
                        .child(
                            v_flex()
                                .id("composer-slash-menu")
                                .role(Role::Menu)
                                .aria_label("Slash commands")
                                .track_scroll(&self.slash_scroll)
                                .max_h(px(SLASH_MAX_HEIGHT))
                                .overflow_y_scroll()
                                .py(px(SPACE_1))
                                .children(rows),
                        ),
                ),
        )
    }
}

impl Render for Composer {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let rows = max_input_rows(f32::from(window.viewport_size().height), FONT_CONTENT);
        if rows != self.max_rows {
            self.max_rows = rows;
            self.input
                .update(cx, |input, cx| input.set_auto_grow(1, rows, cx));
        }

        let t = tokens(cx);
        let running = self.watched.running();
        let sendable = can_send(&self.value(cx), self.attachments.len());
        let primary = primary_action(sendable, running);
        let content_size = ui_font_size(FONT_CONTENT, cx);
        let matches = self.slash_matches(cx);
        let model_glyph = self.model_glyph(t.foreground_muted);
        let weak = cx.weak_entity();

        let queue = (!self.watched.queued.is_empty()).then(|| {
            v_flex()
                .gap(px(SPACE_2))
                .children(self.watched.queued.iter().enumerate().map(|(ix, queued)| {
                    let queued_id = queued.id.clone();
                    h_flex()
                        .id(("composer-queued", ix))
                        .role(Role::ListItem)
                        .aria_label(format!("Queued: {}", queued.text))
                        .px(px(SPACE_3))
                        .py(px(SPACE_2))
                        .gap(px(SPACE_2))
                        .items_center()
                        .justify_between()
                        .bg(t.surface1)
                        .border_1()
                        .border_color(t.border)
                        .rounded(px(RADIUS_LG))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .line_clamp(2)
                                .text_size(ui_font_size(FONT_BASE, cx))
                                .text_color(t.foreground)
                                .child(queued.text.clone()),
                        )
                        .child(
                            round_button(
                                ("composer-queued-cancel", ix),
                                IconName::X,
                                "Cancel queued message",
                                32.,
                                Some(t.surface2),
                                t.foreground,
                                t.surface3,
                            )
                            .on_click(cx.listener(
                                move |this, _, _, cx| {
                                    let agent_id = this.agent_id.clone();
                                    let queued_id = queued_id.clone();
                                    this.client.update(cx, |client, cx| {
                                        client.cancel_queued(&agent_id, &queued_id, cx)
                                    });
                                },
                            )),
                        )
                }))
        });

        let chips = (!self.attachments.is_empty()).then(|| {
            h_flex()
                .flex_wrap()
                .gap(px(SPACE_2))
                .pt(px(SPACE_2))
                .pl(px(SPACE_2))
                .children(self.attachments.iter().enumerate().map(|(index, a)| {
                    div()
                        .relative()
                        .child(
                            div()
                                .id(("composer-chip", index))
                                .role(Role::ListItem)
                                .aria_label(format!("Attachment {}", a.name))
                                .size(px(48.))
                                .overflow_hidden()
                                .border_1()
                                .border_color(t.border_accent)
                                .rounded(px(RADIUS_MD))
                                .child(
                                    img(a.preview.clone())
                                        .size_full()
                                        .object_fit(ObjectFit::Cover),
                                ),
                        )
                        .child(
                            div().absolute().top(px(-8.)).left(px(-8.)).child(
                                round_button(
                                    ("composer-chip-remove", index),
                                    IconName::X,
                                    "Remove attachment",
                                    24.,
                                    Some(t.surface2),
                                    t.foreground_muted,
                                    t.surface3,
                                )
                                .border_1()
                                .border_color(t.border_accent)
                                .on_click(cx.listener(
                                    move |this, _, _, cx| {
                                        if index < this.attachments.len() {
                                            this.attachments.remove(index);
                                            cx.notify();
                                        }
                                    },
                                )),
                            ),
                        )
                }))
        });

        let primary_button = match primary {
            PrimaryAction::Send => Some(
                round_button(
                    "composer-send",
                    IconName::ArrowUp,
                    "Send",
                    CONTROL,
                    Some(t.accent),
                    t.accent_foreground,
                    t.accent_bright,
                )
                .ml(px(SPACE_1))
                .on_click(cx.listener(|this, _, window, cx| this.submit(false, window, cx)))
                .into_any_element(),
            ),
            PrimaryAction::Stop => Some(
                div()
                    .id("composer-stop")
                    .role(Role::Button)
                    .aria_label("Stop")
                    .flex()
                    .flex_none()
                    .items_center()
                    .justify_center()
                    .size(px(CONTROL))
                    .ml(px(SPACE_1))
                    .rounded_full()
                    .cursor_pointer()
                    .bg(rgb(STOP_RED))
                    .child(div().size(px(10.)).rounded(px(2.)).bg(rgb(0xffffff)))
                    .on_click(cx.listener(|this, _, _, cx| this.stop(cx)))
                    .into_any_element(),
            ),
            PrimaryAction::None => None,
        };

        v_flex()
            .relative()
            .w_full()
            .gap(px(SPACE_2))
            .children(self.slash_menu(&matches, cx))
            .children(queue)
            .child(
                v_flex()
                    .gap(px(SPACE_1_5))
                    .px(px(SPACE_3))
                    .py(px(SPACE_2))
                    .bg(t.surface1)
                    .border_1()
                    .border_color(t.border_accent)
                    .rounded(px(RADIUS_2XL))
                    .children(chips)
                    .child(
                        Textarea::new(&self.input)
                            .w_full()
                            .appearance(false)
                            .bordered(false)
                            .text_size(content_size)
                            .text_color(t.foreground)
                            .line_height(px(line_height(f32::from(content_size))))
                            .accessibility_id("composer-input")
                            .aria_label("Message")
                            .on_paste(move |item, _, cx| {
                                weak.update(cx, |this, cx| this.paste(item, cx))
                                    .unwrap_or(false)
                            }),
                    )
                    .child(
                        h_flex()
                            .mx(px(-6.))
                            .items_center()
                            .justify_between()
                            .child(
                                h_flex()
                                    .min_w_0()
                                    .flex_1()
                                    .items_center()
                                    .child(
                                        round_button(
                                            "composer-attach",
                                            IconName::Plus,
                                            "Attach images",
                                            CONTROL,
                                            None,
                                            t.foreground_muted,
                                            t.surface2,
                                        )
                                        .on_click(
                                            cx.listener(|this, _, window, cx| {
                                                this.pick_images(window, cx)
                                            }),
                                        ),
                                    )
                                    .children(self.pills(model_glyph)),
                            )
                            .children(primary_button),
                    ),
            )
    }
}

#[cfg(test)]
mod tests {
    // Named imports: `use super::*` would bring in GPUI's `test` macro over `#[test]`.
    use super::{CLAUDE_LOGO, filled_svg, outgoing_text, pill_label};
    use gpui_kit::{Hsla, Rgba};

    #[test]
    fn the_logo_takes_the_fill_color() {
        let color: Hsla = Rgba {
            r: 1.,
            g: 0.5,
            b: 0.,
            a: 0.5,
        }
        .into();
        let svg = filled_svg(CLAUDE_LOGO, color);
        assert!(svg.contains(r##"fill="#ff8000""##), "{svg}");
        assert!(svg.contains(r#"fill-opacity="0.500""#), "{svg}");
        assert!(!svg.contains('{'));
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
