# TASK-285 Outcome

## Status

DONE

## Changed

- [branch.rs](packages/desktop-gpui/plugins/thread-branch/src/branch.rs): `MenuAction::Pull`, the `pull` row after `fetch` (shown with an upstream), `PULL_DIRTY`, `PULL_NO_UPSTREAM`, `pull_blocked`, and tests. The two existing menu tests now expect `"pull"` between `"fetch"` and `"refresh"`.
- [store.rs](packages/desktop-gpui/plugins/thread-branch/src/store.rs): `State::begin_pull`, `State::end_pull`, a `pulling` set, `Branches::pull`, the `MenuAction::Pull` arm of `run_menu`, and a test.

## Contract

- The `draft_menu` list stays `["copy-branch", "-", "fetch", "refresh"]`. A draft `MenuAction::Pull` does nothing.
- The Pull flow: `begin_pull`, `Force` `load`, `pull_blocked`, `client.git_action(cwd, GitAction::Pull)`, `Force` `load`, then `end_pull` on every path.
- No client shows "No host connection." and does not pull. Every error shows as `format!("{error:#}")`. A success shows no message.
- `pull_blocked` checks the upstream first, then dirty.

## Verified

- Red: the first `./cargo.sh test -p bachuc-plugin-thread-branch` failed to compile (7 errors, missing `pull_blocked`, `PULL_DIRTY`, `PULL_NO_UPSTREAM`, `MenuAction::Pull`, `begin_pull`, `end_pull`).
- Green: the same command passes (43 lib tests, 1 plugin test), including the 3 red tests.
- `./cargo.sh fmt --check -p bachuc-plugin-thread-branch` exits 0. `./cargo.sh clippy -p bachuc-plugin-thread-branch --all-targets` gives no warning in this crate.
- Flow read in `Branches::pull`: `end_pull` runs after the no-client check, and in the single final step for the blocked, failed, and success paths.
- Impact: GitNexus gave `UNKNOWN` for `run_menu`. `rg` shows one caller (`pills.rs` `press`) and no other `MenuAction` user. The `match` stays exhaustive.
- Owner check, clean behind branch: DEFERRED (UI checker).
- Owner check, uncommitted change: DEFERRED (UI checker).
- Verify: `./cargo.sh test -p bachuc-client -p bachuc-plugin-board -p bachuc-plugin-next-prompt-actions -p bachuc-plugin-thread-branch` on branch head 243dbd9 -> 348 passed, 0 failed (verify, 2026-10-21 14:55); the 3 red tests pass.
- Owner check (Hiep, 2026-10-21): Pull on a clean branch that is behind -> the `↓` count goes to 0 -> PASS.
- Owner check (Hiep, 2026-10-21): Pull with an uncommitted change -> the dirty message shows and nothing is pulled -> PASS.

## Anti-goal

Tripwire: the existing menu rows keep ids, order, and visibility.

| Checkpoint               | Value                                                                                                                                                 | Source                                                  | Time             |
| ------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------- | ---------------- |
| Before change            | full: `copy-branch, open-repo, open-issues, open-pulls, -, fetch, refresh`; plain: `copy-branch, -, refresh`; draft: `copy-branch, -, fetch, refresh` | `branch.rs` tests read before the edit                  | 2026-10-21 14:36 |
| After `branch.rs` change | same lists, with `pull` added between `fetch` and `refresh` for the tracked menus only                                                                | `git diff -U0` of `branch.rs`                           | 2026-10-21 14:38 |
| Completion               | same as the checkpoint above; draft and untracked lists unchanged                                                                                     | `./cargo.sh test -p bachuc-plugin-thread-branch` passed | 2026-10-21 14:38 |

The tripwire did not cross.

- Verify read: the tracked-menu test lists differ from main only by `"pull"` between `"fetch"` and `"refresh"` (929cca9); verify, 2026-10-21.
- Result: PASS; existing rows kept their ids, order, and rules.

## Lessons

- `spawn_load` drops the info, so the pre-pull read uses `load`.
- A rustfmt run changed the import and row layout in `branch.rs`; run `fmt` before the diff check.
