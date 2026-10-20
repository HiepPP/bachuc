//! The plugin rules of the timeline, as pure functions: transformer projection
//! (`packages/app/src/plugins/timeline/model.ts` and `projection.ts`), renderer lookup
//! (`plugins/timeline/view.tsx`), the step rule (`turn/view.tsx`), the assistant selection
//! payload (`selection-actions.web.tsx`), and the passage reveal match
//! (`agent-stream/passage-reveal`).
//!
//! Projection runs at the entry level after grouping, so grouping still reads the source
//! entries (ADR-0039). `Projector` keeps a result per entry key, so a flush transforms only the
//! changed entries and the entries whose phase changed.

use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::ops::Range;
use std::rc::Rc;

use bachuc_plugin_host::{
    AssistantSelection, Contributed, ContributionIndex, PluginTimelineItem, SourcePlacement,
    SourcePosition, TimelinePhase, TimelineRenderer, TimelineTransformInput,
    TimelineTransformResult, TimelineTransformer,
};
use gpui_kit::{Bounds, Pixels, Point, Size, point, px};
use serde_json::Value;

use crate::grouping::{entry_key, item_type};

/// `TOOLBAR_GAP`: the space between the selection toolbar and the text under it.
pub const TOOLBAR_GAP: f32 = 8.;

/// One plugin item that a transformer made from an entry, with its timeline-wide id.
#[derive(Clone, Debug, PartialEq)]
pub struct ProjectedItem {
    /// `<plugin_id>/<item.id>`, else `<plugin_id>/<entry key>/<index>`.
    pub id: String,
    pub item: PluginTimelineItem,
    /// The hash of `item.data`, so a view is built again only when its data changes.
    pub data_hash: u64,
}

/// What a transformer made of one entry.
#[derive(Clone, Debug, PartialEq)]
pub struct Projection {
    pub plugin_id: &'static str,
    pub items: Vec<ProjectedItem>,
    /// `None`: the items replace the entry.
    pub source: Option<SourcePlacement>,
    /// Changes whenever what the projection draws changes.
    pub hash: u64,
}

/// One drawn part of a projected entry, in order.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Part<'a> {
    Source,
    Item(&'a ProjectedItem),
}

impl Projection {
    fn new(plugin_id: &'static str, entry_key: u64, result: TimelineTransformResult) -> Self {
        let items: Vec<ProjectedItem> = result
            .items
            .into_iter()
            .enumerate()
            .map(|(position, item)| ProjectedItem {
                id: item_id(plugin_id, item.id.as_deref(), entry_key, position),
                data_hash: hash_value(&item.data),
                item,
            })
            .collect();
        let mut hasher = DefaultHasher::new();
        plugin_id.hash(&mut hasher);
        for item in &items {
            item.id.hash(&mut hasher);
            item.item.kind.hash(&mut hasher);
            item.item.version.hash(&mut hasher);
            item.data_hash.hash(&mut hasher);
        }
        if let Some(source) = &result.source {
            matches!(source.position, SourcePosition::First).hash(&mut hasher);
            source.text.hash(&mut hasher);
        }
        Projection {
            plugin_id,
            items,
            source: result.source,
            hash: hasher.finish(),
        }
    }

    /// No items and no kept source: the entry draws nothing.
    pub fn is_hidden(&self) -> bool {
        self.items.is_empty() && self.source.is_none()
    }

    /// The source and the items in the order they draw.
    pub fn parts(&self) -> Vec<Part<'_>> {
        let items = self.items.iter().map(Part::Item);
        match self.source.as_ref().map(|source| source.position) {
            None => items.collect(),
            Some(SourcePosition::First) => std::iter::once(Part::Source).chain(items).collect(),
            Some(SourcePosition::Last) => items.chain(std::iter::once(Part::Source)).collect(),
        }
    }
}

/// The parts of a projection that draw: the source, and each item whose plugin registered a
/// renderer. An item with no renderer draws nothing and takes no space; an empty result means
/// the entry shows nothing.
pub fn shown_parts<'a>(
    projection: &'a Projection,
    has_renderer: impl Fn(&ProjectedItem) -> bool,
) -> Vec<Part<'a>> {
    projection
        .parts()
        .into_iter()
        .filter(|part| match part {
            Part::Source => true,
            Part::Item(item) => has_renderer(item),
        })
        .collect()
}

/// `<plugin_id>/<item.id>`, else `<plugin_id>/<entry key>/<index>`.
pub fn item_id(plugin_id: &str, id: Option<&str>, entry_key: u64, index: usize) -> String {
    match id {
        Some(id) => format!("{plugin_id}/{id}"),
        None => format!("{plugin_id}/{entry_key}/{index}"),
    }
}

/// `^[a-z][a-z0-9-]*$`.
pub fn is_valid_kind(kind: &str) -> bool {
    let mut chars = kind.chars();
    chars.next().is_some_and(|first| first.is_ascii_lowercase())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// The checks of `parseTransformResult` that a Rust result can fail: the kind, a version of 0,
/// and an empty id.
pub fn check_result(result: &TimelineTransformResult) -> Result<(), String> {
    for item in &result.items {
        if !is_valid_kind(&item.kind) {
            return Err(format!(
                "invalid transformed timeline item kind: {}",
                item.kind
            ));
        }
        if item.version == 0 {
            return Err(format!(
                "invalid transformed timeline item version: {}",
                item.version
            ));
        }
        if item.id.as_deref() == Some("") {
            return Err("invalid transformed timeline item id: empty".to_string());
        }
    }
    Ok(())
}

/// Whether an entry streams while its turn runs, when it is the last of these.
fn is_stream_item(entry: &Value) -> bool {
    matches!(item_type(entry), "assistant_message" | "reasoning")
}

/// `Streaming` for a running tool call and for the entry that streams, else `Complete`.
pub fn phase_of(entry: &Value, is_streaming_entry: bool) -> TimelinePhase {
    let running_call = item_type(entry) == "tool_call" && entry["item"]["status"] == "running";
    if running_call || is_streaming_entry {
        TimelinePhase::Streaming
    } else {
        TimelinePhase::Complete
    }
}

/// Runs the transformers of the entry's item type in index order; the first valid `Some`
/// wins. A result that fails `check_result` logs a warning and counts as `None`.
pub fn transform_entry(
    transformers: &[Contributed<TimelineTransformer>],
    agent_id: &str,
    labels: &BTreeMap<String, String>,
    entry: &Value,
    phase: TimelinePhase,
) -> Option<Projection> {
    let input = TimelineTransformInput {
        agent_id,
        labels,
        item: &entry["item"],
        phase,
    };
    for transformer in transformers {
        let Some(result) = (transformer.value.transform)(&input) else {
            continue;
        };
        if let Err(reason) = check_result(&result) {
            log::warn!(
                "timeline transformer failed: {}/{}: {reason}",
                transformer.plugin_id,
                transformer.value.id
            );
            continue;
        }
        return Some(Projection::new(
            transformer.plugin_id,
            entry_key(entry),
            result,
        ));
    }
    None
}

/// The text an entry shows: a kept source's `text` replaces it only for user and assistant
/// messages.
pub fn shown_text<'a>(entry: &'a Value, projection: Option<&'a Projection>) -> &'a str {
    let replaced = matches!(item_type(entry), "user_message" | "assistant_message")
        .then(|| projection?.source.as_ref()?.text.as_deref())
        .flatten();
    replaced.unwrap_or_else(|| entry["item"]["text"].as_str().unwrap_or(""))
}

/// One part of a step item's drawing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum StepPart<'a> {
    /// The call's tool call row. Edit diffs are the edit-diffs plugin's items.
    ToolCall,
    Item(&'a ProjectedItem),
}

/// What a step draws for one of its items, as `turn/view.tsx`: plugin items always, the tool
/// call row for a kept edit call, nothing for another kept call (it stays a count and shows in
/// a picked step's call list).
pub fn step_parts(projection: Option<&Projection>, is_edit: bool) -> Vec<StepPart<'_>> {
    let source = || is_edit.then_some(StepPart::ToolCall);
    match projection {
        None => source().into_iter().collect(),
        Some(projection) => projection
            .parts()
            .into_iter()
            .filter_map(|part| match part {
                Part::Source => source(),
                Part::Item(item) => Some(StepPart::Item(item)),
            })
            .collect(),
    }
}

/// Whether a picked step still lists a call: not when plugin items replaced it.
pub fn call_listed(projection: Option<&Projection>) -> bool {
    projection.is_none_or(|projection| projection.source.is_some())
}

/// The renderer of an item, only when the plugin that made the item registered it.
pub fn renderer_for_item<'a>(
    index: &'a ContributionIndex,
    plugin_id: &str,
    kind: &str,
    version: u32,
) -> Option<&'a Contributed<TimelineRenderer>> {
    index
        .renderer_for(kind, version)
        .filter(|renderer| renderer.plugin_id == plugin_id)
}

/// A daemon plugin item (`type: "plugin"`): its plugin id, its row id
/// (`<pluginId>/<id>`), and the item.
pub fn daemon_item(item: &Value) -> Option<(String, String, PluginTimelineItem)> {
    if item["type"] != "plugin" {
        return None;
    }
    let plugin_id = item["pluginId"].as_str()?.to_string();
    let id = item["id"].as_str()?.to_string();
    let version = u32::try_from(item["version"].as_u64()?).ok()?;
    let row_id = format!("{plugin_id}/{id}");
    Some((
        plugin_id,
        row_id,
        PluginTimelineItem {
            id: Some(id),
            kind: item["kind"].as_str()?.to_string(),
            version,
            data: item["data"].clone(),
        },
    ))
}

pub fn hash_value(value: &Value) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.to_string().hash(&mut hasher);
    hasher.finish()
}

pub fn labels_hash(labels: &BTreeMap<String, String>) -> u64 {
    let mut hasher = DefaultHasher::new();
    labels.hash(&mut hasher);
    hasher.finish()
}

/// What the projector reads besides the entries.
pub struct ProjectInput<'a> {
    /// `None` without a plugin host: nothing is projected.
    pub index: Option<&'a ContributionIndex>,
    pub revision: u64,
    pub agent_id: &'a str,
    pub labels: &'a BTreeMap<String, String>,
    pub turn_active: bool,
}

/// What one `Projector::sync` did.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct ProjectOutcome {
    /// Every entry was projected again: the revision or the labels changed.
    pub full: bool,
    /// The entries whose projection changed, in order.
    pub changed: Vec<usize>,
}

struct Cached {
    seq_end: Option<u64>,
    phase: TimelinePhase,
    result: Option<Rc<Projection>>,
}

/// The projection of each entry, cached by entry key with the entry's `seqEnd` and phase.
#[derive(Default)]
pub struct Projector {
    cache: HashMap<u64, Cached>,
    projections: Vec<Option<Rc<Projection>>>,
    revision: Option<u64>,
    labels: Option<u64>,
    /// The last assistant message or thought, which streams while the turn runs.
    last_stream_item: Option<usize>,
    streaming: Option<usize>,
}

impl Projector {
    /// Projects the entries that `changed` lists and the entries whose phase changed, or every
    /// entry when the revision or the labels changed. `changed` and `shifted` come from the
    /// grouper's sync of the same entries.
    pub fn sync(
        &mut self,
        input: &ProjectInput<'_>,
        entries: &[Value],
        changed: &[usize],
        shifted: bool,
    ) -> ProjectOutcome {
        let labels = labels_hash(input.labels);
        let full = self.revision != Some(input.revision) || self.labels != Some(labels);
        self.revision = Some(input.revision);
        self.labels = Some(labels);
        self.projections.resize(entries.len(), None);
        if full {
            self.cache.clear();
        } else if shifted {
            let live: HashSet<u64> = entries.iter().map(entry_key).collect();
            self.cache.retain(|key, _| live.contains(key));
        }
        if full
            || shifted
            || self
                .last_stream_item
                .is_some_and(|last| last >= entries.len())
        {
            self.last_stream_item = entries.iter().rposition(is_stream_item);
        } else {
            for &index in changed {
                if is_stream_item(&entries[index]) {
                    self.last_stream_item = self.last_stream_item.max(Some(index));
                }
            }
        }
        let old_streaming = self.streaming;
        self.streaming = self.last_stream_item.filter(|_| input.turn_active);

        let mut outcome = ProjectOutcome {
            full,
            changed: Vec::new(),
        };
        let mut project = |this: &mut Self, index: usize| {
            let result = this.project(input, &entries[index], Some(index) == this.streaming);
            let slot = &mut this.projections[index];
            let same = match (&*slot, &result) {
                (None, None) => true,
                (Some(a), Some(b)) => Rc::ptr_eq(a, b),
                _ => false,
            };
            if !same {
                *slot = result;
                outcome.changed.push(index);
            }
        };
        if full {
            for index in 0..entries.len() {
                project(self, index);
            }
        } else {
            let mut indices: Vec<usize> = changed.to_vec();
            indices.extend(old_streaming.filter(|&index| index < entries.len()));
            indices.extend(self.streaming);
            indices.sort_unstable();
            indices.dedup();
            for index in indices {
                project(self, index);
            }
        }
        outcome
    }

    fn project(
        &mut self,
        input: &ProjectInput<'_>,
        entry: &Value,
        is_streaming_entry: bool,
    ) -> Option<Rc<Projection>> {
        let index = input.index?;
        let kind = item_type(entry);
        // A daemon plugin item is never a transformer source.
        if kind == "plugin" {
            return None;
        }
        let transformers = index.transformers_for(kind);
        if transformers.is_empty() {
            return None;
        }
        let phase = phase_of(entry, is_streaming_entry);
        let key = entry_key(entry);
        let seq_end = entry["seqEnd"].as_u64();
        if let Some(cached) = self.cache.get(&key)
            && cached.seq_end == seq_end
            && cached.phase == phase
        {
            return cached.result.clone();
        }
        let result =
            transform_entry(transformers, input.agent_id, input.labels, entry, phase).map(Rc::new);
        self.cache.insert(
            key,
            Cached {
                seq_end,
                phase,
                result: result.clone(),
            },
        );
        result
    }

    /// The projection of the entry at `index`, if a transformer made one.
    pub fn get(&self, index: usize) -> Option<&Rc<Projection>> {
        self.projections.get(index)?.as_ref()
    }

    /// Whether the entry at `index` draws nothing.
    pub fn is_hidden(&self, index: usize) -> bool {
        self.get(index)
            .is_some_and(|projection| projection.is_hidden())
    }

    /// Every projected item id, to drop the views of entries that went.
    pub fn item_ids(&self) -> HashSet<&str> {
        self.projections
            .iter()
            .flatten()
            .flat_map(|projection| projection.items.iter().map(|item| item.id.as_str()))
            .collect()
    }
}

/// The text a selection action gets: the Markdown source of the selected range when the view
/// maps it, else the selected text.
pub fn selection_text(
    source: &str,
    source_range: Option<Range<usize>>,
    selected: String,
) -> String {
    source_range
        .and_then(|range| source.get(range))
        .map_or(selected, str::to_string)
}

/// The payload of a selection action, or `None` for an empty selection.
pub fn assistant_selection(
    agent_id: &str,
    workspace_id: Option<String>,
    message_id: String,
    source: &str,
    source_range: Option<Range<usize>>,
    selected: String,
) -> Option<AssistantSelection> {
    if selected.is_empty() {
        return None;
    }
    Some(AssistantSelection {
        workspace_id,
        agent_id: agent_id.to_string(),
        message_id: Some(message_id),
        text: selection_text(source, source_range, selected),
    })
}

/// What the selection toolbar sits over: the box from the caret line at the first selected
/// character to the one at the last, else the whole text view when no selection was painted.
pub fn selection_anchor(
    view: Bounds<Pixels>,
    edges: Option<(Bounds<Pixels>, Bounds<Pixels>)>,
) -> Bounds<Pixels> {
    let Some((start, end)) = edges else {
        return view;
    };
    Bounds::from_corners(
        point(start.left().min(end.left()), start.top().min(end.top())),
        point(
            start.right().max(end.right()),
            start.bottom().max(end.bottom()),
        ),
    )
}

/// The selection toolbar's place in the timeline, relative to the timeline's origin: centered
/// over the top of the selected text, `gap` above it (`TOOLBAR_GAP` with the zoom), clamped
/// inside the timeline. `None` once the text scrolled out of the timeline.
pub fn toolbar_origin(
    text: Bounds<Pixels>,
    toolbar: Size<Pixels>,
    area: Bounds<Pixels>,
    gap: Pixels,
) -> Option<Point<Pixels>> {
    if text.bottom() < area.top() || text.top() > area.bottom() {
        return None;
    }
    let center = text.center().x - area.left();
    let max_left = (area.size.width - toolbar.width).max(px(0.));
    let left = (center - toolbar.width / 2.).clamp(px(0.), max_left);
    let top = text.top().max(area.top()) - area.top() - gap - toolbar.height;
    Some(point(left, top.max(px(0.))))
}

/// Where a passage text is in a message: the segment and the byte range in its source, only
/// when the text occurs exactly once in the whole message.
pub fn unique_match(sources: &[&str], text: &str) -> Option<(usize, Range<usize>)> {
    if text.is_empty() {
        return None;
    }
    let mut found = None;
    for (segment, source) in sources.iter().enumerate() {
        for (start, _) in source.match_indices(text) {
            if found.is_some() {
                return None;
            }
            found = Some((segment, start..start + text.len()));
        }
    }
    found
}

/// What a passage reveal does now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RevealPlan {
    /// The history has not loaded: wait, up to `REVEAL_WAIT`.
    Wait,
    /// The message is not in the loaded chat: tell the user.
    NotLoaded,
    Reveal,
}

pub fn reveal_plan(loaded: bool, found: bool) -> RevealPlan {
    match (loaded, found) {
        (_, true) => RevealPlan::Reveal,
        (false, false) => RevealPlan::Wait,
        (true, false) => RevealPlan::NotLoaded,
    }
}
