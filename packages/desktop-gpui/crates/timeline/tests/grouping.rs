//! The cases of `packages/app/src/tool-calls/detail-level/turn/model.test.ts`, rewritten as
//! projected-entry fixtures. The app's edit-diffs plugin rows become plain edit calls, because
//! the client does not ask for plugin timeline items. The case that needs the edit-diffs plugin
//! presentation ("joins plugin edit rows through the stream presentation") is skipped, and the
//! compact-screen half of "regroups history when the turn ends" has no GPUI counterpart.

use bachuc_theme::TurnLabel;
use std::rc::Rc;

use bachuc_timeline::grouping::{
    Row, SharedRow, TurnGroup, TurnGrouper, group_turns, turn_note_hosts,
};
use serde_json::{Value, json};

const BASE_MS: i64 = 1_767_225_600_000; // 2026-01-01T00:00:00Z

fn at(second: i64) -> i64 {
    BASE_MS + second * 1000
}

fn stamp(second: i64) -> String {
    format!("2026-01-01T00:00:{second:02}.000Z")
}

/// A named fixture item; `build` gives it its sequence numbers.
struct Fx {
    name: String,
    second: i64,
    item: Value,
}

fn call(id: &str, second: i64, detail: Value, status: &str) -> Fx {
    let name = detail["type"].as_str().unwrap().to_string();
    Fx {
        name: id.into(),
        second,
        item: json!({
            "type": "tool_call", "callId": id, "name": name, "status": status,
            "error": null, "detail": detail
        }),
    }
}

fn read(id: &str, second: i64) -> Fx {
    call(
        id,
        second,
        json!({"type": "read", "filePath": format!("/repo/{id}.ts")}),
        "completed",
    )
}

fn shell(id: &str, second: i64) -> Fx {
    call(
        id,
        second,
        json!({"type": "shell", "command": id}),
        "completed",
    )
}

fn running_shell(id: &str, second: i64) -> Fx {
    call(
        id,
        second,
        json!({"type": "shell", "command": id}),
        "running",
    )
}

fn edit(id: &str, second: i64, path: &str) -> Fx {
    call(
        id,
        second,
        json!({"type": "edit", "filePath": path}),
        "completed",
    )
}

/// An assistant message. `group` is its message id, shared by the blocks of one note.
fn note_in(id: &str, second: i64, group: Option<&str>) -> Fx {
    let mut item = json!({"type": "assistant_message", "text": id});
    if let Some(group) = group {
        item["messageId"] = json!(group);
    }
    Fx {
        name: id.into(),
        second,
        item,
    }
}

fn note(id: &str, second: i64) -> Fx {
    note_in(id, second, None)
}

fn user() -> Fx {
    Fx {
        name: "user".into(),
        second: 0,
        item: json!({"type": "user_message", "text": "Go"}),
    }
}

fn notification(id: &str, second: i64) -> Fx {
    Fx {
        name: id.into(),
        second,
        item: json!({"type": "notification", "level": "info", "message": id}),
    }
}

fn thought(id: &str, second: i64) -> Fx {
    Fx {
        name: id.into(),
        second,
        item: json!({"type": "reasoning", "text": "Hmm"}),
    }
}

struct Timeline {
    entries: Vec<Value>,
    names: Vec<String>,
}

fn build(items: Vec<Fx>) -> Timeline {
    let mut entries = Vec::new();
    let mut names = Vec::new();
    for (index, fx) in items.into_iter().enumerate() {
        let seq = index as u64 + 1;
        entries.push(json!({
            "item": fx.item, "timestamp": stamp(fx.second), "seqStart": seq, "seqEnd": seq,
            "sourceSeqRanges": [{"startSeq": seq, "endSeq": seq}], "collapsed": []
        }));
        names.push(fx.name);
    }
    Timeline { entries, names }
}

impl Timeline {
    fn rows(&self, active: bool) -> Vec<Row> {
        group_turns(&self.entries, active)
    }

    fn ids(&self, indices: &[usize]) -> Vec<&str> {
        indices.iter().map(|&i| self.names[i].as_str()).collect()
    }

    /// Row ids: an entry's name, or the name of the first item of a turn's run.
    fn row_ids(&self, rows: &[Row]) -> Vec<&str> {
        rows.iter()
            .map(|row| match row {
                Row::Item(index) => self.names[*index].as_str(),
                Row::Turn(group) => self.names[group.run.items[0]].as_str(),
            })
            .collect()
    }

    fn groups(&self, active: bool) -> Vec<TurnGroup> {
        self.rows(active)
            .into_iter()
            .filter_map(|row| match row {
                Row::Turn(group) => Some(group),
                Row::Item(_) => None,
            })
            .collect()
    }

    fn only_group(&self, active: bool) -> TurnGroup {
        let mut groups = self.groups(active);
        assert_eq!(groups.len(), 1);
        groups.remove(0)
    }

    fn step_items(&self, group: &TurnGroup) -> Vec<Vec<&str>> {
        group.steps.iter().map(|s| self.ids(&s.items)).collect()
    }

    fn step_notes(&self, group: &TurnGroup) -> Vec<Vec<&str>> {
        group.steps.iter().map(|s| self.ids(&s.note)).collect()
    }
}

fn labels(group: &TurnGroup) -> Vec<TurnLabel> {
    group.steps.iter().map(|step| step.label).collect()
}

/// The 4-step turn of the TypeScript suite, ending with its answer `final`.
fn turn() -> Vec<Fx> {
    vec![
        user(),
        read("r1", 1),
        shell("s1", 2),
        note("note1", 3),
        read("r2", 4),
        note("note2", 5),
        note("note3", 6),
        edit("e1", 7, "/repo/a.ts"),
        edit("e2", 8, "/repo/b.ts"),
        shell("test", 9),
        shell("lint", 10),
        note("final", 12),
    ]
}

use TurnLabel::{Edit, Explore, Verify};

#[test]
fn groups_a_whole_turn_into_one_run_of_4_steps_with_the_answer_outside_it() {
    let t = build(turn());
    let rows = t.rows(false);
    assert_eq!(t.row_ids(&rows), ["user", "r1", "final"]);
    let group = t.only_group(false);
    assert_eq!(group.run.items, (1..11).collect::<Vec<_>>());
    assert_eq!(
        t.step_items(&group),
        [
            vec!["r1", "s1"],
            vec!["r2"],
            vec![],
            vec!["e1", "e2", "test", "lint"]
        ]
    );
    assert_eq!(
        t.step_notes(&group),
        [vec![], vec!["note1"], vec!["note2"], vec!["note3"]]
    );
    assert_eq!(t.ids(&group.steps[3].edits), ["e1", "e2"]);
    assert_eq!(group.steps[0].counts.read_file_count, 1);
    assert_eq!(group.steps[0].counts.command_count, 1);
    assert_eq!(group.steps[3].counts.edited_file_count, 2);
}

#[test]
fn opens_each_step_with_its_note_so_the_note_sits_above_the_work_it_announced() {
    let t = build(vec![
        user(),
        note("Step 1: reading calc.py", 1),
        read("r1", 2),
        shell("run1", 3),
        note("Step 2: reading the tests", 4),
        read("r2", 5),
        shell("test1", 6),
        note("Step 4: fixing add()", 7),
        edit("e1", 8, "/repo/calc.py"),
        note("Step 5: fixing sub()", 9),
        edit("e2", 10, "/repo/calc.py"),
        note("Step 6: running calc.py again", 11),
        shell("run2", 12),
        note("final", 12),
    ]);
    let group = t.only_group(false);
    assert_eq!(
        t.step_notes(&group),
        [
            vec!["Step 1: reading calc.py"],
            vec!["Step 2: reading the tests"],
            vec!["Step 4: fixing add()"],
            vec!["Step 5: fixing sub()"],
            vec!["Step 6: running calc.py again"],
        ]
    );
    assert_eq!(
        t.step_items(&group),
        [
            vec!["r1", "run1"],
            vec!["r2", "test1"],
            vec!["e1"],
            vec!["e2"],
            vec!["run2"]
        ]
    );
    assert_eq!(labels(&group), [Explore, Explore, Edit, Edit, Verify]);
    let starts: Vec<i64> = group.steps.iter().map(|s| s.started_at).collect();
    assert_eq!(starts, [at(1), at(4), at(7), at(9), at(11)]);
    let ends: Vec<Option<i64>> = group.steps.iter().map(|s| s.ended_at).collect();
    assert_eq!(
        ends,
        [
            Some(at(3)),
            Some(at(6)),
            Some(at(8)),
            Some(at(10)),
            Some(at(12))
        ]
    );
}

#[test]
fn gives_a_turn_that_starts_with_a_tool_call_a_first_step_with_no_note() {
    let t = build(vec![
        user(),
        read("r1", 1),
        note("n1", 2),
        shell("s1", 3),
        note("final", 12),
    ]);
    let group = t.only_group(false);
    assert_eq!(t.step_notes(&group), [vec![], vec!["n1"]]);
    assert_eq!(t.step_items(&group), [vec!["r1"], vec!["s1"]]);
    assert_eq!(group.steps[0].started_at, at(0));
    assert_eq!(group.steps[0].ended_at, Some(at(1)));
}

#[test]
fn labels_steps_explore_before_the_first_edit_edit_for_edits_and_verify_after() {
    assert_eq!(
        labels(&build(turn()).only_group(false)),
        [Explore, Explore, Explore, Edit]
    );
    let no_edit = build(vec![
        user(),
        read("r1", 1),
        note("n1", 2),
        shell("s1", 3),
        note("final", 12),
    ]);
    assert_eq!(labels(&no_edit.only_group(false)), [Explore, Explore]);
}

#[test]
fn counts_paseo_edit_rows_as_edits_when_edit_diffs_are_off() {
    let t = build(vec![
        user(),
        read("r1", 1),
        edit("e1", 2, "/repo/a.ts"),
        note("n1", 3),
        shell("s1", 4),
        note("final", 12),
    ]);
    let group = t.only_group(false);
    assert_eq!(labels(&group), [Edit, Verify]);
    assert_eq!(t.ids(&group.steps[0].edits), ["e1"]);
    assert_eq!(group.changed_files, ["/repo/a.ts"]);
}

#[test]
fn gives_each_step_an_end_time_and_a_duration_from_item_timestamps() {
    let group = build(turn()).only_group(false);
    let ends: Vec<Option<i64>> = group.steps.iter().map(|s| s.ended_at).collect();
    assert_eq!(ends, [Some(at(2)), Some(at(4)), Some(at(5)), Some(at(10))]);
    let durations: Vec<Option<i64>> = group.steps.iter().map(|s| s.duration_ms).collect();
    assert_eq!(durations, [Some(2000), Some(1000), Some(0), Some(4000)]);
    assert_eq!(group.steps[0].started_at, at(0));
    assert_eq!(group.running_step, None);
    assert!(!group.is_loading);
}

#[test]
fn starts_the_first_step_at_the_turn_start_not_at_a_rewritten_tool_call_time() {
    // A tool call takes the timestamp of its latest update, so it can lag the turn start.
    let t = build(vec![
        user(),
        read("r1", 2),
        note("n1", 3),
        notification("split", 5),
        shell("s1", 7),
        note("n2", 8),
        // A tool call keeps n2 a step note: every message after the last tool call is the answer.
        shell("s2", 9),
        note("final", 12),
    ]);
    let groups = t.groups(false);
    let starts: Vec<i64> = groups.iter().map(|g| g.run.started_at).collect();
    assert_eq!(starts, [at(0), at(5)]);
    let first: Vec<Option<i64>> = groups.iter().map(|g| g.steps[0].duration_ms).collect();
    assert_eq!(first, [Some(2000), Some(2000)]);

    let window = build(vec![read("r1", 2), note("n1", 3), note("final", 12)]);
    assert_eq!(window.only_group(false).run.started_at, at(2));
}

#[test]
fn labels_a_non_edit_step_between_two_edit_steps_as_edit() {
    let t = build(vec![
        user(),
        edit("e1", 1, "/repo/a.ts"),
        note("n1", 2),
        read("r1", 3),
        note("n2", 4),
        edit("e2", 5, "/repo/b.ts"),
        note("n3", 6),
        shell("test", 7),
        note("final", 12),
    ]);
    assert_eq!(labels(&t.only_group(false)), [Edit, Edit, Edit, Verify]);
}

#[test]
fn leaves_the_running_step_without_an_end_time() {
    let t = build(vec![
        user(),
        read("r1", 1),
        note("n1", 3),
        running_shell("s1", 5),
    ]);
    let group = t.only_group(true);
    let running: Vec<bool> = group.steps.iter().map(|s| s.is_running).collect();
    assert_eq!(running, [false, true]);
    assert_eq!(group.steps[1].started_at, at(3));
    assert_eq!(group.steps[1].ended_at, None);
    assert_eq!(group.steps[1].duration_ms, None);
    assert_eq!(group.running_step, Some(1));
    assert!(group.is_loading);
}

#[test]
fn summarizes_the_step_count_and_the_changed_file_paths() {
    let mut items = turn();
    let last = items.pop().unwrap();
    items.push(edit("e3", 11, "/repo/a.ts"));
    items.push(last);
    let group = build(items).only_group(false);
    assert_eq!(group.step_count, 4);
    assert_eq!(group.changed_files, ["/repo/a.ts", "/repo/b.ts"]);
}

#[test]
fn hides_thoughts_and_joins_the_markdown_blocks_of_one_note() {
    let t = build(vec![
        user(),
        thought("thought", 1),
        read("r1", 2),
        note_in("n1:block:0", 3, Some("n1")),
        note_in("n1:block:1", 3, Some("n1")),
        shell("s1", 4),
        note_in("a:block:0", 6, Some("a")),
        note_in("a:block:1", 6, Some("a")),
    ]);
    let rows = t.rows(false);
    assert_eq!(
        t.row_ids(&rows),
        ["user", "thought", "a:block:0", "a:block:1"]
    );
    let group = t.only_group(false);
    assert_eq!(t.names[group.run.items[0]], "thought");
    assert_eq!(group.steps.len(), 2);
    assert!(group.steps[0].note.is_empty());
    assert_eq!(t.ids(&group.steps[0].items), ["r1"]);
    assert_eq!(group.steps[0].started_at, at(0));
    assert_eq!(t.ids(&group.steps[1].note), ["n1:block:0", "n1:block:1"]);
    assert_eq!(t.ids(&group.steps[1].items), ["s1"]);
    assert_eq!(group.steps[1].started_at, at(3));
}

#[test]
fn maps_each_step_note_message_to_the_host_row_of_its_turn() {
    let t = build(vec![
        user(),
        read("r1", 1),
        note_in("n1:block:0", 3, Some("n1")),
        note_in("n1:block:1", 3, Some("n1")),
        shell("s1", 4),
        note("final", 12),
    ]);
    let hosts = turn_note_hosts(&t.entries, &t.rows(false));
    let named: Vec<(String, &str)> = hosts
        .into_iter()
        .map(|(key, host)| (key, t.names[host].as_str()))
        .collect();
    assert_eq!(named, [("n1".to_string(), "r1")]);
}

#[test]
fn keeps_an_answer_sent_as_several_messages_outside_the_run() {
    let mut items = turn();
    items.pop();
    items.push(note_in("a:block:0", 12, Some("a")));
    items.push(note_in("b:block:0", 13, Some("b")));
    items.push(note_in("b:block:1", 13, Some("b")));
    let t = build(items);
    let rows = t.rows(false);
    assert_eq!(
        t.row_ids(&rows),
        ["user", "r1", "a:block:0", "b:block:0", "b:block:1"]
    );
    let group = t.only_group(false);
    assert_eq!(t.ids(&group.steps.last().unwrap().note), ["note3"]);
    assert_eq!(group.run.items, (1..11).collect::<Vec<_>>());
}

#[test]
fn builds_the_4_step_turn_live_and_moves_the_answer_out_once_it_ends() {
    let t = build(turn());
    let first_notes = |group: &TurnGroup| -> Vec<Option<String>> {
        group
            .steps
            .iter()
            .map(|s| s.note.first().map(|&i| t.names[i].clone()))
            .collect()
    };

    let running_rows = t.rows(true);
    assert_eq!(t.row_ids(&running_rows), ["user", "r1"]);
    let running = t.only_group(true);
    assert_eq!(
        first_notes(&running),
        [
            None,
            Some("note1".into()),
            Some("note2".into()),
            Some("note3".into()),
            Some("final".into())
        ]
    );
    assert_eq!(running.running_step, Some(4));

    let ended_rows = t.rows(false);
    assert_eq!(t.row_ids(&ended_rows), ["user", "r1", "final"]);
    let ended = t.only_group(false);
    assert_eq!(
        first_notes(&ended).last().unwrap().as_deref(),
        Some("note3")
    );
    assert_eq!(ended.running_step, None);
    assert_eq!(ended.steps.len(), 4);
}

#[test]
fn regroups_history_when_the_turn_ends() {
    let t = build(vec![
        user(),
        read("r1", 1),
        note("n1", 2),
        shell("s1", 3),
        note("final", 12),
        notification("done", 13),
    ]);
    assert_eq!(t.row_ids(&t.rows(true)), ["user", "r1", "done"]);
    assert_eq!(t.row_ids(&t.rows(false)), ["user", "r1", "final", "done"]);
    assert_eq!(t.only_group(false).run.items.len(), 3);
}

/// One projected entry with its own sequence numbers, as a page or a live row brings it.
fn entry(seq_start: u64, seq_end: u64, fx: Fx) -> Value {
    json!({
        "item": fx.item, "timestamp": stamp(fx.second), "seqStart": seq_start,
        "seqEnd": seq_end, "sourceSeqRanges": [{"startSeq": seq_start, "endSeq": seq_end}],
        "collapsed": []
    })
}

/// The `Rc` addresses of the turn groups of the grouper's last sync.
fn turn_addresses(grouper: &TurnGrouper) -> Vec<*const TurnGroup> {
    grouper
        .rows()
        .iter()
        .filter_map(|row| match row {
            SharedRow::Turn(group) => Some(Rc::as_ptr(group)),
            SharedRow::Item(_) => None,
        })
        .collect()
}

#[test]
fn incremental_rows_match_full_regroup() {
    let mut grouper = TurnGrouper::default();
    let mut check = |entries: &[Value], active: bool, case: &str| {
        let full = group_turns(entries, active);
        assert_eq!(grouper.update(entries, active), full, "{case}");
        grouper.rows().len()
    };

    // The first page starts inside a finished turn; an open turn follows.
    let mut entries = vec![
        entry(12, 12, read("a", 2)),
        entry(13, 13, edit("e1", 3, "/repo/a.ts")),
        entry(14, 14, note("answer", 4)),
        entry(20, 20, user()),
        entry(21, 21, note("look", 6)),
        entry(22, 22, read("b", 7)),
    ];
    check(&entries, true, "first sync");

    // Appending to the open turn keeps the finished turn's group and rebuilds the open one.
    let mut grouper_before = TurnGrouper::default();
    grouper_before.update(&entries, true);
    let before = turn_addresses(&grouper_before);
    entries.push(entry(23, 23, running_shell("s1", 8)));
    assert_eq!(
        grouper_before.update(&entries, true),
        group_turns(&entries, true)
    );
    let after = turn_addresses(&grouper_before);
    assert_eq!(after.len(), 2);
    assert_eq!(after[0], before[0], "the finished turn keeps its group");
    assert_ne!(after[1], before[1], "the open turn regroups");
    assert_eq!(grouper_before.changed(), [6]);
    check(&entries, true, "append to the open turn");

    // A call completes in place: same seqStart, a larger seqEnd.
    entries[6] = entry(23, 24, shell("s1", 9));
    check(&entries, true, "a call completes");
    entries.push(entry(25, 25, note("next", 10)));
    entries.push(entry(26, 26, edit("e2", 11, "/repo/b.ts")));
    check(&entries, true, "a new step");

    // An assistant message grows: its seqEnd changes.
    let mut grown = entry(25, 27, note("next", 10));
    grown["item"]["text"] = json!("next, and more");
    entries[7] = grown;
    check(&entries, true, "a note grows");

    // The agent status flips with no entry change.
    check(&entries, false, "the agent goes idle");
    check(&entries, true, "the agent runs again");

    // The open turn finishes.
    entries.push(entry(28, 28, note("done", 12)));
    check(&entries, true, "the answer streams");
    check(&entries, false, "the turn finishes");

    // A new turn starts while the older one is finished.
    entries.push(entry(30, 30, user()));
    entries.push(entry(31, 31, read("c", 14)));
    check(&entries, true, "a new turn starts");
    entries.push(entry(32, 32, running_shell("s2", 15)));
    check(&entries, true, "the new turn runs a call");

    // An older page arrives and regroups the top turn.
    let older = [entry(10, 10, user()), entry(11, 11, note("plan", 1))];
    entries.splice(0..0, older);
    check(&entries, true, "an older page");

    // An entry in the middle goes away.
    entries.remove(6);
    check(&entries, true, "a middle entry is removed");

    // The timeline is replaced.
    let replaced = vec![
        entry(40, 40, user()),
        entry(41, 41, note("fresh", 20)),
        entry(42, 42, read("d", 21)),
    ];
    let rows = check(&replaced, true, "the timeline is replaced");
    assert_eq!(rows, 2);
    check(&[], false, "the timeline empties");
}
