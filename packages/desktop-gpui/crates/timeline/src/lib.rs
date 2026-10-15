//! `bachuc-timeline`: the timeline of one agent as a virtualized GPUI list. Each turn draws as
//! an always-open step list with its answer below (ADR-0018, ADR-0019, ADR-0021); mermaid and
//! display math blocks show as images from `bachuc-diagram` (ADR-0037).

pub mod diff;
pub mod grouping;
pub mod markdown;

use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::rc::Rc;
use std::time::Duration;

use bachuc_client::{Client, ClientEvent};
use bachuc_diagram::{DiagramReady, DiagramRenderer, image_size};
use bachuc_theme::{TurnLabel, scheme, turn_label_colors};
use gpui_kit::base::StyledExt as _;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::text::{TextView, TextViewState};
use gpui_kit::component::{ActiveTheme as _, Disableable as _, Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use serde_json::Value;

use diff::LineKind;
use grouping::{Row, StepCounts, TurnGroup, TurnStep, entry_key, item_type};
use markdown::{Block, Segment};

/// Rows within this many of the top load the page before the first entry.
const LOAD_OLDER_WITHIN: usize = 2;
/// The running step's segment and number.
const RUNNING_COLOR: u32 = 0x256ABF;
/// The smallest share of the strip one step gets, so short steps stay visible.
const MIN_SEGMENT_SHARE: f32 = 0.04;
const STRIP_WIDTH: f32 = 170.;
/// Live box lines of the running step's last command.
const OUTPUT_LINES: usize = 2;
const MAX_THOUGHT_CHARS: usize = 400;

#[derive(Clone, Debug, PartialEq, Eq)]
enum RowKey {
    Entry(u64),
    Turn(u64),
    Step(u64),
    Permission(String),
}

enum RowKind {
    Entry(usize),
    TurnHeader(Rc<TurnGroup>),
    /// A turn and the index of one of its steps.
    Step(Rc<TurnGroup>, usize),
    Permission(Value),
}

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

/// The timeline of one agent. Create it with `cx.new(|cx| TimelineView::new(...))`.
pub struct TimelineView {
    client: Entity<Client>,
    agent_id: String,
    diagrams: Entity<DiagramRenderer>,
    list: ListState,
    /// The entries the rows point into: a copy of the client's, so a sync between two
    /// `TimelineChanged` events cannot move them under the rows.
    entries: Vec<Value>,
    rows: Vec<ViewRow>,
    version: Option<u64>,
    has_older: bool,
    loaded: bool,
    turn_active: bool,
    messages: HashMap<u64, Message>,
    /// Edits whose diff the user hid; diffs are open by default (ADR-0019).
    hidden_diffs: HashSet<u64>,
    /// Permission requests answered from this view, waiting for the daemon to resolve them.
    answered: HashSet<String>,
    clock: Option<Task<()>>,
    _subscriptions: Vec<Subscription>,
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
            cx.subscribe(&diagrams, |this, _, _: &DiagramReady, cx| {
                for (index, row) in this.rows.iter().enumerate() {
                    if row.has_diagram {
                        this.list.remeasure_items(index..index + 1);
                    }
                }
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
            diagrams,
            list,
            entries: Vec::new(),
            rows: Vec::new(),
            version: None,
            has_older: false,
            loaded: false,
            turn_active: false,
            messages: HashMap::new(),
            hidden_diffs: HashSet::new(),
            answered: HashSet::new(),
            clock: None,
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
        let turn_active = client
            .agents()
            .iter()
            .find(|agent| agent.id == self.agent_id)
            .is_some_and(|agent| agent.status == "running");
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
        self.entries = reuse_entries(std::mem::take(&mut self.entries), &state.entries);
        self.answered
            .retain(|id| permissions.iter().any(|request| request["id"] == **id));
        self.update_messages(cx);

        let mut rows: Vec<ViewRow> = Vec::new();
        for row in grouping::group_turns(&self.entries, turn_active) {
            self.push_rows(row, &mut rows);
        }
        rows.extend(permissions.into_iter().map(|request| {
            let id = request["id"].as_str().unwrap_or("").to_string();
            let mut hasher = DefaultHasher::new();
            id.hash(&mut hasher);
            self.answered.contains(&id).hash(&mut hasher);
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
        cx.notify();
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
    fn hash_entry(&self, index: usize, hasher: &mut DefaultHasher) -> bool {
        let entry = &self.entries[index];
        let key = entry_key(entry);
        key.hash(hasher);
        entry["seqEnd"].as_u64().hash(hasher);
        self.hidden_diffs.contains(&key).hash(hasher);
        self.messages.get(&key).is_some_and(|message| {
            message
                .segments
                .iter()
                .any(|segment| matches!(segment, Segment::Diagram { .. }))
        })
    }

    /// A turn takes one list item for its header and one per step, so a long turn lays out
    /// only the steps on screen.
    fn push_rows(&self, row: Row, rows: &mut Vec<ViewRow>) {
        match row {
            Row::Item(index) => {
                let mut hasher = DefaultHasher::new();
                let has_diagram = self.hash_entry(index, &mut hasher);
                let key = entry_key(&self.entries[index]);
                rows.push(ViewRow {
                    key: RowKey::Entry(key),
                    fingerprint: hasher.finish(),
                    anchor: Some(key),
                    last: Some(key),
                    has_diagram,
                    kind: RowKind::Entry(index),
                });
            }
            Row::Turn(group) => {
                let group = Rc::new(group);
                let mut hasher = DefaultHasher::new();
                for &index in &group.run.items {
                    self.hash_entry(index, &mut hasher);
                }
                group.is_loading.hash(&mut hasher);
                group.running_step.hash(&mut hasher);
                let key = entry_key(&self.entries[group.run.items[0]]);
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
                        has_diagram |= self.hash_entry(index, &mut hasher);
                    }
                    (step.label as u8).hash(&mut hasher);
                    step.is_running.hash(&mut hasher);
                    // The last step draws the rule under a finished turn.
                    (position + 1 == group.steps.len() && !group.is_loading).hash(&mut hasher);
                    let key = entry_key(&self.entries[step.id]);
                    let last = step
                        .note
                        .iter()
                        .chain(&step.items)
                        .map(|&index| entry_key(&self.entries[index]))
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
        }
    }

    /// Splits each new or changed assistant entry into segments and updates its text states.
    fn update_messages(&mut self, cx: &mut Context<Self>) {
        let mut live: HashSet<u64> = HashSet::new();
        for entry in &self.entries {
            if item_type(entry) != "assistant_message" {
                continue;
            }
            let key = entry_key(entry);
            live.insert(key);
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
                    let Segment::Markdown(text) = segment else {
                        return None;
                    };
                    Some(match previous {
                        Some(state) => {
                            state.update(cx, |state, cx| state.set_text(text, cx));
                            state
                        }
                        None => cx.new(|cx| TextViewState::markdown(text, cx)),
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
        self.messages.retain(|key, _| live.contains(key));
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
                    if this.update(cx, |_, cx| cx.notify()).is_err() {
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
        self.version = None;
        self.sync(cx);
    }

    fn respond(&mut self, request_id: String, allow: bool, cx: &mut Context<Self>) {
        let agent_id = self.agent_id.clone();
        self.client.update(cx, |client, cx| {
            client.respond_permission(&agent_id, &request_id, allow, cx)
        });
        self.answered.insert(request_id);
        self.version = None;
        self.sync(cx);
    }

    fn render_row(&mut self, index: usize, _: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let this = &*self;
        let Some(row) = this.rows.get(index) else {
            return div().into_any_element();
        };
        let content = match &row.kind {
            RowKind::Entry(entry) => this.render_entry(*entry, cx),
            RowKind::TurnHeader(group) => this.render_turn_header(index, group, cx),
            RowKind::Step(group, position) => this.render_step(*position, group, cx),
            RowKind::Permission(request) => this.render_permission(index, request, cx),
        };
        div()
            .id(("row", index))
            .w_full()
            .px_4()
            .py_1p5()
            .child(content)
            .into_any_element()
    }

    fn render_entry(&self, index: usize, cx: &mut Context<Self>) -> AnyElement {
        let entry = &self.entries[index];
        let key = entry_key(entry);
        let item = &entry["item"];
        let theme = cx.theme().clone();
        match item_type(entry) {
            "user_message" => {
                let text = item["text"].as_str().unwrap_or("").to_string();
                h_flex()
                    .justify_end()
                    .child(
                        div()
                            .id(("user", key))
                            .role(Role::Article)
                            .aria_label(format!("User message: {}", preview(&text)))
                            .max_w(relative(0.8))
                            .px_3()
                            .py_2()
                            .rounded_lg()
                            .bg(theme.secondary)
                            .child(text),
                    )
                    .into_any_element()
            }
            "assistant_message" => {
                let text = item["text"].as_str().unwrap_or("");
                v_flex()
                    .id(("message", key))
                    .role(Role::Article)
                    .aria_label(format!("Assistant message: {}", preview(text)))
                    .gap_2()
                    .children(self.render_markdown(key, cx))
                    .into_any_element()
            }
            "reasoning" => {
                let text: String = item["text"]
                    .as_str()
                    .unwrap_or("")
                    .chars()
                    .take(MAX_THOUGHT_CHARS)
                    .collect();
                div()
                    .id(("thought", key))
                    .role(Role::Note)
                    .aria_label("Thought")
                    .italic()
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .child(text)
                    .into_any_element()
            }
            "tool_call" => {
                let name = item["name"].as_str().unwrap_or("tool");
                let status = item["status"].as_str().unwrap_or("");
                let plan = (item["detail"]["type"] == "plan")
                    .then(|| item["detail"]["text"].as_str().unwrap_or("").to_string());
                v_flex()
                    .id(("tool", key))
                    .role(Role::Group)
                    .aria_label(format!("Tool call {name}, {status}"))
                    .gap_1()
                    .p_2()
                    .rounded_md()
                    .border_1()
                    .border_color(theme.border)
                    .child(
                        div()
                            .text_xs()
                            .font_family(theme.mono_font_family.clone())
                            .text_color(theme.muted_foreground)
                            .child(format!("{name} · {status}")),
                    )
                    .children(plan.map(|plan| {
                        TextView::markdown(ElementId::Name(format!("plan-{key}").into()), plan)
                    }))
                    .into_any_element()
            }
            "todo" => {
                let items = item["items"].as_array().cloned().unwrap_or_default();
                v_flex()
                    .id(("todo", key))
                    .role(Role::List)
                    .aria_label("Tasks")
                    .gap_0p5()
                    .text_sm()
                    .children(items.iter().enumerate().map(|(position, task)| {
                        let done = task["completed"].as_bool().unwrap_or(false);
                        let text = task["text"].as_str().unwrap_or("").to_string();
                        div()
                            .id(("task", position))
                            .role(Role::ListItem)
                            .aria_label(text.clone())
                            .text_color(if done {
                                theme.muted_foreground
                            } else {
                                theme.foreground
                            })
                            .child(format!("{} {text}", if done { "☑" } else { "☐" }))
                    }))
                    .into_any_element()
            }
            "error" => div()
                .id(("error", key))
                .role(Role::Alert)
                .aria_label("Error")
                .text_sm()
                .text_color(theme.danger)
                .child(item["message"].as_str().unwrap_or("").to_string())
                .into_any_element(),
            other => {
                let text = match other {
                    "notification" => item["message"].as_str().unwrap_or("").to_string(),
                    "compaction" => "Context compacted".to_string(),
                    _ => other.replace('_', " "),
                };
                div()
                    .id(("note", key))
                    .role(Role::Note)
                    .aria_label(text.clone())
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .child(text)
                    .into_any_element()
            }
        }
    }

    /// The blocks of one assistant entry: markdown, diagram images, or diagram fallbacks.
    fn render_markdown(&self, key: u64, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let Some(message) = self.messages.get(&key) else {
            return Vec::new();
        };
        let diagrams = self.diagrams.clone();
        let blocks = markdown::blocks(&message.segments, |kind, source| {
            diagrams.update(cx, |renderer, cx| renderer.render(kind, source, cx))
        });
        let theme = cx.theme().clone();
        blocks
            .into_iter()
            .enumerate()
            .map(|(position, block)| match block {
                Block::Markdown(_) => match &message.states[position] {
                    Some(state) => TextView::new(state).selectable(true).into_any_element(),
                    None => div().into_any_element(),
                },
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
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .child(format!("Rendering {}...", label.to_lowercase()))
                    .into_any_element(),
                Block::Code { markdown, error } => v_flex()
                    .id(("diagram", position))
                    .role(Role::Group)
                    .aria_label("Diagram source, failed to render")
                    .gap_1()
                    .child(TextView::markdown(
                        ElementId::Name(format!("fallback-{key}-{position}").into()),
                        markdown,
                    ))
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.danger)
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
        let theme = cx.theme().clone();
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
            .map(|&edit| diff::stats(&diff::entry_lines(&self.entries[edit])))
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
        let mut summary = steps_text.clone();
        if files.len() == 1 {
            summary.push_str(&format!(" · {}", short_path(&files[0])));
        } else if files.len() > 1 {
            summary.push_str(&format!(" · {}", plural(files.len(), "file", "files")));
        }

        let weights: Vec<f32> = durations
            .iter()
            .map(|&ms| (ms as f32).max(total as f32 * MIN_SEGMENT_SHARE).max(1.))
            .collect();
        let weight_sum: f32 = weights.iter().sum();
        let scheme = scheme(cx);
        let strip = h_flex()
            .id(("strip", row))
            .role(Role::Group)
            .aria_label(format!("{} step strip", group.step_count))
            .w(px(STRIP_WIDTH))
            .h(px(8.))
            .gap(px(2.))
            .rounded(px(3.))
            .overflow_hidden()
            .flex_none()
            .children(group.steps.iter().zip(&weights).map(|(step, weight)| {
                let color: Hsla = if step.is_running {
                    rgb(RUNNING_COLOR).into()
                } else {
                    turn_label_colors(step.label, scheme).segment
                };
                div()
                    .h_full()
                    .flex_shrink(1.)
                    .w(px(STRIP_WIDTH * weight / weight_sum))
                    .bg(color)
            }));

        let header = h_flex()
            .gap_3()
            .items_center()
            .child(div().text_sm().font_semibold().child(title))
            .children(group.is_loading.then(|| {
                div()
                    .text_xs()
                    .font_family(theme.mono_font_family.clone())
                    .text_color(theme.muted_foreground)
                    .child(format_clock(now - started_at))
            }))
            .child(strip)
            .child(
                h_flex()
                    .ml_auto()
                    .gap_1()
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .child(summary)
                    .when(!files.is_empty() && additions + deletions > 0, |line| {
                        line.child(
                            div()
                                .text_color(theme.success)
                                .child(format!("+{additions}")),
                        )
                        .child(
                            div()
                                .text_color(theme.danger)
                                .child(format!("−{deletions}")),
                        )
                    }),
            );

        div()
            .id(("turn", row))
            .role(Role::Heading)
            .aria_label(format!(
                "Turn, {}, {}",
                if group.is_loading { "working" } else { "done" },
                plural(group.step_count, "step", "steps")
            ))
            .pt_2()
            .pb_2()
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
        let is_last_of_finished = position + 1 == group.steps.len() && !group.is_loading;
        let theme = cx.theme().clone();
        let colors = turn_label_colors(step.label, scheme(cx));
        let running: Hsla = rgb(RUNNING_COLOR).into();
        let (text_color, border_color) = if step.is_running {
            (running, running)
        } else {
            (colors.text, colors.border)
        };
        let label = label_name(step.label);
        let counts = counts_text(&step.counts);
        let time = step
            .ended_at
            .map_or("now".to_string(), |end| format_clock(end - run_started_at));
        let step_key = entry_key(&self.entries[step.id]);

        let header = h_flex()
            .gap_3()
            .items_center()
            .child(
                div()
                    .size(px(20.))
                    .flex_none()
                    .rounded_full()
                    .border_1()
                    .border_color(border_color)
                    .bg(theme.background)
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(px(10.))
                    .font_semibold()
                    .font_family(theme.mono_font_family.clone())
                    .text_color(text_color)
                    .child((position + 1).to_string()),
            )
            .child(
                h_flex()
                    .flex_1()
                    .min_w_0()
                    .gap_1p5()
                    .items_baseline()
                    .child(
                        div()
                            .text_size(px(10.))
                            .font_semibold()
                            .font_family(theme.mono_font_family.clone())
                            .text_color(text_color)
                            .child(label.to_uppercase()),
                    )
                    .when(!counts.is_empty(), |meta| {
                        meta.child(
                            div()
                                .text_sm()
                                .text_color(theme.muted_foreground)
                                .child(counts.clone()),
                        )
                    }),
            )
            .child(
                div()
                    .w(px(40.))
                    .text_right()
                    .text_xs()
                    .font_family(theme.mono_font_family.clone())
                    .text_color(theme.muted_foreground)
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
        let live = step.is_running.then(|| self.render_live_box(step, cx));

        let mut aria = format!("Step {}, {label}", position + 1);
        if !counts.is_empty() {
            aria.push_str(&format!(", {counts}"));
        }
        v_flex()
            .id(("step", step_key))
            .role(Role::ListItem)
            .aria_label(aria)
            .gap(px(3.))
            .py_1()
            .child(header)
            .child(
                v_flex()
                    .min_w_0()
                    .gap(px(3.))
                    .ml(px(34.))
                    .mr(px(52.))
                    .children(notes)
                    .children(edits)
                    .children(live),
            )
            .when(is_last_of_finished, |step| {
                step.child(div().mt_3().h(px(1.)).w_full().bg(theme.border))
            })
            .into_any_element()
    }

    /// An edit or write call: its file row toggles the diff below it.
    fn render_edit(&self, index: usize, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme().clone();
        let entry = &self.entries[index];
        let key = entry_key(entry);
        let path = grouping::edited_file_path(entry).unwrap_or("").to_string();
        let lines = diff::entry_lines(entry);
        let (additions, deletions) = diff::stats(&lines);
        let hidden = self.hidden_diffs.contains(&key);
        let file_row = h_flex()
            .id(("edit-file", key))
            .role(Role::Button)
            .aria_label(format!(
                "{} diff of {}",
                if hidden { "Show" } else { "Hide" },
                short_path(&path)
            ))
            .aria_expanded(!hidden)
            .cursor_pointer()
            .gap_2()
            .px_2()
            .py_1()
            .text_xs()
            .font_family(theme.mono_font_family.clone())
            .child(div().flex_1().min_w_0().truncate().child(short_path(&path)))
            .child(
                div()
                    .text_color(theme.success)
                    .child(format!("+{additions}")),
            )
            .child(
                div()
                    .text_color(theme.danger)
                    .child(format!("−{deletions}")),
            )
            .on_click(cx.listener(move |this, _, _, cx| this.toggle_diff(key, cx)));
        let (foreground, muted) = (theme.foreground, theme.muted_foreground);
        let border = theme.border;
        let mono = theme.mono_font_family.clone();
        let body = (!hidden).then(|| {
            let added = theme.success.opacity(0.15);
            let removed = theme.danger.opacity(0.15);
            v_flex()
                .id(("diff", key))
                .role(Role::Group)
                .aria_label(format!(
                    "Diff of {}, {additions} added, {deletions} removed",
                    short_path(&path)
                ))
                .border_t_1()
                .border_color(border)
                .py_1()
                .text_xs()
                .font_family(mono)
                .children(lines.into_iter().map(move |line| {
                    let (sign, bg, color) = match line.kind {
                        LineKind::Add => ("+", added, foreground),
                        LineKind::Remove => ("-", removed, foreground),
                        LineKind::Context => (" ", transparent_black(), foreground),
                        LineKind::Hunk => ("", transparent_black(), muted),
                    };
                    h_flex()
                        .px_2()
                        .bg(bg)
                        .text_color(color)
                        .child(div().w(px(12.)).flex_none().child(sign))
                        .child(div().flex_1().min_w_0().child(line.text))
                }))
        });
        v_flex()
            .rounded_md()
            .border_1()
            .border_color(theme.border)
            .overflow_hidden()
            .child(file_row)
            .children(body)
            .into_any_element()
    }

    /// The running step's last call: its command and the last lines of its output.
    fn render_live_box(&self, step: &TurnStep, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme().clone();
        let Some(&last) = step.items.last() else {
            return div().into_any_element();
        };
        let item = &self.entries[last]["item"];
        let detail = &item["detail"];
        let is_shell = detail["type"] == "shell";
        let command = if is_shell {
            format!("$ {}", detail["command"].as_str().unwrap_or(""))
        } else {
            item["name"].as_str().unwrap_or("").to_string()
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
            .mt_1p5()
            .px_2p5()
            .py_1p5()
            .rounded_lg()
            .border_1()
            .border_color(theme.border)
            .text_xs()
            .font_family(theme.mono_font_family.clone())
            .child(div().truncate().child(command))
            .children(shown.into_iter().map(|line| {
                div()
                    .truncate()
                    .text_color(theme.muted_foreground)
                    .child(line.to_string())
            }))
            .into_any_element()
    }

    fn render_permission(&self, row: usize, request: &Value, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme().clone();
        let id = request["id"].as_str().unwrap_or("").to_string();
        let title = request["title"]
            .as_str()
            .or_else(|| request["name"].as_str())
            .unwrap_or("Permission")
            .to_string();
        let description = request["description"].as_str().map(str::to_string);
        let input = &request["input"];
        let detail = ["command", "file_path", "filePath", "plan", "url"]
            .iter()
            .find_map(|key| input[*key].as_str())
            .map(str::to_string);
        let answered = self.answered.contains(&id);
        let allow_id = id.clone();
        let deny_id = id.clone();
        v_flex()
            .id(("permission", row))
            .role(Role::Group)
            .aria_label(format!("Permission request: {title}"))
            .gap_2()
            .p_3()
            .rounded_lg()
            .border_1()
            .border_color(theme.warning)
            .child(div().text_sm().font_semibold().child(title))
            .children(description.map(|text| {
                div()
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .child(text)
            }))
            .children(detail.map(|text| {
                div()
                    .text_xs()
                    .font_family(theme.mono_font_family.clone())
                    .p_2()
                    .rounded_md()
                    .bg(theme.muted)
                    .child(text)
            }))
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        Button::new(("allow", row))
                            .label("Allow")
                            .primary()
                            .small()
                            .disabled(answered)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.respond(allow_id.clone(), true, cx)
                            })),
                    )
                    .child(
                        Button::new(("deny", row))
                            .label("Deny")
                            .outline()
                            .small()
                            .disabled(answered)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.respond(deny_id.clone(), false, cx)
                            })),
                    )
                    .when(answered, |buttons| {
                        buttons.child(
                            div()
                                .text_xs()
                                .text_color(theme.muted_foreground)
                                .child("Waiting for the agent..."),
                        )
                    }),
            )
            .into_any_element()
    }
}

impl Render for TimelineView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let base = div()
            .id("timeline")
            .role(Role::List)
            .aria_label("Timeline")
            .size_full()
            .bg(theme.background)
            .text_color(theme.foreground);
        if self.rows.is_empty() {
            let text = if self.loaded {
                "No messages yet"
            } else {
                "Loading timeline..."
            };
            return base.child(
                div()
                    .id("timeline-empty")
                    .role(Role::Status)
                    .aria_label(text)
                    .size_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(theme.muted_foreground)
                    .child(text),
            );
        }
        base.child(
            list(self.list.clone(), cx.processor(Self::render_row))
                .size_full()
                .py_2(),
        )
    }
}

/// The client's entries, reusing the copies of entries that did not change.
fn reuse_entries(old: Vec<Value>, current: &[Value]) -> Vec<Value> {
    let identity = |entry: &Value| {
        (
            entry_key(entry),
            entry["seqEnd"].as_u64(),
            entry["timestamp"].as_str().map(str::to_string),
        )
    };
    let mut kept: HashMap<_, Value> = old.into_iter().map(|e| (identity(&e), e)).collect();
    current
        .iter()
        .map(|entry| {
            kept.remove(&identity(entry))
                .unwrap_or_else(|| entry.clone())
        })
        .collect()
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
