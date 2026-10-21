# TASK-282 Outcome

## Status

DONE

## Changed

- [pending.rs](packages/desktop-gpui/crates/client/src/pending.rs) (new): `PendingTurns`, `PendingTurnsChanged`, `PENDING_TURN_TTL_MS`, and the 3 red tests.
- [lib.rs](packages/desktop-gpui/crates/client/src/lib.rs): `mod pending`, the `pending_turns` field, the three `Client` methods, `EventEmitter<PendingTurnsChanged>`, the re-exports, and a private `unix_ms` helper.

## Contract

- `bachuc_client::PendingTurnsChanged`: unit struct, `Clone, Copy, Debug, PartialEq, Eq`.
- `bachuc_client::PENDING_TURN_TTL_MS: i64` = 15_000.
- `Client::mark_turn_pending(&mut self, agent_id: &str, cx: &mut Context<Client>)` emits `PendingTurnsChanged`.
- `Client::clear_turn_pending(&mut self, agent_id: &str, cx: &mut Context<Client>)` emits only when `PendingTurns::clear` returns `true`.
- `Client::pending_turn(&self, agent_id: &str) -> Option<i64>` returns the mark time in UNIX ms, or `None` at or past the limit.
- The clock is `SystemTime` since the UNIX epoch. The client crate has no `chrono` dependency and this TASK may not change its `Cargo.toml`.

## Verified

- Red: before the code, `./cargo.sh test -p bachuc-client --lib pending` failed to compile with E0425 and E0433 for the missing `PENDING_TURN_TTL_MS` and `PendingTurns` (6 errors).
- Green: the same command after the change -> 3 passed, 0 failed.
- `./cargo.sh check -p bachuc-client` -> exit 0, no warning.
- `./cargo.sh fmt --check -p bachuc-client` -> exit 0.
- I read `clear_turn_pending`: it emits only inside `if self.pending_turns.clear(agent_id)`.
- `git status` shows only my two files changed in the client crate. The other changes in the checkout belong to group D and the plan file.

## Anti-goal

Tripwire: `git diff main -- packages/desktop-gpui/crates/client/src/model.rs` must stay empty.

| Checkpoint                | Value | Source                             | Time (UNIX s) |
| ------------------------- | ----- | ---------------------------------- | ------------- |
| Before change             | empty | `git diff main --stat -- model.rs` | 1792568125    |
| After the `lib.rs` change | empty | same command                       | 1792568273    |
| Completion                | empty | same command                       | 1792568290    |

The tripwire did not cross. `ClientEvent` has no new variant.

## Lessons

- `sed -i` on macOS needs a backup suffix argument. Use the Edit tool for one-line inserts.
- `./cargo.sh` lives in `packages/desktop-gpui`. Run it from there, not from a crate folder.
