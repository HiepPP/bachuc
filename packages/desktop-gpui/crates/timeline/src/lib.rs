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
mod tool_view;
pub mod tools;

pub use image_view::{display_size, image_dimensions, read_target};
pub use tool_view::{change_bar, display_path, edit_status_label};

use std::cell::RefCell;
use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use bachuc_client::{Client, ClientEvent};
use bachuc_diagram::{DiagramReady, DiagramRenderer, image_size};
use bachuc_explorer::syntax::Run;
use bachuc_icons::{IconName, icon};
use bachuc_theme::metrics::{
    FONT_BASE, FONT_CODE, FONT_CONTENT, ICON_MD, ICON_XS, MAX_CONTENT_WIDTH, RADIUS_2XL,
    RADIUS_BASE, RADIUS_LG, RADIUS_MD, RADIUS_SM, SPACE_1, SPACE_2, SPACE_3, SPACE_4,
};
use bachuc_theme::{
    TurnLabel, code_font_size, mono_font_family, scheme, tokens, turn_label_colors, ui_font_size,
};
use gpui_kit::component::text::{TextView, TextViewState};
use gpui_kit::component::{Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use serde_json::Value;

use card_view::FormState;
use diff::DiffLine;
use find_bar::FindState;
use grouping::{SharedRow, StepCounts, TurnGroup, TurnGrouper, TurnStep, entry_key, item_type};
use image_view::ImageLoad;
use markdown::{Block, Segment};

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
/// `STREAM_METADATA_FONT_SIZE`, the compaction marker text.
const METADATA_SIZE: f32 = 13.;

#[derive(Clone, Debug, PartialEq, Eq)]
enum RowKey {
    Entry(u64),
    Turn(u64),
    Step(u64),
    Permission(String),
}

#[derive(Clone)]
enum RowKind {
    Entry(usize),
    TurnHeader(Rc<TurnGroup>),
    /// A turn and the index of one of its steps.
    Step(Rc<TurnGroup>, usize),
    Permission(Value),
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
    segments: Vec<Segment>,
    states: Vec<Option<Entity<TextViewState>>>,
}

/// Syntax runs per line, as `bachuc_explorer::syntax::highlight` returns them.
type LineRuns = Rc<Vec<Vec<Run>>>;

/// The diff lines of one edit or write call and their added and removed counts.
#[derive(Default)]
struct Diff {
    lines: Vec<DiffLine>,
    stats: (usize, usize),
    /// Syntax runs and the scheme they were colored for, made on the first draw.
    runs: RefCell<Option<(bachuc_theme::Scheme, LineRuns)>>,
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
    clock: Option<Task<()>>,
    /// When the last sync redrew the view.
    synced_at: Option<Instant>,
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
        let list = ListState::new(0, ListAlignment::Bottom, px(800.));
        list.set_follow_mode(FollowMode::Tail);
        let view = cx.entity().downgrade();
        list.set_scroll_handler(move |event, _, cx| {
            if event.visible_range.start < LOAD_OLDER_WITHIN {
                let _ = view.update(cx, |this, cx| this.load_older(cx));
            }
        });

        let subscriptions = vec![
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
                this.list.remeasure();
                cx.notify();
            }),
            cx.on_release(|this, cx| {
                let agent_id = this.agent_id.clone();
                this.client
                    .update(cx, |client, cx| client.unview_agent(&agent_id, cx));
            }),
        ];

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
            clock: None,
            synced_at: None,
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
        if let Some(agent) = agent
            && agent.cwd != self.cwd
        {
            self.cwd = agent.cwd.clone();
        }
        let Some(state) = client.timeline(&self.agent_id) else {
            return;
        };
        if self.version == Some(state.version) && self.turn_active == turn_active {
            return;
        }
        self.version = Some(state.version);
        self.turn_active = turn_active;
        self.loaded = true;
        self.has_older = state.has_older;
        let permissions = state.pending_permissions.clone();
        copy_changed(&mut self.entries, &state.entries);
        let pending = |id: &String| permissions.iter().any(|request| request["id"] == **id);
        self.responding.retain(|id, _| pending(id));
        self.forms.retain(|id, _| pending(id));
        self.grouper.sync(&self.entries, turn_active);
        let images = self.update_messages(cx);
        self.update_hashes();
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
        for row in self.grouper.rows() {
            match row {
                SharedRow::Item(index) => rows.push(self.entry_row(*index)),
                SharedRow::Turn(group) => match old_turns.get(&Rc::as_ptr(group)) {
                    Some(&header) => {
                        rows.extend_from_slice(&self.rows[header..=header + group.steps.len()])
                    }
                    None => self.push_turn_rows(group, &mut rows),
                },
            }
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
        self.apply_rows(rows);
        self.update_clock(cx);
        self.synced_at = Some(Instant::now());
        cx.notify();
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

    /// Hashes the entries the last grouper sync read again, and builds the diffs of the edit
    /// and write calls among them.
    fn update_hashes(&mut self) {
        if self.grouper.shifted() {
            let live: HashSet<u64> = (0..self.entries.len())
                .map(|index| self.grouper.key(index))
                .collect();
            self.diffs.retain(|key, _| live.contains(key));
        }
        self.entry_hashes.truncate(self.entries.len());
        for index in self.grouper.changed().to_vec() {
            let entry = &self.entries[index];
            if grouping::edited_file_path(entry).is_some() {
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
    fn update_messages(&mut self, cx: &mut Context<Self>) -> Vec<String> {
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
        for &index in self.grouper.changed() {
            let entry = &self.entries[index];
            if item_type(entry) != "assistant_message" {
                continue;
            }
            let key = entry_key(entry);
            let seq_end = entry["seqEnd"].as_u64().unwrap_or(0);
            if self
                .messages
                .get(&key)
                .is_some_and(|message| message.seq_end == seq_end)
            {
                continue;
            }
            let text = entry["item"]["text"].as_str().unwrap_or("");
            let segments = markdown::segments(text);
            let old_states = self
                .messages
                .remove(&key)
                .map(|message| message.states)
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
                        None => cx.new(|cx| TextViewState::markdown(&text, cx)),
                    })
                })
                .collect();
            self.messages.insert(
                key,
                Message {
                    seq_end,
                    segments,
                    states,
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
        if !self.hidden_diffs.remove(&key) {
            self.hidden_diffs.insert(key);
        }
        // Only the edit's hash changes; the rows rebuild from the cached hashes.
        self.rehash_keys(&[key]);
        self.refresh(cx);
    }

    fn toggle_expanded(&mut self, key: u64, cx: &mut Context<Self>) {
        if !self.expanded.remove(&key) {
            self.expanded.insert(key);
        }
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
        let this = &*self;
        let Some(row) = this.rows.get(index) else {
            return div().into_any_element();
        };
        let content = match &row.kind {
            RowKind::Entry(entry) => this.render_entry(*entry, cx),
            RowKind::TurnHeader(group) => this.render_turn_header(index, group, cx),
            RowKind::Step(group, position) => this.render_step(*position, group, cx),
            RowKind::Permission(request) => this.render_permission(request, cx),
        };
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
                    .child(content),
            )
            .into_any_element()
    }

    fn render_entry(&self, index: usize, cx: &mut Context<Self>) -> AnyElement {
        let entry = &self.entries[index];
        let key = entry_key(entry);
        let item = &entry["item"];
        let t = tokens(cx);
        let base = ui_font_size(FONT_BASE, cx);
        match item_type(entry) {
            "user_message" => {
                let text = item["text"].as_str().unwrap_or("").to_string();
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
                h_flex()
                    .justify_end()
                    .py(px(SPACE_4))
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
                let text = item["text"].as_str().unwrap_or("");
                v_flex()
                    .id(("message", key))
                    .role(Role::Article)
                    .aria_label(format!("Assistant message: {}", preview(text)))
                    .py(px(SPACE_3))
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
        let size = ui_font_size(FONT_CONTENT, cx);
        let view = cx.weak_entity();
        blocks
            .into_iter()
            .enumerate()
            .map(|(position, block)| match block {
                Block::Markdown(_) => match &message.states[position] {
                    Some(state) => {
                        let view = view.clone();
                        div()
                            .text_size(size)
                            .child(TextView::new(state).selectable(true).on_link_click(
                                move |href, _, _, cx| {
                                    let href = href.to_string();
                                    let _ = view.update(cx, |this, cx| this.open_link(&href, cx));
                                },
                            ))
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

        let notes = step
            .note
            .iter()
            .flat_map(|&note| self.render_markdown(entry_key(&self.entries[note]), cx))
            .collect::<Vec<_>>();
        let edits = step
            .edits
            .iter()
            .map(|&edit| self.render_edit(edit, cx))
            .collect::<Vec<_>>();
        // A picked step also lists its other calls, so each tool kind can open its details.
        let calls = if is_selected {
            step.items
                .iter()
                .filter(|index| !step.edits.contains(index))
                .filter(|&&index| item_type(&self.entries[index]) == "tool_call")
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
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = tokens(cx);
        let base = div()
            .id("timeline")
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
                        .py(px(SPACE_4)),
                )
                .into_any_element()
        };
        let lightbox = self.render_lightbox(cx);
        base.child(body)
            .children(self.render_find_bar(cx))
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
    use super::{StepCounts, counts_text, format_clock, format_duration};

    #[test]
    fn clock_and_duration_match_the_app() {
        assert_eq!(format_clock(65_400), "1:05");
        assert_eq!(format_duration(4_000), "4s");
        assert_eq!(format_duration(65_000), "1m 05s");
        assert_eq!(format_duration(3_720_000), "1h 02m");
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
