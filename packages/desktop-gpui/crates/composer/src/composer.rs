//! The composer view: the Electron message box. It has a multiline input, image pills, a slash
//! command menu, mode, model, and thinking pills, the provider feature controls, the diff-count
//! row, the queued prompts, and Send or Stop.

use crate::actions::{
    PrimaryAction, SendPlan, line_height, max_input_rows, primary_action, send_plan,
};
use crate::attachments::{Attachment, can_send, mime_for_path};
use crate::features::{
    FeatureRequests, Highlight, active_highlight, diff_row, feature_icon, feature_tooltip,
    select_choices, select_current, select_request, toggle_request, visible_diff_stat,
};
use crate::slash::{
    ClientCommand, SlashItem, arrow_delta, filter_items, insert_text, menu_items,
    resolve_client_command, slash_query, step,
};
use bachuc_client::{
    AgentControls, AgentFeature, AgentFeatureKind, Client, ClientEvent, DiffStat, QueuedPrompt,
    SlashCommand, ThinkingControls,
};
use bachuc_command::{Shortcut, display_combos, load_shortcuts};
use bachuc_icons::{IconName, icon, provider_fallback_icon, provider_logo};
use bachuc_theme::metrics::{
    FONT_BASE, FONT_CONTENT, FONT_SM, ICON_SM, MAX_CONTENT_WIDTH, RADIUS_2XL, RADIUS_LG, RADIUS_MD,
    SPACE_1, SPACE_1_5, SPACE_2, SPACE_3, SPACE_4,
};
use bachuc_theme::{ShadowSize, shadow, tokens, ui_font_size};
use gpui_kit::component::input::{InputEvent, Position, Textarea, TextareaState};
use gpui_kit::component::popover::Popover;
use gpui_kit::component::theme::Theme;
use gpui_kit::component::tooltip::Tooltip;
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
/// The left and right editor padding of an `XSmall` gpui-kit textarea. The input box takes it
/// back with a negative margin, so text starts at the surface padding as in Electron.
const INPUT_INSET: f32 = 4.;
/// The shortcut whose chord the focus hint shows.
const FOCUS_ACTION: &str = "message-input.focus";

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
    features: Vec<AgentFeature>,
    /// The workspace stat when it has changes; `None` hides the diff row.
    diff_stat: Option<DiffStat>,
}

impl Watched {
    fn read(client: &Client, agent_id: &str) -> Self {
        let agent = client.agents().iter().find(|agent| agent.id == agent_id);
        Self {
            provider: agent.map(|agent| agent.provider.clone()),
            diff_stat: visible_diff_stat(
                agent
                    .and_then(|agent| agent.workspace_id.as_deref())
                    .and_then(|workspace_id| client.workspace_diff_stat(workspace_id)),
            ),
            features: client.agent_features(agent_id),
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
    /// The feature requests that wait for the daemon.
    requests: FeatureRequests,
    /// The row limit the input was last given, from the window height.
    max_rows: usize,
    /// "⌘L to focus", shown while the input is empty and unfocused; None when unassigned.
    focus_hint: Option<String>,
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
            // No placeholder here: gpui-kit draws it in the theme's muted color at the input
            // size, and Electron draws it in `surface4` at `sm`. The view draws its own.
            TextareaState::new(window, cx)
                .auto_grow(1, max_rows)
                .submit_on_enter(true)
        });
        let watched = Watched::read(client.read(cx), &agent_id);
        let weak = cx.weak_entity();
        let input_focus = input.read(cx).focus_handle(cx);
        let subscriptions = vec![
            // The focus hint shows only while the input is unfocused.
            cx.on_focus(&input_focus, window, |_, _, cx| cx.notify()),
            cx.on_blur(&input_focus, window, |_, _, cx| cx.notify()),
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
            requests: FeatureRequests::default(),
            max_rows,
            focus_hint: focus_hint(&load_shortcuts()),
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
                        // The menu opened with the client commands only; pin the full list
                        // to the bottom with the best match selected, as a query change does.
                        this.slash_active = 0;
                        this.slash_scroll.scroll_to_bottom();
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

    /// Sends a feature request, unless one for the same feature waits for the daemon. After the
    /// answer it reads the features again, so a failed request leaves the daemon value on screen.
    fn send_feature(
        &mut self,
        request: Option<(String, serde_json::Value)>,
        cx: &mut Context<Self>,
    ) {
        let Some((feature_id, value)) = request else {
            return;
        };
        if !self.requests.begin(&feature_id) {
            return;
        }
        let agent_id = self.agent_id.clone();
        let sent = feature_id.clone();
        let task = self.client.update(cx, |client, cx| {
            client.set_agent_feature(&agent_id, &sent, value, cx)
        });
        cx.notify();
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |this, cx| {
                this.requests.finish(&feature_id);
                if let Err(error) = result {
                    log::warn!(
                        "cannot set feature {feature_id} of {}: {error}",
                        this.agent_id
                    );
                }
                this.watched = Watched::read(this.client.read(cx), &this.agent_id);
                cx.notify();
            });
        })
        .detach();
    }

    /// The provider feature controls, in the order the daemon sends them, drawn after the mode
    /// pill as `DesktopFeatureItem` does.
    fn feature_pills(&self, weak: &WeakEntity<Composer>, cx: &App) -> Vec<AnyElement> {
        let t = tokens(cx);
        self.watched
            .features
            .iter()
            .map(|feature| match &feature.kind {
                AgentFeatureKind::Toggle { .. } => {
                    let color = match active_highlight(feature) {
                        Highlight::Yellow => t.status_warning,
                        Highlight::Green => t.status_success,
                        Highlight::Blue => t.accent,
                        Highlight::Default => t.foreground_muted,
                    };
                    let (weak, feature_id) = (weak.clone(), feature.id.clone());
                    feature_toggle(
                        feature,
                        color,
                        t.surface2,
                        self.requests.is_pending(&feature.id),
                        move |cx| {
                            weak.update(cx, |this, cx| {
                                let request = toggle_request(&this.watched.features, &feature_id);
                                this.send_feature(request, cx);
                            })
                            .ok();
                        },
                    )
                }
                AgentFeatureKind::Select { .. } => {
                    let (weak, feature_id) = (weak.clone(), feature.id.clone());
                    choice_pill(
                        feature_element_id("composer-feature", &feature.id),
                        feature.label.clone(),
                        Glyph::Icon(feature_icon(feature.icon.as_deref())),
                        select_choices(feature),
                        select_current(feature),
                        &feature.label,
                        Some(feature_tooltip(feature).to_string()),
                        move |option_id, cx| {
                            weak.update(cx, |this, cx| {
                                let request =
                                    select_request(&this.watched.features, &feature_id, option_id);
                                this.send_feature(request, cx);
                            })
                            .ok();
                        },
                    )
                }
            })
            .collect()
    }

    /// The pills, in the Electron order: model, thinking, mode, then the provider features.
    fn pills(&self, model_glyph: Glyph, weak: &WeakEntity<Composer>, cx: &App) -> Vec<AnyElement> {
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
            "composer-model".into(),
            "Model".into(),
            model_glyph,
            controls.models,
            controls.current_model,
            "Model",
            None,
            move |id, cx| client.update(cx, |c, cx| c.set_model(&agent_id, id, cx)),
        ));

        if let Some(thinking) = self.watched.thinking.clone() {
            let (client, agent_id) = (self.client.clone(), self.agent_id.clone());
            pills.push(choice_pill(
                "composer-thinking".into(),
                "Thinking".into(),
                Glyph::Icon(IconName::Brain),
                thinking.options,
                thinking.current,
                "Thinking",
                None,
                move |id, cx| client.update(cx, |c, cx| c.set_thinking(&agent_id, id, cx)),
            ));
        }

        // `AgentModeControl` renders nothing while the agent reports no modes (a closed agent).
        if !controls.modes.is_empty() {
            let (client, agent_id) = (self.client.clone(), self.agent_id.clone());
            pills.push(choice_pill(
                "composer-mode".into(),
                "Mode".into(),
                Glyph::Icon(IconName::ShieldCheck),
                controls.modes,
                controls.current_mode,
                "Mode",
                None,
                move |id, cx| client.update(cx, |c, cx| c.set_mode(&agent_id, id, cx)),
            ));
        }
        pills.extend(self.feature_pills(weak, cx));
        pills
    }
}

/// The model pill icon: the provider logo, or the fallback icon of `getProviderIcon`.
fn provider_glyph(provider: Option<&str>, color: Hsla) -> Glyph {
    let provider = provider.unwrap_or_default();
    match provider_logo(provider, color) {
        Some(image) => Glyph::Image(image),
        None => Glyph::Icon(provider_fallback_icon(provider)),
    }
}

fn feature_element_id(prefix: &'static str, feature_id: &str) -> ElementId {
    ElementId::Name(format!("{prefix}-{feature_id}").into())
}

/// A toggle feature: an icon button with a tooltip, tinted while the feature is on.
fn feature_toggle(
    feature: &AgentFeature,
    color: Hsla,
    hover: Hsla,
    pending: bool,
    on_click: impl Fn(&mut App) + 'static,
) -> AnyElement {
    let tooltip = feature_tooltip(feature).to_string();
    h_flex()
        .id(feature_element_id("composer-feature", &feature.id))
        .role(Role::Button)
        .aria_label(tooltip.clone())
        .h(px(CONTROL))
        .px(px(SPACE_2))
        .flex_none()
        .items_center()
        .rounded(px(RADIUS_2XL))
        .cursor_pointer()
        .hover(move |pill| pill.bg(hover))
        .when(pending, |pill| pill.opacity(0.5))
        .tooltip(move |window, cx| Tooltip::new(tooltip.clone()).build(window, cx))
        .on_click(move |_, _, cx| on_click(cx))
        .child(
            icon(feature_icon(feature.icon.as_deref()))
                .with_size(px(GLYPH))
                .text_color(color),
        )
        .into_any_element()
}

/// `composer.input.focusHint` with the first combo of the focus shortcut, or None when the
/// shortcut is unassigned, as `FocusHint` renders nothing then.
fn focus_hint(shortcuts: &[Shortcut]) -> Option<String> {
    let chord = shortcuts
        .iter()
        .find(|shortcut| shortcut.action == FOCUS_ACTION)?
        .chord
        .as_deref()?;
    let combo = display_combos(chord).into_iter().next()?;
    Some(format!("{combo} to focus"))
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
    kind: String,
    glyph: Glyph,
    label: String,
    tooltip: Option<String>,
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
            .when_some(self.tooltip, |pill, text| {
                pill.tooltip(move |window, cx| Tooltip::new(text.clone()).build(window, cx))
            })
            .child(match self.glyph {
                Glyph::Icon(name) => icon(name)
                    .with_size(px(GLYPH))
                    .text_color(t.foreground_muted)
                    .into_any_element(),
                Glyph::Image(image) => img(image).flex_none().size(px(GLYPH)).into_any_element(),
            })
            .child(div().truncate().max_w(px(160.)).child(self.label))
            // `ComboboxTrigger` draws a 14 px muted chevron 1 px low after the toolbar value.
            .child(
                div().flex_none().mt(px(1.)).child(
                    icon(IconName::ChevronDown)
                        .with_size(px(ICON_SM))
                        .text_color(t.foreground_muted),
                ),
            )
    }
}

type PickFn = Rc<dyn Fn(&str, &mut App)>;

/// A pill that opens a menu of choices above itself. With no choices it only shows the label.
#[allow(clippy::too_many_arguments)]
fn choice_pill(
    id: ElementId,
    kind: String,
    glyph: Glyph,
    choices: Vec<(String, String)>,
    current: Option<String>,
    fallback: &str,
    tooltip: Option<String>,
    on_pick: impl Fn(&str, &mut App) + 'static,
) -> AnyElement {
    let label = pill_label(&choices, current.as_deref(), fallback);
    let trigger = PillTrigger {
        id: id.clone(),
        kind: kind.clone(),
        glyph,
        label,
        tooltip,
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
            // React Native Web text has the CSS `normal` line height, about 1.2.
            v_flex()
                .gap(px(SPACE_1))
                .px(px(SPACE_3))
                .py(px(SPACE_3))
                .line_height(relative(1.2))
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
        // Above the surface, as an overlay: it takes no height from the layout. The gap is the
        // `AutocompletePopover` `OFFSET_FROM_ANCHOR`.
        Some(
            v_flex()
                .absolute()
                .bottom_full()
                .left_0()
                .right_0()
                .pb(px(SPACE_3))
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
        let value = self.value(cx);
        let empty = value.is_empty();
        let sendable = can_send(&value, self.attachments.len());
        let primary = primary_action(sendable, running);
        let content_size = ui_font_size(FONT_CONTENT, cx);
        let small_size = ui_font_size(FONT_SM, cx);
        let input_line = px(line_height(f32::from(content_size)));
        let focused = self.input.read(cx).focus_handle(cx).is_focused(window);
        let focus_hint = self.focus_hint.clone().filter(|_| empty && !focused);
        let matches = self.slash_matches(cx);
        let model_glyph = provider_glyph(self.watched.provider.as_deref(), t.foreground_muted);
        let weak = cx.weak_entity();
        let pills = self.pills(model_glyph, &weak, cx);
        let diff_strip = diff_row(self.watched.diff_stat).map(|(additions, deletions)| {
            // `ComposerDiffStatPill` in the track strip above the input, at the right end.
            h_flex().justify_end().child(
                h_flex()
                    .id("composer-diff-stat")
                    .role(Role::Group)
                    .aria_label(format!("Changes {additions} {deletions}"))
                    .h(px(CONTROL))
                    .px(px(SPACE_1_5))
                    .gap(px(SPACE_1))
                    .items_center()
                    .rounded(px(RADIUS_MD))
                    .text_size(small_size)
                    .child(div().text_color(t.status_success).child(additions))
                    .child(div().text_color(t.status_danger).child(deletions)),
            )
        });

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

        // `XSmall` gives the editor no top or bottom padding and 4 px at the sides; the margin
        // takes the sides back. The placeholder and the focus hint draw over the input, as the
        // Electron `::placeholder` rule (`sm`, `surface4`, the input line height) and
        // `focusHintText` do.
        let input_box = div()
            .relative()
            .mx(px(-INPUT_INSET))
            .child(
                Textarea::new(&self.input)
                    .xsmall()
                    .w_full()
                    .appearance(false)
                    .bordered(false)
                    .text_size(content_size)
                    .text_color(t.foreground)
                    .line_height(input_line)
                    .accessibility_id("composer-input")
                    .aria_label("Message")
                    .on_paste(move |item, _, cx| {
                        weak.update(cx, |this, cx| this.paste(item, cx))
                            .unwrap_or(false)
                    }),
            )
            .when(empty, |input_box| {
                input_box.child(
                    div()
                        .absolute()
                        .top_0()
                        .left(px(INPUT_INSET))
                        .right(px(INPUT_INSET))
                        .truncate()
                        .text_size(small_size)
                        .line_height(input_line)
                        .text_color(t.surface4)
                        .child(PLACEHOLDER),
                )
            })
            .when_some(focus_hint, |input_box, hint| {
                input_box.child(
                    div()
                        .absolute()
                        .top_0()
                        .right(px(INPUT_INSET))
                        .text_size(small_size)
                        .text_color(t.foreground_muted)
                        .opacity(0.5)
                        .child(hint),
                )
            });

        // `inputAreaContainer`: 16 px at the sides and bottom, the content centered at 820 px.
        div()
            .w_full()
            .flex()
            .justify_center()
            .px(px(SPACE_4))
            .pb(px(SPACE_4))
            .child(
                v_flex()
                    .relative()
                    .w_full()
                    .max_w(px(MAX_CONTENT_WIDTH))
                    .gap(px(SPACE_2))
                    .children(self.slash_menu(&matches, cx))
                    .children(queue)
                    .children(diff_strip)
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
                            .child(input_box)
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
                                                .on_click(cx.listener(|this, _, window, cx| {
                                                    this.pick_images(window, cx)
                                                })),
                                            )
                                            .children(pills),
                                    )
                                    .children(primary_button),
                            ),
                    ),
            )
    }
}

#[cfg(test)]
mod tests {
    // Named imports: `use super::*` would bring in GPUI's `test` macro over `#[test]`.
    use super::{
        FOCUS_ACTION, Glyph, Watched, focus_hint, outgoing_text, pill_label, provider_glyph,
    };
    use bachuc_client::{AgentFeature, AgentFeatureKind, DiffStat};
    use bachuc_command::default_shortcuts;
    use bachuc_icons::IconName;
    use gpui_kit::{Hsla, Rgba};

    fn watched(diff_stat: Option<DiffStat>, features: Vec<AgentFeature>) -> Watched {
        Watched {
            controls: None,
            thinking: None,
            queued: Vec::new(),
            supports_queue: true,
            provider: Some("codex".into()),
            features,
            diff_stat,
        }
    }

    fn fast_mode(value: bool) -> AgentFeature {
        AgentFeature {
            id: "fast_mode".into(),
            label: "Fast mode".into(),
            description: None,
            tooltip: None,
            icon: Some("zap".into()),
            kind: AgentFeatureKind::Toggle { value },
        }
    }

    #[test]
    fn an_unchanged_snapshot_causes_no_redraw() {
        // The client subscription calls `cx.notify()` only when the new `Watched` differs.
        let stat = Some(DiffStat {
            additions: 2,
            deletions: 1,
        });
        let first = watched(stat, vec![fast_mode(false)]);
        assert_eq!(first, watched(stat, vec![fast_mode(false)]));
        assert_ne!(first, watched(stat, vec![fast_mode(true)]));
        assert_ne!(first, watched(None, vec![fast_mode(false)]));
        assert_ne!(first, watched(stat, Vec::new()));
    }

    #[test]
    fn the_provider_logo_or_the_fallback_icon_is_drawn() {
        let color: Hsla = Rgba {
            r: 1.,
            g: 0.5,
            b: 0.,
            a: 0.5,
        }
        .into();
        assert!(matches!(
            provider_glyph(Some("claude"), color),
            Glyph::Image(_)
        ));
        assert!(matches!(
            provider_glyph(Some("codex"), color),
            Glyph::Image(_)
        ));
        assert!(matches!(
            provider_glyph(Some("custom"), color),
            Glyph::Icon(IconName::Bot)
        ));
        assert!(matches!(
            provider_glyph(Some("kiro"), color),
            Glyph::Icon(IconName::PackagePlus)
        ));
        assert!(matches!(
            provider_glyph(None, color),
            Glyph::Icon(IconName::Bot)
        ));
    }

    #[test]
    fn a_select_pill_shows_its_label_until_an_option_is_chosen() {
        let choices = vec![("low".to_string(), "Low".to_string())];
        assert_eq!(pill_label(&choices, None, "Effort"), "Effort");
        assert_eq!(pill_label(&choices, Some("low"), "Effort"), "Low");
    }

    #[test]
    fn the_focus_hint_shows_the_focus_chord() {
        let mut shortcuts = default_shortcuts();
        assert_eq!(focus_hint(&shortcuts).as_deref(), Some("⌘L to focus"));
        for shortcut in &mut shortcuts {
            if shortcut.action == FOCUS_ACTION {
                shortcut.chord = None;
            }
        }
        assert_eq!(focus_hint(&shortcuts), None);
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
