# TASK-211 Outcome

## Status

DONE

## Changed

- `packages/desktop-gpui/crates/explorer/src/branch_switcher.rs` (new): `BranchSwitcher`, the label trigger, the searchable list, and the inline confirm. It also holds the pure rules `rank_branches`, `should_switch`, `offers_stash`, `restore_index`, and `cache_fresh`, with 7 tests.
- `packages/desktop-gpui/crates/explorer/src/explorer.rs`: the Explorer owns one `Entity<BranchSwitcher>`. It sets the branch in `apply_status`, resets the switcher in `set_cwd`, asks for the status (and commits, when open) on `BranchSwitched`, and draws the switcher in place of the branch label in `render_changes_header`.
- `packages/desktop-gpui/crates/explorer/src/lib.rs`: `mod branch_switcher;`.

## Contract

- The spec has no "Produces:" line. Each "Consumes:" line from TASK-204 (`switch_branch`, `stash_save`, `stash_pop`, `stash_list`, `StashEntry`) is used by its exact name. `GitStatus` is not needed: the branch comes from `checkout_details`, which the Changes tab already loads.
- Branches load only in `BranchSwitcher::open`, with `branch_suggestions(cwd, None, Some(200))`. The list stays fresh for 15 s. Opening the Changes tab sends no branch request.
- A pick of the current branch does nothing. A switch error that contains "uncommitted" (any case) shows the "Stash & Switch" confirm. Yes runs `stash_save` with the current branch name, then `switch_branch`. Cancel keeps the branch.
- After a plain switch succeeds, the switcher lists Paseo stashes (`paseo_only: true`). If one has the target branch, the "Restore" / "Later" confirm shows, and Restore runs `stash_pop`. After "Stash & Switch", there is no restore check, as in Electron.
- New event type `BranchSwitched` with its own `EventEmitter`, crate-private. `ExplorerEvent` is unchanged.
- Safe default: the GPUI client has no toast host, so a failed switch, stash, or pop shows an inline notice with one OK button. Electron's "Stashed changes restored" success toast is not drawn.
- Safe default: a detached HEAD keeps the passive "Unknown" label. Electron hides the switcher there.
- The search ranks a port of `filterAndRankComboboxOptions` tiers (exact, whole word, prefix, word start, substring, letters in order within one word), without the fuzzy tier and the compact-text pass. Enter picks the first row. Escape or a click outside closes the list. Arrow keys do not move a highlight.

## Verified

- `./cargo.sh test -p bachuc-explorer branch` -> 8 passed (7 switcher tests, including the stash decision and restore pick tests).
- `./cargo.sh test -p bachuc-explorer` -> 28 + 38 + 21 passed.
- `./cargo.sh build -p bachuc-explorer` -> exit 0.
- `./cargo.sh fmt --check -p bachuc-explorer` -> exit 0.
- `./cargo.sh clippy -p bachuc-explorer --tests` -> no warnings in the changed files. The existing warnings are in `file_view.rs:86` and the `bachuc-protocol` build script.
- `./cargo.sh metadata --format-version 1` -> exit 0.
- `rg "branch_suggestions\(" crates/explorer/src` -> one call, in `BranchSwitcher::load`, which only `open` calls.
- UI check, clean repo switch updates the label: DEFERRED (UI checker).
- UI check, dirty repo stash and switch, then the restore confirm: DEFERRED (UI checker).
- Anti-goal, no `branch_suggestions_request` in the live `daemon.log` on tab open: DEFERRED (UI checker). The "before" read was not taken by the builder.

## Lessons

- In GPUI 0.3.9, `flex_shrink` takes an `f32`; use `.flex_shrink(1.)`.
- A rustfmt hook reformats `.rs` files on write in this checkout, so line numbers move after a Write.
- `bachuc-app` was not rebuilt. The Explorer public API did not change.
