//! The Electron placement test of `background-process-placement.test.ts`, and the result
//! wording of `finishedResult` in `background-process-card.tsx`.

use bachuc_client::BackgroundProcess;
use bachuc_timeline::background::{self, PlacementRow, ResultTone};

fn process(id: &str, tool_call_id: &str, status: &str) -> BackgroundProcess {
    BackgroundProcess {
        id: id.to_string(),
        kind: "bash".to_string(),
        description: id.to_string(),
        command: None,
        status: status.to_string(),
        tool_call_id: Some(tool_call_id.to_string()),
        started_at_ms: 1_000,
        ended_at_ms: (status != "running").then_some(2_000),
        exit_code: None,
        output_file: None,
        tail: Vec::new(),
    }
}

fn turn(key: u64, call_ids: &[&str]) -> PlacementRow {
    PlacementRow::Turn {
        key,
        call_ids: call_ids.iter().map(|id| id.to_string()).collect(),
    }
}

#[test]
fn a_running_process_shows_in_the_newest_turn_and_marks_where_it_started() {
    let rows = [
        PlacementRow::UserMessage,
        turn(7, &["bash-call"]),
        PlacementRow::UserMessage,
        turn(8, &["read-call"]),
    ];
    let placement = background::place(&rows, &[process("p1", "bash-call", "running")]);
    let newest = placement.by_turn.get(&8).expect("the newest turn hosts it");
    assert_eq!(
        newest
            .processes
            .iter()
            .map(|p| p.id.as_str())
            .collect::<Vec<_>>(),
        ["p1"]
    );
    let start = placement
        .by_turn
        .get(&7)
        .expect("the start turn has a note");
    assert!(start.processes.is_empty());
    assert_eq!(start.moved_count, 1);
    assert!(placement.bottom.is_empty());
}

#[test]
fn a_finished_process_returns_to_the_turn_that_started_it() {
    let rows = [
        PlacementRow::UserMessage,
        turn(7, &["bash-call"]),
        PlacementRow::UserMessage,
        turn(8, &["read-call"]),
    ];
    let placement = background::place(&rows, &[process("p1", "bash-call", "completed")]);
    let start = placement.by_turn.get(&7).expect("the start turn hosts it");
    assert_eq!(
        start
            .processes
            .iter()
            .map(|p| p.id.as_str())
            .collect::<Vec<_>>(),
        ["p1"]
    );
    assert!(!placement.by_turn.contains_key(&8));
    assert_eq!(start.moved_count, 0);
}

#[test]
fn a_finished_process_with_an_unknown_call_is_not_shown() {
    let rows = [PlacementRow::UserMessage, turn(7, &["bash-call"])];
    let placement = background::place(&rows, &[process("p1", "gone", "completed")]);
    assert!(placement.by_turn.is_empty());
    assert!(placement.bottom.is_empty());
}

#[test]
fn a_running_process_after_a_user_message_shows_at_the_bottom() {
    let rows = [
        PlacementRow::UserMessage,
        turn(7, &["bash-call"]),
        PlacementRow::UserMessage,
    ];
    let placement = background::place(&rows, &[process("p1", "bash-call", "running")]);
    assert_eq!(
        placement
            .bottom
            .iter()
            .map(|p| p.id.as_str())
            .collect::<Vec<_>>(),
        ["p1"]
    );
    assert_eq!(
        placement.by_turn.get(&7).map(|slot| slot.moved_count),
        Some(1)
    );
}

#[test]
fn a_finished_process_reads_as_the_app_words_it() {
    let stopped = background::finished_result("stopped", None);
    assert_eq!(stopped.text, "stopped");
    assert_eq!(stopped.tone, ResultTone::Muted);

    let ok = background::finished_result("completed", Some(0));
    assert_eq!(ok.text, "exit 0");
    assert_eq!(ok.tone, ResultTone::Success);

    let exit2 = background::finished_result("completed", Some(2));
    assert_eq!(exit2.text, "exit 2");
    assert_eq!(exit2.tone, ResultTone::Danger);

    // `failed` with exit 0 keeps the exit text and takes the danger tone.
    let failed_exit0 = background::finished_result("failed", Some(0));
    assert_eq!(failed_exit0.text, "exit 0");
    assert_eq!(failed_exit0.tone, ResultTone::Danger);

    let failed = background::finished_result("failed", None);
    assert_eq!(failed.text, "failed");
    assert_eq!(failed.tone, ResultTone::Danger);

    let done = background::finished_result("completed", None);
    assert_eq!(done.text, "done");
    assert_eq!(done.tone, ResultTone::Success);
}

#[test]
fn the_row_reads_the_command_or_the_description() {
    let mut process = process("p1", "call", "running");
    process.description = "watcher".to_string();
    assert_eq!(background::process_label(&process), "watcher");
    process.command = Some("tail -f log".to_string());
    assert_eq!(background::process_label(&process), "tail -f log");
    assert_eq!(background::kind_label("monitor"), "MONITOR");
    assert_eq!(background::kind_label("bash"), "BASH");
    assert_eq!(background::kind_label("other"), "BASH");
}
