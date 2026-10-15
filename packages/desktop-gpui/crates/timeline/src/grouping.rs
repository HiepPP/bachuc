//! Turn grouping over projected timeline entries: a port of `prepareTurnHistory` and
//! `groupLiveTurns` in `packages/app/src/tool-calls/detail-level/grouping.ts` and of
//! `buildTurnGroup` in `packages/app/src/tool-calls/detail-level/turn/model.ts`.
//!
//! The app splits its stream into a committed tail and a live head; the client keeps one list,
//! so one pass gives what the app's two passes give together. Rows point into the entry slice
//! by index.

use bachuc_theme::TurnLabel;
use serde_json::Value;

/// One timeline row: an entry on its own, or a turn's step list in place of its run.
#[derive(Clone, Debug, PartialEq)]
pub enum Row {
    Item(usize),
    Turn(TurnGroup),
}

/// One agent turn: its notes, thoughts, and tool calls (entry indices).
#[derive(Clone, Debug, PartialEq)]
pub struct TurnRun {
    pub items: Vec<usize>,
    /// The last tool call.
    pub latest: usize,
    /// The user message before the run, or the boundary that split its turn, in Unix ms.
    pub started_at: i64,
    pub is_sealed: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StepCounts {
    pub edited_file_count: usize,
    pub command_count: usize,
    pub read_file_count: usize,
    pub search_count: usize,
    pub other_tool_count: usize,
    pub paseo_call_count: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TurnStep {
    /// The entry that opened the step.
    pub id: usize,
    pub label: TurnLabel,
    /// The assistant entries of the note that opens the step; empty when it has none.
    pub note: Vec<usize>,
    pub items: Vec<usize>,
    /// The edit and write calls of `items`, in order.
    pub edits: Vec<usize>,
    pub counts: StepCounts,
    pub started_at: i64,
    /// None while the step runs.
    pub ended_at: Option<i64>,
    pub duration_ms: Option<i64>,
    pub is_running: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TurnGroup {
    pub run: TurnRun,
    pub steps: Vec<TurnStep>,
    pub step_count: usize,
    pub changed_files: Vec<String>,
    /// Index into `steps`.
    pub running_step: Option<usize>,
    pub is_loading: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    User,
    Assistant,
    Thought,
    Work,
    Other,
}

pub fn item_type(entry: &Value) -> &str {
    entry["item"]["type"].as_str().unwrap_or("")
}

fn kind(entry: &Value) -> Kind {
    match item_type(entry) {
        "user_message" => Kind::User,
        "assistant_message" => Kind::Assistant,
        "reasoning" => Kind::Thought,
        "tool_call" if is_groupable_call(entry) => Kind::Work,
        _ => Kind::Other,
    }
}

/// A plan or a speak call keeps its own row.
fn is_groupable_call(entry: &Value) -> bool {
    let item = &entry["item"];
    item["detail"]["type"] != "plan"
        && item["name"].as_str().unwrap_or("").trim().to_lowercase() != "speak"
}

fn is_run_item(entry: &Value) -> bool {
    matches!(kind(entry), Kind::Assistant | Kind::Thought | Kind::Work)
}

fn is_running_work(entry: &Value) -> bool {
    kind(entry) == Kind::Work
        && matches!(
            entry["item"]["status"].as_str(),
            Some("running" | "executing")
        )
}

/// The entry time in Unix ms. A tool call takes the time of its latest update.
pub fn timestamp_ms(entry: &Value) -> i64 {
    entry["timestamp"]
        .as_str()
        .and_then(|text| chrono::DateTime::parse_from_rfc3339(text).ok())
        .map_or(0, |time| time.timestamp_millis())
}

/// The file an edit or write call changes.
pub fn edited_file_path(entry: &Value) -> Option<&str> {
    let detail = &entry["item"]["detail"];
    match detail["type"].as_str() {
        Some("edit" | "write") => detail["filePath"].as_str(),
        _ => None,
    }
}

/// A stable identity for an entry across syncs.
pub fn entry_key(entry: &Value) -> u64 {
    entry["seqStart"].as_u64().unwrap_or(0)
}

/// The message an assistant entry belongs to: its `messageId`, or the entry itself.
pub fn message_key(entry: &Value) -> String {
    match entry["item"]["messageId"].as_str() {
        Some(id) => id.to_string(),
        None => format!("seq:{}", entry_key(entry)),
    }
}

/// Every assistant message after the run's last tool call, with thoughts between them, is the
/// answer (ADR-0018). Returns `items.len()` when the run has no answer.
fn find_answer_start(entries: &[Value], items: &[usize]) -> usize {
    let mut start = items.len();
    for (position, &index) in items.iter().enumerate().rev() {
        match kind(&entries[index]) {
            Kind::Thought => continue,
            Kind::Assistant => start = position,
            _ => break,
        }
    }
    start
}

/// Whether the turn ends at each boundary: no note or tool work follows it before the next
/// user message, and that message came or the turn is idle.
fn find_turn_ends(entries: &[Value], is_turn_active: bool) -> Vec<bool> {
    let mut ends = vec![false; entries.len()];
    let mut is_ended = !is_turn_active;
    for (index, entry) in entries.iter().enumerate().rev() {
        let kind = kind(entry);
        if kind == Kind::User {
            is_ended = true;
        }
        ends[index] = is_ended;
        if matches!(kind, Kind::Assistant | Kind::Work) {
            is_ended = false;
        }
    }
    ends
}

/// The run after a boundary starts there, unless the boundary ended a turn with no user message.
fn start_after(entry: &Value, ends_turn: bool) -> Option<i64> {
    (kind(entry) == Kind::User || !ends_turn).then(|| timestamp_ms(entry))
}

/// A turn run needs a tool call; notes and thoughts alone keep their own rows.
fn create_turn_run(
    entries: &[Value],
    items: &[usize],
    started_at: Option<i64>,
    is_sealed: bool,
) -> Option<TurnRun> {
    let latest = items
        .iter()
        .rev()
        .copied()
        .find(|&index| kind(&entries[index]) == Kind::Work)?;
    let first = *items.first()?;
    Some(TurnRun {
        items: items.to_vec(),
        latest,
        started_at: started_at.unwrap_or_else(|| timestamp_ms(&entries[first])),
        is_sealed,
    })
}

fn append_run(
    rows: &mut Vec<Row>,
    entries: &[Value],
    items: &[usize],
    started_at: Option<i64>,
    ends_turn: bool,
    is_sealed: bool,
) {
    let answer_start = if ends_turn {
        find_answer_start(entries, items)
    } else {
        items.len()
    };
    match create_turn_run(entries, &items[..answer_start], started_at, is_sealed) {
        Some(run) => {
            rows.push(Row::Turn(build_turn_group(entries, run)));
            rows.extend(items[answer_start..].iter().map(|&index| Row::Item(index)));
        }
        None => rows.extend(items.iter().map(|&index| Row::Item(index))),
    }
}

/// Groups entries into rows. Notes, thoughts, and tool calls of one turn form one run. A
/// boundary where the turn goes on splits the run and keeps its last note inside.
pub fn group_turns(entries: &[Value], is_turn_active: bool) -> Vec<Row> {
    let ends = find_turn_ends(entries, is_turn_active);
    let mut rows = Vec::new();
    let mut pending: Vec<usize> = Vec::new();
    let mut started_at = None;
    for (index, entry) in entries.iter().enumerate() {
        if is_run_item(entry) {
            pending.push(index);
            continue;
        }
        append_run(&mut rows, entries, &pending, started_at, ends[index], true);
        pending.clear();
        started_at = start_after(entry, ends[index]);
        rows.push(Row::Item(index));
    }
    // A call that still shows as running keeps the run open, but an idle turn has its answer.
    let trailing_is_active = is_turn_active
        || pending
            .iter()
            .any(|&index| is_running_work(&entries[index]));
    append_run(
        &mut rows,
        entries,
        &pending,
        started_at,
        !is_turn_active,
        !trailing_is_active,
    );
    rows
}

struct StepDraft {
    id: usize,
    note: Vec<usize>,
    items: Vec<usize>,
    edits: Vec<usize>,
    last_at: i64,
}

fn draft_for(
    drafts: &mut Vec<StepDraft>,
    index: usize,
    at: i64,
    starts_step: bool,
) -> &mut StepDraft {
    if starts_step || drafts.is_empty() {
        drafts.push(StepDraft {
            id: index,
            note: Vec::new(),
            items: Vec::new(),
            edits: Vec::new(),
            last_at: at,
        });
    }
    let draft = drafts.last_mut().expect("a draft");
    draft.last_at = at;
    draft
}

/// A step is one note plus the tool calls after it, up to the next note (ADR-0021). Tool calls
/// before the first note form a step with no note. Edits in a step with no note end at the next
/// other tool call, so they form their own Edit step. Thoughts stay hidden.
fn split_steps(entries: &[Value], run: &TurnRun) -> Vec<StepDraft> {
    let mut drafts: Vec<StepDraft> = Vec::new();
    for &index in &run.items {
        let entry = &entries[index];
        let at = timestamp_ms(entry);
        match kind(entry) {
            Kind::Assistant => {
                // Only the blocks of the same note join a step without work yet.
                let is_same_note = drafts.last().is_some_and(|current| {
                    current.items.is_empty()
                        && current
                            .note
                            .last()
                            .is_some_and(|&last| message_key(&entries[last]) == message_key(entry))
                });
                draft_for(&mut drafts, index, at, !is_same_note)
                    .note
                    .push(index);
            }
            Kind::Work => {
                let is_edit = edited_file_path(entry).is_some();
                let ends_edit_only_step = !is_edit
                    && drafts.last().is_some_and(|current| {
                        current.note.is_empty() && !current.edits.is_empty()
                    });
                let starts_step = drafts.is_empty() || ends_edit_only_step;
                let draft = draft_for(&mut drafts, index, at, starts_step);
                draft.items.push(index);
                if is_edit {
                    draft.edits.push(index);
                }
            }
            _ => {}
        }
    }
    drafts
}

fn label_step(index: usize, edits: Option<(usize, usize)>) -> TurnLabel {
    match edits {
        Some((first, last)) if index >= first => {
            if index > last {
                TurnLabel::Verify
            } else {
                TurnLabel::Edit
            }
        }
        _ => TurnLabel::Explore,
    }
}

fn is_paseo_tool_name(name: &str) -> bool {
    let normalized = name.trim().to_lowercase();
    let is_speak = normalized
        .split(|c: char| !c.is_ascii_alphanumeric())
        .rfind(|token| !token.is_empty())
        == Some("speak");
    if is_speak {
        return false;
    }
    if normalized.contains("__") {
        let segments: Vec<&str> = normalized.split("__").filter(|s| !s.is_empty()).collect();
        return segments.len() >= 3
            && segments[0] == "mcp"
            && (segments[1] == "paseo" || segments[1].starts_with("paseo_"));
    }
    if normalized.contains('.') {
        let first = normalized.split('.').next().unwrap_or("");
        return first == "paseo" || first.starts_with("paseo_");
    }
    false
}

fn is_search_name(name: &str) -> bool {
    ["web_search", "llm_context"].iter().any(|suffix| {
        name.strip_suffix(suffix)
            .is_some_and(|rest| rest.is_empty() || rest.ends_with(['_', '.', ':', '/']))
    })
}

/// The tool counts of `buildOverviewGroup` in `overview/model.ts`.
pub fn count_calls(entries: &[Value], calls: &[usize]) -> StepCounts {
    let mut edited: Vec<&str> = Vec::new();
    let mut read: Vec<&str> = Vec::new();
    let mut counts = StepCounts::default();
    for &index in calls {
        let item = &entries[index]["item"];
        let name = item["name"].as_str().unwrap_or("");
        let normalized = name.trim().to_lowercase();
        let detail = &item["detail"];
        let path = detail["filePath"].as_str().unwrap_or("");
        if is_paseo_tool_name(name) || normalized.starts_with("paseo_") {
            counts.paseo_call_count += 1;
        } else if matches!(detail["type"].as_str(), Some("edit" | "write")) {
            if !edited.contains(&path) {
                edited.push(path);
            }
        } else if detail["type"] == "shell" {
            counts.command_count += 1;
        } else if detail["type"] == "read" {
            if !read.contains(&path) {
                read.push(path);
            }
        } else if detail["type"] == "search" || is_search_name(&normalized) {
            counts.search_count += 1;
        } else {
            counts.other_tool_count += 1;
        }
    }
    counts.edited_file_count = edited.len();
    counts.read_file_count = read.len();
    counts
}

pub fn build_turn_group(entries: &[Value], run: TurnRun) -> TurnGroup {
    let drafts = split_steps(entries, &run);
    let first_edit = drafts.iter().position(|draft| !draft.edits.is_empty());
    let last_edit = drafts.iter().rposition(|draft| !draft.edits.is_empty());
    let edit_span = first_edit.zip(last_edit);
    let mut previous_end = run.started_at;
    let count = drafts.len();

    let steps: Vec<TurnStep> = drafts
        .into_iter()
        .enumerate()
        .map(|(index, draft)| {
            // A step starts at its note; a step with no note starts where the one before ended.
            let started_at = draft
                .note
                .first()
                .map_or(previous_end, |&note| timestamp_ms(&entries[note]));
            let is_running = !run.is_sealed && index + 1 == count;
            let ended_at = (!is_running).then_some(draft.last_at);
            previous_end = ended_at.unwrap_or(started_at);
            TurnStep {
                id: draft.id,
                label: label_step(index, edit_span),
                counts: count_calls(entries, &draft.items),
                note: draft.note,
                items: draft.items,
                edits: draft.edits,
                started_at,
                ended_at,
                duration_ms: ended_at.map(|end| (end - started_at).max(0)),
                is_running,
            }
        })
        .collect();

    let mut changed_files: Vec<String> = Vec::new();
    for step in &steps {
        for &edit in &step.edits {
            if let Some(path) = edited_file_path(&entries[edit])
                && !changed_files.iter().any(|known| known == path)
            {
                changed_files.push(path.to_string());
            }
        }
    }

    TurnGroup {
        running_step: steps.iter().position(|step| step.is_running),
        is_loading: !run.is_sealed,
        step_count: steps.len(),
        changed_files,
        steps,
        run,
    }
}

/// The host row of each step note, by message key: a note has no row of its own, so find and
/// scrolling reach it through its turn. Returns (message key, first entry of the turn).
pub fn turn_note_hosts(entries: &[Value], rows: &[Row]) -> Vec<(String, usize)> {
    let mut hosts: Vec<(String, usize)> = Vec::new();
    for row in rows {
        let Row::Turn(group) = row else { continue };
        let host = group.run.items[0];
        for step in &group.steps {
            for &note in &step.note {
                let key = message_key(&entries[note]);
                if !hosts.iter().any(|(known, _)| *known == key) {
                    hosts.push((key, host));
                }
            }
        }
    }
    hosts
}
