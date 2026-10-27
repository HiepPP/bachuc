# TASK-307 Show hooks in GPUI tool rows and hook rows

Group: B (standalone; writes only `packages/desktop-gpui/crates/timeline/**`; starts after TASK-305 by order)
Class: code

## Brief

Goal: The GPUI timeline shows the hooks that ran on a tool call, and shows non-tool hooks as their own compact row.

Change: GPUI ignores `hooks` and draws a `hook` row as a generic wrench tool -> a tool row shows a hook badge and a Hooks section, and a `hook` row shows a Zap icon with the hook name.

Boundaries:

- Follows [ADR-0068](watchtower/decisions/ADR-0068-hooks-in-tool-calls-and-stored-usage-summary.md): hooks stay inside their tool row.
- Follows [ADR-0039](watchtower/decisions/ADR-0039-incremental-timeline-sync-over-throttle-or-budget.md): a hook update is a tool call update with the same `callId`. Keep the incremental grouper equal to a full regroup.
- Use the shapes in `## Wire Shapes` of [watchtower/CONTEXT.md](watchtower/CONTEXT.md).
- Planner call: the tool row header shows `IconName::Zap` and the hook count when `hooks` is not empty. It uses the error color when any hook has status `error`.
- Planner call: the expanded tool row adds one Hooks section after the detail sections. Each line shows the hook name, the status, the exit code when set, and the output when set.
- Planner call: a `hook` row shows the name "Hook", the `detail.label` as its summary, and `IconName::Zap`.
- Electron is the visual reference for the tool row, but it has no hook UI. Reuse the existing badge and section styles of [packages/desktop-gpui/crates/timeline/src/tool_view.rs](packages/desktop-gpui/crates/timeline/src/tool_view.rs); add no new tokens.
- `HookLine` derives `Clone, Debug, PartialEq, Eq` and has the fields `name: String`, `status: String`, `exit_code: Option<i64>`, `output: Option<String>`.
- Add `HookLine` and `hook_lines` to the existing `use bachuc_timeline::tools::{...}` line of the test file; the red tests below assume that line.
- Trap: `has_details` in `tools.rs`: it reads only `detail` and `error` today. A tool call with an empty detail but with hooks must still open.

How:

- In `tools.rs`, add `HookLine` and `hook_lines`, map the `hook` name in `presentation` and `icon_for`, and count hooks in `has_details`.
- In `tool_view.rs`, draw the header badge and the Hooks section from `hook_lines`.

Files:

- [packages/desktop-gpui/crates/timeline/src/tools.rs](packages/desktop-gpui/crates/timeline/src/tools.rs) (`HookLine`, `hook_lines`, `hook` presentation, `has_details`)
- [packages/desktop-gpui/crates/timeline/src/tool_view.rs](packages/desktop-gpui/crates/timeline/src/tool_view.rs) (badge and Hooks section)
- [packages/desktop-gpui/crates/timeline/tests/tools.rs](packages/desktop-gpui/crates/timeline/tests/tools.rs) (tests)

Interfaces:

- Consumes: `hooks?: ToolCallHook[]` on tool call timeline items from TASK-305
- Consumes: tool call item `name: "hook"`, `callId: "hook:<hook_id>"` for non-tool hooks, from TASK-305

Expected result:

- WHEN a tool call item has two `hooks` entries THE timeline SHALL show a Zap badge with `2` on that row, and SHALL add no row.
- WHEN any hook of a tool call has status `error` THE timeline SHALL draw the badge in the error color.
- WHEN the row opens THE timeline SHALL list each hook with its name, status, exit code, and output.
- WHEN an item has `name: "hook"` THE timeline SHALL show "Hook", the `detail.label`, and the Zap icon.
- IF a tool call has no `hooks` field THEN THE timeline SHALL draw it as before.

Red tests:

File: [packages/desktop-gpui/crates/timeline/tests/tools.rs](packages/desktop-gpui/crates/timeline/tests/tools.rs)

```rust
#[test]
fn hooks_of_a_tool_call_become_hook_lines() {
    let item = json!({
        "name": "Bash", "status": "completed",
        "detail": {"type": "shell", "command": "ls"},
        "hooks": [
            {"id": "h1", "name": "PreToolUse:Bash", "event": "PreToolUse", "status": "success", "output": "ctx"},
            {"id": "h2", "name": "PostToolUse:Bash", "event": "PostToolUse", "status": "error", "exitCode": 2}
        ]
    });
    assert_eq!(
        hook_lines(&item),
        vec![
            HookLine { name: "PreToolUse:Bash".into(), status: "success".into(), exit_code: None, output: Some("ctx".into()) },
            HookLine { name: "PostToolUse:Bash".into(), status: "error".into(), exit_code: Some(2), output: None },
        ]
    );
}

#[test]
fn a_tool_call_without_hooks_has_no_hook_lines() {
    let item = call("Bash", "completed", json!({"type": "shell", "command": "ls"}));
    assert!(hook_lines(&item).is_empty());
}

#[test]
fn a_hook_row_shows_hook_and_its_label() {
    let item = call(
        "hook",
        "completed",
        json!({"type": "plain_text", "label": "SessionStart:startup", "icon": "wrench"}),
    );
    let shown = presentation(&item, CWD);
    assert_eq!(shown.name, "Hook");
    assert_eq!(shown.summary.as_deref(), Some("SessionStart:startup"));
    assert_eq!(shown.icon, IconName::Zap);
}

#[test]
fn hooks_alone_make_a_tool_call_openable() {
    let mut item = call("Grep", "completed", json!({"type": "unknown", "input": null, "output": null}));
    item["hooks"] = json!([{"id": "h1", "name": "PreToolUse:Grep", "event": "PreToolUse", "status": "success"}]);
    assert!(has_details(&item));
}
```

Anti-goal: timeline flush work; `incremental_rows_match_full_regroup` stays green and the hook update re-groups only its turn; tripwire; read from `./cargo.sh test -p bachuc-timeline` before changes and at completion.

## Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-timeline` -> the four new tests and all old tests pass, including `incremental_rows_match_full_regroup`.
- Read check of `tool_view.rs` -> the badge sits in the tool row header, and the Hooks section sits inside the expanded body.
- Screen check, UI checker on live with a separate check bundle: one Claude turn with a Bash call and the GitNexus hook -> the Bash row shows the Zap badge with a count, and the open row lists the hooks.
- Screen check: a session with a `SessionStart` hook -> one compact "Hook" row with the Zap icon.
- After tests: quit the check app and run `./cargo.sh clean --profile dev` when no other cargo process runs.
- Anti-goal read: `./cargo.sh test -p bachuc-timeline` -> `incremental_rows_match_full_regroup` passes at both checkpoints.
