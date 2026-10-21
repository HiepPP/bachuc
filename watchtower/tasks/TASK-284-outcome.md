# TASK-284 outcome

## Status

DONE. Both red tests failed first, then passed. All 58 crate tests pass.

## Changed

- [rules.rs](../../packages/desktop-gpui/plugins/next-prompt-actions/src/rules.rs): add `SendStep::MarkPending(String)`, put it first in `send_plan`, add `send_landed`. Replace the test `back_to_board_opens_before_send` with the two red tests.
- [model.rs](../../packages/desktop-gpui/plugins/next-prompt-actions/src/model.rs): `PanelModel::send` runs `MarkPending` with `Client::mark_turn_pending`. A wrapper task around the RPC calls `Client::clear_turn_pending` when `send_landed` is `false`, then passes the result to `act`.
- [contracts.rs](../../packages/desktop-gpui/plugins/next-prompt-actions/src/contracts.rs): remove `#[allow(dead_code)]` from `SendOutput.sent`.

## Contract

- Consumes `Client::mark_turn_pending(&mut self, &str, &mut Context<Client>)` and `Client::clear_turn_pending(...)` from TASK-282 (`crates/client/src/lib.rs:253,259`). Names match the spec.
- `send_plan` returns `[MarkPending, OpenBoard, Rpc]` with back-to-board, and `[MarkPending, Rpc]` without.
- With no client, the loop skips the mark. The Board still opens and the send still runs.
- `act` and `start` are unchanged. `start` sets no mark.

## Verified

- Red: `./cargo.sh test -p bachuc-plugin-next-prompt-actions rules` failed to compile at 14:39 (E0432 `send_landed` missing, E0599 `SendStep::MarkPending` missing).
- Green: the same command, 9 rules tests pass. `back_to_board_opens_before_send` no longer exists.
- Full crate: 58 unit tests and 1 plugin test pass.
- `fmt --check -p bachuc-plugin-next-prompt-actions` exits 0. `clippy --all-targets` shows no warning in this crate.
- Read `PanelModel::send`: the loop has no await. `start` calls no pending method.
- No window opened. No UI checker line in this spec.

## Anti-goal

Tripwire: no await before `OpenBoard`. Source: order in `send_plan` and the loop in `PanelModel::send`. Not crossed.

| Checkpoint              | Value                                                                                                               | Time  |
| ----------------------- | ------------------------------------------------------------------------------------------------------------------- | ----- |
| Before change           | `[OpenBoard, Rpc]`, loop has no await                                                                               | 14:39 |
| After `model.rs` change | `[MarkPending, OpenBoard, Rpc]`, loop has no await; the only new await is inside the spawned wrapper after the loop | 14:40 |
| Completion              | same as above                                                                                                       | 14:41 |

## Lessons

- `AsyncApp::update` on an entity returns the closure value directly in this GPUI. Do not add `.ok()`.
- Impact analysis: GitNexus matched three `send_plan` symbols. The plugin one has LOW risk. `rg` confirmed only `model.rs` calls it.
