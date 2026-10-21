# TASK-281 outcome

## Status

DONE

## Changed

- `packages/desktop-gpui/plugins/thread-branch/src/branch.rs`: `MenuAction::OpenIssues` and `OpenPulls`, rows `open-issues` and `open-pulls` after `open-repo` (GitHub only), `menu_url`, doc comment, tests.
- `packages/desktop-gpui/plugins/thread-branch/src/store.rs`: `run_menu` merges the three open actions into one arm that opens `menu_url(&url, action)`.

## Contract

- WHEN `remote_url` contains `github.com`, `branch_menu` shows "Issues on GitHub" and "Pull requests on GitHub" right below the Open row.
- The rows open `<remote_url>/issues` and `<remote_url>/pulls`; other hosts and no remote show neither row.
- `draft_menu` is unchanged.

## Verified

- Red (11:09:20, `cargo.sh test -p bachuc-plugin-thread-branch`): compile errors E0432/E0599 for missing `menu_url`, `MenuAction::OpenIssues`, `OpenPulls`. Counts as red.
- Green (11:11, same command): 40 lib tests + 1 plugin test passed, 0 failed; the three new tests pass.
- `cargo.sh fmt --check -p bachuc-plugin-thread-branch` -> exit 0.
- `git diff -U0 branch.rs`: removed lines are the doc comment, the import list, and the full-menu line of `menu_rows_visibility`. No other expected value changed.
- `git diff --stat`: only `branch.rs` and `store.rs` changed under `thread-branch`; `crates/app` has no diff.
- Manual pill-menu check: DEFERRED (UI checker).
- `cargo clean`: left to the lead.

## Anti-goal

PASS. Existing menu rows keep their order, titles, and visibility. No existing test failed. The only changed existing expectation is the full-menu line of `menu_rows_visibility` (Intended by this TASK).

## Lessons

- none

## Lead Re-check

- 2026-10-21 lead: `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test -p bachuc-plugin-thread-branch` -> 40 + 1 passed, 0 failed.
- 2026-10-21 lead: `packages/desktop-gpui/cargo.sh fmt --check -p bachuc-plugin-thread-branch` -> exit 0.
- 2026-10-21 lead: scope matches `Files:`; GitNexus detect-changes risk low; committed `b6b50696a`.
- Anti-goal final: PASS; only the full-menu line of `menu_rows_visibility` changed, as the spec allows.
- Manual pill menu check in a GPUI build: UNVERIFIED (unattended); owner check.

## UI Checks

- 2026-10-21 lead, round 1: built `Bachuc GPUI Check.app` from `b6b50696a` (the binary holds "Pull requests on GitHub") and opened it with `open -n -g` against live. The window painted only right after a click and then kept a stale frame, so the command center search never showed a result. Pill menu rows and the opened pages: UNVERIFIED (background frame). Owner hand check needed.
