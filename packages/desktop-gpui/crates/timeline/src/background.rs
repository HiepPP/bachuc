//! Background processes: the placement rule of `background-process-placement.ts` and the
//! result wording of `background-process-card.tsx`, as pure code. The card draws in
//! `background_view.rs`.
//!
//! A running process shows under the newest turn's step list, so it stays in view while the
//! user keeps talking. A finished process goes back to the turn whose tool call started it. A
//! finished process whose turn is not loaded is not shown.

use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use bachuc_client::BackgroundProcess;

/// The processes one turn hosts, and the running ones it started that now show later.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TurnSlot {
    pub processes: Vec<BackgroundProcess>,
    pub moved_count: usize,
}

impl TurnSlot {
    /// Whether the turn draws a card: a process of its own, or a moved note.
    pub fn is_shown(&self) -> bool {
        !self.processes.is_empty() || self.moved_count > 0
    }
}

/// Where each process shows: per turn key, and at the stream bottom.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Placement {
    pub by_turn: HashMap<u64, TurnSlot>,
    pub bottom: Vec<BackgroundProcess>,
}

/// One row the placement reads, in stream order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlacementRow {
    /// A user message resets the newest turn.
    UserMessage,
    /// A turn and the provider call ids of its run items.
    Turn { key: u64, call_ids: Vec<String> },
}

/// `placeBackgroundProcesses`: a running process shows in the newest turn, else at the bottom;
/// a finished process shows in the turn that started it, and nowhere when that turn is not
/// loaded. A running process shown in a later turn adds one to its start turn's `moved_count`.
pub fn place(rows: &[PlacementRow], processes: &[BackgroundProcess]) -> Placement {
    let mut placement = Placement::default();
    if processes.is_empty() {
        return placement;
    }
    let mut turn_by_call: HashMap<&str, u64> = HashMap::new();
    let mut newest: Option<u64> = None;
    for row in rows {
        match row {
            PlacementRow::UserMessage => newest = None,
            PlacementRow::Turn { key, call_ids } => {
                newest = Some(*key);
                for id in call_ids {
                    turn_by_call.insert(id.as_str(), *key);
                }
            }
        }
    }
    for process in processes {
        let origin = process
            .tool_call_id
            .as_deref()
            .and_then(|id| turn_by_call.get(id).copied());
        if process.status != "running" {
            if let Some(origin) = origin {
                slot(&mut placement.by_turn, origin)
                    .processes
                    .push(process.clone());
            }
            continue;
        }
        match newest {
            Some(newest) => slot(&mut placement.by_turn, newest)
                .processes
                .push(process.clone()),
            None => placement.bottom.push(process.clone()),
        }
        if let Some(origin) = origin
            && Some(origin) != newest
        {
            slot(&mut placement.by_turn, origin).moved_count += 1;
        }
    }
    placement
}

fn slot(slots: &mut HashMap<u64, TurnSlot>, key: u64) -> &mut TurnSlot {
    slots.entry(key).or_default()
}

/// The tone of a finished row's dot and result text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResultTone {
    Success,
    Danger,
    Muted,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FinishedResult {
    pub text: String,
    pub tone: ResultTone,
}

/// `finishedResult`: `stopped` is muted; otherwise the process failed when its status is
/// `failed` or its exit code is not 0. An exit code reads "exit N" with that tone, else the row
/// reads "failed" or "done". So `failed` with exit 0 shows "exit 0" in danger.
pub fn finished_result(status: &str, exit_code: Option<i64>) -> FinishedResult {
    if status == "stopped" {
        return FinishedResult {
            text: "stopped".to_string(),
            tone: ResultTone::Muted,
        };
    }
    let failed = status == "failed" || exit_code.unwrap_or(0) != 0;
    let tone = if failed {
        ResultTone::Danger
    } else {
        ResultTone::Success
    };
    match exit_code {
        Some(code) => FinishedResult {
            text: format!("exit {code}"),
            tone,
        },
        None => FinishedResult {
            text: if failed { "failed" } else { "done" }.to_string(),
            tone,
        },
    }
}

/// The kind label, as `KindLabel`: `monitor` or `bash`, uppercased by the style.
pub fn kind_label(kind: &str) -> &'static str {
    if kind == "monitor" { "MONITOR" } else { "BASH" }
}

/// The one-line label of a row: the command, or the description when it has none.
pub fn process_label(process: &BackgroundProcess) -> &str {
    process.command.as_deref().unwrap_or(&process.description)
}

/// Hashes the processes a sync reads, so `sync` returns early when they did not change.
pub fn processes_hash(processes: &[BackgroundProcess]) -> u64 {
    let mut hasher = DefaultHasher::new();
    processes.len().hash(&mut hasher);
    for process in processes {
        process.id.hash(&mut hasher);
        process.kind.hash(&mut hasher);
        process.description.hash(&mut hasher);
        process.command.hash(&mut hasher);
        process.status.hash(&mut hasher);
        process.tool_call_id.hash(&mut hasher);
        process.started_at_ms.hash(&mut hasher);
        process.ended_at_ms.hash(&mut hasher);
        process.exit_code.hash(&mut hasher);
        process.output_file.hash(&mut hasher);
        process.tail.hash(&mut hasher);
    }
    hasher.finish()
}

/// Hashes what one process row draws, with its open log state.
pub fn hash_process(process: &BackgroundProcess, expanded: bool, hasher: &mut impl Hasher) {
    process.id.hash(hasher);
    process.kind.hash(hasher);
    process.description.hash(hasher);
    process.command.hash(hasher);
    process.status.hash(hasher);
    process.started_at_ms.hash(hasher);
    process.ended_at_ms.hash(hasher);
    process.exit_code.hash(hasher);
    process.output_file.hash(hasher);
    process.tail.hash(hasher);
    expanded.hash(hasher);
}
