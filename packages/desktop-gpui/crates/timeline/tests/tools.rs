//! One test per tool detail kind of `buildDetailSections` in
//! `packages/app/src/components/tool-call-details.tsx`, and the badge text of
//! `buildToolCallDisplayModel` in `packages/protocol/src/tool-call-display.ts`.

use bachuc_icons::IconName;
use bachuc_timeline::diff::{DiffLine, LineKind, numbers};
use bachuc_timeline::tools::{Section, has_details, presentation, sections};
use bachuc_timeline::{change_bar, display_path, edit_status_label};
use serde_json::json;

const CWD: &str = "/Users/test/project";

fn call(name: &str, status: &str, detail: serde_json::Value) -> serde_json::Value {
    json!({"name": name, "status": status, "detail": detail})
}

#[test]
fn shell_shows_the_command_and_its_output() {
    let detail = json!({"type": "shell", "command": "npm test\n\n", "output": "\n\nok 3 tests\n"});
    assert_eq!(
        sections(&detail, None),
        vec![Section::Shell {
            command: "npm test".into(),
            output: "ok 3 tests\n".into()
        }]
    );
    let shown = presentation(&call("Bash", "completed", detail), CWD);
    assert_eq!(shown.name, "Shell");
    assert_eq!(shown.summary.as_deref(), Some("npm test\n\n"));
    assert_eq!(shown.icon, IconName::SquareTerminal);
    assert_eq!(shown.file_path, None);
}

#[test]
fn read_shows_the_file_with_syntax_and_a_gutter_from_its_offset() {
    let detail = json!({
        "type": "read", "filePath": "/Users/test/project/src/app.ts",
        "content": "const a = 1;\n", "offset": 40
    });
    assert_eq!(
        sections(&detail, None),
        vec![Section::Code {
            text: "const a = 1;\n".into(),
            path: Some("/Users/test/project/src/app.ts".into()),
            start_line: Some(40),
            full_bleed: false,
        }]
    );
    let shown = presentation(&call("Read", "completed", detail), CWD);
    assert_eq!((shown.name.as_str(), shown.icon), ("Read", IconName::Eye));
    assert_eq!(shown.summary.as_deref(), Some("src/app.ts"));
    assert_eq!(
        shown.file_path.as_deref(),
        Some("/Users/test/project/src/app.ts")
    );
    // A read without content has nothing to open.
    let empty = call(
        "Read",
        "completed",
        json!({"type": "read", "filePath": "/a.ts"}),
    );
    assert!(!has_details(&empty));
}

#[test]
fn edit_shows_its_diff_and_counts_lines_like_the_edit_row() {
    let detail = json!({
        "type": "edit", "filePath": "/Users/test/project/a.rs",
        "unifiedDiff": "--- a/a.rs\n+++ b/a.rs\n@@ -10,3 +10,3 @@\n keep\n-old\n+new\n keep\n"
    });
    let found = sections(&detail, None);
    let [Section::Diff(lines)] = found.as_slice() else {
        panic!("expected one diff section, got {found:?}");
    };
    let kinds: Vec<LineKind> = lines.iter().map(|line| line.kind).collect();
    assert_eq!(
        kinds,
        [
            LineKind::Hunk,
            LineKind::Context,
            LineKind::Remove,
            LineKind::Add,
            LineKind::Context
        ]
    );
    // A removed line shows its old number; the others show the new one.
    assert_eq!(
        numbers(lines),
        [None, Some(10), Some(11), Some(11), Some(12)]
    );
    let shown = presentation(&call("Edit", "running", detail), CWD);
    assert_eq!(
        (shown.name.as_str(), shown.icon),
        ("Edit", IconName::Pencil)
    );
    assert_eq!(shown.summary.as_deref(), Some("a.rs"));
    assert_eq!(
        edit_status_label(&call("Edit", "running", json!({"type": "edit"}))).as_deref(),
        Some("Editing")
    );
    assert_eq!(
        edit_status_label(&call("Edit", "failed", json!({"type": "edit"}))).as_deref(),
        Some("Edit failed")
    );
    assert_eq!(
        edit_status_label(&call("Edit", "completed", json!({"type": "edit"}))),
        None
    );
    assert_eq!(
        change_bar(3, 1),
        [true, true, true, true, true, false, false]
    );
    assert_eq!(
        change_bar(100, 1),
        [true, true, true, true, true, true, false]
    );
    assert_eq!(change_bar(0, 0), Vec::<bool>::new());
}

#[test]
fn write_shows_the_new_file_full_bleed_numbered_from_one() {
    let detail = json!({"type": "write", "filePath": "/tmp/new.py", "content": "a = 1\nb = 2\n"});
    assert_eq!(
        sections(&detail, None),
        vec![Section::Code {
            text: "a = 1\nb = 2\n".into(),
            path: Some("/tmp/new.py".into()),
            start_line: None,
            full_bleed: true,
        }]
    );
    let added = |text: &str| DiffLine {
        kind: LineKind::Add,
        text: text.into(),
    };
    assert_eq!(
        numbers(&[added("a = 1"), added("b = 2")]),
        [Some(1), Some(2)]
    );
    assert_eq!(
        edit_status_label(&call("Write", "running", json!({"type": "write"}))).as_deref(),
        Some("Writing")
    );
    assert_eq!(display_path("/private/tmp/x/new.py", "/tmp/x"), "new.py");
}

#[test]
fn search_shows_content_files_web_results_and_annotations() {
    let detail = json!({
        "type": "search", "query": "TODO",
        "content": "src/a.ts:1: TODO",
        "filePaths": ["src/a.ts", "src/b.ts"],
        "webResults": [{"title": "Docs", "url": "https://x.dev"}],
        "annotations": ["note one", "note two"]
    });
    let texts: Vec<String> = sections(&detail, None)
        .into_iter()
        .map(|section| match section {
            Section::Code { text, .. } => text,
            other => panic!("unexpected {other:?}"),
        })
        .collect();
    assert_eq!(
        texts,
        [
            "src/a.ts:1: TODO",
            "src/a.ts\nsrc/b.ts",
            "Docs\nhttps://x.dev",
            "note one\n\nnote two"
        ]
    );
    let shown = presentation(&call("Grep", "completed", detail), CWD);
    assert_eq!(
        (shown.name.as_str(), shown.summary.as_deref(), shown.icon),
        ("Search", Some("TODO"), IconName::Search)
    );
}

#[test]
fn fetch_shows_the_url_then_the_result() {
    let detail = json!({"type": "fetch", "url": "https://x.dev", "result": "Hello"});
    assert_eq!(
        sections(&detail, None),
        vec![Section::Code {
            text: "https://x.dev\n\nHello".into(),
            path: None,
            start_line: None,
            full_bleed: false,
        }]
    );
    let shown = presentation(&call("WebFetch", "completed", detail), CWD);
    assert_eq!(
        (shown.name.as_str(), shown.summary.as_deref()),
        ("Fetch", Some("https://x.dev"))
    );
}

#[test]
fn sub_agent_splits_actions_from_its_log() {
    let detail = json!({
        "type": "sub_agent", "subAgentType": "Explore", "description": "Find the parser",
        "childSessionId": "s-1",
        "log": "\n[read_file] src/a.ts\n[Grep]\nthinking about it\n[bad]text\n"
    });
    assert_eq!(
        sections(&detail, None),
        vec![Section::SubAgent {
            session: Some("s-1".into()),
            actions: vec![
                ("Read File".into(), Some("src/a.ts".into())),
                ("Grep".into(), None)
            ],
            log: "thinking about it\n[bad]text".into(),
        }]
    );
    let quiet = json!({"type": "sub_agent", "subAgentType": "Explore", "description": "Look"});
    assert_eq!(
        sections(&quiet, None),
        vec![Section::SubAgent {
            session: None,
            actions: vec![],
            log: "Explore: Look".into()
        }]
    );
    let shown = presentation(&call("Task", "running", detail), CWD);
    assert_eq!(
        (shown.name.as_str(), shown.summary.as_deref(), shown.icon),
        ("Explore", Some("Find the parser"), IconName::Bot)
    );
}

#[test]
fn plan_has_no_detail_sections_and_draws_as_a_plan_card() {
    let detail = json!({"type": "plan", "text": "1. Do it"});
    assert!(sections(&detail, None).is_empty());
    let shown = presentation(&call("ExitPlanMode", "completed", detail), CWD);
    assert_eq!((shown.name.as_str(), shown.icon), ("Plan", IconName::Brain));
}

#[test]
fn unknown_tools_show_input_and_output_and_a_failed_call_its_error() {
    let detail = json!({"type": "unknown", "input": {"a": 1}, "output": null});
    let error = "boom";
    assert_eq!(
        sections(&detail, Some(error)),
        vec![
            Section::Header("Input".into()),
            Section::Code {
                text: "{\n  \"a\": 1\n}".into(),
                path: None,
                start_line: None,
                full_bleed: false
            },
            Section::Error("boom".into()),
        ]
    );
    let item = json!({"name": "my_tool.run", "status": "failed", "error": {"content": "boom"},
        "detail": detail});
    let shown = presentation(&item, CWD);
    assert_eq!(shown.name, "my_tool.run");
    assert_eq!(shown.error.as_deref(), Some("boom"));
    let thought = call(
        "thinking",
        "completed",
        json!({"type": "unknown", "input": "hmm", "output": null}),
    );
    let shown = presentation(&thought, CWD);
    assert_eq!(
        (shown.name.as_str(), shown.icon),
        ("Thinking", IconName::Brain)
    );
    assert_eq!(
        sections(&thought["detail"], None),
        vec![Section::Plain("hmm".into())]
    );
    assert_eq!(
        presentation(&call("run_tests", "completed", json!({})), CWD).name,
        "Run tests"
    );
}
