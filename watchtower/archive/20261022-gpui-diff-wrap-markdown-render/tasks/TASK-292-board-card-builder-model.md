# TASK-292 Board card shows builder provider, model, and effort

Group: D (writes the client agent getter and the GPUI watchtower-board model files; no other TASK writes them)
Class: code

## Brief

Goal: When a TASK on the GPUI Watchtower board has a builder agent, its card shows that agent's provider, model, and effort on the second line, in the same muted style.

Change: second line "Builder A, active 5m ago" -> "Builder A, active 5m ago · claude, claude-opus-5-5, high".

Boundaries:

- Owner choice on 2026-10-22: use the real builder agent only. A row with no matched builder (planned rows, and done rows whose builder was archived) keeps today's text. Do not guess a route from `Class:`.
- Owner choice on 2026-10-22: append to the existing second line from `secondary_line` in [packages/desktop-gpui/plugins/watchtower-board/src/model/overview_view.rs](packages/desktop-gpui/plugins/watchtower-board/src/model/overview_view.rs). Add no new line, no new color, and no new font size.
- Format: `<line> · <provider>, <model>, <effort>`. Trim each part and skip an empty or missing one. When `<line>` is empty, show the run part alone with no leading `·`. When all three are missing, add no `·`. Show ids as they come (for example `claude-opus-5-5`, `high`); no label lookup.
- Append for every row state that has a builder: Waiting, Running, Done, and Blocked.
- Effort is the agent's raw `thinkingOptionId`, which `AgentExtra::thinking` in [packages/desktop-gpui/crates/client/src/core.rs](packages/desktop-gpui/crates/client/src/core.rs) already stores from the snapshot.
- Trap: `Client::agent_thinking` in [packages/desktop-gpui/crates/client/src/lib.rs](packages/desktop-gpui/crates/client/src/lib.rs) returns None until the model's thinking options load for the composer. Do not use it; add a plain getter of the stored id.
- Trap: `AgentSummary` in [packages/desktop-gpui/crates/client/src/model.rs](packages/desktop-gpui/crates/client/src/model.rs) is built as a struct literal in 7 files. Do not add a field to it; use a getter, as `agent_updated_at` does.
- Non-goal: the frozen React client in [hiep-plugins/plugins/watchtower-board/client](hiep-plugins/plugins/watchtower-board/client) does not change.

How:

- Add `agent_thinking_option_id(&self, agent_id: &str) -> Option<String>` to `Core` in `core.rs` and to `Client` in `lib.rs`, shaped like `agent_updated_at`.
- Add `model: Option<String>` and `thinking: Option<String>` to `AgentCandidate` in `actions.rs`; fill them in `candidates` in `store.rs` from `AgentSummary::model` and the new getter.
- Add `provider: String`, `model: Option<String>`, and `thinking: Option<String>` to `TaskBuilder` in `builders.rs`; copy them in `match_builders`.
- In `secondary_line`, append the run part from the builder after the existing text.
- Add one assertion to the `builders.rs` tests that `match_builders` copies `provider`, `model`, and `thinking`, and one test of the new getter in the client tests, shaped like the `agent_thinking` checks in [packages/desktop-gpui/crates/client/tests/composer.rs](packages/desktop-gpui/crates/client/tests/composer.rs): a known agent, an unknown agent, and an agent with no `thinkingOptionId`.

Files:

- [packages/desktop-gpui/crates/client/src/core.rs](packages/desktop-gpui/crates/client/src/core.rs) (add `agent_thinking_option_id`)
- [packages/desktop-gpui/crates/client/src/lib.rs](packages/desktop-gpui/crates/client/src/lib.rs) (add `Client::agent_thinking_option_id`)
- [packages/desktop-gpui/plugins/watchtower-board/src/model/actions.rs](packages/desktop-gpui/plugins/watchtower-board/src/model/actions.rs) (`AgentCandidate` fields)
- [packages/desktop-gpui/plugins/watchtower-board/src/store.rs](packages/desktop-gpui/plugins/watchtower-board/src/store.rs) (fill the new fields in `candidates`)
- [packages/desktop-gpui/plugins/watchtower-board/src/model/builders.rs](packages/desktop-gpui/plugins/watchtower-board/src/model/builders.rs) (`TaskBuilder` fields, copy in `match_builders`, test helpers)
- [packages/desktop-gpui/plugins/watchtower-board/src/model/overview_view.rs](packages/desktop-gpui/plugins/watchtower-board/src/model/overview_view.rs) (`secondary_line` run part, the `builder` test helper, red tests)
- [packages/desktop-gpui/crates/client/tests/composer.rs](packages/desktop-gpui/crates/client/tests/composer.rs) (a test of `agent_thinking_option_id`)

Expected result:

- WHILE a builder runs with provider, model, and effort, THE card SHALL show `Builder A, active 5m ago · claude, claude-opus-5-5, high`.
- WHILE a builder waits for permission, THE card SHALL show `Builder A asks permission · claude, claude-opus-5-5, high`.
- IF the builder has no model or no effort THEN THE card SHALL skip that part, for example `Builder A, active 5m ago · codex`.
- IF a row has no builder THEN THE card SHALL show the same text as before.
- WHEN a done row still has its builder THE card SHALL show `committed abc1234 · claude, claude-opus-5-5, high`.

Red tests (pure logic, no UI):

File: [packages/desktop-gpui/plugins/watchtower-board/src/model/overview_view.rs](packages/desktop-gpui/plugins/watchtower-board/src/model/overview_view.rs) (the existing `tests` module; the `builder` helper gains `provider: "claude".into(), model: None, thinking: None`)

```rust
    fn run_builder(state: BuilderState, model: Option<&str>, thinking: Option<&str>) -> TaskBuilder {
        TaskBuilder {
            model: model.map(str::to_string),
            thinking: thinking.map(str::to_string),
            ..builder(state, Some(5 * 60_000))
        }
    }

    #[test]
    fn secondary_line_appends_the_builder_provider_model_and_effort() {
        let running = plan_row(BarKind::Running, Some("A"));
        let full = run_builder(BuilderState::Running, Some("claude-opus-5-5"), Some("high"));
        assert_eq!(
            secondary_line(&running, RowState::Running, Some(&full), None, None),
            "Builder A, active 5m ago · claude, claude-opus-5-5, high"
        );
        let waiting = run_builder(BuilderState::Waiting, Some("claude-opus-5-5"), Some("high"));
        assert_eq!(
            secondary_line(&running, RowState::Waiting, Some(&waiting), None, None),
            "Builder A asks permission · claude, claude-opus-5-5, high"
        );
        let done = plan_row(BarKind::Done, Some("A"));
        assert_eq!(
            secondary_line(&done, RowState::Done, Some(&full), Some("code"), Some("abc1234")),
            "committed abc1234 · claude, claude-opus-5-5, high"
        );
    }

    #[test]
    fn secondary_line_skips_missing_run_parts() {
        let running = plan_row(BarKind::Running, Some("A"));
        let bare = TaskBuilder {
            provider: "codex".into(),
            ..run_builder(BuilderState::Running, None, None)
        };
        assert_eq!(
            secondary_line(&running, RowState::Running, Some(&bare), None, None),
            "Builder A, active 5m ago · codex"
        );
        let empty = TaskBuilder {
            provider: String::new(),
            ..run_builder(BuilderState::Running, None, None)
        };
        assert_eq!(
            secondary_line(&running, RowState::Running, Some(&empty), None, None),
            "Builder A, active 5m ago"
        );
        let spaces = TaskBuilder {
            model: Some("  ".into()),
            ..run_builder(BuilderState::Running, None, Some("high"))
        };
        assert_eq!(
            secondary_line(&running, RowState::Running, Some(&spaces), None, None),
            "Builder A, active 5m ago · claude, high"
        );
        let blocked = plan_row(BarKind::Blocked, None);
        assert_eq!(
            secondary_line(&blocked, RowState::Blocked, Some(&spaces), Some("risky"), None),
            "Blocked, risky · claude, high"
        );
        let planned = plan_row(BarKind::Planned, None);
        assert_eq!(
            secondary_line(&planned, RowState::Planned, Some(&spaces), None, None),
            "claude, high"
        );
    }
```

Anti-goal: every existing `secondary_line_covers_each_row_state` assertion keeps its exact expected string, except the 3 that pass a builder, which only gain ` · claude`: "Builder A asks permission", "Builder A, active 5m ago", and "Builder, idle"; tripwire; read from `./cargo.sh test -p bachuc-plugin-watchtower-board` before the change, after the `secondary_line` change, and at completion.

## Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-plugin-watchtower-board` -> all tests pass, including the 2 new tests.
- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-client` -> all tests pass.
- `cd packages/desktop-gpui && ./cargo.sh clippy -p bachuc-plugin-watchtower-board -p bachuc-client -- -D warnings` -> no warnings.
- `git diff -- packages/desktop-gpui/crates/client/src/model.rs hiep-plugins/plugins/watchtower-board` -> no change.
- Owner check on a live GPUI build during a watchtower implement run: a running TASK card shows `· claude, <model id>, <effort>` after the builder text, in the same muted color and size; a planned card shows `Group X, <class>` as before.
- Anti-goal: in the test diff, the only changed expected strings in `secondary_line_covers_each_row_state` are rows that pass a builder, and each change only appends ` · claude`.
