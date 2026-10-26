# TASK-308 Show the skill and hook summary as a GPUI header chip

Group: C (standalone; writes `packages/desktop-gpui/crates/client/**` and `packages/desktop-gpui/crates/app/src/chrome.rs`; starts after TASK-306 by order)
Class: code

## Brief

Goal: The GPUI workspace header shows a chip with the skill and hook counts of the focused agent, and the chip opens a list of each skill and hook.

Change: no summary in GPUI -> the client keeps `skillHookUsage` per agent, and the header shows a chip such as `3 skills · 12 hooks` that opens a popover.

Boundaries:

- Follows [ADR-0068](watchtower/decisions/ADR-0068-hooks-in-tool-calls-and-stored-usage-summary.md): read the summary from the agent snapshot; do not count from timeline rows.
- Follows [ADR-0040](watchtower/decisions/ADR-0040-one-host-plain-ws-over-merged-hosts-and-relay.md): one window shows one host, so the summary keys by agent id only.
- Use the snapshot shape in `## Wire Shapes` of [watchtower/CONTEXT.md](watchtower/CONTEXT.md).
- Planner call: the chip shows only for the agent of the focused tab, found the way `toggle_header_actions` in `chrome.rs` finds `active_agent`.
- Planner call: the label is `<n> skill` or `<n> skills`, then `·`, then `<m> hook` or `<m> hooks`. A zero part is left out. Here n and m count distinct entries, not uses.
- Planner call: the popover uses `header_popover` in `chrome.rs`, opened through a new `HeaderMenu::Usage` value of `header_menu`, like `HeaderMenu::Scripts`. It has a Skills group, then a Hooks group, newest first. Each row shows the name, `×<count>`, and the time since `lastUsedAt`. A hook row also shows its error count when it is not zero.
- Trap: `BackgroundProcess::parse_all` in [packages/desktop-gpui/crates/client/src/model.rs](packages/desktop-gpui/crates/client/src/model.rs) is the pattern for snapshot parsing. A missing field must give `None`, not a panic or an empty chip.
- Trap: the workspace header in `render_header` of `chrome.rs` already holds Scripts, Open in editor, git actions, and the Explorer toggle. Put the chip before those buttons, and let the title truncate first.

How:

- In `model.rs`, add `SkillHookUsage`, `UsageEntry`, `SkillHookUsage::parse`, and `SkillHookUsage::chip_label`.
- In `core.rs`, parse `skillHookUsage` from each agent snapshot into the agent extras, next to `background_processes`. Add `skill_hook_usage(&self, agent_id: &str) -> Option<SkillHookUsage>` on the core and on `Client` in `lib.rs`; it returns a clone, as `background_processes` does.
- In `chrome.rs`, draw the chip in `render_header` and its popover with `header_popover`.

Files:

- [packages/desktop-gpui/crates/client/src/model.rs](packages/desktop-gpui/crates/client/src/model.rs) (`SkillHookUsage`, `UsageEntry`, parse, label)
- [packages/desktop-gpui/crates/client/src/core.rs](packages/desktop-gpui/crates/client/src/core.rs) (snapshot parse into extras, accessor)
- [packages/desktop-gpui/crates/client/src/lib.rs](packages/desktop-gpui/crates/client/src/lib.rs) (`Client::skill_hook_usage`, export of the types)
- [packages/desktop-gpui/crates/client/tests/rpcs.rs](packages/desktop-gpui/crates/client/tests/rpcs.rs) (snapshot tests)
- [packages/desktop-gpui/crates/app/src/chrome.rs](packages/desktop-gpui/crates/app/src/chrome.rs) (chip and popover)

Interfaces:

- Consumes: agent snapshot field `skillHookUsage?: { skills: SkillHookUsageEntry[]; hooks: SkillHookUsageEntry[] }` with `SkillHookUsageEntry = { name: string; event?: string; count: number; errorCount: number; lastUsedAt: string }` from TASK-306

Expected result:

- WHEN an agent snapshot carries `skillHookUsage` THE client SHALL keep it for that agent, in the daemon's order.
- WHEN the focused agent has 3 skills and 12 hooks THE header SHALL show the chip `3 skills · 12 hooks`.
- WHEN Hiep clicks the chip THE header SHALL open a popover with a Skills group and a Hooks group, each row with its name, count, and time since last use.
- IF the snapshot has no `skillHookUsage` THEN THE header SHALL show no chip.
- IF the focused tab is not an agent THEN THE header SHALL show no chip.

Red tests:

File: [packages/desktop-gpui/crates/client/tests/rpcs.rs](packages/desktop-gpui/crates/client/tests/rpcs.rs)

```rust
fn agent_with_usage(id: &str, usage: Value) -> Value {
    let mut agent = agent_snapshot(id, "running");
    agent["skillHookUsage"] = usage;
    json!({"type": "agent_update", "payload": {"kind": "upsert", "agent": agent}})
}

#[test]
fn an_agent_snapshot_keeps_its_skill_hook_usage() {
    let server = FakeServer::start();
    server.add_agent("a1", "idle");
    let mut harness = connect(&server);
    server.broadcast(agent_with_usage(
        "a1",
        json!({
            "skills": [{"name": "watchtower", "count": 2, "errorCount": 0, "lastUsedAt": "2026-10-27T10:00:00.000Z"}],
            "hooks": [{"name": "PreToolUse:Bash", "event": "PreToolUse", "count": 5, "errorCount": 1, "lastUsedAt": "2026-10-27T10:01:00.000Z"}]
        }),
    ));
    assert!(
        harness.pump_until(WAIT, |h| h.core.skill_hook_usage("a1").is_some()),
        "the usage arrives"
    );
    let usage = harness.core.skill_hook_usage("a1").unwrap();
    assert_eq!(usage.skills[0].name, "watchtower");
    assert_eq!(usage.skills[0].count, 2);
    assert_eq!(usage.hooks[0].event.as_deref(), Some("PreToolUse"));
    assert_eq!(usage.hooks[0].error_count, 1);
    assert_eq!(usage.chip_label().as_deref(), Some("1 skill · 1 hook"));
}

#[test]
fn an_agent_snapshot_without_usage_has_no_chip() {
    let server = FakeServer::start();
    server.add_agent("a1", "idle");
    let mut harness = connect(&server);
    server.broadcast(agent_with_processes("a1", json!([])));
    assert!(
        harness.pump_until(WAIT, |h| h.core.agents()[0].status == "running"),
        "the snapshot arrives"
    );
    assert!(harness.core.skill_hook_usage("a1").is_none());
}
```

Anti-goal: header fit; at a 900 px wide window the header shows the chip, Scripts, Open in editor, git actions, and the Explorer toggle with no overlap or clipped button; drift gauge; read from the screen check at completion.

## Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-client --test rpcs` -> the two new tests and all old tests pass.
- Read check of `core.rs` -> `skillHookUsage` parses next to `backgroundProcesses` and needs no new protocol type.
- Screen check, UI checker on live with a separate check bundle and `BACHUC_WINDOW=900x800`: focus a Claude agent tab after a turn that used a skill and a hook -> the chip shows the counts, and clicking it opens the Skills and Hooks groups.
- Screen check: focus a terminal tab or an agent without use -> no chip.
- After tests: quit the check app and run `./cargo.sh clean --profile dev` when no other cargo process runs.
- Anti-goal read: the 900 px screen check -> no header button overlaps or clips.
