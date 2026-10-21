# TASK-283 Outcome

## Status

BLOCKED

Blocked: BUILT, UNVERIFIED (Q-183). Every automated check passed. The two owner checks need Hiep on live GPUI.

## Changed

- [pending.rs](packages/desktop-gpui/plugins/board/src/model/pending.rs): new. `shows_pending` and `apply_pending`, with the 8 red tests.
- [mod.rs](packages/desktop-gpui/plugins/board/src/model/mod.rs): registers `pending`.
- [store.rs](packages/desktop-gpui/plugins/board/src/store.rs): `BoardStore` keeps `raw` (the last decoded answer) and `snapshot` (the shown one). It subscribes to `PendingTurnsChanged`, rebuilds the shown snapshot, clears confirmed marks after each answer, and arms one expiry timer.

## Contract

- Consumed from TASK-282 with the exact names: `PendingTurnsChanged`, `PENDING_TURN_TTL_MS`, `Client::pending_turn`, `Client::clear_turn_pending`.
- `snapshot()` and `revision()` keep their signatures. They now return the shown snapshot.
- `rebuild` is the only place that sets `snapshot` and bumps `revision`. It returns early when the shown snapshot equals the last one.
- `refresh` runs only from `pending_changed`, and only when a run newly shows as pending. A clear and the expiry timer never refresh.
- A mark with no run in the raw snapshot adds no card and is not cleared. It waits for its run or the 15,000 ms limit.
- A `Changed` answer that decodes to an equal snapshot now keeps the revision. Before, it bumped it. The views draw the same content in both cases.

## Verified

- Red: `cargo.sh test -p bachuc-plugin-board pending` failed to compile (`E0425`/`E0422`, `shows_pending` and `apply_pending` missing) at 07:40 UTC, before any other edit.
- Green: the same command passes 8 of 8.
- `cargo.sh test -p bachuc-plugin-board`: 105 unit tests and 1 integration test pass.
- `cargo.sh fmt --check -p bachuc-plugin-board`: exit 0. `cargo.sh clippy -p bachuc-plugin-board --all-targets`: no warning in the board crate.
- Read `store.rs`: `interval` reads `self.snapshot`, the shown snapshot. `revision += 1` sits after the equality guard in `rebuild`. `refresh` is called only in `pending_changed`.
- Owner check, Send then Board opens with the card in Running: DEFERRED (UI checker).
- Owner check, send to a busy agent, card returns to Just finished: DEFERRED (UI checker).

## Anti-goal

Goal: 0 redraws per `PendingTurnsChanged` with no card change, and at most 1 `board.snapshot` request per Send. The tripwire did not cross.

| Checkpoint              | Name                                     | Value                                                                                 | Source                                                            | Time (UTC) |
| ----------------------- | ---------------------------------------- | ------------------------------------------------------------------------------------- | ----------------------------------------------------------------- | ---------- |
| Before the store change | redraws per `PendingTurnsChanged`        | 0 (no subscription yet)                                                               | read of `store.rs`, `revision += 1` runs on each `Changed` answer | 07:40      |
| Before the store change | test `no_mark_returns_an_equal_snapshot` | red (symbol missing)                                                                  | `cargo.sh test ... pending`                                       | 07:40      |
| After the store change  | redraws per event with no card change    | 0: `rebuild` returns before `revision += 1` and `cx.notify()`                         | read of `store.rs` equality guard                                 | 07:41      |
| After the store change  | requests per Send                        | at most 1: `refresh` only when `added`; `confirm_pending` and the timer never call it | read of `store.rs`                                                | 07:41      |
| Completion              | test `no_mark_returns_an_equal_snapshot` | pass                                                                                  | `cargo.sh test -p bachuc-plugin-board` (105 pass)                 | 07:41      |

No store-level unit test covers the guard, because `BoardStore` needs a GPUI app. The checkpoint values come from the code read and the pure-function test.

## Lessons

- The client offers no list of marks. The store asks `pending_turn` once for each raw run, which is enough because the overlay never adds a card.
- Compute the mark map once in `rebuild`, then pass it to `apply_pending`. This keeps the shown snapshot, the pending set, and the expiry time from one read.
- A mark on a run that already runs stays until the next answer clears it. This costs no redraw.
