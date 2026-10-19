//! The composer root view (`packages/app/src/composer/index.tsx`): the queued prompts, the send
//! error, and the card with its track strip, attachment tray, input, toolbar, meter, and Send
//! or Stop. Each part lives in the module named after its Electron component.
//!
//! A composer is an agent composer (`Composer::new`) or the New workspace draft
//! (`Composer::new_draft`). Both draw the plugin slots of the active host's plugins.

use crate::actions::{
    PrimaryAction, SendPlan, line_height, max_input_rows, primary_action, send_plan,
};
use crate::agent_controls::features::{FeatureRequests, select_request, toggle_request};
use crate::agent_controls::layout::{ControlPresence, Density, resolve_density};
use crate::agent_controls::{
    ControlHandlers, ControlsView, FeatureHandlers, PickFn, agent_controls, control_presence,
    provider_glyph,
};
use crate::agent_tracks::{has_agent_tracks, strip_visible};
use crate::attachment_tray::{attachment_tray, image_chips, plugin_chip};
use crate::attachments::{Attachment, can_send, mime_for_path};
use crate::context_window_meter::{SLOT, context_window_meter};
use crate::diff_stat_pill::{diff_stat_pill, visible_diff_stat};
use crate::draft::{
    DraftOptions, DraftSeed, DraftSubmission, Generation, SubmitIcon, apply_feature,
    default_provider, draft_config, min_input_height, model_groups, model_key, offered,
    outgoing_attachments, seed_feature_values, seed_provider_model, split_model_key,
};
use crate::input::{CONTROL, MessageInput, round_button};
use crate::mention::{
    DEBOUNCE_MS, FileMention, LIMIT, MentionMenu, apply_mention, find_active_file_mention,
    insert_mentions, mention_path,
};
use crate::plugin_slots::{
    HostAction, PopoverViews, ShowPanel, draft_pill_row, host_action, needs_intercept,
    picker_sources, pill_slots, pill_states, plugin_pill, run_chain, stop_button, toggle_chip,
    upsert_chip, visible_commands,
};
use crate::queue::{QueueAction, move_target, queue_track};
use crate::slash::{
    ClientCommand, DaemonCommand, SlashItem, SlashRange, arrow_delta, find_active_slash,
    insert_text, menu_matches, replace_slash, resolve_client_command, resolve_plugin_command, step,
};
use crate::subagents_track::{
    ArchiveFinished, SubagentActions, finished_paseo_ids, row_bucket, subagents_pill,
};
use crate::task_list::task_pill;
use crate::tracks::TrackStrip;
use crate::turn_metrics_pill::{
    ComposerTurnMetrics, LIVE_TICK_MS, MetricPart, active_turn_items, composer_parts, live_metrics,
    live_tick, parse_time_ms, turn_metrics_pill,
};
use bachuc_client::{
    ActiveTurn, AgentControls, AgentFeature, AgentUsage, Client, ClientEvent, DiffStat,
    OutgoingMessage, ProviderSummary, QueuedPrompt, SubagentRow, TaskEntry, ThinkingControls,
    TurnMetrics,
};
use bachuc_command::{Shortcut, display_combos, load_shortcuts};
use bachuc_icons::{IconName, icon};
use bachuc_plugin_host::{
    AttachmentItem, AttachmentSource, ComposerAttachment, ComposerDraft, ComposerTarget,
    Contributed, ContributionIndex, InterceptInput, InterceptOutcome, PluginHostEvent, SendAction,
    SendKey, SlashInput, StopButtonProps, decode_attachment_search,
};
use bachuc_sidebar::model::StatusBucket;
use bachuc_sidebar::{Knockout, RunningRing, StatusBadge};
use bachuc_theme::metrics::{
    FONT_BASE, FONT_CONTENT, FONT_SM, ICON_SM, POPUP_GAP, RADIUS_LG, RADIUS_MD, SPACE_1, SPACE_2,
    SPACE_3, SPACE_4,
};
use bachuc_theme::{ShadowSize, shadow, tokens, ui_font_size, zoomed};
use gpui_kit::component::input::{
    Input, InputEvent, InputState, Position, Textarea, TextareaState,
};
use gpui_kit::component::popover::Popover;
use gpui_kit::component::spinner::Spinner;
use gpui_kit::component::theme::Theme;
use gpui_kit::component::{Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use serde_json::json;
use std::cell::Cell;
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Electron draws the stop button with `palette.red[600]` in every theme. The theme crate carries
/// that value only as the light terminal red (`ansi[1]`), which is a different role.
const STOP_RED: u32 = 0xdc2626;
/// `palette.red[500]` (`packages/app/src/styles/theme.ts` line 89): the send error text.
const SEND_ERROR_RED: u32 = 0xef4444;
/// `composer.placeholders.desktop` in `packages/app/src/i18n/resources/en.ts`.
const PLACEHOLDER: &str = "Message the agent, tag @files, or use /commands and /skills";
/// The slash list height cap: the `Autocomplete` default `maxHeight`.
const SLASH_MAX_HEIGHT: f32 = 220.;
/// The left and right editor padding of an `XSmall` gpui-kit textarea. The input box takes it
/// back with a negative margin, so text starts at the surface padding as in Electron.
const INPUT_INSET: f32 = 4.;
/// The shortcut whose chord the focus hint shows.
const FOCUS_ACTION: &str = "message-input.focus";
/// `FOOTER_HEIGHT` in `packages/app/src/constants/layout.ts`.
const FOOTER_HEIGHT: f32 = 75.;
/// `inputAreaLocked`.
const LOCKED_OPACITY: f32 = 0.6;
/// The stop glyph the plugin face gets: the toolbar glyph size.
const TOOLBAR_GLYPH: f32 = 14.;
/// `FileDropBackdrop`: the overlay fades in and out over 150 ms.
const DROP_FADE: Duration = Duration::from_millis(150);
/// `FileDropBackdrop`: `surface0` at 0.7 behind a 32 px Upload glyph and the text.
const DROP_BACKDROP_OPACITY: f32 = 0.7;
const DROP_GLYPH: f32 = 32.;
/// `composer.attachments.dropFilesHere` in `packages/app/src/i18n/resources/en.ts`.
const DROP_FILES_HERE: &str = "Drop files here";

/// What a `Composer` tells its owner.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ComposerEvent {
    /// A prompt went to the client.
    Sent { text: String, images: usize },
    /// `/clear` archived the agent; the owner opens a fresh draft in its place.
    ReplaceWithDraft,
    /// The diff stat pill: open the Changes tab of this workspace.
    OpenChanges { workspace_id: String },
    /// A Paseo subagent row: open that agent.
    OpenAgent { agent_id: String },
    /// A provider subagent row: open its read-only timeline tab.
    OpenProviderSubagent {
        parent_agent_id: String,
        subagent_id: String,
    },
    /// A draft submit press; the text may be empty. The owner reads `draft_submission`.
    DraftSubmit,
    /// A draft `Action` pill opens this New workspace panel instead of running `on_press`.
    ShowNewWorkspacePanel {
        plugin_id: &'static str,
        panel_id: &'static str,
    },
}

/// What the composer draws from the client. The composer redraws only when this changes.
#[derive(Clone, Debug, PartialEq)]
struct Watched {
    controls: Option<AgentControls>,
    thinking: Option<ThinkingControls>,
    queued: Vec<QueuedPrompt>,
    queue_held: bool,
    supports_queue: bool,
    provider: Option<String>,
    status: Option<String>,
    workspace_id: Option<String>,
    features: Vec<AgentFeature>,
    /// The workspace stat when it has changes; `None` hides the diff pill.
    diff_stat: Option<DiffStat>,
    active_turn: Option<ActiveTurn>,
    /// The last turn's daemon metrics; read only while no turn runs, because a running turn
    /// shows the live estimate instead.
    last_metrics: Option<TurnMetrics>,
    usage: Option<AgentUsage>,
    tasks: Vec<TaskEntry>,
    subagents: Vec<SubagentRow>,
    /// The providers a draft picks from.
    providers: Vec<ProviderSummary>,
}

impl Watched {
    fn read(client: &Client, agent_id: &str, is_draft: bool) -> Self {
        if is_draft {
            return Self::draft(client);
        }
        let agent = client.agents().iter().find(|agent| agent.id == agent_id);
        let workspace_id = agent.and_then(|agent| agent.workspace_id.clone());
        let active_turn = client.agent_active_turn(agent_id);
        Self {
            provider: agent.map(|agent| agent.provider.clone()),
            status: agent.map(|agent| agent.status.clone()),
            diff_stat: visible_diff_stat(
                workspace_id
                    .as_deref()
                    .and_then(|workspace_id| client.workspace_diff_stat(workspace_id)),
            ),
            workspace_id,
            features: client.agent_features(agent_id),
            controls: client.agent_controls(agent_id),
            thinking: client.agent_thinking(agent_id),
            queued: client.queued_prompts(agent_id),
            queue_held: client.queue_held(agent_id),
            supports_queue: client.supports_queue(),
            last_metrics: if active_turn.is_some() {
                None
            } else {
                client.agent_last_turn_metrics(agent_id)
            },
            active_turn,
            usage: client.agent_last_usage(agent_id),
            tasks: client.agent_tasks(agent_id),
            subagents: client.subagents(agent_id),
            providers: Vec::new(),
        }
    }

    /// A draft has no agent: it watches the provider list only.
    fn draft(client: &Client) -> Self {
        Self {
            controls: None,
            thinking: None,
            queued: Vec::new(),
            queue_held: false,
            supports_queue: false,
            provider: None,
            status: None,
            workspace_id: None,
            features: Vec::new(),
            diff_stat: None,
            active_turn: None,
            last_metrics: None,
            usage: None,
            tasks: Vec::new(),
            subagents: Vec::new(),
            providers: client.providers().to_vec(),
        }
    }

    fn running(&self) -> bool {
        self.controls.as_ref().is_some_and(|c| c.running)
    }

    /// `useHasComposerTurnMetrics`: a running turn, or a last turn with metrics.
    fn has_turn_metrics(&self) -> bool {
        self.active_turn.is_some() || self.last_metrics.is_some()
    }

    /// `contextWindowPending`: the ring keeps its slot while the agent starts or runs.
    fn usage_pending(&self) -> bool {
        matches!(self.status.as_deref(), Some("initializing" | "running"))
    }
}

/// The redraw rule: a client event redraws the composer only when the snapshot changed.
fn snapshot_changed(old: &Watched, new: &Watched) -> bool {
    old != new
}

/// The live turn metrics of the running turn, kept across ticks.
#[derive(Clone, Debug, Default, PartialEq)]
struct LiveMetrics {
    turn_key: String,
    /// The earliest first output seen for this turn.
    seen_first_output: Option<i64>,
    parts: Vec<MetricPart>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CommandsState {
    NotLoaded,
    Loading,
    Loaded,
    /// The agent cannot list commands (a mock agent, or no session yet). The next `/` tries again.
    Failed,
}

/// The agent settings a draft picks before the agent exists.
struct DraftState {
    options: DraftOptions,
    provider: Option<String>,
    model: Option<String>,
    modes: Vec<(String, String)>,
    mode_id: Option<String>,
    thinking_id: Option<String>,
    features: Vec<AgentFeature>,
    generation: Generation,
    /// The owner creates the workspace: the input keeps its text, locked.
    submitting: bool,
    plugin_draft: Entity<ComposerDraft>,
    /// The seeded mode and feature values wait for the provider's lists.
    seed_mode: Option<String>,
    seed_features: Option<serde_json::Value>,
}

impl DraftState {
    fn cwd(&self) -> Option<&str> {
        self.options.cwd.as_deref()
    }

    fn config(&self) -> Option<bachuc_client::DraftConfig> {
        draft_config(
            self.provider.as_deref(),
            self.cwd(),
            self.model.as_deref(),
            self.mode_id.as_deref(),
            self.thinking_id.as_deref(),
        )
    }
}

/// The "Drop files here" overlay: on while a Finder drag is over the composer, faded by opacity.
/// GPUI has no per-element leave event, so the composer sets it from each drag move, and clears
/// it on a drop and in render once the drag has ended.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct DropOverlay {
    over: bool,
    /// The opacity when `over` last changed.
    from: f32,
    /// When `over` last changed; `None` once the fade is done.
    since: Option<Instant>,
}

impl DropOverlay {
    /// Sets the drag state. Returns true when it changed, so the view renders only then.
    fn set_over(&mut self, over: bool, now: Instant) -> bool {
        if over == self.over {
            return false;
        }
        self.from = self.opacity(now);
        self.over = over;
        self.since = Some(now);
        true
    }

    fn opacity(&self, now: Instant) -> f32 {
        let target = if self.over { 1. } else { 0. };
        let Some(since) = self.since else {
            return target;
        };
        let t = now.saturating_duration_since(since).as_secs_f32() / DROP_FADE.as_secs_f32();
        self.from + (target - self.from) * t.min(1.)
    }

    fn fading(&self, now: Instant) -> bool {
        self.since
            .is_some_and(|since| now.saturating_duration_since(since) < DROP_FADE)
    }
}

/// A drag move is over the composer when it is inside the drop zone and a drop would attach.
fn drag_over(bounds: Bounds<Pixels>, position: Point<Pixels>, accepts: bool) -> bool {
    accepts && bounds.contains(&position)
}

/// An attachment the composer sends as given, such as a fork's chat history.
struct RawAttachment {
    label: String,
    attachment: serde_json::Value,
}

/// A plugin chip and its comment field.
struct PluginChip {
    attachment: ComposerAttachment,
    comment: Option<Entity<InputState>>,
}

/// The searchable list a picker source opens.
struct Picker {
    source: Contributed<AttachmentSource>,
    query: Entity<InputState>,
    items: Vec<AttachmentItem>,
    generation: Generation,
    status: Option<String>,
    _subscription: Subscription,
}

/// The message box for one agent, or the draft of a new one. It sends through the client.
pub struct Composer {
    client: Entity<Client>,
    agent_id: String,
    input: Entity<TextareaState>,
    attachments: Vec<Attachment>,
    pasted: u32,
    watched: Watched,
    commands: Vec<DaemonCommand>,
    commands_state: CommandsState,
    /// The draft config `commands` was listed for.
    commands_config: Option<bachuc_client::DraftConfig>,
    /// The highlighted row of the slash menu.
    slash_active: usize,
    /// Escape closed the menu for the text being typed.
    slash_dismissed: bool,
    slash_scroll: ScrollHandle,
    /// The `@` file menu.
    mention: MentionMenu,
    mention_scroll: ScrollHandle,
    /// The debounced suggestion request; a newer keystroke drops it.
    mention_fetch: Option<Task<()>>,
    /// The feature requests that wait for the daemon.
    requests: FeatureRequests,
    /// The row limit the input was last given, from the window height.
    max_rows: usize,
    /// "⌘L to focus", shown while the input is empty and unfocused; None when unassigned.
    focus_hint: Option<String>,
    /// The toolbar density from the last measured toolbar width.
    density: Density,
    /// The text of the last failed send, shown under the queue.
    send_error: Option<String>,
    live: LiveMetrics,
    /// The 1 s tick of the live metrics; `Some` only during an active turn.
    tick: Option<Task<()>>,
    archive: ArchiveFinished,
    /// One ring per track pill key, so a turn redraws only the ring.
    rings: HashMap<String, Entity<RunningRing>>,
    /// One status badge per subagent row key, so a ring does not restart its clock.
    badges: HashMap<String, Entity<StatusBadge>>,
    draft: Option<DraftState>,
    raw_attachments: Vec<RawAttachment>,
    plugin_chips: Vec<PluginChip>,
    /// Interceptors run: the primary button shows a spinner.
    processing: bool,
    /// A stop press waits for the agent to leave the running status.
    cancelling: bool,
    drop: DropOverlay,
    picker: Option<Picker>,
    popovers: PopoverViews,
    /// The corner pills' width, so the column keeps the timeline's right edge.
    corner_reserve: Pixels,
    /// The composer's width at the last paint.
    width: Rc<Cell<Pixels>>,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<ComposerEvent> for Composer {}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_millis() as i64)
}

/// The active plugins' contributions; empty without a plugin host.
fn plugin_index(cx: &App) -> ContributionIndex {
    bachuc_plugin_host::host(cx)
        .map(|host| host.read(cx).index().clone())
        .unwrap_or_default()
}

impl Composer {
    pub fn new(
        client: Entity<Client>,
        agent_id: String,
        window: &mut Window,
        cx: &mut Context<Composer>,
    ) -> Composer {
        Self::build(client, agent_id, None, window, cx)
    }

    /// The New workspace draft. `agent_id()` returns `key`.
    pub fn new_draft(
        client: Entity<Client>,
        key: String,
        options: DraftOptions,
        window: &mut Window,
        cx: &mut Context<Composer>,
    ) -> Composer {
        let mut composer = Self::build(client, key, Some(options), window, cx);
        composer.reload_draft(cx);
        composer
    }

    fn build(
        client: Entity<Client>,
        agent_id: String,
        options: Option<DraftOptions>,
        window: &mut Window,
        cx: &mut Context<Composer>,
    ) -> Composer {
        let max_rows = max_input_rows(f32::from(window.viewport_size().height), FONT_CONTENT);
        let min_lines = options
            .as_ref()
            .map_or(1, |options| options.min_lines.max(1));
        let input = cx.new(|cx| {
            // No placeholder here: gpui-kit draws it in the theme's muted color at the input
            // size, and Electron draws it in `surface4` at `sm`. The view draws its own.
            TextareaState::new(window, cx)
                .auto_grow(min_lines, max_rows.max(min_lines))
                .submit_on_enter(true)
        });
        let is_draft = options.is_some();
        let watched = Watched::read(client.read(cx), &agent_id, is_draft);
        let draft = options.map(|options| {
            let cwd = options.cwd.clone();
            let provider = default_provider(&watched.providers);
            let model = provider
                .as_deref()
                .and_then(|provider| client.read(cx).default_model(provider));
            DraftState {
                options,
                provider,
                model,
                modes: Vec::new(),
                mode_id: None,
                thinking_id: None,
                features: Vec::new(),
                generation: Generation::default(),
                submitting: false,
                plugin_draft: cx.new(|_| ComposerDraft {
                    cwd,
                    values: BTreeMap::new(),
                }),
                seed_mode: None,
                seed_features: None,
            }
        });
        let weak = cx.weak_entity();
        let input_focus = input.read(cx).focus_handle(cx);
        let mut subscriptions = vec![
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
                |this: &mut Composer, _, event: &ClientEvent, cx| {
                    let ours = match event {
                        ClientEvent::AgentsChanged | ClientEvent::Connected => true,
                        ClientEvent::TimelineChanged { agent_id } => *agent_id == this.agent_id,
                        _ => false,
                    };
                    if ours {
                        this.refresh(cx);
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
                    .update(cx, |this, cx| {
                        this.mention_key(&key, window, cx) || this.slash_key(&key, window, cx)
                    })
                    .unwrap_or(false);
                if handled {
                    cx.stop_propagation();
                }
            }),
        ];
        if let Some(host) = bachuc_plugin_host::host(cx) {
            subscriptions.push(cx.subscribe_in(
                &host,
                window,
                |this: &mut Composer, _, event: &PluginHostEvent, window, cx| {
                    this.host_event(event, window, cx)
                },
            ));
        }
        let mut composer = Composer {
            client,
            agent_id,
            input,
            attachments: Vec::new(),
            pasted: 0,
            watched,
            commands: Vec::new(),
            commands_state: CommandsState::NotLoaded,
            commands_config: None,
            slash_active: 0,
            slash_dismissed: false,
            slash_scroll: ScrollHandle::new(),
            mention: MentionMenu::default(),
            mention_scroll: ScrollHandle::new(),
            mention_fetch: None,
            requests: FeatureRequests::default(),
            max_rows,
            focus_hint: focus_hint(&load_shortcuts()),
            density: Density::Full,
            send_error: None,
            live: LiveMetrics::default(),
            tick: None,
            archive: ArchiveFinished::default(),
            rings: HashMap::new(),
            badges: HashMap::new(),
            draft,
            raw_attachments: Vec::new(),
            plugin_chips: Vec::new(),
            processing: false,
            cancelling: false,
            drop: DropOverlay::default(),
            picker: None,
            popovers: PopoverViews::default(),
            corner_reserve: px(0.),
            width: Rc::new(Cell::new(px(0.))),
            _subscriptions: subscriptions,
        };
        composer.sync_tick(cx);
        composer
    }

    pub fn agent_id(&self) -> &str {
        &self.agent_id
    }

    /// Sets the width of the corner pills over the timeline. The column takes the same right
    /// inset as the timeline's column, so the composer and the message text share both edges.
    pub fn set_corner_reserve(&mut self, width: Pixels, cx: &mut Context<Self>) {
        if (f32::from(width) - f32::from(self.corner_reserve)).abs() > 0.5 {
            self.corner_reserve = width;
            cx.notify();
        }
    }

    pub fn focus(&self, window: &mut Window, cx: &mut App) {
        self.input.update(cx, |input, cx| input.focus(window, cx));
    }

    pub fn attachment_count(&self) -> usize {
        self.attachments.len()
    }

    /// What the owner needs to create the draft's workspace; `None` for an agent composer.
    pub fn draft_submission(&self, cx: &App) -> Option<DraftSubmission> {
        let draft = self.draft.as_ref()?;
        Some(DraftSubmission::build(
            self.value(cx).trim().to_string(),
            self.attachments.iter().map(Attachment::to_json).collect(),
            outgoing_attachments(&self.raw_json(), self.plugin_attachments(cx)),
            draft.config(),
            &draft.features,
            draft.plugin_draft.read(cx),
        ))
    }

    /// Sets the draft's folder; modes and features load again for it.
    pub fn set_draft_cwd(&mut self, cwd: Option<String>, cx: &mut Context<Composer>) {
        let Some(draft) = self.draft.as_mut() else {
            return;
        };
        if draft.options.cwd == cwd {
            return;
        }
        draft.options.cwd = cwd.clone();
        draft.plugin_draft.update(cx, |values, cx| {
            values.cwd = cwd;
            cx.notify();
        });
        self.reload_draft(cx);
    }

    /// Seeds the draft from a source agent as if the owner picked each field. A field the
    /// provider does not offer is ignored; the mode and features apply when their lists load.
    pub fn set_draft_seed(
        &mut self,
        seed: DraftSeed,
        _window: &mut Window,
        cx: &mut Context<Composer>,
    ) {
        let Some(draft) = self.draft.as_mut() else {
            return;
        };
        let Some((provider, model)) =
            seed_provider_model(&seed, draft.provider.as_deref(), &self.watched.providers)
        else {
            return;
        };
        let client = self.client.read(cx);
        let model = model.or_else(|| client.default_model(&provider));
        let thinking = model
            .as_deref()
            .and_then(|model| client.model_thinking(&provider, model))
            .map(|thinking| thinking.options)
            .unwrap_or_default();
        draft.thinking_id = offered(seed.thinking_option_id.as_deref(), &thinking);
        draft.provider = Some(provider);
        draft.model = model;
        draft.mode_id = None;
        draft.seed_mode = seed.mode;
        draft.seed_features = seed.features;
        self.reload_draft(cx);
    }

    /// Adds a chip that the next prompt sends unchanged, such as a fork's chat history.
    pub fn add_raw_attachment(
        &mut self,
        label: String,
        attachment: serde_json::Value,
        cx: &mut Context<Composer>,
    ) {
        self.raw_attachments
            .push(RawAttachment { label, attachment });
        cx.notify();
    }

    /// While the owner submits, the input keeps its text, locked, and the primary button spins.
    pub fn set_submitting(&mut self, submitting: bool, cx: &mut Context<Composer>) {
        if let Some(draft) = self.draft.as_mut() {
            draft.submitting = submitting;
            cx.notify();
        }
    }

    /// Empties the input, the images, the raw attachments, the plugin chips, and the send error.
    pub fn clear(&mut self, window: &mut Window, cx: &mut Context<Composer>) {
        self.attachments.clear();
        self.raw_attachments.clear();
        self.plugin_chips.clear();
        self.send_error = None;
        self.clear_input(window, cx);
    }

    fn value(&self, cx: &App) -> String {
        self.input.read(cx).value().to_string()
    }

    fn is_draft(&self) -> bool {
        self.draft.is_some()
    }

    fn locked(&self) -> bool {
        self.draft.as_ref().is_some_and(|draft| draft.submitting)
    }

    /// The composer a pill or interceptor acts on.
    fn target(&self) -> ComposerTarget {
        match &self.draft {
            Some(draft) => ComposerTarget {
                workspace_id: None,
                agent_id: None,
                draft: Some(draft.plugin_draft.clone()),
            },
            None => ComposerTarget {
                workspace_id: self.watched.workspace_id.clone(),
                agent_id: Some(self.agent_id.clone()),
                draft: None,
            },
        }
    }

    /// The agent id plugin events address; a draft has none.
    fn plugin_agent_id(&self) -> Option<&str> {
        (!self.is_draft()).then_some(self.agent_id.as_str())
    }

    fn raw_json(&self) -> Vec<serde_json::Value> {
        self.raw_attachments
            .iter()
            .map(|raw| raw.attachment.clone())
            .collect()
    }

    /// The images, raw attachments, and plugin chips the next prompt carries.
    fn attachment_total(&self) -> usize {
        self.attachments.len() + self.raw_attachments.len() + self.plugin_chips.len()
    }

    fn plugin_attachments(&self, cx: &App) -> Vec<serde_json::Value> {
        self.plugin_chips
            .iter()
            .map(|chip| {
                let comment = chip
                    .comment
                    .as_ref()
                    .map(|comment| comment.read(cx).value().to_string());
                chip.attachment.to_agent_attachment(comment.as_deref())
            })
            .collect()
    }

    /// Reads the snapshot again and redraws only when it changed.
    fn refresh(&mut self, cx: &mut Context<Self>) {
        let now = Watched::read(self.client.read(cx), &self.agent_id, self.is_draft());
        if !snapshot_changed(&self.watched, &now) {
            return;
        }
        self.watched = now;
        if self.archive.progress(&self.watched.subagents).is_none() {
            self.archive = ArchiveFinished::default();
        }
        if !self.watched.running() {
            self.cancelling = false;
        }
        if let Some(draft) = self.draft.as_mut()
            && draft.provider.is_none()
        {
            draft.provider = default_provider(&self.watched.providers);
            draft.model = draft
                .provider
                .as_deref()
                .and_then(|provider| self.client.read(cx).default_model(provider));
            self.reload_draft(cx);
        }
        self.sync_tick(cx);
        cx.notify();
    }

    fn host_event(&mut self, event: &PluginHostEvent, window: &mut Window, cx: &mut Context<Self>) {
        match host_action(event, self.plugin_agent_id()) {
            Some(HostAction::Redraw) => {
                self.popovers.borrow_mut().clear();
                cx.notify();
            }
            Some(HostAction::SetText(text)) => {
                self.set_text(text, window, cx);
                self.focus(window, cx);
                cx.notify();
            }
            Some(HostAction::AddChip(chip)) => {
                let mut chips: Vec<ComposerAttachment> = self
                    .plugin_chips
                    .iter()
                    .map(|chip| chip.attachment.clone())
                    .collect();
                upsert_chip(&mut chips, chip);
                self.set_chips(chips, window, cx);
            }
            None => {}
        }
    }

    /// Replaces the chips, keeping the comment field of each chip that stays.
    fn set_chips(
        &mut self,
        chips: Vec<ComposerAttachment>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let mut previous = std::mem::take(&mut self.plugin_chips);
        self.plugin_chips = chips
            .into_iter()
            .map(|attachment| {
                let kept = previous.iter().position(|chip| {
                    chip.attachment.plugin_id == attachment.plugin_id
                        && chip.attachment.source_id == attachment.source_id
                        && chip.attachment.item.id == attachment.item.id
                });
                let comment = match kept.map(|at| previous.remove(at)) {
                    Some(chip) if attachment.commentable => chip.comment,
                    _ if attachment.commentable => {
                        Some(cx.new(|cx| InputState::new(window, cx).placeholder("Add a comment")))
                    }
                    _ => None,
                };
                PluginChip {
                    attachment,
                    comment,
                }
            })
            .collect();
        cx.notify();
    }

    /// Starts the 1 s tick when a turn becomes active and drops it when the turn ends.
    fn sync_tick(&mut self, cx: &mut Context<Self>) {
        if !live_tick(self.watched.active_turn.is_some()) {
            self.tick = None;
            self.live = LiveMetrics::default();
            return;
        }
        self.refresh_live(cx);
        if self.tick.is_some() {
            return;
        }
        self.tick = Some(cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(LIVE_TICK_MS))
                    .await;
                let alive = this.update(cx, |this, cx| {
                    if this.refresh_live(cx) {
                        cx.notify();
                    }
                });
                if alive.is_err() {
                    break;
                }
            }
        }));
    }

    /// Computes the live metrics of the active turn. It scans back from the end of the
    /// timeline and stops at the turn's first entry. Returns whether the parts changed.
    fn refresh_live(&mut self, cx: &App) -> bool {
        let Some(turn) = self.watched.active_turn.clone() else {
            return false;
        };
        let key = format!(
            "{}:{}",
            turn.turn_id,
            turn.started_at.as_deref().unwrap_or_default()
        );
        if key != self.live.turn_key {
            self.live = LiveMetrics {
                turn_key: key,
                ..LiveMetrics::default()
            };
        }
        let parts = match turn.started_at.as_deref().and_then(parse_time_ms) {
            None => Vec::new(),
            Some(started) => {
                let entries = self
                    .client
                    .read(cx)
                    .timeline(&self.agent_id)
                    .map(|timeline| timeline.entries.as_slice())
                    .unwrap_or_default();
                let items = active_turn_items(entries, &turn.turn_id);
                let value = live_metrics(items, started, now_ms(), self.live.seen_first_output);
                if let ComposerTurnMetrics::Streaming {
                    first_output_at, ..
                } = value
                {
                    self.live.seen_first_output = Some(first_output_at);
                }
                composer_parts(Some(&value))
            }
        };
        let changed = parts != self.live.parts;
        self.live.parts = parts;
        changed
    }

    /// The metrics pill parts: live during a turn, else the last turn's daemon metrics.
    fn metric_parts(&self) -> Vec<MetricPart> {
        if self.watched.active_turn.is_some() {
            return self.live.parts.clone();
        }
        self.watched
            .last_metrics
            .map(|metrics| composer_parts(Some(&ComposerTurnMetrics::Completed(metrics))))
            .unwrap_or_default()
    }

    /// Loads the draft's modes and features for its provider, model, and folder. An answer
    /// that arrives after a newer request is dropped.
    fn reload_draft(&mut self, cx: &mut Context<Self>) {
        let Some(draft) = self.draft.as_mut() else {
            return;
        };
        let generation = draft.generation.next();
        draft.modes.clear();
        draft.features.clear();
        let (Some(provider), Some(config)) = (draft.provider.clone(), draft.config()) else {
            cx.notify();
            return;
        };
        let cwd = config.cwd.clone();
        let (modes, features) = self.client.update(cx, |client, cx| {
            (
                client.provider_modes(&provider, Some(&cwd), cx),
                client.draft_features(config, cx),
            )
        });
        cx.spawn(async move |this, cx| {
            let modes = modes.await;
            let _ = this.update(cx, |this, cx| {
                let Some(draft) = this.draft.as_mut() else {
                    return;
                };
                if !draft.generation.accepts(generation) {
                    return;
                }
                match modes {
                    Ok(modes) => {
                        if let Some(mode) = draft.seed_mode.take() {
                            draft.mode_id = offered(Some(&mode), &modes);
                        }
                        draft.modes = modes;
                    }
                    Err(error) => log::warn!("cannot list modes of {provider}: {error}"),
                }
                cx.notify();
            });
            let features = features.await;
            let _ = this.update(cx, |this, cx| {
                let Some(draft) = this.draft.as_mut() else {
                    return;
                };
                if !draft.generation.accepts(generation) {
                    return;
                }
                match features {
                    Ok(mut features) => {
                        let seeded =
                            seed_feature_values(draft.seed_features.take().as_ref(), &features);
                        for (feature_id, value) in seeded {
                            apply_feature(&mut features, &feature_id, &value);
                        }
                        draft.features = features;
                    }
                    Err(error) => log::warn!("cannot list draft features: {error}"),
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    /// A draft model pick sets the provider and the model.
    fn pick_draft_model(&mut self, key: &str, cx: &mut Context<Self>) {
        let Some((provider, model)) = split_model_key(key) else {
            return;
        };
        let Some(draft) = self.draft.as_mut() else {
            return;
        };
        if draft.provider.as_deref() != Some(provider) {
            draft.mode_id = None;
            draft.seed_mode = None;
            draft.seed_features = None;
        }
        draft.provider = Some(provider.to_string());
        draft.model = Some(model.to_string());
        draft.thinking_id = None;
        self.reload_draft(cx);
    }

    fn text_changed(&mut self, cx: &mut Context<Self>) {
        self.request_mentions(cx);
        self.refresh_slash(cx);
        cx.notify();
    }

    /// Loads the commands once a `/` is in the text and resets the menu to its best match.
    /// Runs on a change event, and after a code edit, which emits none.
    fn refresh_slash(&mut self, cx: &mut Context<Self>) {
        let value = self.value(cx);
        // Any `/` loads the list once, so the menu opens with it ready when the caret moves in.
        if value.contains('/') {
            self.load_commands(cx);
        }
        if self.active_slash(cx).is_none() {
            self.slash_dismissed = false;
            if self.commands_state == CommandsState::Failed {
                self.commands_state = CommandsState::NotLoaded;
            }
        }
        // The best match sits at the bottom, next to the input, as Electron pins the list there.
        self.slash_active = 0;
        self.slash_scroll.scroll_to_bottom();
    }

    /// Asks the daemon for the commands and skills the first time a `/` is typed. A draft asks
    /// again only when its provider, folder, model, or mode changes, because the daemon reads
    /// project skills from the folder.
    fn load_commands(&mut self, cx: &mut Context<Self>) {
        let draft_config = match &self.draft {
            Some(draft) => match draft.config() {
                Some(config) => Some(config),
                // A draft with no provider or folder has no commands to list.
                None => return,
            },
            None => None,
        };
        let stale = draft_config.is_some() && draft_config != self.commands_config;
        if self.commands_state != CommandsState::NotLoaded && !stale {
            return;
        }
        self.commands_state = CommandsState::Loading;
        self.commands_config = draft_config.clone();
        let agent_id = self.agent_id.clone();
        let task = self.client.update(cx, |client, cx| match &draft_config {
            Some(config) => client.list_draft_commands(&agent_id, config, cx),
            None => client.list_commands(&agent_id, cx),
        });
        let requested = draft_config;
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |this, cx| {
                // A newer draft config started its own request.
                if this.commands_config != requested {
                    return;
                }
                match result {
                    Ok(commands) => {
                        this.commands = commands.iter().map(DaemonCommand::from).collect();
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

    /// The plugin slash commands this composer shows.
    fn plugin_commands(
        &self,
        cx: &App,
    ) -> Vec<Contributed<bachuc_plugin_host::PluginSlashCommand>> {
        visible_commands(
            plugin_index(cx).slash_commands(),
            self.watched.workspace_id.is_some(),
            !self.is_draft(),
        )
    }

    /// The menu rows for the text in the input; empty when the menu is closed.
    fn slash_matches(&self, cx: &App) -> Vec<SlashItem> {
        // A file mention at the caret wins, as `resolveAutocompleteMode` picks "file" first.
        if self.slash_dismissed || self.mention_open(cx) {
            return Vec::new();
        }
        // A draft with no provider or folder yet has no commands to list.
        if self
            .draft
            .as_ref()
            .is_some_and(|draft| draft.config().is_none())
        {
            return Vec::new();
        }
        let Some(range) = self.active_slash(cx) else {
            return Vec::new();
        };
        let plugins: Vec<(String, String)> = self
            .plugin_commands(cx)
            .iter()
            .map(|command| {
                (
                    command.value.name.to_string(),
                    command.value.description.to_string(),
                )
            })
            .collect();
        menu_matches(&self.commands, &plugins, &range, self.is_draft())
    }

    /// The `/name` the caret is in.
    fn active_slash(&self, cx: &App) -> Option<SlashRange> {
        find_active_slash(&self.value(cx), self.input.read(cx).cursor())
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

    /// Puts `text` in the input with the caret at its end. The app uses it to put a rewound
    /// prompt back.
    pub fn set_text(&mut self, text: String, window: &mut Window, cx: &mut Context<Self>) {
        let lines: Vec<&str> = text.split('\n').collect();
        let position = Position {
            line: (lines.len() - 1) as u32,
            character: lines
                .last()
                .map_or(0, |line| line.encode_utf16().count() as u32),
        };
        self.input.update(cx, |input, cx| {
            input.set_value(text, window, cx);
            input.set_cursor_position(position, window, cx);
        });
        self.request_mentions(cx);
    }

    /// Puts the command where the typed `/name` is, with the caret after it.
    fn pick(&mut self, item: &SlashItem, window: &mut Window, cx: &mut Context<Self>) {
        let (text, caret) = self.picked_text(item, cx);
        self.set_text_at(text, caret, window, cx);
        self.slash_active = 0;
        cx.notify();
    }

    /// The input after picking `item`, and the caret in it.
    fn picked_text(&self, item: &SlashItem, cx: &App) -> (String, usize) {
        match self.active_slash(cx) {
            Some(range) => replace_slash(&self.value(cx), &range, item),
            None => {
                let text = insert_text(item);
                let caret = text.len();
                (text, caret)
            }
        }
    }

    /// The folder file mentions resolve in: the draft's, or the agent's.
    fn mention_cwd(&self, cx: &App) -> Option<String> {
        match &self.draft {
            Some(draft) => draft.cwd().map(String::from),
            None => self
                .client
                .read(cx)
                .agents()
                .iter()
                .find(|agent| agent.id == self.agent_id)
                .map(|agent| agent.cwd.trim().to_string()),
        }
    }

    /// The `@query` that ends at the caret.
    fn active_mention(&self, cx: &App) -> Option<FileMention> {
        let input = self.input.read(cx);
        find_active_file_mention(&input.value(), input.cursor())
    }

    fn mention_open(&self, cx: &App) -> bool {
        let mention = self.active_mention(cx);
        self.mention
            .open(self.mention_cwd(cx).as_deref(), mention.as_ref())
    }

    /// Asks for the files that match the mention at the caret once typing pauses. A newer
    /// keystroke drops the waiting request, so a burst of keys sends one request.
    fn request_mentions(&mut self, cx: &mut Context<Self>) {
        let mention = self.active_mention(cx);
        let cwd = self.mention_cwd(cx);
        let Some(request) = self.mention.update(cwd.as_deref(), mention.as_ref()) else {
            if mention.is_none() {
                self.mention_fetch = None;
            }
            return;
        };
        self.mention_scroll.scroll_to_bottom();
        self.mention_fetch = Some(cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(DEBOUNCE_MS))
                .await;
            let Ok(task) = this.update(cx, |this, cx| {
                this.client.update(cx, |client, cx| {
                    client.directory_suggestions(
                        &request.cwd,
                        &request.query,
                        true,
                        true,
                        LIMIT,
                        cx,
                    )
                })
            }) else {
                return;
            };
            let answer = task.await.map_err(|error| error.to_string());
            let _ = this.update(cx, |this, cx| {
                if this.mention.accept(request.generation, answer) {
                    this.mention_scroll.scroll_to_bottom();
                    cx.notify();
                }
            });
        }));
    }

    /// Up, Down, Tab, and Escape while the `@` menu is open. Returns whether it took the key.
    fn mention_key(&mut self, key: &str, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if !self.input.read(cx).focus_handle(cx).is_focused(window) {
            return false;
        }
        let Some(mention) = self.active_mention(cx) else {
            return false;
        };
        if !self
            .mention
            .open(self.mention_cwd(cx).as_deref(), Some(&mention))
        {
            return false;
        }
        let len = self.mention.items.len();
        if let Some(delta) = arrow_delta(key) {
            if len == 0 {
                return false;
            }
            self.mention.active = step(self.mention.active.min(len - 1), len, delta);
            // Rows draw worst first, as the slash menu does.
            self.mention_scroll
                .scroll_to_item(len - 1 - self.mention.active);
            cx.notify();
            return true;
        }
        match key {
            "tab" => return self.pick_mention(self.mention.active, window, cx),
            "escape" => self.mention.dismiss(&mention),
            _ => return false,
        }
        cx.notify();
        true
    }

    /// Replaces the `@query` with the quoted path of row `index`. Returns whether it picked.
    fn pick_mention(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some(mention) = self.active_mention(cx) else {
            return false;
        };
        if !self
            .mention
            .open(self.mention_cwd(cx).as_deref(), Some(&mention))
        {
            return false;
        }
        let Some(item) = self
            .mention
            .items
            .get(index.min(self.mention.items.len().saturating_sub(1)))
        else {
            return false;
        };
        let (text, caret) = apply_mention(&self.value(cx), &mention, &item.path);
        self.set_text_at(text, caret, window, cx);
        true
    }

    /// Puts `text` in the input with the caret at byte `caret`. `set_value` emits no change
    /// event, so the mention menu reads the new text here.
    fn set_text_at(
        &mut self,
        text: String,
        caret: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.input.update(cx, |input, cx| {
            input.set_value(text, window, cx);
            input.set_selected_range(caret..caret, cx);
            input.focus(window, cx);
        });
        self.request_mentions(cx);
        cx.notify();
    }

    /// Puts `path` in at the caret as a quoted mention. A path inside the folder becomes
    /// relative to it.
    pub fn insert_path_mention(
        &mut self,
        path: &str,
        window: &mut Window,
        cx: &mut Context<Composer>,
    ) {
        let cwd = self.mention_cwd(cx);
        let path = mention_path(Path::new(path), cwd.as_deref());
        self.insert_mentions_at_caret(&[path], window, cx);
    }

    /// Types `text` at the caret and focuses the input, for keys typed while focus is
    /// elsewhere in the thread.
    pub fn insert_text(&mut self, text: &str, window: &mut Window, cx: &mut Context<Composer>) {
        let value = self.value(cx);
        let mut at = self.input.read(cx).cursor().min(value.len());
        while !value.is_char_boundary(at) {
            at -= 1;
        }
        let mut next = value;
        next.insert_str(at, text);
        self.set_text_at(next, at + text.len(), window, cx);
        // `set_value` emits no change event, so a typed `/` opens the menu here.
        self.refresh_slash(cx);
    }

    fn insert_mentions_at_caret(
        &mut self,
        paths: &[String],
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let cursor = self.input.read(cx).cursor();
        let (text, caret) = insert_mentions(&self.value(cx), cursor, paths);
        self.set_text_at(text, caret, window, cx);
    }

    /// Files dropped from Finder: images attach, as a pasted file does; any other file or
    /// folder goes in at the caret as a quoted path. A drop while the composer submits is
    /// ignored.
    fn drop_paths(&mut self, paths: &[PathBuf], window: &mut Window, cx: &mut Context<Self>) {
        if self.processing || self.locked() {
            return;
        }
        let (images, others): (Vec<PathBuf>, Vec<PathBuf>) = paths
            .iter()
            .cloned()
            .partition(|path| mime_for_path(path).is_some());
        if !others.is_empty() {
            let cwd = self.mention_cwd(cx);
            let mentions: Vec<String> = others
                .iter()
                .map(|path| mention_path(path, cwd.as_deref()))
                .collect();
            self.insert_mentions_at_caret(&mentions, window, cx);
        }
        self.focus(window, cx);
        if images.is_empty() {
            return;
        }
        cx.spawn(async move |this, cx| {
            let loaded = cx
                .background_executor()
                .spawn(async move { load_images(images) })
                .await;
            let _ = this.update(cx, |this, cx| {
                this.attachments.extend(loaded);
                cx.notify();
            });
        })
        .detach();
    }

    /// A Finder drag moved somewhere in the window: the overlay shows while it is over the
    /// composer and a drop would attach.
    fn drag_move(&mut self, event: &DragMoveEvent<ExternalPaths>, cx: &mut Context<Self>) {
        let accepts = !self.processing && !self.locked();
        let over = drag_over(event.bounds, event.event.position, accepts);
        if self.drop.set_over(over, Instant::now()) {
            cx.notify();
        }
    }

    /// `FileDropBackdrop`, faded by opacity. It has no listeners, so it takes no hitbox and the
    /// root `on_drop` still gets the drop. `None` once it has faded out.
    fn drop_backdrop(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Option<Div> {
        if self.drop.since.is_none() && !self.drop.over {
            return None;
        }
        let now = Instant::now();
        // A drag that left the window or ended clears the active drag and refreshes.
        if !cx.has_active_drag() {
            self.drop.set_over(false, now);
        }
        if self.drop.fading(now) {
            window.request_animation_frame();
        } else {
            self.drop.since = None;
            if !self.drop.over {
                return None;
            }
        }
        let t = tokens(cx);
        Some(
            div()
                .absolute()
                .inset_0()
                .flex()
                .items_center()
                .justify_center()
                .opacity(self.drop.opacity(now))
                .child(
                    div()
                        .absolute()
                        .inset_0()
                        .bg(t.surface0)
                        .opacity(DROP_BACKDROP_OPACITY),
                )
                .child(
                    v_flex()
                        .items_center()
                        .gap(zoomed(SPACE_2, cx))
                        .child(
                            icon(IconName::Upload)
                                .with_size(zoomed(DROP_GLYPH, cx))
                                .text_color(t.foreground),
                        )
                        .child(
                            div()
                                .text_size(ui_font_size(FONT_BASE, cx))
                                .line_height(relative(1.2))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(t.foreground)
                                .child(DROP_FILES_HERE),
                        ),
                ),
        )
    }

    /// Enter picks a menu row, or sends. Cmd+Enter (`secondary`) queues during a run. The
    /// textarea inserts the new line itself for Shift+Enter, and still reports it here.
    fn enter(&mut self, secondary: bool, shift: bool, window: &mut Window, cx: &mut Context<Self>) {
        if shift {
            return;
        }
        if self.pick_mention(self.mention.active, window, cx) {
            return;
        }
        let matches = self.slash_matches(cx);
        if !matches.is_empty() {
            let item = &matches[self.slash_active.min(matches.len() - 1)];
            // A client command that is already complete sends; picking it again would loop.
            if self.value(cx) != self.picked_text(item, cx).0 {
                self.pick(item, window, cx);
                return;
            }
        }
        let key = if secondary {
            SendKey::ModEnter
        } else {
            SendKey::Enter
        };
        self.submit(secondary, Some(key), window, cx);
    }

    fn submit(
        &mut self,
        queue: bool,
        send_key: Option<SendKey>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.processing || self.locked() {
            return;
        }
        let value = self.value(cx);
        let attachment_count = self.attachment_total();
        if !self.is_draft()
            && let Some(command) = resolve_client_command(&value, attachment_count)
        {
            self.run_client_command(command, window, cx);
            return;
        }
        let commands = self.plugin_commands(cx);
        let names: Vec<&'static str> = commands.iter().map(|command| command.value.name).collect();
        if let Some((name, args)) = resolve_plugin_command(&value, attachment_count, &names) {
            let Some(command) = commands.iter().find(|command| command.value.name == name) else {
                return;
            };
            let input = SlashInput {
                workspace_id: self.watched.workspace_id.clone(),
                agent_id: self.plugin_agent_id().map(String::from),
                args,
            };
            self.send_error = None;
            self.clear_input(window, cx);
            (command.value.on_submit)(input, window, cx);
            return;
        }
        let allow_empty = self
            .draft
            .as_ref()
            .is_some_and(|draft| draft.options.allow_empty_submit);
        let text = match outgoing_text(&value, attachment_count) {
            Some(text) => text,
            None if allow_empty => String::new(),
            None => return,
        };
        let (running, supports_queue) = (self.watched.running(), self.watched.supports_queue);
        let plan = send_plan(running, queue, supports_queue);
        let interceptors: Vec<_> = plugin_index(cx)
            .composer_interceptors()
            .iter()
            .map(|interceptor| interceptor.value.intercept.clone())
            .collect();
        if !needs_intercept(interceptors.len()) {
            self.finish_submit(text, plan, window, cx);
            return;
        }
        // Interceptors run in registration order; each sees the text the previous one gave.
        self.processing = true;
        cx.notify();
        let target = self.target();
        let action = match plan {
            SendPlan::Queue => SendAction::Queue,
            SendPlan::Send(_) => SendAction::Send,
        };
        cx.spawn_in(window, async move |this, cx| {
            let count = interceptors.len();
            let outcome = run_chain(text, count, |at, text| {
                let input = InterceptInput {
                    target: target.clone(),
                    text,
                    action,
                    send_key,
                };
                let task = cx.update(|_, cx| (interceptors[at])(input, cx)).ok();
                async move {
                    match task {
                        Some(task) => task.await,
                        None => InterceptOutcome::Cancel,
                    }
                }
            })
            .await;
            let _ = this.update_in(cx, |this, window, cx| {
                this.processing = false;
                // A cancel stops the send and keeps the input as it is.
                if let Some(text) = outcome {
                    this.finish_submit(text, plan, window, cx);
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Sends `text` with the images and chips, or emits `DraftSubmit` for a draft.
    fn finish_submit(
        &mut self,
        text: String,
        plan: SendPlan,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.is_draft() {
            // An interceptor may have changed the text; the owner reads it from the input.
            if text != self.value(cx).trim_end() {
                self.set_text(text, window, cx);
            }
            cx.emit(ComposerEvent::DraftSubmit);
            return;
        }
        let attachments = std::mem::take(&mut self.attachments);
        let chips = outgoing_attachments(&self.raw_json(), self.plugin_attachments(cx));
        let count = attachments.len();
        let message = OutgoingMessage {
            text: text.clone(),
            images: attachments.iter().map(Attachment::to_json).collect(),
            attachments: chips,
        };
        let agent_id = self.agent_id.clone();
        let task = self.client.update(cx, |client, cx| match plan {
            SendPlan::Send(behavior) => client.send_message(&agent_id, message, behavior, cx),
            SendPlan::Queue => client.enqueue_message(&agent_id, message, cx),
        });
        // Clear at once so the sent row and the input stay in step; a failure puts it back.
        let raw_attachments = std::mem::take(&mut self.raw_attachments);
        let plugin_chips = std::mem::take(&mut self.plugin_chips);
        self.send_error = None;
        self.clear_input(window, cx);
        cx.emit(ComposerEvent::Sent {
            text: text.clone(),
            images: count,
        });
        cx.spawn_in(window, async move |this, cx| {
            let result = task.await;
            let _ = this.update_in(cx, |this, window, cx| {
                let Some(error) = send_error_text(&result) else {
                    return;
                };
                log::warn!("cannot send to {}: {error}", this.agent_id);
                this.send_error = Some(error);
                if let Some(text) = restore_after_error(&this.value(cx), &text) {
                    this.set_text(text, window, cx);
                }
                if this.attachments.is_empty() {
                    this.attachments = attachments;
                }
                if this.raw_attachments.is_empty() {
                    this.raw_attachments = raw_attachments;
                }
                if this.plugin_chips.is_empty() {
                    this.plugin_chips = plugin_chips;
                }
                cx.notify();
            });
        })
        .detach();
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
        self.request_mentions(cx);
        self.slash_dismissed = false;
        self.slash_active = 0;
        cx.notify();
    }

    fn stop(&mut self, cx: &mut Context<Self>) {
        let agent_id = self.agent_id.clone();
        self.cancelling = true;
        self.client
            .update(cx, |client, cx| client.cancel_agent(&agent_id, cx));
        cx.notify();
    }

    fn queue_action(&mut self, action: QueueAction, window: &mut Window, cx: &mut Context<Self>) {
        let agent_id = self.agent_id.clone();
        match action {
            QueueAction::Move(id, direction) => {
                if let Some(before) = move_target(&self.watched.queued, &id, direction) {
                    self.client.update(cx, |client, cx| {
                        client.reorder_queued(&agent_id, &id, before.as_deref(), cx)
                    });
                }
            }
            QueueAction::Cancel(id) => self
                .client
                .update(cx, |client, cx| client.cancel_queued(&agent_id, &id, cx)),
            // Edit takes the item back: it leaves the queue and its text goes into the input.
            // A queued prompt holds text only, so its images do not come back.
            QueueAction::Edit(id) => {
                let Some(text) = self
                    .watched
                    .queued
                    .iter()
                    .find(|item| item.id == id)
                    .map(|item| item.text.clone())
                else {
                    return;
                };
                self.client
                    .update(cx, |client, cx| client.cancel_queued(&agent_id, &id, cx));
                self.set_text(text, window, cx);
                self.focus(window, cx);
            }
            QueueAction::SendNow(id) => self
                .client
                .update(cx, |client, cx| client.promote_queued(&agent_id, &id, cx)),
            QueueAction::Resume => self
                .client
                .update(cx, |client, cx| client.resume_queue(&agent_id, cx)),
        }
        cx.notify();
    }

    fn archive_finished(&mut self, cx: &mut Context<Self>) {
        let ids = finished_paseo_ids(&self.watched.subagents);
        if ids.is_empty() {
            return;
        }
        for id in &ids {
            self.client
                .update(cx, |client, cx| client.archive_agent(id, cx));
        }
        self.archive = ArchiveFinished { ids };
        cx.notify();
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

    /// Opens the searchable list of a picker source.
    fn open_picker(
        &mut self,
        source: Contributed<AttachmentSource>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(picker) = source.value.picker.clone() else {
            return;
        };
        let query =
            cx.new(|cx| InputState::new(window, cx).placeholder(picker.search_placeholder.clone()));
        let subscription = cx.subscribe_in(
            &query,
            window,
            |this: &mut Composer, _, event: &InputEvent, window, cx| match event {
                InputEvent::Change => this.search_picker(cx),
                InputEvent::PressEnter { .. } => {
                    if let Some(item) = this.picker.as_ref().and_then(|p| p.items.first().cloned())
                    {
                        this.pick_item(item, window, cx);
                    }
                }
                _ => {}
            },
        );
        query.update(cx, |query, cx| query.focus(window, cx));
        self.picker = Some(Picker {
            source,
            query,
            items: Vec::new(),
            generation: Generation::default(),
            status: Some("Searching...".into()),
            _subscription: subscription,
        });
        self.search_picker(cx);
    }

    /// Asks the plugin for the items that match the query; an older answer is dropped.
    fn search_picker(&mut self, cx: &mut Context<Self>) {
        let Some(picker) = self.picker.as_mut() else {
            return;
        };
        let Some(method) = picker.source.value.picker.as_ref().map(|p| p.search_method) else {
            return;
        };
        let generation = picker.generation.next();
        let query = picker.query.read(cx).value().to_string();
        let plugin_id = picker.source.plugin_id;
        picker.status = Some("Searching...".into());
        let task = self.client.update(cx, |client, cx| {
            client.invoke_plugin_rpc(plugin_id, method, json!({ "query": query }), cx)
        });
        cx.spawn(async move |this, cx| {
            let answer = task.await.and_then(decode_attachment_search);
            let _ = this.update(cx, |this, cx| {
                let Some(picker) = this.picker.as_mut() else {
                    return;
                };
                if !picker.generation.accepts(generation) {
                    return;
                }
                match answer {
                    Ok(items) => {
                        picker.status = items.is_empty().then(|| "No results".into());
                        picker.items = items;
                    }
                    Err(error) => {
                        picker.items.clear();
                        picker.status = Some(error.to_string());
                    }
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    /// A pick toggles the item's chip and closes the list.
    fn pick_item(&mut self, item: AttachmentItem, window: &mut Window, cx: &mut Context<Self>) {
        let Some(picker) = self.picker.take() else {
            return;
        };
        let source = &picker.source;
        let chip = ComposerAttachment {
            agent_id: self.agent_id.clone(),
            plugin_id: source.plugin_id.to_string(),
            source_id: source.value.id.to_string(),
            source_title: source.value.title.to_string(),
            icon: source.value.icon,
            item,
            commentable: false,
        };
        let mut chips: Vec<ComposerAttachment> = self
            .plugin_chips
            .iter()
            .map(|chip| chip.attachment.clone())
            .collect();
        toggle_chip(&mut chips, chip);
        self.set_chips(chips, window, cx);
        self.focus(window, cx);
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
    /// A draft has no agent yet, so it keeps the value itself.
    fn send_feature(
        &mut self,
        request: Option<(String, serde_json::Value)>,
        cx: &mut Context<Self>,
    ) {
        let Some((feature_id, value)) = request else {
            return;
        };
        if let Some(draft) = self.draft.as_mut() {
            apply_feature(&mut draft.features, &feature_id, &value);
            cx.notify();
            return;
        }
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
                this.watched = Watched::read(this.client.read(cx), &this.agent_id, false);
                cx.notify();
            });
        })
        .detach();
    }

    /// The features the controls show: the agent's, or the draft's own.
    fn features(&self) -> &[AgentFeature] {
        match &self.draft {
            Some(draft) => &draft.features,
            None => &self.watched.features,
        }
    }

    fn control_handlers(&self, cx: &Context<Self>) -> ControlHandlers {
        let draft_pick = |set: fn(&mut Composer, &str, &mut Context<Composer>)| -> PickFn {
            let weak = cx.weak_entity();
            Rc::new(move |id: &str, cx: &mut App| {
                weak.update(cx, |this, cx| set(this, id, cx)).ok();
            })
        };
        let pick = |set: fn(&mut Client, &str, &str, &mut Context<Client>)| -> PickFn {
            let (client, agent_id) = (self.client.clone(), self.agent_id.clone());
            Rc::new(move |id: &str, cx: &mut App| {
                client.update(cx, |client, cx| set(client, &agent_id, id, cx))
            })
        };
        let (model, thinking, mode) = if self.is_draft() {
            (
                draft_pick(|this, key, cx| this.pick_draft_model(key, cx)),
                draft_pick(|this, id, cx| {
                    if let Some(draft) = this.draft.as_mut() {
                        draft.thinking_id = Some(id.to_string());
                        this.reload_draft(cx);
                    }
                }),
                draft_pick(|this, id, cx| {
                    if let Some(draft) = this.draft.as_mut() {
                        draft.mode_id = Some(id.to_string());
                        cx.notify();
                    }
                }),
            )
        } else {
            (
                pick(Client::set_model),
                pick(Client::set_thinking),
                pick(Client::set_mode),
            )
        };
        let (toggle_weak, select_weak) = (cx.weak_entity(), cx.weak_entity());
        ControlHandlers {
            model,
            thinking,
            mode,
            features: FeatureHandlers {
                toggle: Rc::new(move |feature_id: &str, cx: &mut App| {
                    toggle_weak
                        .update(cx, |this, cx| {
                            let request = toggle_request(this.features(), feature_id);
                            this.send_feature(request, cx);
                        })
                        .ok();
                }),
                select: Rc::new(move |feature_id: &str, option_id: &str, cx: &mut App| {
                    select_weak
                        .update(cx, |this, cx| {
                            let request = select_request(this.features(), feature_id, option_id);
                            this.send_feature(request, cx);
                        })
                        .ok();
                }),
            },
        }
    }

    /// The model, thinking, mode, and feature controls in a box that measures the width it
    /// gets, so the next frame picks the density for it.
    fn controls(&self, cx: &mut Context<Self>) -> AnyElement {
        let t = tokens(cx);
        let handlers = self.control_handlers(cx);
        let (view, presence) = match &self.draft {
            Some(draft) => {
                let client = self.client.read(cx);
                // With no folder the draft has no config: provider and model only.
                let configured = draft.cwd().is_some();
                let thinking = configured
                    .then(|| {
                        let provider = draft.provider.as_deref()?;
                        let model = draft.model.as_deref()?;
                        let mut thinking = client.model_thinking(provider, model)?;
                        if draft.thinking_id.is_some() {
                            thinking.current = draft.thinking_id.clone();
                        }
                        Some(thinking)
                    })
                    .flatten();
                let current = draft
                    .provider
                    .as_deref()
                    .zip(draft.model.as_deref())
                    .map(|(provider, model)| model_key(provider, model));
                let features: &[AgentFeature] = if configured { &draft.features } else { &[] };
                let modes = if configured {
                    draft.modes.clone()
                } else {
                    Vec::new()
                };
                let presence =
                    control_presence(true, thinking.is_some(), !modes.is_empty(), features);
                (
                    ControlsView {
                        model_glyph: provider_glyph(draft.provider.as_deref(), t.foreground_muted),
                        model_groups: Some(model_groups(&self.watched.providers)),
                        models: Vec::new(),
                        current_model: current,
                        thinking,
                        modes,
                        current_mode: draft.mode_id.clone(),
                        features,
                        requests: &self.requests,
                    },
                    presence,
                )
            }
            None => {
                let watched = &self.watched;
                let controls = watched.controls.clone().unwrap_or(AgentControls {
                    modes: Vec::new(),
                    current_mode: None,
                    models: Vec::new(),
                    current_model: None,
                    running: false,
                });
                let presence = control_presence(
                    true,
                    watched.thinking.is_some(),
                    !controls.modes.is_empty(),
                    &watched.features,
                );
                (
                    ControlsView {
                        model_glyph: provider_glyph(
                            watched.provider.as_deref(),
                            t.foreground_muted,
                        ),
                        model_groups: None,
                        models: controls.models,
                        current_model: controls.current_model,
                        thinking: watched.thinking.clone(),
                        modes: controls.modes,
                        current_mode: controls.current_mode,
                        features: &watched.features,
                        requests: &self.requests,
                    },
                    presence,
                )
            }
        };
        let pills = agent_controls(view, self.density, &handlers, cx);
        density_box(self.density, presence, cx.weak_entity(), pills)
    }

    /// The plugin pills of each slot, drawn with their state. A pill whose `state` is `None`
    /// is not drawn.
    fn plugin_pills(&self, index: &ContributionIndex, cx: &mut Context<Self>) -> PluginPills {
        let slots = pill_slots(
            index.composer_pills(),
            self.watched.workspace_id.as_deref(),
            self.plugin_agent_id(),
            self.is_draft(),
        );
        if slots.track.is_empty() && slots.toolbar.is_empty() {
            return PluginPills::default();
        }
        let target = self.target();
        let weak = cx.weak_entity();
        let show_panel: ShowPanel = Rc::new(move |plugin_id, panel_id, cx| {
            weak.update(cx, |_, cx| {
                cx.emit(ComposerEvent::ShowNewWorkspacePanel {
                    plugin_id,
                    panel_id,
                })
            })
            .ok();
        });
        let draw = |pills| -> Vec<AnyElement> {
            pill_states(pills, &target, cx)
                .into_iter()
                .map(|(pill, state)| {
                    plugin_pill(
                        &pill,
                        state,
                        target.clone(),
                        show_panel.clone(),
                        self.popovers.clone(),
                        cx,
                    )
                })
                .collect()
        };
        PluginPills {
            track: draw(&slots.track),
            toolbar: draw(&slots.toolbar),
        }
    }

    /// The ring of a track pill, cached by key so its clock does not restart each frame.
    fn ring(
        &mut self,
        key: String,
        knockout: Knockout,
        cx: &mut Context<Self>,
    ) -> Entity<RunningRing> {
        self.rings
            .entry(key)
            .or_insert_with(|| cx.new(|cx| RunningRing::new(knockout, cx)))
            .clone()
    }

    /// The badge of a subagent row, cached by row key and set to the row's bucket.
    fn badge(
        &mut self,
        key: String,
        bucket: StatusBucket,
        cx: &mut Context<Self>,
    ) -> Entity<StatusBadge> {
        let badge = self
            .badges
            .entry(key)
            .or_insert_with(|| cx.new(|cx| StatusBadge::new(bucket, Knockout::Surface1, cx)));
        if badge.read(cx).bucket() != bucket {
            badge.update(cx, |badge, cx| badge.set_bucket(bucket, cx));
        }
        badge.clone()
    }

    /// The docked strip: tasks, subagents, and plugin track pills, then the turn metrics and
    /// the diff stat at the right edge. `None` when nothing shows.
    fn track_strip(
        &mut self,
        plugin_track: Vec<AnyElement>,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let progress = self.archive.progress(&self.watched.subagents);
        let leading = has_agent_tracks(
            self.watched.subagents.len(),
            self.watched.tasks.len(),
            progress.is_some(),
            !plugin_track.is_empty(),
        );
        if !strip_visible(
            self.watched.diff_stat.is_some(),
            self.watched.has_turn_metrics(),
            leading,
        ) {
            return None;
        }
        // The pill's ring entity lives as long as the pill runs, and is dropped with the pill.
        let key = "subagents-track-header".to_string();
        let running = self
            .watched
            .subagents
            .iter()
            .any(|row| row_bucket(row) == StatusBucket::Running);
        let ring = if running {
            Some(self.ring(key.clone(), Knockout::None, cx))
        } else {
            self.rings.remove(&key);
            None
        };
        // One badge per listed row, kept across frames so its ring does not restart.
        let rows: Vec<(String, StatusBucket)> = self
            .watched
            .subagents
            .iter()
            .map(|row| (row.id.clone(), row_bucket(row)))
            .collect();
        let badges: HashMap<String, Entity<StatusBadge>> = rows
            .iter()
            .filter(|(_, bucket)| *bucket != StatusBucket::Done)
            .map(|(id, bucket)| (id.clone(), self.badge(id.clone(), *bucket, cx)))
            .collect();
        self.badges.retain(|key, _| {
            rows.iter()
                .any(|(id, bucket)| id == key && *bucket != StatusBucket::Done)
        });
        let watched = &self.watched;
        let weak = cx.weak_entity();
        let (open_weak, provider_weak, archive_weak, changes_weak) =
            (weak.clone(), weak.clone(), weak.clone(), weak);
        let client = self.client.clone();
        let supports_open = self.client.read(cx).supports_provider_subagent_timeline();
        let parent_agent_id = self.agent_id.clone();
        let actions = SubagentActions {
            open: Rc::new(move |agent_id: &str, cx: &mut App| {
                let agent_id = agent_id.to_string();
                open_weak
                    .update(cx, |_, cx| cx.emit(ComposerEvent::OpenAgent { agent_id }))
                    .ok();
            }),
            open_provider: Rc::new(
                move |parent_agent_id: &str, subagent_id: &str, cx: &mut App| {
                    let (parent_agent_id, subagent_id) =
                        (parent_agent_id.to_string(), subagent_id.to_string());
                    provider_weak
                        .update(cx, |_, cx| {
                            cx.emit(ComposerEvent::OpenProviderSubagent {
                                parent_agent_id,
                                subagent_id,
                            })
                        })
                        .ok();
                },
            ),
            archive: Rc::new(move |agent_id: &str, cx: &mut App| {
                client.update(cx, |client, cx| client.archive_agent(agent_id, cx));
            }),
            archive_finished: Rc::new(move |cx: &mut App| {
                archive_weak
                    .update(cx, |this, cx| this.archive_finished(cx))
                    .ok();
            }),
        };
        let leading: Vec<AnyElement> = task_pill(watched.tasks.clone())
            .into_iter()
            .chain(subagents_pill(
                watched.subagents.clone(),
                &parent_agent_id,
                progress,
                ring,
                badges,
                supports_open,
                actions,
            ))
            .chain(plugin_track)
            .collect();
        let workspace_id = watched.workspace_id.clone();
        let trailing: Vec<AnyElement> = turn_metrics_pill(&self.metric_parts(), cx)
            .into_iter()
            .chain(diff_stat_pill(
                watched.diff_stat,
                move |_, cx| {
                    let Some(workspace_id) = workspace_id.clone() else {
                        return;
                    };
                    changes_weak
                        .update(cx, |_, cx| {
                            cx.emit(ComposerEvent::OpenChanges { workspace_id })
                        })
                        .ok();
                },
                cx,
            ))
            .collect();
        Some(TrackStrip::new(leading, trailing).into_any_element())
    }
}

/// The plugin pills of the track and the toolbar, drawn.
#[derive(Default)]
struct PluginPills {
    track: Vec<AnyElement>,
    toolbar: Vec<AnyElement>,
}

/// Wraps the controls in a box that fills the free toolbar width and reads that width after
/// layout. A width that crosses a density floor sets the new density for the next frame.
fn density_box(
    current: Density,
    presence: ControlPresence,
    weak: WeakEntity<Composer>,
    pills: Vec<AnyElement>,
) -> AnyElement {
    let measure = canvas(
        move |bounds, _, cx| {
            // The density floors are 100% widths, and the controls grow with the zoom.
            let available = f32::from(bounds.size.width) / bachuc_theme::zoom(cx);
            let next = resolve_density(available, current, &presence);
            if next != current {
                cx.defer(move |cx| {
                    weak.update(cx, |this, cx| {
                        if this.density != next {
                            this.density = next;
                            cx.notify();
                        }
                    })
                    .ok();
                });
            }
        },
        |_, _, _, _| {},
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full();
    div()
        .relative()
        .flex_1()
        .min_w_0()
        .overflow_hidden()
        .child(measure)
        .child(
            h_flex()
                .w_full()
                .min_w_0()
                .overflow_hidden()
                .items_center()
                .children(pills),
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

/// The text to send, or `None` when there is nothing to send: blank text and no attachment.
fn outgoing_text(value: &str, attachments: usize) -> Option<String> {
    can_send(value, attachments).then(|| value.trim_end().to_string())
}

/// The send error under the queue: the error text of a failed send.
fn send_error_text(result: &anyhow::Result<()>) -> Option<String> {
    result.as_ref().err().map(|error| error.to_string())
}

/// After a failed send the sent text goes back into the input, unless the user typed again.
fn restore_after_error(current: &str, sent: &str) -> Option<String> {
    current.is_empty().then(|| sent.to_string())
}

/// The primary button of a draft: a spinner while it submits or interceptors run, else the
/// submit button, shown even with empty text when the draft allows it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DraftPrimary {
    Spinner,
    Submit(SubmitIcon),
    None,
}

fn draft_primary(options: &DraftOptions, busy: bool, sendable: bool) -> DraftPrimary {
    if busy {
        DraftPrimary::Spinner
    } else if options.allow_empty_submit || sendable {
        DraftPrimary::Submit(options.submit_icon)
    } else {
        DraftPrimary::None
    }
}

fn load_images(paths: Vec<PathBuf>) -> Vec<Attachment> {
    paths
        .iter()
        .filter(|path| mime_for_path(path).is_some())
        .filter_map(|path| Attachment::from_path(path))
        .collect()
}

fn spinner(cx: &App) -> AnyElement {
    div()
        .id("composer-busy")
        .role(Role::ProgressIndicator)
        .aria_label("Sending")
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .size(zoomed(CONTROL, cx))
        .ml(zoomed(SPACE_1, cx))
        .child(
            Spinner::new()
                .with_size(zoomed(ICON_SM, cx))
                .color(tokens(cx).foreground_muted),
        )
        .into_any_element()
}

/// Where an autocomplete menu sits: above the card, as an overlay that takes no layout height.
/// The gap is the `AutocompletePopover` `OFFSET_FROM_ANCHOR`.
fn popup_anchor(cx: &App) -> Div {
    v_flex()
        .absolute()
        .bottom_full()
        .left_0()
        .right_0()
        .pb(zoomed(SPACE_3, cx))
        .gap(zoomed(SPACE_1, cx))
}

/// The bordered box of an autocomplete menu.
fn popup_box(cx: &App) -> Div {
    let t = tokens(cx);
    div()
        .bg(t.surface1)
        .border_1()
        .border_color(t.border_accent)
        .rounded(zoomed(RADIUS_LG, cx))
        .shadow(vec![shadow(ShadowSize::Md, cx)])
        .overflow_hidden()
        .occlude()
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
                .min_h(zoomed(36., cx))
                .px(zoomed(SPACE_3, cx))
                .py(zoomed(SPACE_2, cx))
                .gap(zoomed(SPACE_2, cx))
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
                .gap(zoomed(SPACE_1, cx))
                .px(zoomed(SPACE_3, cx))
                .py(zoomed(SPACE_3, cx))
                .line_height(relative(1.2))
                .bg(t.surface1)
                .border_1()
                .border_color(t.border_accent)
                .rounded(zoomed(RADIUS_LG, cx))
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
        Some(
            popup_anchor(cx).children(detail).child(
                popup_box(cx).child(
                    v_flex()
                        .id("composer-slash-menu")
                        .role(Role::Menu)
                        .aria_label("Slash commands")
                        .track_scroll(&self.slash_scroll)
                        .max_h(zoomed(SLASH_MAX_HEIGHT, cx))
                        .overflow_y_scroll()
                        .py(zoomed(SPACE_1, cx))
                        .children(rows),
                ),
            ),
        )
    }

    /// The `@` file menu, in the slash menu's frame: a folder or file glyph and the path per
    /// row, best match at the bottom, or the search status while it has no rows.
    fn mention_menu(&self, cx: &mut Context<Self>) -> Option<impl IntoElement + use<>> {
        if !self.mention_open(cx) {
            return None;
        }
        let t = tokens(cx);
        let size = ui_font_size(FONT_BASE, cx);
        let items = &self.mention.items;
        let active = self.mention.active.min(items.len().saturating_sub(1));
        let rows = items.iter().enumerate().rev().map(|(ix, item)| {
            let glyph = if item.is_directory {
                IconName::Folder
            } else {
                IconName::File
            };
            let pick = cx.listener(move |this, _, window, cx| {
                this.pick_mention(ix, window, cx);
            });
            let weak = cx.weak_entity();
            h_flex()
                .id(("composer-mention-item", ix))
                .role(Role::Button)
                .aria_label(item.path.clone())
                .aria_selected(ix == active)
                .min_h(zoomed(36., cx))
                .px(zoomed(SPACE_3, cx))
                .py(zoomed(SPACE_2, cx))
                .gap(zoomed(SPACE_2, cx))
                .items_center()
                .cursor_pointer()
                .text_size(size)
                .line_height(relative(1.2))
                .when(ix == active, |row| row.bg(t.surface2))
                .hover(|row| row.bg(t.surface2))
                .on_click(pick)
                .on_a11y_action(AccessibleAction::Click, move |_, window, cx| {
                    weak.update(cx, |this, cx| {
                        this.pick_mention(ix, window, cx);
                    })
                    .ok();
                })
                .child(
                    icon(glyph)
                        .with_size(zoomed(ICON_SM, cx))
                        .text_color(t.foreground_muted),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .text_color(t.foreground)
                        .child(item.path.clone()),
                )
        });
        let status = self.mention.status().map(|status| {
            div()
                .px(zoomed(SPACE_3, cx))
                .py(zoomed(SPACE_2, cx))
                .text_size(ui_font_size(FONT_SM, cx))
                .line_height(relative(1.2))
                .text_color(t.foreground_muted)
                .child(status)
        });
        Some(
            popup_anchor(cx).child(
                popup_box(cx).child(
                    v_flex()
                        .id("composer-mention-menu")
                        .role(Role::Menu)
                        .aria_label("Files")
                        .track_scroll(&self.mention_scroll)
                        .max_h(zoomed(SLASH_MAX_HEIGHT, cx))
                        .overflow_y_scroll()
                        .py(zoomed(SPACE_1, cx))
                        .children(rows)
                        .children(status),
                ),
            ),
        )
    }

    /// The searchable list of a picker source, above the card like the slash menu.
    fn picker_list(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let picker = self.picker.as_ref()?;
        let t = tokens(cx);
        let size = ui_font_size(FONT_BASE, cx);
        let small = ui_font_size(FONT_SM, cx);
        let title = picker
            .source
            .value
            .picker
            .as_ref()
            .map(|picker| picker.title.clone())
            .unwrap_or_default();
        let rows = picker.items.iter().enumerate().map(|(index, item)| {
            let picked = item.clone();
            h_flex()
                .id(("composer-picker-item", index))
                .role(Role::MenuItem)
                .aria_label(format!("{} {}", item.identifier, item.title))
                .min_h(zoomed(32., cx))
                .px(zoomed(SPACE_3, cx))
                .gap(zoomed(SPACE_2, cx))
                .items_center()
                .cursor_pointer()
                .text_size(size)
                .hover(move |row| row.bg(t.surface2))
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.pick_item(picked.clone(), window, cx)
                }))
                .child(
                    div()
                        .flex_none()
                        .text_color(t.foreground_muted)
                        .child(item.identifier.clone()),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .text_color(t.foreground)
                        .child(item.title.clone()),
                )
        });
        Some(
            v_flex()
                .absolute()
                .bottom_full()
                .left_0()
                .w(zoomed(360., cx))
                .pb(zoomed(SPACE_3, cx))
                .child(
                    v_flex()
                        .id("composer-picker")
                        .role(Role::Dialog)
                        .aria_label(title.clone())
                        .bg(t.surface1)
                        .border_1()
                        .border_color(t.border_accent)
                        .rounded(zoomed(RADIUS_LG, cx))
                        .shadow(vec![shadow(ShadowSize::Md, cx)])
                        .occlude()
                        .p(zoomed(SPACE_2, cx))
                        .gap(zoomed(SPACE_1, cx))
                        .child(
                            h_flex()
                                .justify_between()
                                .items_center()
                                .text_size(small)
                                .text_color(t.foreground_muted)
                                .child(title)
                                .child(
                                    round_button(
                                        "composer-picker-close",
                                        IconName::X,
                                        "Close",
                                        24.,
                                        None,
                                        t.foreground_muted,
                                        t.surface2,
                                        cx,
                                    )
                                    .on_click(cx.listener(
                                        |this, _, _, cx| {
                                            this.picker = None;
                                            cx.notify();
                                        },
                                    )),
                                ),
                        )
                        .child(Input::new(&picker.query).small().aria_label("Search"))
                        .child(
                            v_flex()
                                .id("composer-picker-list")
                                .role(Role::Menu)
                                .aria_label("Results")
                                .max_h(zoomed(SLASH_MAX_HEIGHT, cx))
                                .overflow_y_scroll()
                                .children(rows),
                        )
                        .children(picker.status.clone().map(|status| {
                            div()
                                .px(zoomed(SPACE_3, cx))
                                .py(zoomed(SPACE_2, cx))
                                .text_size(small)
                                .text_color(t.foreground_muted)
                                .child(status)
                        })),
                )
                .into_any_element(),
        )
    }

    /// The text box with its placeholder and focus hint.
    fn input_box(&self, window: &mut Window, cx: &mut Context<Self>) -> Div {
        let t = tokens(cx);
        let content_size = ui_font_size(FONT_CONTENT, cx);
        let small_size = ui_font_size(FONT_SM, cx);
        let input_line = px(line_height(f32::from(content_size)));
        let empty = self.value(cx).is_empty();
        let focused = self.input.read(cx).focus_handle(cx).is_focused(window);
        let focus_hint = self.focus_hint.clone().filter(|_| empty && !focused);
        let weak = cx.weak_entity();
        // A draft starts `min_lines` tall (`minInputLines`).
        let min_height = self
            .draft
            .as_ref()
            .map(|draft| min_input_height(draft.options.min_lines, f32::from(input_line)));
        // `XSmall` gives the editor no top or bottom padding and 4 px at the sides; the margin
        // takes the sides back. The placeholder and the focus hint draw over the input, as the
        // Electron `::placeholder` rule (`sm`, `surface4`, the input line height) and
        // `focusHintText` do.
        div()
            .relative()
            .mx(px(-INPUT_INSET))
            .when_some(min_height, |input_box, height| input_box.min_h(px(height)))
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
                // One row: the placeholder shrinks and ends in "…" before the hint, which
                // keeps its width. Electron lets the two overlap; Q-163 cuts the placeholder.
                input_box.child(
                    h_flex()
                        .absolute()
                        .top_0()
                        .left(px(INPUT_INSET))
                        .right(px(INPUT_INSET))
                        .items_start()
                        .gap(zoomed(SPACE_2, cx))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .truncate()
                                .text_size(small_size)
                                .line_height(input_line)
                                .text_color(t.surface4)
                                .child(PLACEHOLDER),
                        )
                        .when_some(focus_hint, |row, hint| {
                            row.child(
                                div()
                                    .flex_none()
                                    .text_size(small_size)
                                    .text_color(t.foreground_muted)
                                    .opacity(0.5)
                                    .child(hint),
                            )
                        }),
                )
            })
    }

    /// The attach button. With plugin picker sources it opens a menu: images first, then
    /// "Attach <title>" per source. Without one it picks images at once.
    fn attach_button(&self, index: &ContributionIndex, cx: &mut Context<Self>) -> AnyElement {
        let t = tokens(cx);
        let sources = picker_sources(index);
        let button = round_button(
            "composer-attach",
            IconName::Plus,
            "Attach",
            CONTROL,
            None,
            t.foreground_muted,
            t.surface2,
            cx,
        );
        if sources.is_empty() {
            return button
                .on_click(cx.listener(|this, _, window, cx| this.pick_images(window, cx)))
                .into_any_element();
        }
        let weak = cx.weak_entity();
        Popover::new("composer-attach-menu")
            .anchor(Anchor::BottomLeft)
            .offset(zoomed(POPUP_GAP, cx))
            .p_1()
            .trigger(AttachTrigger { open: false })
            .content(move |_, _, cx| {
                let images_weak = weak.clone();
                let mut rows = vec![
                    crate::agent_controls::menu_row(
                        "composer-attach-images",
                        "Attach images".into(),
                        false,
                        cx,
                    )
                    .on_click(cx.listener(move |state, _, window, cx| {
                        state.dismiss(window, cx);
                        images_weak
                            .update(cx, |this, cx| this.pick_images(window, cx))
                            .ok();
                    }))
                    .into_any_element(),
                ];
                for (index, source) in sources.iter().enumerate() {
                    let (source, weak) = (source.clone(), weak.clone());
                    let glyph = source.value.icon;
                    rows.push(
                        crate::agent_controls::menu_row(
                            ("composer-attach-source", index),
                            format!("Attach {}", source.value.title),
                            false,
                            cx,
                        )
                        .child(icon(glyph).with_size(zoomed(ICON_SM, cx)))
                        .on_click(cx.listener(move |state, _, window, cx| {
                            state.dismiss(window, cx);
                            let source = source.clone();
                            weak.update(cx, |this, cx| this.open_picker(source, window, cx))
                                .ok();
                        }))
                        .into_any_element(),
                    );
                }
                v_flex()
                    .id("composer-attach-menu-list")
                    .role(Role::Menu)
                    .aria_label("Attach")
                    .min_w(zoomed(200., cx))
                    .children(rows)
            })
            .into_any_element()
    }

    /// Send, Stop (or a plugin's stop face), a spinner while busy, or the draft submit button.
    fn primary_button(
        &self,
        index: &ContributionIndex,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let t = tokens(cx);
        let sendable = can_send(&self.value(cx), self.attachment_total());
        if let Some(draft) = &self.draft {
            return match draft_primary(
                &draft.options,
                draft.submitting || self.processing,
                sendable,
            ) {
                DraftPrimary::Spinner => Some(spinner(cx)),
                DraftPrimary::None => None,
                DraftPrimary::Submit(submit_icon) => Some(
                    round_button(
                        "composer-submit",
                        match submit_icon {
                            SubmitIcon::Arrow => IconName::ArrowUp,
                            SubmitIcon::Return => IconName::CornerDownLeft,
                        },
                        "Submit",
                        CONTROL,
                        Some(t.accent),
                        t.accent_foreground,
                        t.accent_bright,
                        cx,
                    )
                    .ml(zoomed(SPACE_1, cx))
                    .on_click(
                        cx.listener(|this, _, window, cx| this.submit(false, None, window, cx)),
                    )
                    .into_any_element(),
                ),
            };
        }
        if self.processing {
            return Some(spinner(cx));
        }
        match primary_action(sendable, self.watched.running()) {
            PrimaryAction::Send => Some(
                round_button(
                    "composer-send",
                    IconName::ArrowUp,
                    "Send",
                    CONTROL,
                    Some(t.accent),
                    t.accent_foreground,
                    t.accent_bright,
                    cx,
                )
                .ml(zoomed(SPACE_1, cx))
                .on_click(cx.listener(|this, _, window, cx| this.submit(false, None, window, cx)))
                .into_any_element(),
            ),
            PrimaryAction::Stop => {
                // A plugin stop face replaces the red one; the press still cancels the agent.
                let face = match stop_button(index) {
                    Some(button) => {
                        let props = StopButtonProps {
                            agent_id: self.agent_id.clone(),
                            size: zoomed(CONTROL, cx),
                            icon_size: zoomed(TOOLBAR_GLYPH, cx),
                            cancelling: self.cancelling,
                        };
                        (button.value.render)(&props, window, cx)
                    }
                    None => div()
                        .size_full()
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded_full()
                        .bg(rgb(STOP_RED))
                        .child(
                            div()
                                .size(zoomed(10., cx))
                                .rounded(zoomed(2., cx))
                                .bg(rgb(0xffffff)),
                        )
                        .into_any_element(),
                };
                Some(
                    div()
                        .id("composer-stop")
                        .role(Role::Button)
                        .aria_label("Stop")
                        .flex()
                        .flex_none()
                        .items_center()
                        .justify_center()
                        .size(zoomed(CONTROL, cx))
                        .ml(zoomed(SPACE_1, cx))
                        .rounded_full()
                        .cursor_pointer()
                        .child(face)
                        .on_click(cx.listener(|this, _, _, cx| this.stop(cx)))
                        .into_any_element(),
                )
            }
            PrimaryAction::None => None,
        }
    }

    /// The image chips, the raw attachment chips, then the plugin chips.
    fn tray(&self, index: &ContributionIndex, cx: &mut Context<Self>) -> Option<AnyElement> {
        let remove_weak = cx.weak_entity();
        let mut chips = image_chips(
            &self.attachments,
            move |index, cx| {
                remove_weak
                    .update(cx, |this, cx| {
                        if index < this.attachments.len() {
                            this.attachments.remove(index);
                            cx.notify();
                        }
                    })
                    .ok();
            },
            cx,
        );
        for (at, raw) in self.raw_attachments.iter().enumerate() {
            let weak = cx.weak_entity();
            let remove = Rc::new(move |cx: &mut App| {
                weak.update(cx, |this, cx| {
                    if at < this.raw_attachments.len() {
                        this.raw_attachments.remove(at);
                        cx.notify();
                    }
                })
                .ok();
            });
            chips.push(raw_chip(at, &raw.label, remove, cx));
        }
        for (at, chip) in self.plugin_chips.iter().enumerate() {
            let on_open = index
                .attachment_sources()
                .iter()
                .find(|source| {
                    source.plugin_id == chip.attachment.plugin_id
                        && source.value.id == chip.attachment.source_id
                })
                .and_then(|source| source.value.on_open.clone())
                .map(|on_open| {
                    let item = chip.attachment.item.clone();
                    Rc::new(move |window: &mut Window, cx: &mut App| {
                        on_open(item.clone(), window, cx)
                    }) as Rc<dyn Fn(&mut Window, &mut App)>
                });
            let weak = cx.weak_entity();
            chips.push(plugin_chip(
                at,
                &chip.attachment,
                chip.comment.as_ref(),
                on_open,
                move |at, cx| {
                    weak.update(cx, |this, cx| {
                        if at < this.plugin_chips.len() {
                            this.plugin_chips.remove(at);
                            cx.notify();
                        }
                    })
                    .ok();
                },
                cx,
            ));
        }
        attachment_tray(chips, cx)
    }
}

/// A raw attachment chip (`AttachmentFrame` with the chat history label): a file glyph, the
/// label, and its remove button. It is drawn like a plugin chip: 48 px tall, at most 280 wide.
fn raw_chip(index: usize, label: &str, on_remove: Rc<dyn Fn(&mut App)>, cx: &App) -> AnyElement {
    let t = tokens(cx);
    let on_ax_remove = on_remove.clone();
    h_flex()
        .id(("composer-raw-chip", index))
        .role(Role::ListItem)
        .aria_label(format!("Attachment {label}"))
        .items_center()
        .gap(zoomed(SPACE_2, cx))
        .h(zoomed(48., cx))
        .px(zoomed(SPACE_2, cx))
        .max_w(zoomed(280., cx))
        .border_1()
        .border_color(t.border_accent)
        .rounded(zoomed(RADIUS_MD, cx))
        .bg(t.surface1)
        .child(
            icon(IconName::FileText)
                .with_size(zoomed(ICON_SM, cx))
                .text_color(t.foreground_muted),
        )
        .child(
            div()
                .min_w_0()
                .flex_1()
                .truncate()
                .text_size(ui_font_size(FONT_SM, cx))
                .text_color(t.foreground)
                .child(label.to_string()),
        )
        .child(
            round_button(
                ("composer-raw-chip-remove", index),
                IconName::X,
                "Remove attachment",
                24.,
                None,
                t.foreground_muted,
                t.surface3,
                cx,
            )
            .on_click(move |_, _, cx| {
                cx.stop_propagation();
                on_remove(cx);
            })
            .on_a11y_action(AccessibleAction::Click, move |_, _, cx| on_ax_remove(cx)),
        )
        .into_any_element()
}

/// The attach button as a menu trigger.
#[derive(IntoElement)]
struct AttachTrigger {
    open: bool,
}

impl gpui_kit::component::Selectable for AttachTrigger {
    fn selected(mut self, selected: bool) -> Self {
        self.open = selected;
        self
    }

    fn is_selected(&self) -> bool {
        self.open
    }
}

impl RenderOnce for AttachTrigger {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = tokens(cx);
        round_button(
            "composer-attach",
            IconName::Plus,
            "Attach",
            CONTROL,
            self.open.then_some(t.surface2),
            t.foreground_muted,
            t.surface2,
            cx,
        )
        .aria_expanded(self.open)
    }
}

impl Render for Composer {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let rows = max_input_rows(f32::from(window.viewport_size().height), FONT_CONTENT);
        if rows != self.max_rows {
            self.max_rows = rows;
            let min = self
                .draft
                .as_ref()
                .map_or(1, |draft| draft.options.min_lines.max(1));
            self.input
                .update(cx, |input, cx| input.set_auto_grow(min, rows.max(min), cx));
        }

        // Pill `state` closures run here, in render, not on every client event.
        let index = plugin_index(cx);
        let plugin_pills = self.plugin_pills(&index, cx);
        let matches = self.slash_matches(cx);
        let weak = cx.weak_entity();
        let queue = queue_track(
            &self.watched.queued,
            self.watched.queue_held,
            move |action, window, cx| {
                weak.update(cx, |this, cx| this.queue_action(action, window, cx))
                    .ok();
            },
            cx,
        );
        let send_error = self.send_error.clone().map(|error| {
            div()
                .id("composer-send-error")
                .role(Role::Alert)
                .aria_label(error.clone())
                .text_size(ui_font_size(FONT_BASE, cx))
                .text_color(rgb(SEND_ERROR_RED))
                .child(error)
        });
        let is_draft = self.is_draft();
        let (header, draft_row) = if is_draft {
            (None, draft_pill_row(plugin_pills.track))
        } else {
            (self.track_strip(plugin_pills.track, cx), None)
        };
        // A draft has no agent, so no meter slot.
        let trailing = (!is_draft)
            .then(|| {
                div()
                    .flex_none()
                    .size(zoomed(SLOT, cx))
                    .flex()
                    .items_center()
                    .justify_center()
                    .children(context_window_meter(
                        self.watched.usage,
                        self.watched.usage_pending(),
                        cx,
                    ))
                    .into_any_element()
            })
            .into_iter()
            .collect();
        let card = MessageInput::new(self.input_box(window, cx))
            .header(header)
            .tray(self.tray(&index, cx))
            .attach(Some(self.attach_button(&index, cx)))
            .toolbar(plugin_pills.toolbar)
            .controls(Some(self.controls(cx)))
            .trailing(trailing)
            .primary(self.primary_button(&index, window, cx));
        let picker = self.picker_list(cx);
        let mention_menu = self.mention_menu(cx);
        let drop_backdrop = self.drop_backdrop(window, cx);

        // The timeline's column math, so the card's right edge follows the message text when
        // the corner pills push it left. The width is from the last paint, as the timeline's.
        let inset = bachuc_timeline::column_right_inset(
            f32::from(self.width.get()),
            f32::from(self.corner_reserve),
            bachuc_theme::reading_look(cx).content_width,
            bachuc_theme::zoom(cx),
        );
        let width = self.width.clone();
        let measure = canvas(
            move |area, _, _| width.set(area.size.width),
            |_, _, _, _| {},
        )
        .absolute()
        .size_full();

        // `inputAreaContainer`: 16 px at the sides and bottom, the content centered at 820 px.
        div()
            .w_full()
            .min_h(zoomed(FOOTER_HEIGHT, cx))
            .flex()
            .justify_center()
            .px(zoomed(SPACE_4, cx))
            .pb(zoomed(SPACE_4, cx))
            .relative()
            .when(self.locked(), |area| area.opacity(LOCKED_OPACITY))
            .on_drag_move::<ExternalPaths>(
                cx.listener(|this, event, _, cx| this.drag_move(event, cx)),
            )
            .on_drop::<ExternalPaths>(cx.listener(|this, paths: &ExternalPaths, window, cx| {
                if this.drop.set_over(false, Instant::now()) {
                    cx.notify();
                }
                this.drop_paths(paths.paths(), window, cx)
            }))
            .child(
                v_flex()
                    .w_full()
                    .max_w(zoomed(bachuc_theme::reading_look(cx).content_width, cx))
                    .pr(px(inset))
                    .gap(zoomed(SPACE_3, cx))
                    .children(queue)
                    .children(send_error)
                    .child(
                        v_flex()
                            .relative()
                            .w_full()
                            .children(self.slash_menu(&matches, cx))
                            .children(mention_menu)
                            .children(picker)
                            .children(draft_row)
                            .child(card),
                    ),
            )
            .child(measure)
            .children(drop_backdrop)
    }
}

#[cfg(test)]
mod tests {
    // Named imports: `use super::*` would bring in GPUI's `test` macro over `#[test]`.
    use super::{
        DROP_FADE, DraftPrimary, DropOverlay, FOCUS_ACTION, LOCKED_OPACITY, Watched, draft_primary,
        drag_over, focus_hint, outgoing_text, restore_after_error, send_error_text,
        snapshot_changed,
    };
    use crate::draft::{DraftOptions, SubmitIcon};
    use bachuc_client::{
        ActiveTurn, AgentFeature, AgentFeatureKind, AgentUsage, DiffStat, QueuedPrompt,
        SubagentKind, SubagentRow, TaskEntry, TaskStatus, TurnMetrics,
    };
    use bachuc_command::default_shortcuts;
    use gpui_kit::{Bounds, point, px, size};
    use std::time::{Duration, Instant};

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

    /// A snapshot with every strip source, the usage, and a held queue.
    fn full(diff_stat: Option<DiffStat>, features: Vec<AgentFeature>) -> Watched {
        Watched {
            controls: None,
            thinking: None,
            queued: vec![QueuedPrompt {
                id: "q1".into(),
                text: "next".into(),
            }],
            queue_held: true,
            supports_queue: true,
            provider: Some("codex".into()),
            status: Some("idle".into()),
            workspace_id: Some("w1".into()),
            features,
            diff_stat,
            active_turn: None,
            last_metrics: Some(TurnMetrics {
                ttft_ms: Some(1200.),
                output_tokens: Some(90),
                output_tokens_per_second: Some(45.),
            }),
            usage: Some(AgentUsage {
                context_window_max_tokens: Some(200_000),
                context_window_used_tokens: Some(140_000),
                total_cost_usd: Some(0.0042),
                ..AgentUsage::default()
            }),
            tasks: vec![TaskEntry {
                id: Some("t1".into()),
                text: "Build".into(),
                completed: false,
                status: Some(TaskStatus::InProgress),
                active_form: None,
            }],
            subagents: vec![SubagentRow {
                id: "s1".into(),
                kind: SubagentKind::Paseo,
                provider: "claude".into(),
                title: Some("Child".into()),
                description: None,
                subtitle: None,
                status: "running".into(),
                requires_attention: false,
                created_at: "2026-10-16T10:00:00Z".into(),
            }],
            providers: Vec::new(),
        }
    }

    #[test]
    fn an_unchanged_snapshot_causes_no_redraw() {
        // The client subscription calls `cx.notify()` only when the new `Watched` differs.
        let stat = Some(DiffStat {
            additions: 2,
            deletions: 1,
        });
        let first = full(stat, vec![fast_mode(false)]);
        assert!(!snapshot_changed(
            &first,
            &full(stat, vec![fast_mode(false)])
        ));
        assert!(snapshot_changed(&first, &full(stat, vec![fast_mode(true)])));
        assert!(snapshot_changed(
            &first,
            &full(None, vec![fast_mode(false)])
        ));
        assert!(snapshot_changed(&first, &full(stat, Vec::new())));
    }

    #[test]
    fn each_strip_field_counts_for_the_redraw() {
        let first = full(None, Vec::new());
        let mut held = first.clone();
        held.queue_held = false;
        assert!(snapshot_changed(&first, &held));
        let mut tasks = first.clone();
        tasks.tasks[0].completed = true;
        assert!(snapshot_changed(&first, &tasks));
        let mut subagents = first.clone();
        subagents.subagents[0].status = "idle".into();
        assert!(snapshot_changed(&first, &subagents));
        let mut usage = first.clone();
        usage.usage = None;
        assert!(snapshot_changed(&first, &usage));
        let mut turn = first.clone();
        turn.active_turn = Some(ActiveTurn {
            turn_id: "t2".into(),
            started_at: None,
        });
        assert!(snapshot_changed(&first, &turn));
    }

    #[test]
    fn a_turn_without_metrics_shows_no_older_metrics() {
        // The snapshot keeps the client's answer; `agent_last_turn_metrics` stops at the last
        // user message, so a later turn without metrics reads `None`.
        let mut ended = full(None, Vec::new());
        ended.last_metrics = None;
        assert!(!ended.has_turn_metrics());
        ended.active_turn = Some(ActiveTurn {
            turn_id: "t".into(),
            started_at: None,
        });
        assert!(ended.has_turn_metrics());
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
    fn a_failed_send_shows_its_error_and_keeps_the_text() {
        let failed: anyhow::Result<()> = Err(anyhow::anyhow!("agent busy"));
        assert_eq!(send_error_text(&failed).as_deref(), Some("agent busy"));
        assert_eq!(send_error_text(&Ok(())), None);
        assert_eq!(
            restore_after_error("", "fix the build").as_deref(),
            Some("fix the build")
        );
        assert_eq!(restore_after_error("new text", "fix the build"), None);
    }

    #[test]
    fn a_draft_submits_empty_text_with_the_return_icon_and_spins_while_busy() {
        let options = DraftOptions::default();
        assert_eq!(
            draft_primary(&options, false, false),
            DraftPrimary::Submit(SubmitIcon::Return)
        );
        assert_eq!(draft_primary(&options, true, true), DraftPrimary::Spinner);
        let strict = DraftOptions {
            allow_empty_submit: false,
            ..DraftOptions::default()
        };
        assert_eq!(draft_primary(&strict, false, false), DraftPrimary::None);
        assert_eq!(LOCKED_OPACITY, 0.6);
    }

    #[test]
    fn the_drop_overlay_fades_in_over_the_composer_and_out_on_leave_or_drop() {
        let zone = Bounds::new(point(px(0.), px(500.)), size(px(800.), px(120.)));
        let inside = point(px(400.), px(560.));
        let outside = point(px(400.), px(200.));
        assert!(drag_over(zone, inside, true));
        assert!(!drag_over(zone, outside, true));
        // A drop while the composer submits is ignored, so no overlay.
        assert!(!drag_over(zone, inside, false));

        let start = Instant::now();
        let half = start + DROP_FADE / 2;
        let done = start + DROP_FADE;
        let mut overlay = DropOverlay::default();
        assert_eq!(overlay.opacity(start), 0.);
        // Inside: it fades in over 150 ms. Further moves inside change nothing, so no render.
        assert!(overlay.set_over(drag_over(zone, inside, true), start));
        assert!(!overlay.set_over(drag_over(zone, inside, true), half));
        assert_eq!(overlay.opacity(start), 0.);
        assert!((overlay.opacity(half) - 0.5).abs() < 0.01);
        assert!(overlay.fading(half));
        assert_eq!(overlay.opacity(done), 1.);
        assert!(!overlay.fading(done));

        // Outside: it fades out from where it is.
        let left = done + Duration::from_millis(10);
        assert!(overlay.set_over(drag_over(zone, outside, true), left));
        assert_eq!(overlay.opacity(left), 1.);
        assert_eq!(overlay.opacity(left + DROP_FADE), 0.);

        // Back inside, then a drop halfway through the fade in: it fades out from 0.5.
        let back = left + DROP_FADE * 2;
        assert!(overlay.set_over(true, back));
        let dropped = back + DROP_FADE / 2;
        assert!(overlay.set_over(false, dropped));
        assert!((overlay.opacity(dropped) - 0.5).abs() < 0.01);
        assert_eq!(overlay.opacity(dropped + DROP_FADE), 0.);
        assert!(!overlay.fading(dropped + DROP_FADE));
    }
}
