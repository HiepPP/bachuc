# TASK-284 What next Send marks the turn

Group: C (writes `packages/desktop-gpui/plugins/next-prompt-actions/src/**` only; starts after TASK-282 by order)
Class: code

## Brief

Goal: A manual What next Send marks the agent's turn as pending before it opens the Board, and clears the mark when the send does not land.

Change: `send_plan` returns `[OpenBoard, Rpc]` -> `[MarkPending(agent_id), OpenBoard, Rpc]`, and a failed send clears the mark.

Boundaries:

- Follows [ADR-0058](watchtower/decisions/ADR-0058-gpui-pending-turn-mark-over-waiting-for-the-board-poll.md): only the manual Send marks a turn. `start` (Start in new thread) and server-side automatic sends do not.
- Planner call: mark on every manual Send, with or without back-to-board. The Board then shows the right column when Hiep opens it later.
- The mark must be set in the same update as the click, before `open_surface`. Add no await before `OpenBoard`.
- IF `ctx.client(cx)` is `None` THEN skip the mark; the Send works as before.
- This changes the expected value of the existing test `back_to_board_opens_before_send` in `rules.rs`: the plan length goes from 2 to 3. The change is intended per ADR-0058. Replace that test with the red test below.
- Trap: `act` in `model.rs`: it is shared by `start` and `send`, and it drops the result. Keep `act` and `start` unchanged. In `send`, wrap the RPC task: read the result with `send_landed`, clear the mark when it is `false`, then pass the result on to `act`.
- Trap: `SendOutput` in `contracts.rs` has `#[allow(dead_code)]` on `sent`. Remove the attribute once `send_landed` reads the field.

How:

- Add `SendStep::MarkPending(String)` and put it first in `send_plan` in `rules.rs`.
- Add `send_landed` in `rules.rs`: `true` only for `Ok` with `sent: true`.
- In `PanelModel::send` in `model.rs`, run `MarkPending` with `Client::mark_turn_pending`, and call `Client::clear_turn_pending` when `send_landed` is `false`.

Files:

- [packages/desktop-gpui/plugins/next-prompt-actions/src/rules.rs](packages/desktop-gpui/plugins/next-prompt-actions/src/rules.rs) (`SendStep::MarkPending`, `send_plan`, `send_landed`, tests)
- [packages/desktop-gpui/plugins/next-prompt-actions/src/model.rs](packages/desktop-gpui/plugins/next-prompt-actions/src/model.rs) (`send` marks and clears)
- [packages/desktop-gpui/plugins/next-prompt-actions/src/contracts.rs](packages/desktop-gpui/plugins/next-prompt-actions/src/contracts.rs) (drop the `dead_code` allow on `SendOutput.sent`)

Interfaces:

- Consumes: `Client::mark_turn_pending(&mut self, agent_id: &str, cx: &mut Context<Client>)` from TASK-282
- Consumes: `Client::clear_turn_pending(&mut self, agent_id: &str, cx: &mut Context<Client>)` from TASK-282

Expected result:

- WHEN Hiep clicks Send on a What next card THE panel SHALL mark the agent before it opens the Board and before the send RPC starts.
- WHEN the send RPC answers `sent: true` THE panel SHALL keep the mark.
- IF the send RPC fails or answers `sent: false` THEN THE panel SHALL clear the mark.
- WHEN Hiep clicks Start in new thread THE panel SHALL set no mark.

Red tests:

File: [packages/desktop-gpui/plugins/next-prompt-actions/src/rules.rs](packages/desktop-gpui/plugins/next-prompt-actions/src/rules.rs) (inside the existing `tests` module; add `send_landed` to its `use super::{...}` list and `SendOutput` to its `use crate::contracts::{...}` list)

```rust
    #[test]
    fn send_marks_the_turn_before_it_opens_the_board() {
        let scope = Scope {
            server_id: "s".into(),
            agent_id: "a".into(),
            workspace_id: "w".into(),
        };
        let on = send_plan(true, scope.clone(), vec!["k".into()]);
        assert_eq!(on.len(), 3);
        assert_eq!(on[0], SendStep::MarkPending("a".into()));
        assert_eq!(on[1], SendStep::OpenBoard);
        assert!(matches!(on[2], SendStep::Rpc(_)));
        let off = send_plan(false, scope, vec!["k".into()]);
        assert!(matches!(&off[..], [SendStep::MarkPending(id), SendStep::Rpc(_)] if id == "a"));
    }

    #[test]
    fn only_an_acknowledged_send_keeps_the_mark() {
        assert!(send_landed(&Ok(SendOutput { sent: true })));
        assert!(!send_landed(&Ok(SendOutput { sent: false })));
        assert!(!send_landed(&Err(anyhow::anyhow!("Agent is busy."))));
    }
```

Anti-goal: Send keeps zero awaits before `OpenBoard`, so the Board opens in the same update as the click; tripwire; read from the order in `send_plan` and the loop in `PanelModel::send` before the change, after the `model.rs` change, and at completion.

## Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-plugin-next-prompt-actions rules` -> both red tests pass, and `back_to_board_opens_before_send` no longer exists.
- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-plugin-next-prompt-actions` -> all tests pass.
- Read `PanelModel::send` in `model.rs` -> `MarkPending` and `OpenBoard` run in the same loop with no await, and `start` calls no pending method.
- Read `PanelModel::send` -> with no client, the loop skips the mark and still opens the Board and sends.
