//! `bachuc-timeline`: the timeline of one agent as a virtualized GPUI list. Each turn draws as
//! an always-open step list with its answer below (ADR-0018, ADR-0019, ADR-0021); mermaid and
//! display math blocks show as images from `bachuc-diagram` (ADR-0037). Rows follow the
//! Electron agent panel: `message.tsx`, `tool-calls/detail-level/turn/view.tsx`, the tool detail
//! views, the permission cards, and the pane find bar.

mod card_view;
pub mod cards;
pub mod diff;
pub mod find;
mod find_bar;
pub mod grouping;
mod image_view;
pub mod links;
pub mod markdown;
mod plugin_view;
pub mod plugins;
mod tool_view;
pub mod tools;

pub use image_view::{display_size, image_dimensions, read_target};
pub use tool_view::{change_bar, display_path, edit_status_label};

use std::cell::{Cell, RefCell};
use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use bachuc_client::{Client, ClientEvent};
use bachuc_diagram::{DiagramReady, DiagramRenderer, image_size};
use bachuc_explorer::syntax::Run;
use bachuc_icons::{IconName, icon};
use bachuc_plugin_host::{PluginHost, PluginHostEvent};
use bachuc_theme::metrics::{
    FONT_2XL, FONT_BASE, FONT_CODE, FONT_CONTENT, FONT_LG, FONT_XL, ICON_MD, ICON_SM, ICON_XS,
    MAX_CONTENT_WIDTH, RADIUS_2XL, RADIUS_BASE, RADIUS_LG, RADIUS_MD, RADIUS_SM, SPACE_1, SPACE_2,
    SPACE_3, SPACE_4, SPACE_6, SPACE_8,
};
use bachuc_theme::{
    TurnLabel, code_font_size, content_font_size, mono_font_family, scheme, syntax_colors, tokens,
    turn_label_colors, ui_font_size,
};
use gpui_kit::base::text::{CodeBlock, TextView as RichText, TextViewStyle as RichTextStyle};
use gpui_kit::component::text::{TextView, TextViewState};
use gpui_kit::component::{ActiveTheme as _, Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use serde_json::Value;

use card_view::FormState;
use diff::DiffLine;
use find_bar::FindState;
use grouping::{
    SharedRow, StepCounts, TurnGroup, TurnGrouper, TurnStep, entry_key, item_type, message_key,
};
use image_view::ImageLoad;
use markdown::{Block, Segment};
use plugin_view::{PendingReveal, Selection};
use plugins::{Part, ProjectInput, Projector, StepPart};

/// What the timeline asks of the app.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TimelineEvent {
    /// A file link or a tool row's open-file button: the app opens the file pane. `path` is
    /// absolute, or relative to `cwd`.
    OpenFile {
        cwd: String,
        path: String,
        line: Option<u32>,
    },
}

/// Rows within this many of the top load the page before the first entry.
const LOAD_OLDER_WITHIN: usize = 2;
/// The smallest share of the strip one step gets, so short steps stay visible.
const MIN_SEGMENT_SHARE: f32 = 0.04;
/// Opacity of the strip segments of the steps that are not selected.
const DIMMED_OPACITY: f32 = 0.3;
const STRIP_WIDTH: f32 = 170.;
const STRIP_HEIGHT: f32 = 8.;
/// Live box lines of the running step's last command.
const OUTPUT_LINES: usize = 2;
/// A clock tick this soon after a sync redraw adds no change, so it draws nothing.
const CLOCK_SKIP_AFTER_SYNC: Duration = Duration::from_millis(250);
/// The turn step list sizes of `turn/view.tsx` that have no metric.
const TURN_TITLE_SIZE: f32 = 13.5;
const TURN_CLOCK_SIZE: f32 = 12.5;
const TURN_SUMMARY_SIZE: f32 = 13.;
const STEP_LABEL_SIZE: f32 = 10.;
const STEP_COUNTS_SIZE: f32 = 12.5;
const STEP_TIME_SIZE: f32 = 11.5;
const STEP_NUMBER: f32 = 20.;
const STEP_BODY_LEFT: f32 = 34.;
const STEP_BODY_RIGHT: f32 = 52.;
const STEP_TIME_WIDTH: f32 = 40.;
const STEP_GAP: f32 = 6.;
const STEP_LIST_TOP: f32 = 16.;
const LIVE_LINE_HEIGHT: f32 = 19.;
const RULE_TOP: f32 = 22.;
const RULE_BOTTOM: f32 = 18.;
/// The user bubble's line height, as `fontSize.content * 1.4`.
const USER_LINE_HEIGHT: f32 = 1.4;
/// The web stream's empty slot above the first row, where older history loads
/// (`HISTORY_START_SLOT_HEIGHT_PX`), under the 16 px content padding.
const HISTORY_START_SLOT: f32 = 32.;
/// The user message's trailing row under the bubble: 8 above, 24 high. It holds the time and
/// copy button, which show on hover; the space stays when they are hidden.
const USER_TRAILING_ROW: f32 = SPACE_2 + 24.;
/// The space between two markdown blocks: the paragraph's `marginBottom: 12` and the block
/// container's 12.
const BLOCK_GAP: f32 = SPACE_6;
/// GPUI's default line height over the font size (`phi`). It gives a line box of
/// `round(15 * 1.618)` = 24 at the content size, as a whole number, where the web's line boxes
/// are fractional.
const BODY_LINE_HEIGHT: f32 = 1.618;
/// The 1 px that GPUI's whole-pixel line box adds to each paragraph over the web's (a pitch of
/// 48 against 47 at the content size, which drifted reply text 1 px per block). It comes out of
/// the paragraph gap, so every block boundary moves up as much as the line box moved it down.
const LINE_BOX_ROUNDING: f32 = 1.;
/// The gap under a paragraph in the text view: `BLOCK_GAP` less the line box rounding.
const PARAGRAPH_GAP: f32 = BLOCK_GAP - LINE_BOX_ROUNDING;
/// A fence's line height over its size, as `splitFenceStyle`.
const FENCE_LINE_HEIGHT: f32 = 1.45;
/// `STREAM_METADATA_FONT_SIZE`, the compaction marker and turn footer text.
const METADATA_SIZE: f32 = 13.;
/// The `FONT_SIZE` tiers `3xl` and `4xl` of the h2 and h1 markdown headings; no metric has them.
const FONT_3XL: f32 = 22.;
const FONT_4XL: f32 = 26.;
/// `contentHeadingLineHeight`: a heading's line height over its size.
const HEADING_LINE_HEIGHT: f32 = 1.3;
/// The completed turn footer of `turn-footer.tsx`: `spacing[2] + 5` above, 24 high,
/// `TURN_FOOTER_BOTTOM_SPACING` below.
const FOOTER_TOP: f32 = SPACE_2 + 5.;
const FOOTER_HEIGHT: f32 = 24.;
/// How long the turn copy button shows its check, as `TurnCopyButton`.
const COPIED_FOR: Duration = Duration::from_millis(1500);

#[derive(Clone, Debug, PartialEq, Eq)]
enum RowKey {
    Entry(u64),
    Turn(u64),
    Step(u64),
    /// The footer of the response that ends at this entry.
    Footer(u64),
    Permission(String),
}

#[derive(Clone)]
enum RowKind {
    Entry(usize),
    TurnHeader(Rc<TurnGroup>),
    /// A turn and the index of one of its steps.
    Step(Rc<TurnGroup>, usize),
    /// A finished response: its user message, if loaded, and its last entry.
    Footer {
        start: Option<usize>,
        end: usize,
    },
    Permission(Value),
}

/// The row under a message row, for the space between them.
enum Below {
    Nothing,
    Footer,
    User,
    Assistant,
    /// Tool calls, thoughts, tasks, or a turn.
    Tools,
    Other,
}

/// The response the row loop of a sync is in: everything after one user message.
#[derive(Default)]
struct Response {
    start: Option<usize>,
    end: Option<usize>,
    has_answer: bool,
}

#[derive(Clone)]
struct ViewRow {
    key: RowKey,
    /// Changes whenever what the row draws changes.
    fingerprint: u64,
    /// The first entry the row shows, to find it again after an older page arrives.
    anchor: Option<u64>,
    /// The last entry the row shows; a turn header shows none.
    last: Option<u64>,
    has_diagram: bool,
    kind: RowKind,
}

/// The markdown of one assistant entry, split into segments with one text state per markdown
/// segment.
struct Message {
    seq_end: u64,
    /// The hash of the shown text, which a plugin's kept source can replace.
    text_hash: u64,
    segments: Vec<Segment>,
    states: Vec<Option<Entity<TextViewState>>>,
    /// Watch the text states for a selection, for the selection actions.
    observers: Vec<Subscription>,
}

/// Syntax runs per line, as `bachuc_explorer::syntax::split_lines` returns them.
type LineRuns = Rc<Vec<Vec<Run>>>;

/// The colored ranges of a markdown fence.
type CodeHighlighter = Arc<dyn Fn(&CodeBlock) -> Vec<Run> + Send + Sync>;

/// The diff lines of one edit or write call and their added and removed counts.
#[derive(Default)]
struct Diff {
    lines: Vec<DiffLine>,
    stats: (usize, usize),
    /// Syntax runs and the colors they were made with, made on the first draw.
    runs: RefCell<Option<(bachuc_theme::SyntaxColors, LineRuns)>>,
}

/// The timeline of one agent. Create it with `cx.new(|cx| TimelineView::new(...))`.
pub struct TimelineView {
    client: Entity<Client>,
    agent_id: String,
    /// The agent folder: links resolve under it and images read from it.
    cwd: String,
    diagrams: Entity<DiagramRenderer>,
    list: ListState,
    focus: FocusHandle,
    /// The entries the rows point into: a copy of the client's, so a sync between two
    /// `TimelineChanged` events cannot move them under the rows.
    entries: Vec<Value>,
    grouper: TurnGrouper,
    /// Per entry: the hash of what it draws, and whether it holds a diagram. A sync hashes only
    /// the entries the grouper read again.
    entry_hashes: Vec<(u64, bool)>,
    rows: Vec<ViewRow>,
    /// Set when an entry hash changed outside a sync, so no turn keeps its old rows.
    rows_stale: bool,
    version: Option<u64>,
    has_older: bool,
    loaded: bool,
    turn_active: bool,
    messages: HashMap<u64, Message>,
    /// Diffs of edit and write calls by entry key, built when the entry changes rather than on
    /// every frame.
    diffs: HashMap<u64, Rc<Diff>>,
    /// Edits whose diff the user hid; diffs are open by default (ADR-0019).
    hidden_diffs: HashSet<u64>,
    /// Tool badges the user opened.
    expanded: HashSet<u64>,
    /// The step the user picked in the strip or the list, by its entry key.
    selected_step: Option<u64>,
    /// Plan cards folded or opened against their default, by permission id or `tool-<key>`.
    plan_toggled: HashSet<String>,
    /// Question forms being filled in, by permission id.
    forms: HashMap<String, FormState>,
    /// Permissions answered from this view, waiting for the daemon to resolve them, with the
    /// action that answered.
    responding: HashMap<String, String>,
    /// Message images by source.
    images: HashMap<String, ImageLoad>,
    lightbox: Option<Arc<Image>>,
    finder: Option<FindState>,
    /// Syntax runs of open code sections, by text.
    syntax_cache: RefCell<HashMap<u64, LineRuns>>,
    /// The markdown style of the message text and its fence highlighter, made on the first draw
    /// after a theme change.
    markdown_style: RefCell<Option<(RichTextStyle, CodeHighlighter)>>,
    /// The footer whose copy button shows its check, by the response's last entry, and the
    /// timer that clears it.
    copied: Option<(u64, Task<()>)>,
    clock: Option<Task<()>>,
    /// When the last sync redrew the view.
    synced_at: Option<Instant>,
    /// The plugin host, when the app made one: transformers, renderers, and selection actions.
    host: Option<Entity<PluginHost>>,
    projector: Projector,
    /// The host's timeline revision and the hash of the agent's labels at the last sync.
    plugin_revision: u64,
    labels_hash: Option<u64>,
    workspace_id: Option<String>,
    /// Plugin item views by item id, with the data hash they were built from.
    plugin_views: HashMap<String, (u64, AnyView)>,
    selection: Option<Selection>,
    /// The timeline's bounds and the selection toolbar's size at the last paint.
    bounds: Rc<Cell<Bounds<Pixels>>>,
    toolbar_size: Rc<Cell<Size<Pixels>>>,
    pending_reveal: Option<PendingReveal>,
    reveal_highlight: Option<Task<()>>,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<TimelineEvent> for TimelineView {}

impl Focusable for TimelineView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl TimelineView {
    pub fn new(
        client: Entity<Client>,
        agent_id: String,
        diagrams: Entity<DiagramRenderer>,
        window: &mut Window,
        cx: &mut Context<TimelineView>,
    ) -> TimelineView {
        let _ = window;
        // Top alignment with tail follow, as the app's stream: a short timeline starts at the top,
        // and once it fills the pane the list keeps the newest row in view.
        let list = ListState::new(0, ListAlignment::Top, px(800.));
        list.set_follow_mode(FollowMode::Tail);
        let view = cx.entity().downgrade();
        list.set_scroll_handler(move |event, _, cx| {
            if event.visible_range.start < LOAD_OLDER_WITHIN {
                let _ = view.update(cx, |this, cx| this.load_older(cx));
            }
        });

        let host = bachuc_plugin_host::host(cx);
        let mut subscriptions = vec![
            cx.subscribe(&client, |this, _, event: &ClientEvent, cx| match event {
                ClientEvent::TimelineChanged { agent_id }
                | ClientEvent::PermissionRequested { agent_id }
                | ClientEvent::AgentFinished { agent_id } => {
                    if *agent_id == this.agent_id {
                        this.sync(cx);
                    }
                }
                ClientEvent::AgentsChanged => this.sync(cx),
                ClientEvent::Connected => {
                    let agent_id = this.agent_id.clone();
                    this.client
                        .update(cx, |client, cx| client.view_agent(&agent_id, cx));
                    cx.notify();
                }
                ClientEvent::Disconnected { .. } => cx.notify(),
            }),
            cx.subscribe(&diagrams, |this, _, ready: &DiagramReady, cx| {
                for (index, row) in this.rows.iter().enumerate() {
                    if row.has_diagram && this.shows_diagram(row, ready) {
                        this.list.remeasure_items(index..index + 1);
                    }
                }
                cx.notify();
            }),
            cx.observe_global::<gpui_kit::component::theme::Theme>(|this, cx| {
                // Syntax colors follow the scheme, and font sizes change row heights.
                this.syntax_cache.borrow_mut().clear();
                this.markdown_style.borrow_mut().take();
                this.list.remeasure();
                cx.notify();
            }),
            cx.on_release(|this, cx| {
                let agent_id = this.agent_id.clone();
                this.client
                    .update(cx, |client, cx| client.unview_agent(&agent_id, cx));
            }),
        ];
        if let Some(host) = &host {
            subscriptions.push(cx.subscribe(
                host,
                |this, _, event: &PluginHostEvent, cx| match event {
                    // Both grow the timeline revision, which the sync compares.
                    PluginHostEvent::TimelineTransformsChanged
                    | PluginHostEvent::ContributionsChanged => this.sync(cx),
                    PluginHostEvent::PluginStateChanged { .. } => cx.notify(),
                    _ => {}
                },
            ));
        }

        let id = agent_id.clone();
        client.update(cx, |client, cx| client.view_agent(&id, cx));

        let mut this = TimelineView {
            client,
            agent_id,
            cwd: String::new(),
            diagrams,
            list,
            focus: cx.focus_handle(),
            entries: Vec::new(),
            grouper: TurnGrouper::default(),
            entry_hashes: Vec::new(),
            rows: Vec::new(),
            rows_stale: false,
            version: None,
            has_older: false,
            loaded: false,
            turn_active: false,
            messages: HashMap::new(),
            diffs: HashMap::new(),
            hidden_diffs: HashSet::new(),
            expanded: HashSet::new(),
            selected_step: None,
            plan_toggled: HashSet::new(),
            forms: HashMap::new(),
            responding: HashMap::new(),
            images: HashMap::new(),
            lightbox: None,
            finder: None,
            syntax_cache: RefCell::new(HashMap::new()),
            markdown_style: RefCell::new(None),
            copied: None,
            clock: None,
            synced_at: None,
            host,
            projector: Projector::default(),
            plugin_revision: 0,
            labels_hash: None,
            workspace_id: None,
            plugin_views: HashMap::new(),
            selection: None,
            bounds: Rc::new(Cell::new(Bounds::default())),
            toolbar_size: Rc::new(Cell::new(Size::default())),
            pending_reveal: None,
            reveal_highlight: None,
            _subscriptions: subscriptions,
        };
        this.sync(cx);
        this
    }

    pub fn agent_id(&self) -> &str {
        &self.agent_id
    }

    fn load_older(&mut self, cx: &mut Context<Self>) {
        if !self.has_older {
            return;
        }
        let agent_id = self.agent_id.clone();
        self.client
            .update(cx, |client, cx| client.load_older(&agent_id, cx));
    }

    /// Reads the client's timeline and updates the rows that changed.
    fn sync(&mut self, cx: &mut Context<Self>) {
        let client = self.client.read(cx);
        let agent = client
            .agents()
            .iter()
            .find(|agent| agent.id == self.agent_id);
        let turn_active = agent.is_some_and(|agent| agent.status == "running");
        if let Some(agent) = agent {
            if agent.cwd != self.cwd {
                self.cwd = agent.cwd.clone();
            }
            if agent.workspace_id != self.workspace_id {
                self.workspace_id = agent.workspace_id.clone();
            }
        }
        let labels: BTreeMap<String, String> =
            client.agent_labels(&self.agent_id).unwrap_or_default();
        let labels_hash = plugins::labels_hash(&labels);
        let revision = self
            .host
            .as_ref()
            .map_or(0, |host| host.read(cx).timeline_revision());
        let Some(state) = client.timeline(&self.agent_id) else {
            return;
        };
        if self.version == Some(state.version)
            && self.turn_active == turn_active
            && self.plugin_revision == revision
            && self.labels_hash == Some(labels_hash)
        {
            return;
        }
        self.version = Some(state.version);
        self.turn_active = turn_active;
        self.plugin_revision = revision;
        self.labels_hash = Some(labels_hash);
        self.loaded = true;
        self.has_older = state.has_older;
        let permissions = state.pending_permissions.clone();
        copy_changed(&mut self.entries, &state.entries);
        let pending = |id: &String| permissions.iter().any(|request| request["id"] == **id);
        self.responding.retain(|id, _| pending(id));
        self.forms.retain(|id, _| pending(id));
        self.grouper.sync(&self.entries, turn_active);
        let projected = self.projector.sync(
            &ProjectInput {
                index: self.host.as_ref().map(|host| host.read(cx).index()),
                revision,
                agent_id: &self.agent_id,
                labels: &labels,
                turn_active,
            },
            &self.entries,
            self.grouper.changed(),
            self.grouper.shifted(),
        );
        // The entries to read again: the changed ones and the ones whose projection changed,
        // or every entry when the transformers or labels changed.
        let touched: Vec<usize> = if projected.full {
            (0..self.entries.len()).collect()
        } else {
            let mut touched = self.grouper.changed().to_vec();
            touched.extend(&projected.changed);
            touched.sort_unstable();
            touched.dedup();
            touched
        };
        // A turn keeps its rows while its group is the same, so a projection that changed
        // outside the grouper's changes rebuilds them.
        if touched.len() > self.grouper.changed().len() {
            self.rows_stale = true;
        }
        if projected.full {
            self.plugin_views.clear();
        } else if self.grouper.shifted() {
            self.prune_plugin_views();
        }
        let images = self.update_messages(&touched, cx);
        self.update_hashes(&touched);
        for source in images {
            self.load_image(&source, cx);
        }

        // A turn whose group did not change keeps its rows; its `Rc` stays alive in the old
        // rows, so no new group can take its address.
        let old_turns: HashMap<*const TurnGroup, usize> = if self.rows_stale {
            HashMap::new()
        } else {
            self.rows
                .iter()
                .enumerate()
                .filter_map(|(index, row)| match &row.kind {
                    RowKind::TurnHeader(group) => Some((Rc::as_ptr(group), index)),
                    _ => None,
                })
                .collect()
        };
        self.rows_stale = false;
        let mut rows: Vec<ViewRow> = Vec::new();
        // A footer closes each response with an answer, as `layout.ts` places them; the facts
        // the loop reads are cached by the grouper, so a flush reads no entry for it.
        let mut response = Response::default();
        for row in self.grouper.rows() {
            match row {
                SharedRow::Item(index) => {
                    if self.grouper.is_user(*index) {
                        self.push_footer(&response, &mut rows);
                        response = Response {
                            start: Some(*index),
                            ..Response::default()
                        };
                    } else {
                        response.end = Some(*index);
                        response.has_answer |= self.grouper.is_assistant(*index);
                    }
                    rows.push(self.entry_row(*index));
                }
                SharedRow::Turn(group) => {
                    response.end = group.run.items.last().copied();
                    response.has_answer =
                        response.has_answer || group.steps.iter().any(|step| !step.note.is_empty());
                    match old_turns.get(&Rc::as_ptr(group)) {
                        Some(&header) => {
                            rows.extend_from_slice(&self.rows[header..=header + group.steps.len()])
                        }
                        None => self.push_turn_rows(group, &mut rows),
                    }
                }
            }
        }
        // The response still running gets no completed footer.
        if !turn_active {
            self.push_footer(&response, &mut rows);
        }
        rows.extend(permissions.into_iter().map(|request| {
            let id = request["id"].as_str().unwrap_or("").to_string();
            let mut hasher = DefaultHasher::new();
            self.hash_permission(&id, &mut hasher);
            ViewRow {
                key: RowKey::Permission(id),
                fingerprint: hasher.finish(),
                anchor: None,
                last: None,
                has_diagram: false,
                kind: RowKind::Permission(request),
            }
        }));
        // A message row's height follows the row below it, so a new row under it redraws it.
        for index in 0..rows.len() {
            if let Some(space) = self.space_below(&rows[index], rows.get(index + 1)) {
                let mut hasher = DefaultHasher::new();
                rows[index].fingerprint.hash(&mut hasher);
                space.to_bits().hash(&mut hasher);
                rows[index].fingerprint = hasher.finish();
            }
        }
        self.apply_rows(rows);
        self.update_clock(cx);
        self.synced_at = Some(Instant::now());
        self.run_pending_reveal(cx);
        cx.notify();
    }

    /// Drops the views of plugin items whose entries went.
    fn prune_plugin_views(&mut self) {
        let mut live: HashSet<String> = self
            .projector
            .item_ids()
            .into_iter()
            .map(str::to_string)
            .collect();
        live.extend(
            self.entries
                .iter()
                .filter_map(|entry| plugins::daemon_item(&entry["item"]))
                .map(|(_, id, _)| id),
        );
        self.plugin_views.retain(|id, _| live.contains(id));
    }

    /// Syncs again after a change made in this view, such as a toggle or an answer.
    fn refresh(&mut self, cx: &mut Context<Self>) {
        self.version = None;
        self.sync(cx);
    }

    /// Splices the rows that changed into the list and keeps the row at the top in place.
    fn apply_rows(&mut self, rows: Vec<ViewRow>) {
        let old = std::mem::replace(&mut self.rows, rows);
        let new = &self.rows;
        let top = self.list.logical_scroll_top();
        let top_anchor = old.get(top.item_ix).and_then(|row| row.anchor);
        let prefix = old
            .iter()
            .zip(new.iter())
            .take_while(|(a, b)| a.key == b.key)
            .count();
        let max_suffix = old.len().min(new.len()) - prefix;
        let suffix = old
            .iter()
            .rev()
            .zip(new.iter().rev())
            .take(max_suffix)
            .take_while(|(a, b)| a.key == b.key)
            .count();
        let removed = prefix..old.len() - suffix;
        let inserted = new.len() - suffix - prefix;
        let top_in_splice = removed.contains(&top.item_ix);
        if !removed.is_empty() || inserted > 0 {
            self.list.splice(removed.clone(), inserted);
        }
        // Rows kept on both sides of the splice draw again when their content changed.
        let kept = (0..prefix)
            .map(|i| (i, i))
            .chain((0..suffix).map(|i| (old.len() - suffix + i, new.len() - suffix + i)));
        for (old_ix, new_ix) in kept {
            if old[old_ix].fingerprint != new[new_ix].fingerprint {
                self.list.remeasure_items(new_ix..new_ix + 1);
            }
        }
        // An older page can regroup the top row; scroll to the row that now holds its entry.
        // Sequence numbers grow down the timeline, so the first row that reaches the anchor
        // holds it, or holds the next shown entry when the anchor is a hidden thought.
        if top_in_splice
            && !self.list.is_following_tail()
            && let Some(anchor) = top_anchor
            && let Some(index) = new
                .iter()
                .position(|row| row.anchor == Some(anchor))
                .or_else(|| new.iter().position(|row| row.last >= Some(anchor)))
        {
            self.list.scroll_to(ListOffset {
                item_ix: index,
                offset_in_item: top.offset_in_item,
            });
        }
    }

    /// Hashes what an entry draws and tells whether it holds a diagram.
    fn hash_entry(&self, index: usize) -> (u64, bool) {
        let entry = &self.entries[index];
        let key = entry_key(entry);
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        entry["seqEnd"].as_u64().hash(&mut hasher);
        self.hidden_diffs.contains(&key).hash(&mut hasher);
        self.expanded.contains(&key).hash(&mut hasher);
        self.plan_toggled
            .contains(&format!("tool-{key}"))
            .hash(&mut hasher);
        // A revision can change the renderers, so plugin rows draw again with it.
        if let Some(projection) = self.projector.get(index) {
            projection.hash.hash(&mut hasher);
            self.plugin_revision.hash(&mut hasher);
        } else if item_type(entry) == "plugin" {
            self.plugin_revision.hash(&mut hasher);
        }
        let mut has_diagram = false;
        if let Some(message) = self.messages.get(&key) {
            for segment in &message.segments {
                match segment {
                    Segment::Diagram { .. } => has_diagram = true,
                    Segment::Image { source, .. } => {
                        self.images.get(source).map(image_phase).hash(&mut hasher);
                    }
                    Segment::Markdown(_) => {}
                }
            }
        }
        (hasher.finish(), has_diagram)
    }

    /// Hashes the entries of `keys` again and rebuilds every turn's rows on the next sync.
    fn rehash_keys(&mut self, keys: &[u64]) {
        for index in 0..self.entries.len() {
            if keys.contains(&self.grouper.key(index)) {
                self.entry_hashes[index] = self.hash_entry(index);
                self.rows_stale = true;
            }
        }
    }

    /// Hashes the `touched` entries again, and builds the diffs of the edit and write calls
    /// among them. `touched` is in order and holds every entry past the old end.
    fn update_hashes(&mut self, touched: &[usize]) {
        if self.grouper.shifted() {
            let live: HashSet<u64> = (0..self.entries.len())
                .map(|index| self.grouper.key(index))
                .collect();
            self.diffs.retain(|key, _| live.contains(key));
        }
        self.entry_hashes.truncate(self.entries.len());
        for &index in touched {
            let entry = &self.entries[index];
            // A diff changes only with its entry.
            if grouping::edited_file_path(entry).is_some()
                && self.grouper.changed().binary_search(&index).is_ok()
            {
                let lines = diff::entry_lines(entry);
                let stats = diff::stats(&lines);
                self.diffs.insert(
                    entry_key(entry),
                    Rc::new(Diff {
                        lines,
                        stats,
                        runs: RefCell::new(None),
                    }),
                );
            }
            let hash = self.hash_entry(index);
            match self.entry_hashes.get_mut(index) {
                Some(slot) => *slot = hash,
                None => self.entry_hashes.push(hash),
            }
        }
    }

    fn entry_row(&self, index: usize) -> ViewRow {
        let key = self.grouper.key(index);
        let (fingerprint, has_diagram) = self.entry_hashes[index];
        ViewRow {
            key: RowKey::Entry(key),
            fingerprint,
            anchor: Some(key),
            last: Some(key),
            has_diagram,
            kind: RowKind::Entry(index),
        }
    }

    /// A turn takes one list item for its header and one per step, so a long turn lays out
    /// only the steps on screen.
    fn push_turn_rows(&self, group: &Rc<TurnGroup>, rows: &mut Vec<ViewRow>) {
        let selected = group
            .steps
            .iter()
            .position(|step| Some(self.grouper.key(step.id)) == self.selected_step);
        // The header is one line of title, clock, strip, and summary: a step body leaves its
        // height alone.
        let mut hasher = DefaultHasher::new();
        group.is_loading.hash(&mut hasher);
        group.step_count.hash(&mut hasher);
        group.running_step.hash(&mut hasher);
        group.changed_files.hash(&mut hasher);
        selected.hash(&mut hasher);
        let key = self.grouper.key(group.run.items[0]);
        rows.push(ViewRow {
            key: RowKey::Turn(key),
            fingerprint: hasher.finish(),
            anchor: Some(key),
            last: None,
            has_diagram: false,
            kind: RowKind::TurnHeader(group.clone()),
        });
        for (position, step) in group.steps.iter().enumerate() {
            let mut hasher = DefaultHasher::new();
            let mut has_diagram = false;
            for &index in step.note.iter().chain(&step.items) {
                let (hash, diagram) = self.entry_hashes[index];
                hash.hash(&mut hasher);
                has_diagram |= diagram;
            }
            (step.label as u8).hash(&mut hasher);
            step.is_running.hash(&mut hasher);
            (selected == Some(position)).hash(&mut hasher);
            // The last step draws the rule under a finished turn.
            (position + 1 == group.steps.len() && !group.is_loading).hash(&mut hasher);
            let key = self.grouper.key(step.id);
            let last = step
                .note
                .iter()
                .chain(&step.items)
                .map(|&index| self.grouper.key(index))
                .max();
            rows.push(ViewRow {
                key: RowKey::Step(key),
                fingerprint: hasher.finish(),
                anchor: Some(key),
                last,
                has_diagram,
                kind: RowKind::Step(group.clone(), position),
            });
        }
    }

    /// Closes a response with its footer when it holds an assistant message.
    fn push_footer(&self, response: &Response, rows: &mut Vec<ViewRow>) {
        let Some(end) = response.end.filter(|_| response.has_answer) else {
            return;
        };
        let key = self.grouper.key(end);
        let mut hasher = DefaultHasher::new();
        response
            .start
            .map(|start| self.grouper.at(start))
            .hash(&mut hasher);
        self.grouper.at(end).hash(&mut hasher);
        self.copied
            .as_ref()
            .is_some_and(|(copied, _)| *copied == key)
            .hash(&mut hasher);
        rows.push(ViewRow {
            key: RowKey::Footer(key),
            fingerprint: hasher.finish(),
            anchor: None,
            last: None,
            has_diagram: false,
            kind: RowKind::Footer {
                start: response.start,
                end,
            },
        });
    }

    /// Copies every assistant message of a response, as `collectAssistantResponseContent`:
    /// messages joined by a blank line, the parts of one message joined as they are.
    fn copy_turn(&mut self, start: Option<usize>, end: usize, cx: &mut Context<Self>) {
        let first = start.map_or(0, |start| start + 1);
        let mut text = String::new();
        let mut message: Option<String> = None;
        for entry in self.entries.get(first..=end).unwrap_or(&[]) {
            if item_type(entry) != "assistant_message" {
                continue;
            }
            let key = grouping::message_key(entry);
            if message.is_some() && message.as_ref() != Some(&key) {
                text.push_str("\n\n");
            }
            text.push_str(entry["item"]["text"].as_str().unwrap_or(""));
            message = Some(key);
        }
        if text.is_empty() {
            return;
        }
        cx.write_to_clipboard(ClipboardItem::new_string(text));
        let key = self.grouper.key(end);
        let timer = cx.spawn(async move |this, cx| {
            cx.background_executor().timer(COPIED_FOR).await;
            let _ = this.update(cx, |this, cx| {
                this.copied = None;
                this.refresh(cx);
            });
        });
        self.copied = Some((key, timer));
        self.refresh(cx);
    }

    /// Whether a row shows the diagram that finished rendering.
    fn shows_diagram(&self, row: &ViewRow, ready: &DiagramReady) -> bool {
        let entries: &[usize] = match &row.kind {
            RowKind::Entry(index) => std::slice::from_ref(index),
            RowKind::Step(group, position) => &group.steps[*position].note,
            _ => &[],
        };
        entries.iter().any(|&index| {
            self.messages
                .get(&self.grouper.key(index))
                .is_some_and(|message| {
                    message.segments.iter().any(|segment| {
                        matches!(segment, Segment::Diagram { kind, source }
                            if *kind == ready.kind && *source == ready.source)
                    })
                })
        })
    }

    /// Splits each new or changed assistant entry into segments and updates its text states.
    /// Returns the image sources of the changed messages, which the caller loads.
    fn update_messages(&mut self, touched: &[usize], cx: &mut Context<Self>) -> Vec<String> {
        if self.grouper.shifted() {
            let live: HashSet<u64> = self
                .entries
                .iter()
                .filter(|entry| item_type(entry) == "assistant_message")
                .map(entry_key)
                .collect();
            self.messages.retain(|key, _| live.contains(key));
        }
        let mut images = Vec::new();
        for &index in touched {
            let entry = &self.entries[index];
            if item_type(entry) != "assistant_message" {
                continue;
            }
            let key = entry_key(entry);
            let seq_end = entry["seqEnd"].as_u64().unwrap_or(0);
            let text = plugins::shown_text(entry, self.projector.get(index).map(|p| &**p));
            let text_hash = {
                let mut hasher = DefaultHasher::new();
                text.hash(&mut hasher);
                hasher.finish()
            };
            if self
                .messages
                .get(&key)
                .is_some_and(|message| message.seq_end == seq_end && message.text_hash == text_hash)
            {
                continue;
            }
            let message_id = message_key(entry);
            let segments = markdown::segments(text);
            let (old_states, mut observers) = self
                .messages
                .remove(&key)
                .map(|message| (message.states, message.observers))
                .unwrap_or_default();
            let mut old_states = old_states.into_iter();
            let states = segments
                .iter()
                .map(|segment| {
                    let previous = old_states.next().flatten();
                    let text = match segment {
                        Segment::Markdown(text) => links::linkify_code_spans(text),
                        Segment::Image { source, .. } => {
                            images.push(source.clone());
                            return None;
                        }
                        Segment::Diagram { .. } => return None,
                    };
                    Some(match previous {
                        Some(state) => {
                            state.update(cx, |state, cx| state.set_text(&text, cx));
                            state
                        }
                        None => {
                            let state = cx.new(|cx| TextViewState::markdown(&text, cx));
                            let message_id = message_id.clone();
                            observers.push(cx.observe(&state, move |this, state, cx| {
                                this.text_state_changed(&message_id, state, cx)
                            }));
                            state
                        }
                    })
                })
                .collect();
            self.messages.insert(
                key,
                Message {
                    seq_end,
                    text_hash,
                    segments,
                    states,
                    observers,
                },
            );
        }
        images
    }

    /// Ticks once a second while a turn runs, for its clock and the running step's time.
    fn update_clock(&mut self, cx: &mut Context<Self>) {
        let running = self
            .rows
            .iter()
            .any(|row| matches!(&row.kind, RowKind::TurnHeader(group) if group.is_loading));
        if !running {
            self.clock = None;
            return;
        }
        if self.clock.is_none() {
            self.clock = Some(cx.spawn(async move |this, cx| {
                loop {
                    cx.background_executor().timer(Duration::from_secs(1)).await;
                    // A stream syncs many times a second, and each sync redraws the clock, so a
                    // tick right after one would draw one more frame with the same text.
                    let tick = |this: &mut TimelineView, cx: &mut Context<TimelineView>| {
                        if this
                            .synced_at
                            .is_none_or(|at| at.elapsed() >= CLOCK_SKIP_AFTER_SYNC)
                        {
                            cx.notify();
                        }
                    };
                    if this.update(cx, tick).is_err() {
                        break;
                    }
                }
            }));
        }
    }

    fn toggle_diff(&mut self, key: u64, cx: &mut Context<Self>) {
        toggle(&mut self.hidden_diffs, key);
        // Only the edit's hash changes; the rows rebuild from the cached hashes.
        self.rehash_keys(&[key]);
        self.refresh(cx);
    }

    fn toggle_expanded(&mut self, key: u64, cx: &mut Context<Self>) {
        toggle(&mut self.expanded, key);
        self.rehash_keys(&[key]);
        self.refresh(cx);
    }

    /// Picks a step, or clears the pick when it is picked already. A pick from the strip
    /// scrolls the step into view.
    fn select_step(&mut self, key: u64, reveal: bool, cx: &mut Context<Self>) {
        self.selected_step = (self.selected_step != Some(key)).then_some(key);
        self.rows_stale = true;
        self.refresh(cx);
        if reveal
            && self.selected_step.is_some()
            && let Some(row) = self
                .rows
                .iter()
                .position(|row| row.key == RowKey::Step(key))
        {
            self.list.scroll_to_reveal_item(row);
        }
    }

    /// A click on a link in a message: a file opens through the app, a web link in the browser.
    fn open_link(&mut self, href: &str, cx: &mut Context<Self>) {
        match links::resolve_click(href, &self.cwd) {
            links::LinkClick::File(target) => cx.emit(TimelineEvent::OpenFile {
                cwd: self.cwd.clone(),
                path: target.path,
                line: target.line_start,
            }),
            links::LinkClick::External(url) => cx.open_url(&url),
            links::LinkClick::Ignored => {}
        }
    }

    fn render_row(
        &mut self,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        if let Some(ViewRow {
            kind: RowKind::Permission(request),
            ..
        }) = self.rows.get(index)
            && cards::is_question(request)
        {
            let request = request.clone();
            self.ensure_form(&request, window, cx);
        }
        self.ensure_plugin_views(index, window, cx);
        let this = &*self;
        let Some(row) = this.rows.get(index) else {
            return div().into_any_element();
        };
        let content = match &row.kind {
            RowKind::Entry(entry) => this.render_entry(*entry, cx),
            RowKind::TurnHeader(group) => this.render_turn_header(index, group, cx),
            RowKind::Step(group, position) => this.render_step(*position, group, cx),
            RowKind::Footer { start, end } => this.render_footer(*start, *end, cx),
            RowKind::Permission(request) => this.render_permission(request, cx),
        };
        let space = this.space_below(row, this.rows.get(index + 1));
        // `streamItemWrapper`: the content column, at most 820 wide, centered.
        div()
            .id(("row", index))
            .w_full()
            .flex()
            .justify_center()
            .px(px(SPACE_4))
            .child(
                div()
                    .w_full()
                    .max_w(px(MAX_CONTENT_WIDTH))
                    .px(px(SPACE_2))
                    .when_some(space, |column, space| column.pb(px(space)))
                    .child(content),
            )
            .into_any_element()
    }

    /// The space under a message row, which depends on the row below it, as the web stream
    /// lays it out. A user message gets `getGapBetweenStreamItems`. An assistant message also
    /// keeps its last block's `marginBottom: 12` and its own `paddingBottom: 12`, which it drops
    /// above a footer (`compactBottom`). None for the other rows.
    fn space_below(&self, row: &ViewRow, next: Option<&ViewRow>) -> Option<f32> {
        let RowKind::Entry(index) = row.kind else {
            return None;
        };
        if self.projector.is_hidden(index) {
            return None;
        }
        let is_user = self.grouper.is_user(index);
        if !is_user && !self.grouper.is_assistant(index) {
            return None;
        }
        let below = match next.map(|next| &next.kind) {
            None => Below::Nothing,
            Some(RowKind::Footer { .. }) => Below::Footer,
            Some(RowKind::Entry(next)) if self.grouper.is_user(*next) => Below::User,
            Some(RowKind::Entry(next)) if self.grouper.is_assistant(*next) => Below::Assistant,
            Some(RowKind::Entry(next))
                if matches!(
                    item_type(&self.entries[*next]),
                    "tool_call" | "reasoning" | "todo"
                ) =>
            {
                Below::Tools
            }
            Some(RowKind::TurnHeader(_)) => Below::Tools,
            Some(_) => Below::Other,
        };
        if is_user {
            return Some(match below {
                Below::Nothing | Below::Assistant => 0.,
                Below::User => SPACE_1,
                _ => SPACE_4,
            });
        }
        Some(match below {
            Below::Footer => SPACE_3,
            Below::Nothing => SPACE_3 * 2.,
            Below::Tools => SPACE_3 * 2. + SPACE_1,
            _ => SPACE_3 * 2. + SPACE_4,
        })
    }

    fn render_entry(&self, index: usize, cx: &mut Context<Self>) -> AnyElement {
        match self.projector.get(index) {
            Some(projection) => self.render_projected(index, projection, cx),
            None => self.render_source_entry(index, cx),
        }
    }

    /// An entry as itself, with the text a plugin's kept source gave it.
    fn render_source_entry(&self, index: usize, cx: &mut Context<Self>) -> AnyElement {
        let entry = &self.entries[index];
        let key = entry_key(entry);
        let item = &entry["item"];
        let t = tokens(cx);
        let base = ui_font_size(FONT_BASE, cx);
        let shown = plugins::shown_text(entry, self.projector.get(index).map(|p| &**p));
        match item_type(entry) {
            "user_message" => {
                let text = shown.to_string();
                let size = ui_font_size(FONT_CONTENT, cx);
                let current = t.status_warning;
                let highlights: Vec<_> = self
                    .user_matches(key)
                    .into_iter()
                    .map(|(range, is_current)| {
                        let color = if is_current {
                            current
                        } else {
                            current.opacity(0.35)
                        };
                        (
                            range,
                            HighlightStyle {
                                background_color: Some(color),
                                ..Default::default()
                            },
                        )
                    })
                    .collect();
                // The web stream gives a user message no outer margin; the row's gap below
                // comes from `space_below`.
                h_flex()
                    .justify_end()
                    .pb(px(USER_TRAILING_ROW))
                    .child(
                        div()
                            .id(("user", key))
                            .role(Role::Article)
                            .aria_label(format!("User message: {}", preview(&text)))
                            .min_w_0()
                            .flex_shrink(1.)
                            .px(px(SPACE_4))
                            .py(px(SPACE_4))
                            .rounded(px(RADIUS_2XL))
                            .rounded_tr(px(RADIUS_SM))
                            .bg(t.surface3)
                            .text_color(t.foreground)
                            .text_size(size)
                            .line_height(size * USER_LINE_HEIGHT)
                            .child(StyledText::new(text).with_highlights(highlights)),
                    )
                    .into_any_element()
            }
            "assistant_message" => {
                let text = shown;
                v_flex()
                    .id(("message", key))
                    .role(Role::Article)
                    .aria_label(format!("Assistant message: {}", preview(text)))
                    .pt(px(SPACE_3))
                    .gap(px(SPACE_2))
                    .children(self.render_markdown(key, cx))
                    .into_any_element()
            }
            "reasoning" => self.render_thought(index, cx),
            "tool_call" => div()
                .mb(px(SPACE_1))
                .child(self.render_tool_call(index, cx))
                .into_any_element(),
            "todo" => {
                let items = item["items"].as_array().cloned().unwrap_or_default();
                let done = items
                    .iter()
                    .filter(|task| task["completed"] == true)
                    .count();
                v_flex()
                    .id(("todo", key))
                    .role(Role::List)
                    .aria_label(format!("Tasks, {done} of {} done", items.len()))
                    .my(px(SPACE_2))
                    .p(px(SPACE_3))
                    .gap(px(SPACE_1))
                    .rounded(px(RADIUS_LG))
                    .border_1()
                    .border_color(t.border)
                    .bg(t.surface1)
                    .text_size(base)
                    .child(
                        div()
                            .text_color(t.foreground_muted)
                            .child(format!("{done}/{} tasks", items.len())),
                    )
                    .children(items.iter().enumerate().map(|(position, task)| {
                        let completed = task["completed"] == true;
                        let text = task["text"].as_str().unwrap_or("").to_string();
                        h_flex()
                            .id(("task", position))
                            .role(Role::ListItem)
                            .aria_label(text.clone())
                            .gap(px(SPACE_2))
                            .items_center()
                            .child(
                                icon(if completed {
                                    IconName::CircleCheck
                                } else {
                                    IconName::Circle
                                })
                                .with_size(px(ICON_XS))
                                .text_color(t.foreground_muted),
                            )
                            .child(
                                div()
                                    .text_color(if completed {
                                        t.foreground_muted
                                    } else {
                                        t.foreground
                                    })
                                    .when(completed, |text| text.line_through())
                                    .child(text),
                            )
                    }))
                    .into_any_element()
            }
            "error" => notification(
                ("error", key),
                Role::Alert,
                IconName::CircleX,
                t.destructive,
                t.tint_danger,
                item["message"].as_str().unwrap_or("").to_string(),
                base,
            ),
            "notification" => {
                let message = item["message"].as_str().unwrap_or("").to_string();
                let (name, color, tint) = match item["level"].as_str() {
                    Some("error") => (IconName::CircleX, t.destructive, t.tint_danger),
                    Some("warning") => (IconName::TriangleAlert, t.status_warning, t.tint_warning),
                    _ => (IconName::Info, t.foreground_muted, t.surface2),
                };
                notification(("note", key), Role::Note, name, color, tint, message, base)
            }
            "compaction" => {
                let line = || div().flex_1().h(px(1.)).bg(t.border);
                h_flex()
                    .id(("compaction", key))
                    .role(Role::Note)
                    .aria_label("Context compacted")
                    .items_center()
                    .gap(px(SPACE_2))
                    .py(px(SPACE_3))
                    .px(px(SPACE_4))
                    .child(line())
                    .child(
                        h_flex()
                            .gap(px(SPACE_2))
                            .items_center()
                            .child(
                                icon(IconName::Scissors)
                                    .with_size(px(ICON_XS))
                                    .text_color(t.foreground_extra_muted),
                            )
                            .child(
                                div()
                                    .text_size(ui_font_size(METADATA_SIZE, cx))
                                    .text_color(t.foreground_muted)
                                    .child("Context compacted"),
                            ),
                    )
                    .child(line())
                    .into_any_element()
            }
            "plugin" => self.render_daemon_item(index, cx),
            other => {
                let text = other.replace('_', " ");
                div()
                    .id(("note", key))
                    .role(Role::Note)
                    .aria_label(text.clone())
                    .text_size(base)
                    .text_color(t.foreground_muted)
                    .child(text)
                    .into_any_element()
            }
        }
    }

    /// The blocks of one assistant entry: markdown, images, diagram images, or diagram
    /// fallbacks.
    fn render_markdown(&self, key: u64, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let Some(message) = self.messages.get(&key) else {
            return Vec::new();
        };
        let diagrams = self.diagrams.clone();
        let blocks = markdown::blocks(&message.segments, |kind, source| {
            diagrams.update(cx, |renderer, cx| renderer.render(kind, source, cx))
        });
        let t = tokens(cx);
        let size = content_font_size(cx);
        let line_height = px(body_line_height(size / px(1.)));
        let view = cx.weak_entity();
        let (style, highlighter) = self.markdown_style(cx);
        let copy_color = t.foreground_muted;
        blocks
            .into_iter()
            .enumerate()
            .map(|(position, block)| match block {
                Block::Markdown(_) => match &message.states[position] {
                    Some(state) => {
                        let view = view.clone();
                        let text = RichText::new(state)
                            .style(style.clone())
                            .shared_code_block_highlighter(highlighter.clone())
                            .selectable(true)
                            .on_link_click(move |href, _, _, cx| {
                                let href = href.to_string();
                                let _ = view.update(cx, |this, cx| this.open_link(&href, cx));
                            })
                            // `HighlightedCodeBlock` shows no language, only a copy button
                            // on hover; an action in the corner takes the language's place.
                            .code_block_actions(move |block, _, _| {
                                let code = block.code();
                                div()
                                    .id("copy-code")
                                    .role(Role::Button)
                                    .aria_label("Copy code")
                                    .p(px(SPACE_1))
                                    .cursor_pointer()
                                    .opacity(0.)
                                    .hover(|button| button.opacity(1.))
                                    .child(
                                        icon(IconName::Copy)
                                            .with_size(px(ICON_SM))
                                            .text_color(copy_color),
                                    )
                                    .on_click(move |_, _, cx| {
                                        let code = code.trim_end_matches('\n').to_string();
                                        cx.write_to_clipboard(ClipboardItem::new_string(code));
                                    })
                            });
                        div()
                            .text_size(size)
                            .line_height(line_height)
                            .child(text)
                            .into_any_element()
                    }
                    None => div().into_any_element(),
                },
                Block::LocalImage { alt, source } => self.render_image(
                    ElementId::Name(format!("image-{key}-{position}").into()),
                    &alt,
                    &source,
                    cx,
                ),
                Block::Image { image, label } => {
                    let size = image_size(&image);
                    div()
                        .id(("diagram", position))
                        .role(Role::Image)
                        .aria_label(label)
                        .max_w_full()
                        .child(img(image).w(size.width).max_w_full())
                        .into_any_element()
                }
                Block::Pending { label } => div()
                    .id(("diagram", position))
                    .role(Role::Image)
                    .aria_label(format!("{label}, rendering"))
                    .text_size(ui_font_size(FONT_BASE, cx))
                    .text_color(t.foreground_muted)
                    .child(format!("Rendering {}...", label.to_lowercase()))
                    .into_any_element(),
                Block::Code { markdown, error } => v_flex()
                    .id(("diagram", position))
                    .role(Role::Group)
                    .aria_label("Diagram source, failed to render")
                    .gap(px(SPACE_1))
                    .child(TextView::markdown(
                        ElementId::Name(format!("fallback-{key}-{position}").into()),
                        markdown,
                    ))
                    .child(
                        div()
                            .text_size(code_font_size(FONT_CODE, cx))
                            .text_color(t.destructive)
                            .child(error.lines().next().unwrap_or("").to_string()),
                    )
                    .into_any_element(),
            })
            .collect()
    }

    /// The theme's rich text style with the headings of `markdown-styles.ts`. Setting a style
    /// replaces the installed default, so the default's colors are made again here, as
    /// gpui-kit's `base_text_view_style` makes them.
    ///
    /// Spacing follows the app's block model: a message splits into blocks at blank lines, a
    /// paragraph keeps `marginBottom: 12` and each block 12 more (`message.tsx`), so blocks sit
    /// 24 apart, less the 1 px that GPUI's whole-pixel line box adds (`PARAGRAPH_GAP`). A fence adds 12 above (`fence_spacing`), padding 12, and a 1.45 line height
    /// (`highlighted-code-block.tsx`). Also returns the fence highlighter, one `Arc` per theme,
    /// so a code block highlights once rather than on every frame.
    fn markdown_style(&self, cx: &App) -> (RichTextStyle, CodeHighlighter) {
        if let Some(style) = self.markdown_style.borrow().as_ref() {
            return style.clone();
        }
        let theme = cx.theme();
        let t = tokens(cx);
        let radius = theme.semantic_tokens().radius.md;
        let mut corners = StyleRefinement::default();
        corners.corner_radii.top_left = Some(radius.into());
        corners.corner_radii.top_right = Some(radius.into());
        corners.corner_radii.bottom_left = Some(radius.into());
        corners.corner_radii.bottom_right = Some(radius.into());
        let code = code_font_size(FONT_CODE, cx);
        let code_block = fence_spacing(corners.clone())
            .p(px(SPACE_3))
            .font_family(mono_font_family(cx))
            .text_size(code)
            .line_height((code * FENCE_LINE_HEIGHT).round());
        let table_head = StyleRefinement::default()
            .bg(theme.table_head)
            .text_color(theme.table_head_foreground);
        let content = content_font_size(cx);
        let border = t.border;
        let style = RichTextStyle::default()
            .with_foreground(theme.foreground)
            .with_muted_foreground(theme.muted_foreground)
            .with_link(theme.link)
            .with_selection(theme.selection)
            .with_code_background(t.surface2)
            .with_border(theme.border)
            .with_paragraph_gap(rems(PARAGRAPH_GAP / FONT_BASE))
            .with_code_block(code_block)
            .with_table(corners)
            .with_table_head(table_head)
            .with_inline_code(HighlightStyle {
                background_color: Some(theme.accent),
                ..Default::default()
            })
            .with_dark(theme.is_dark())
            .with_heading(move |level| heading_style(level, content, border));
        // Read when the style is built, not per flush; a syntax theme change writes the theme
        // global, which drops this cache.
        let colors = syntax_colors(cx);
        let highlighter: CodeHighlighter = Arc::new(move |block: &CodeBlock| {
            block
                .lang()
                .and_then(|info| tool_view::fence_language(&info))
                .map(|language| tool_view::code_styles(language, &block.code(), colors))
                .unwrap_or_default()
        });
        let style = (style, highlighter);
        *self.markdown_style.borrow_mut() = Some(style.clone());
        style
    }

    /// `AssistantTurnFooter`: the copy button and "Worked for N". Fork is out of scope (rewind
    /// and fork), and the end time needs the user message, so a response whose user message
    /// is on an older page shows only the copy button.
    fn render_footer(
        &self,
        start: Option<usize>,
        end: usize,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let t = tokens(cx);
        let key = self.grouper.key(end);
        let copied = self
            .copied
            .as_ref()
            .is_some_and(|(copied, _)| *copied == key);
        let label = start.map(|start| {
            format!(
                "Worked for {}",
                format_elapsed(self.grouper.at(end) - self.grouper.at(start))
            )
        });
        h_flex()
            .id(("footer", key))
            .role(Role::Group)
            .aria_label("Turn actions")
            .mt(px(FOOTER_TOP))
            .pb(px(SPACE_8))
            .h(px(FOOTER_HEIGHT + SPACE_8))
            .gap(px(SPACE_2))
            .items_center()
            .child(
                div()
                    .id(("copy-turn", key))
                    .role(Role::Button)
                    .aria_label(if copied { "Copied" } else { "Copy turn" })
                    .p(px(SPACE_1))
                    .ml(px(-SPACE_1))
                    .cursor_pointer()
                    .child(
                        icon(if copied {
                            IconName::Check
                        } else {
                            IconName::Copy
                        })
                        .with_size(px(ICON_SM))
                        .text_color(t.foreground_muted),
                    )
                    .on_click(cx.listener(move |this, _, _, cx| this.copy_turn(start, end, cx))),
            )
            .children(label.map(|label| {
                div()
                    .text_size(ui_font_size(METADATA_SIZE, cx))
                    .text_color(t.foreground_muted)
                    .child(label)
            }))
            .into_any_element()
    }

    fn render_turn_header(
        &self,
        row: usize,
        group: &TurnGroup,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let t = tokens(cx);
        let mono = mono_font_family(cx);
        let now = chrono::Utc::now().timestamp_millis();
        let started_at = group.run.started_at;
        let last_end = group
            .steps
            .last()
            .and_then(|step| step.ended_at)
            .unwrap_or(started_at);
        let durations: Vec<i64> = group
            .steps
            .iter()
            .map(|step| {
                step.duration_ms
                    .unwrap_or_else(|| (now - step.started_at).max(0))
            })
            .collect();
        let total: i64 = durations.iter().sum();
        let (additions, deletions) = group
            .steps
            .iter()
            .flat_map(|step| step.edits.iter())
            .map(|&edit| {
                self.diffs
                    .get(&self.grouper.key(edit))
                    .map_or((0, 0), |diff| diff.stats)
            })
            .fold((0, 0), |sum, stats| (sum.0 + stats.0, sum.1 + stats.1));

        let title = if group.is_loading {
            "Working".to_string()
        } else {
            format!("Worked for {}", format_duration(last_end - started_at))
        };
        let steps_text = if group.is_loading {
            format!("Step {}", group.step_count)
        } else {
            plural(group.step_count, "step", "steps")
        };
        let files = &group.changed_files;

        let weights: Vec<f32> = durations
            .iter()
            .map(|&ms| (ms as f32).max(total as f32 * MIN_SEGMENT_SHARE).max(1.))
            .collect();
        let weight_sum: f32 = weights.iter().sum();
        let selected = group
            .steps
            .iter()
            .map(|step| self.grouper.key(step.id))
            .find(|key| Some(*key) == self.selected_step);
        let strip = h_flex()
            .id(("strip", row))
            .role(Role::Group)
            .aria_label(format!("{} step strip", group.step_count))
            .w(px(STRIP_WIDTH))
            .h(px(STRIP_HEIGHT))
            .gap(px(2.))
            .rounded(px(3.))
            .overflow_hidden()
            .flex_none()
            .children(group.steps.iter().zip(&weights).enumerate().map(
                |(position, (step, weight))| {
                    // The segment color is the same in both schemes.
                    let color: Hsla = if step.is_running {
                        t.dot_running
                    } else {
                        turn_label_colors(step.label, bachuc_theme::Scheme::Light).segment
                    };
                    let key = self.grouper.key(step.id);
                    let dimmed = selected.is_some_and(|picked| picked != key);
                    div()
                        .id(("segment", key))
                        .role(Role::Button)
                        .aria_label(format!("Step {}", position + 1))
                        .h_full()
                        .flex_shrink(1.)
                        .w(px(STRIP_WIDTH * weight / weight_sum))
                        .bg(color)
                        .when(dimmed, |segment| segment.opacity(DIMMED_OPACITY))
                        .cursor_pointer()
                        .on_click(
                            cx.listener(move |this, _, _, cx| this.select_step(key, true, cx)),
                        )
                },
            ));

        let summary_size = ui_font_size(TURN_SUMMARY_SIZE, cx);
        let summary = h_flex()
            .ml_auto()
            .flex_shrink(1.)
            .min_w_0()
            .gap_1()
            .text_size(summary_size)
            .text_color(t.foreground_muted)
            .child(steps_text)
            .when(!files.is_empty(), |line| line.child("·"))
            .when(files.len() == 1, |line| {
                line.child(
                    div()
                        .truncate()
                        .font_family(mono)
                        .text_size(code_font_size(FONT_CODE, cx))
                        .text_color(t.foreground)
                        .child(short_path(&files[0])),
                )
            })
            .when(files.len() > 1, |line| {
                line.child(plural(files.len(), "file", "files"))
            })
            .when(!files.is_empty() && additions + deletions > 0, |line| {
                line.child(
                    div()
                        .text_color(t.status_success)
                        .child(format!("+{additions}")),
                )
                .child(
                    div()
                        .text_color(t.status_danger)
                        .child(format!("−{deletions}")),
                )
            });

        let header = h_flex()
            .gap(px(SPACE_3))
            .items_center()
            .child(
                div()
                    .text_size(ui_font_size(TURN_TITLE_SIZE, cx))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(t.foreground)
                    .child(title),
            )
            .children(group.is_loading.then(|| {
                div()
                    .font_family(mono)
                    .text_size(code_font_size(TURN_CLOCK_SIZE, cx))
                    .text_color(t.foreground_muted)
                    .child(format_clock(now - started_at))
            }))
            .child(strip)
            .child(summary);

        div()
            .id(("turn", row))
            .role(Role::Heading)
            .aria_label(format!(
                "Turn, {}, {}",
                if group.is_loading { "working" } else { "done" },
                plural(group.step_count, "step", "steps")
            ))
            .pt(px(SPACE_2))
            .child(header)
            .into_any_element()
    }

    fn render_step(
        &self,
        position: usize,
        group: &TurnGroup,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let step = &group.steps[position];
        let run_started_at = group.run.started_at;
        let is_first = position == 0;
        let is_last = position + 1 == group.steps.len();
        let is_last_of_finished = is_last && !group.is_loading;
        let t = tokens(cx);
        let mono = mono_font_family(cx);
        let colors = turn_label_colors(step.label, scheme(cx));
        let (text_color, border_color) = if step.is_running {
            (t.dot_running, t.dot_running)
        } else {
            (colors.text, colors.border)
        };
        let label = label_name(step.label);
        let counts = counts_text(&step.counts);
        let time = step
            .ended_at
            .map_or("now".to_string(), |end| format_clock(end - run_started_at));
        let step_key = entry_key(&self.entries[step.id]);
        let is_selected = self.selected_step == Some(step_key);
        let small = code_font_size(STEP_LABEL_SIZE, cx);

        let mut aria = format!("Step {}, {label}", position + 1);
        if !counts.is_empty() {
            aria.push_str(&format!(", {counts}"));
        }
        // Only the header line is a button: a press in a diff or a note must not pick the step.
        let header = h_flex()
            .id(("step-header", step_key))
            .role(Role::Button)
            .aria_label(aria)
            .aria_selected(is_selected)
            .gap(px(SPACE_3))
            .items_center()
            .cursor_pointer()
            .on_click(cx.listener(move |this, _, _, cx| this.select_step(step_key, false, cx)))
            .child(
                div()
                    .size(px(STEP_NUMBER))
                    .mr(px(2.))
                    .flex_none()
                    .rounded_full()
                    .border_1()
                    .when(step.is_running, |number| number.border(px(1.5)))
                    .border_color(border_color)
                    .bg(t.surface0)
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(small)
                    .font_weight(FontWeight::SEMIBOLD)
                    .font_family(mono)
                    .text_color(text_color)
                    .child((position + 1).to_string()),
            )
            .child(
                h_flex()
                    .flex_1()
                    .min_w_0()
                    .gap(px(SPACE_1 + 2.))
                    .items_baseline()
                    .child(
                        div()
                            .text_size(small)
                            .font_weight(FontWeight::SEMIBOLD)
                            .font_family(mono)
                            .text_color(text_color)
                            .child(label.to_uppercase()),
                    )
                    .when(!counts.is_empty(), |meta| {
                        meta.child(
                            div()
                                .truncate()
                                .text_size(ui_font_size(STEP_COUNTS_SIZE, cx))
                                .text_color(t.foreground_muted)
                                .child(counts.clone()),
                        )
                    }),
            )
            .child(
                div()
                    .w(px(STEP_TIME_WIDTH))
                    .text_right()
                    .font_family(mono)
                    .text_size(code_font_size(STEP_TIME_SIZE, cx))
                    .line_height(px(STEP_NUMBER))
                    .text_color(t.foreground_muted)
                    .child(time),
            );

        let mut notes = Vec::new();
        for &note in &step.note {
            let key = entry_key(&self.entries[note]);
            match self.projector.get(note) {
                None => notes.extend(self.render_markdown(key, cx)),
                Some(projection) => {
                    for part in projection.parts() {
                        match part {
                            Part::Source => notes.extend(self.render_markdown(key, cx)),
                            Part::Item(item) => notes.push(self.render_plugin_item(&item.id, cx)),
                        }
                    }
                }
            }
        }
        // Edits and plugin items, in call order (`turn/view.tsx`): a plugin item always shows,
        // an edit call that a plugin did not replace keeps its edit row.
        let mut edits = Vec::new();
        for &index in &step.items {
            let projection = self.projector.get(index).map(|p| &**p);
            for part in plugins::step_parts(projection, step.edits.contains(&index)) {
                edits.push(match part {
                    StepPart::Edit => self.render_edit(index, cx),
                    StepPart::Item(item) => self.render_plugin_item(&item.id, cx),
                });
            }
        }
        // A picked step also lists its other calls, so each tool kind can open its details.
        let calls = if is_selected {
            step.items
                .iter()
                .filter(|index| !step.edits.contains(index))
                .filter(|&&index| item_type(&self.entries[index]) == "tool_call")
                .filter(|&&index| plugins::call_listed(self.projector.get(index).map(|p| &**p)))
                // The step body has no stream padding, so the badge does not reach past it.
                .map(|&index| {
                    div()
                        .mx(px(tool_view::BADGE_BLEED))
                        .child(self.render_tool_call(index, cx))
                        .into_any_element()
                })
                .collect()
        } else {
            Vec::new()
        };
        let live = step.is_running.then(|| self.render_live_box(step, cx));

        // The rail joins the step numbers, from the middle of the first to the middle of the
        // last.
        let rail = div()
            .absolute()
            .left(px(STEP_NUMBER / 2.))
            .w(px(1.))
            .bg(t.border)
            .top(px(if is_first {
                STEP_LIST_TOP + STEP_NUMBER / 2.
            } else {
                0.
            }))
            .when(is_last, |rail| rail.h(px(SPACE_1 + STEP_NUMBER / 2.)))
            .when(!is_last, |rail| rail.bottom_0());

        v_flex()
            .id(("step", step_key))
            .relative()
            .when(is_first, |row| row.pt(px(STEP_LIST_TOP)))
            .when(!(is_first && is_last), |row| row.child(rail))
            .child(
                v_flex()
                    .relative()
                    .gap(px(3.))
                    .mx(px(-STEP_GAP))
                    .px(px(STEP_GAP))
                    .py(px(SPACE_1))
                    .mb(px(STEP_GAP))
                    .rounded(px(RADIUS_BASE))
                    .when(is_selected, |row| row.bg(t.surface2))
                    .child(header)
                    .child(
                        v_flex()
                            .min_w_0()
                            .gap(px(3.))
                            .ml(px(STEP_BODY_LEFT))
                            .mr(px(STEP_BODY_RIGHT))
                            .children(notes)
                            .children(edits)
                            .children(calls)
                            .children(live),
                    ),
            )
            .when(is_last_of_finished, |step| {
                step.child(
                    div()
                        .mt(px(RULE_TOP - STEP_GAP))
                        .mb(px(RULE_BOTTOM))
                        .h(px(1.))
                        .w_full()
                        .bg(t.border),
                )
            })
            .into_any_element()
    }

    /// The running step's last call: its command and the last lines of its output.
    fn render_live_box(&self, step: &TurnStep, cx: &mut Context<Self>) -> AnyElement {
        let t = tokens(cx);
        let Some(&last) = step.items.last() else {
            return div().into_any_element();
        };
        let item = &self.entries[last]["item"];
        let detail = &item["detail"];
        let is_shell = detail["type"] == "shell";
        let command = if is_shell {
            format!("$ {}", detail["command"].as_str().unwrap_or(""))
        } else {
            tools::presentation(item, &self.cwd).name
        };
        let output = if is_shell {
            detail["output"].as_str().unwrap_or("")
        } else {
            ""
        };
        let lines: Vec<&str> = output.lines().filter(|l| !l.trim().is_empty()).collect();
        let shown = lines[lines.len().saturating_sub(OUTPUT_LINES)..].to_vec();
        v_flex()
            .id(("live", entry_key(&self.entries[last])))
            .role(Role::Status)
            .aria_label(format!("Running {command}"))
            .mt(px(SPACE_1 + 2.))
            .px(px(SPACE_2 + 2.))
            .py(px(7.))
            .rounded(px(RADIUS_LG))
            .border_1()
            .border_color(t.border)
            .font_family(mono_font_family(cx))
            .text_size(code_font_size(FONT_CODE, cx))
            .line_height(px(LIVE_LINE_HEIGHT))
            .child(div().truncate().text_color(t.foreground).child(command))
            .children(shown.into_iter().map(|line| {
                div()
                    .truncate()
                    .text_color(t.foreground_muted)
                    .child(line.to_string())
            }))
            .into_any_element()
    }
}

impl Render for TimelineView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = tokens(cx);
        let base = div()
            .id("timeline")
            .role(Role::Group)
            .aria_label("Timeline")
            .key_context("Timeline")
            .track_focus(&self.focus)
            .relative()
            .size_full()
            .bg(t.surface0)
            .text_color(t.foreground)
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                if event.keystroke.key == "escape" && this.lightbox.take().is_some() {
                    cx.stop_propagation();
                    cx.notify();
                }
            }));
        let body = if self.rows.is_empty() {
            let text = if self.loaded {
                "Start chatting with this agent..."
            } else {
                "Loading timeline..."
            };
            div()
                .id("timeline-empty")
                .role(Role::Status)
                .aria_label(text)
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .text_size(ui_font_size(FONT_BASE, cx))
                .text_color(t.foreground_muted)
                .child(text)
                .into_any_element()
        } else {
            div()
                .id("timeline-list")
                .role(Role::List)
                .aria_label("Timeline")
                .size_full()
                .child(
                    list(self.list.clone(), cx.processor(Self::render_row))
                        .size_full()
                        .pt(px(SPACE_4 + HISTORY_START_SLOT))
                        .pb(px(SPACE_4)),
                )
                .into_any_element()
        };
        let lightbox = self.render_lightbox(cx);
        let bounds = self.bounds.clone();
        // Records where the timeline is, so the selection toolbar can stay inside it.
        let measure = canvas(move |area, _, _| bounds.set(area), |_, _, _, _| {})
            .absolute()
            .size_full();
        base.child(measure)
            .child(body)
            .children(self.render_selection_toolbar(cx))
            .children(self.render_find_bar(window, cx))
            .children(lightbox)
    }
}

/// `Notification`: an icon and the message on a tinted box.
fn notification(
    id: impl Into<ElementId>,
    role: Role,
    name: IconName,
    color: Hsla,
    tint: Hsla,
    message: String,
    size: Pixels,
) -> AnyElement {
    h_flex()
        .id(id)
        .role(role)
        .aria_label(message.clone())
        .mb(px(SPACE_1))
        .items_start()
        .gap(px(SPACE_2))
        .px(px(SPACE_3))
        .py(px(10.))
        .rounded(px(RADIUS_MD))
        .bg(tint)
        .text_size(size)
        .line_height(px(20.))
        .child(
            div()
                .h(px(20.))
                .flex_none()
                .flex()
                .items_center()
                .child(icon(name).with_size(px(ICON_MD)).text_color(color)),
        )
        .child(div().flex_1().child(message))
        .into_any_element()
}

/// Which load phase an image is in, for the row hash.
fn image_phase(load: &ImageLoad) -> u8 {
    match load {
        ImageLoad::Loading => 0,
        ImageLoad::Ready { .. } => 1,
        ImageLoad::Failed(_) => 2,
    }
}

/// Copies the client's entries into `copies`, keeping each copy whose position, `seqStart`, and
/// `seqEnd` did not change: the client changes an entry only with its `seqEnd`.
fn copy_changed(copies: &mut Vec<Value>, current: &[Value]) {
    copies.truncate(current.len());
    for (index, entry) in current.iter().enumerate() {
        match copies.get_mut(index) {
            Some(copy)
                if entry_key(copy) == entry_key(entry)
                    && copy["seqEnd"].as_u64() == entry["seqEnd"].as_u64() => {}
            Some(copy) => *copy = entry.clone(),
            None => copies.push(entry.clone()),
        }
    }
}

fn label_name(label: TurnLabel) -> &'static str {
    match label {
        TurnLabel::Explore => "Explore",
        TurnLabel::Edit => "Edit",
        TurnLabel::Verify => "Verify",
    }
}

/// Adds `key` to `set`, or removes it when it is there.
fn toggle(set: &mut HashSet<u64>, key: u64) {
    if !set.remove(&key) {
        set.insert(key);
    }
}

fn plural(count: usize, one: &str, other: &str) -> String {
    format!("{count} {}", if count == 1 { one } else { other })
}

/// The step meta line, as the app words it.
fn counts_text(counts: &StepCounts) -> String {
    let parts = [
        (counts.edited_file_count, "edited", "file", "files"),
        (counts.command_count, "ran", "command", "commands"),
        (counts.read_file_count, "read", "file", "files"),
        (counts.search_count, "searched", "time", "times"),
        (counts.other_tool_count, "used", "other tool", "other tools"),
        (counts.paseo_call_count, "called Paseo", "time", "times"),
    ];
    parts
        .iter()
        .filter(|(count, ..)| *count > 0)
        .map(|(count, verb, one, other)| format!("{verb} {}", plural(*count, one, other)))
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_clock(ms: i64) -> String {
    let seconds = (ms / 1000).max(0);
    format!("{}:{:02}", seconds / 60, seconds % 60)
}

fn format_duration(ms: i64) -> String {
    let seconds = (ms / 1000).max(0);
    if seconds >= 3600 {
        format!("{}h {:02}m", seconds / 3600, (seconds % 3600) / 60)
    } else if seconds >= 60 {
        format!("{}m {:02}s", seconds / 60, seconds % 60)
    } else {
        format!("{seconds}s")
    }
}

/// The body line box in whole pixels. GPUI rounds a line height to whole pixels, and the text
/// view needs the same number the paragraph pitch is built from.
fn body_line_height(content: f32) -> f32 {
    (content * BODY_LINE_HEIGHT).round()
}

/// Distance between the tops of two paragraphs of one line: the line box and the gap under it.
#[cfg(test)]
fn paragraph_pitch(content: f32) -> f32 {
    body_line_height(content) + PARAGRAPH_GAP
}

/// The fence's `marginTop: 12`. The text view stacks its blocks in block flow, where taffy
/// collapses an h1 or h2 bottom margin with a fence's top margin (24 and 12 give 24), as CSS
/// does. The web stream stacks blocks in flex columns, which add them (36). A relative offset
/// with the same space below moves the fence as a margin would, and no margin collapses with it.
fn fence_spacing(style: StyleRefinement) -> StyleRefinement {
    style.relative().top(px(SPACE_3)).mb(px(SPACE_3))
}

/// One heading level of `markdown-styles.ts`: its `FONT_SIZE` tier over the base size, scaled
/// to the content size and rounded, bold for h1 and h2 with a rule under them, semibold below.
/// The top margin becomes padding, so it replaces the text view's own section gap. A heading
/// gets no paragraph gap below it, so its bottom also holds the block container's 12.
fn heading_style(level: u8, content: Pixels, border: Hsla) -> StyleRefinement {
    let block = BLOCK_GAP - SPACE_3;
    let (tier, top, bottom) = match level {
        1 => (FONT_4XL, SPACE_6, SPACE_3),
        2 => (FONT_3XL, SPACE_6, SPACE_3),
        3 => (FONT_2XL, SPACE_4, SPACE_2),
        4 => (FONT_XL, SPACE_4, SPACE_2),
        _ => (FONT_LG, SPACE_3, SPACE_1),
    };
    let size = (content * (tier / FONT_BASE)).round();
    let style = StyleRefinement::default()
        .text_size(size)
        .line_height((size * HEADING_LINE_HEIGHT).round())
        .pt(px(top));
    if level <= 2 {
        style
            .font_weight(FontWeight::BOLD)
            .pb(px(SPACE_2))
            .mb(px(bottom + block))
            .border_b_1()
            .border_color(border)
    } else {
        style
            .font_weight(FontWeight::SEMIBOLD)
            .pb(px(bottom + block))
    }
}

/// `formatDuration` of `utils/time.ts`, the turn footer's duration: "47s", "2m 12s", "1h 5m".
fn format_elapsed(ms: i64) -> String {
    let seconds = (ms / 1000).max(0);
    let minutes = seconds / 60;
    let join = |big: i64, big_unit: &str, small: i64, small_unit: &str| {
        if small == 0 {
            format!("{big}{big_unit}")
        } else {
            format!("{big}{big_unit} {small}{small_unit}")
        }
    };
    if seconds < 60 {
        format!("{seconds}s")
    } else if minutes < 60 {
        join(minutes, "m", seconds % 60, "s")
    } else {
        join(minutes / 60, "h", minutes % 60, "m")
    }
}

fn short_path(path: &str) -> String {
    let parts: Vec<&str> = path.split('/').filter(|p| !p.is_empty()).collect();
    parts[parts.len().saturating_sub(2)..].join("/")
}

/// The first line of a message, cut short, for accessibility labels.
fn preview(text: &str) -> String {
    let line = text.lines().find(|l| !l.trim().is_empty()).unwrap_or("");
    let mut preview: String = line.chars().take(80).collect();
    if line.chars().count() > 80 {
        preview.push_str("...");
    }
    preview
}

#[cfg(test)]
mod tests {
    // Not `super::*`: the gpui glob brings its own `test` attribute.
    use super::tool_view::{code_styles, fence_language};
    use super::{
        BLOCK_GAP, PARAGRAPH_GAP, StepCounts, body_line_height, counts_text, fence_spacing,
        format_clock, format_duration, format_elapsed, heading_style, paragraph_pitch, toggle,
    };
    use bachuc_theme::metrics::FONT_CONTENT;
    use bachuc_theme::metrics::SPACE_3;
    use bachuc_theme::{Scheme, SyntaxThemeId, syntax_colors_for};
    use gpui_kit::{
        AbsoluteLength, DefiniteLength, FontWeight, Hsla, Length, StyleRefinement, hsla, px, rgb,
    };
    use std::collections::HashSet;

    /// Expanding a tool badge flips its key; the crash fix keeps this state path and changes only
    /// how the badge's icon slot hides its layers (see `render_badge`).
    #[test]
    fn toggle_opens_and_closes_a_key() {
        let mut expanded = HashSet::new();
        toggle(&mut expanded, 7);
        assert!(expanded.contains(&7));
        toggle(&mut expanded, 7);
        assert!(expanded.is_empty());
    }

    #[test]
    fn fence_languages_follow_the_app_aliases() {
        assert_eq!(fence_language("rust"), Some("rust"));
        assert_eq!(fence_language("rs"), Some("rust"));
        assert_eq!(fence_language("ts {1,3}"), Some("typescript"));
        assert_eq!(fence_language("Python"), Some("python"));
        assert_eq!(fence_language("text"), None);
    }

    #[test]
    fn fence_colors_follow_the_one_theme() {
        let code = r#"fn main() { println!("hi"); }"#;
        let styles = code_styles(
            "rust",
            code,
            syntax_colors_for(SyntaxThemeId::One, Scheme::Light),
        );
        let color_at = |at: usize| {
            styles
                .iter()
                .find(|(range, _)| range.contains(&at))
                .and_then(|(_, style)| style.color)
        };
        let one = |hex| Some(Hsla::from(rgb(hex)));
        assert_eq!(color_at(0), one(0xa626a4), "fn is a keyword");
        assert_eq!(color_at(3), one(0x4078f2), "main is a function");
        assert_eq!(color_at(21), one(0x50a14f), "\"hi\" is a string");
        // The app's grammar gives a macro name no role, so it keeps the text color.
        assert_eq!(color_at(12), None, "println has no color");
    }

    /// A syntax theme switch recolors code: the same source takes the new theme's colors.
    #[test]
    fn syntax_themes_give_different_code_colors() {
        let code = r#"fn main() { println!("hi"); }"#;
        let keyword_color = |theme| {
            let colors = syntax_colors_for(theme, Scheme::Light);
            let styles = code_styles("rust", code, colors);
            let found = styles
                .iter()
                .find(|(range, _)| range.contains(&0))
                .and_then(|(_, style)| style.color);
            assert_eq!(found, Some(colors.keyword), "fn takes the keyword color");
            found
        };
        assert_ne!(
            keyword_color(SyntaxThemeId::One),
            keyword_color(SyntaxThemeId::Dracula)
        );
    }

    /// The web's paragraph pitch at the content size is 47; the whole-pixel line box of GPUI
    /// gave 48, which drifted reply text 1 px per block.
    #[test]
    fn paragraph_pitch_matches_the_web_stream() {
        assert_eq!(body_line_height(FONT_CONTENT), 24.);
        assert_eq!(PARAGRAPH_GAP, BLOCK_GAP - 1.);
        assert_eq!(paragraph_pitch(FONT_CONTENT), 47.);
    }

    #[test]
    fn headings_hold_the_block_gap_below() {
        let border = hsla(0., 0., 0.5, 1.);
        // h2: `marginBottom: 12` plus the block's 12; h3: `marginBottom: 8` plus 12.
        let h2 = heading_style(2, px(15.), border);
        assert_eq!(h2.margin.bottom, Some(Length::Definite(px(24.).into())));
        let h3 = heading_style(3, px(15.), border);
        assert_eq!(h3.padding.bottom, Some(px(20.).into()));
    }

    fn length_px(length: Option<Length>) -> f32 {
        match length {
            None => 0.,
            Some(Length::Definite(DefiniteLength::Absolute(AbsoluteLength::Pixels(value)))) => {
                value / px(1.)
            }
            Some(other) => panic!("not a pixel length: {other:?}"),
        }
    }

    /// The space under an h2 rule and under a paragraph before a fence. The text view's block
    /// flow collapses adjoining margins (the larger wins); padding and a relative offset add.
    #[test]
    fn fence_gaps_match_the_web_stream() {
        let border = hsla(0., 0., 0.5, 1.);
        let heading = heading_style(2, px(15.), border);
        let fence = fence_spacing(StyleRefinement::default());
        let fence_margin = length_px(fence.margin.top);
        let fence_offset = length_px(fence.inset.top);
        // The offset is undone below, so the blocks after the fence keep their place.
        assert_eq!(length_px(fence.margin.bottom), fence_offset);

        // Web: h2 `marginBottom: 12`, the block's 12, and the fence's `marginVertical: 12`, in
        // flex columns that add them: 36 under the 1 px rule, 37 from the rule's top.
        let web = SPACE_3 * 3.;
        let collapsed = length_px(heading.margin.bottom).max(fence_margin);
        assert_eq!(collapsed + fence_offset, web, "heading rule to fence");
        // A paragraph's gap is padding, which never collapses.
        assert_eq!(
            BLOCK_GAP + fence_margin + fence_offset,
            web,
            "paragraph to fence"
        );
    }

    #[test]
    fn clock_and_duration_match_the_app() {
        assert_eq!(format_clock(65_400), "1:05");
        assert_eq!(format_duration(4_000), "4s");
        assert_eq!(format_duration(65_000), "1m 05s");
        assert_eq!(format_duration(3_720_000), "1h 02m");
    }

    #[test]
    fn footer_duration_matches_format_duration() {
        assert_eq!(format_elapsed(7_400), "7s");
        assert_eq!(format_elapsed(120_000), "2m");
        assert_eq!(format_elapsed(132_000), "2m 12s");
        assert_eq!(format_elapsed(3_900_000), "1h 5m");
        assert_eq!(format_elapsed(-5), "0s");
    }

    #[test]
    fn headings_match_markdown_styles() {
        let border = hsla(0., 0., 0.5, 1.);
        // Content 15: h1 28, h2 24, h3 21, h4 19, h5 and h6 17.
        let sizes: Vec<_> = (1..=6)
            .map(|level| heading_style(level, px(15.), border).text.font_size)
            .collect();
        let expected = [28., 24., 21., 19., 17., 17.];
        for (size, want) in sizes.iter().zip(expected) {
            assert_eq!(*size, Some(AbsoluteLength::Pixels(px(want))));
        }
        let h2 = heading_style(2, px(15.), border);
        assert_eq!(h2.text.font_weight, Some(FontWeight::BOLD));
        assert_eq!(h2.border_color, Some(border));
        let h3 = heading_style(3, px(15.), border);
        assert_eq!(h3.text.font_weight, Some(FontWeight::SEMIBOLD));
        assert_eq!(h3.border_color, None);
    }

    #[test]
    fn counts_read_like_the_app() {
        let counts = StepCounts {
            edited_file_count: 2,
            command_count: 1,
            ..Default::default()
        };
        assert_eq!(counts_text(&counts), "edited 2 files, ran 1 command");
    }
}
