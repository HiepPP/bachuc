# TASK-222 Outcome

## Status

DONE

## Changed

- `packages/desktop-gpui/crates/explorer/src/file_menu.rs`: `EntryFeatures` gained `discard` and `chat_target`. `menu_entries` shows Discard changes only with `discard`, and Add to chat only with `chat_target`. 2 new tests; the old tests use `..ALL` for the new fields.
- `packages/desktop-gpui/crates/explorer/src/branch_switcher.rs`: `Confirm::Notice` and its inline dialog are gone. A failed switch, stash, or pop and a restored stash emit `ExplorerNoticeEvent`. Pure rules `switch_failure_notice` and `restore_notice`, with 1 new test.
- `packages/desktop-gpui/crates/explorer/src/explorer.rs`: `ExplorerNoticeEvent` and its `EventEmitter`. The Explorer forwards the switcher notices, and a failed file operation or discard emits a notice. The red line under the tab rail (`action_error`, `render_action_error`) is gone. `set_chat_target`, and the `discard` and `chat_target` inputs of the menu model.
- `packages/desktop-gpui/crates/explorer/src/pull_request_view.rs`: `PullRequestNoticeEvent` and its `EventEmitter`. A failed Refresh emits it instead of the red line under the toolbar (`refresh_error` is gone). The first-load error state with Retry is unchanged. The `PrCheck` test literal has `duration` and `traits`.
- `packages/desktop-gpui/crates/explorer/src/lib.rs`: exports `ExplorerNoticeEvent` and `PullRequestNoticeEvent`.

## Contract

- Produces: `ExplorerNoticeEvent { ok: bool, message: String }` (pub fields, `Clone, Debug, PartialEq, Eq`), `impl EventEmitter<ExplorerNoticeEvent> for Explorer`, exported from `bachuc_explorer`.
- Produces: `PullRequestNoticeEvent { ok: bool, message: String }`, `impl EventEmitter<PullRequestNoticeEvent> for PullRequestView`, exported from `bachuc_explorer`. `PullRequestEvent` is unchanged.
- Produces: `Explorer::set_chat_target(&mut self, available: bool, cx: &mut Context<Explorer>)`. It starts `false`, so Add to chat stays hidden until TASK-231 calls it.
- Consumes: `Client::discard_changes_supported()` in `Explorer::entry_features`, tagged `COMPAT(checkoutDiscardChanges)` as Electron's gate is. Consumes the `PrCheck` fields `duration` and `traits` in the test literal.
- `ExplorerEvent`, `ExplorerFileEvent`, and `PullRequestEvent` are unchanged.
- Notices: a switch error that does not offer "Stash & Switch" (`ok` false, the error text); a failed stash or switch after "Stash & Switch" (`ok` false); a restore (`ok` true, "Stashed changes restored", or `ok` false with the error); a failed create, rename, duplicate, delete, or discard (`ok` false); a failed PR Refresh (`ok` false, "Failed to refresh git state." for an empty error).
- Until TASK-231 subscribes, these errors show nowhere in the GPUI app; they still go to the log.

## Verified

- `CARGO_BUILD_JOBS=4 ./cargo.sh test -p bachuc-explorer` -> 48 + 38 + 21 passed. New: `file_menu_hides_discard_on_a_host_without_the_feature`, `file_menu_hides_add_to_chat_without_a_chat_target`, `branch_switch_error_and_restore_success_map_to_notices`.
- `CARGO_BUILD_JOBS=4 ./cargo.sh build -p bachuc-app` -> builds.
- `./cargo.sh fmt --check -p bachuc-explorer` -> exit 0.
- `./cargo.sh clippy -p bachuc-explorer --tests` -> no warnings in the changed files.
- `./cargo.sh metadata --format-version 1` -> exit 0.
- Anti-goal: `./cargo.sh tree -p bachuc-explorer | rg bachuc-app` -> no match before and after.
- GitNexus impact: the index is 67 commits behind; `EntryFeatures` and `PullRequestView` were not found, and `menu_entries` and `BranchSwitcher` resolved to other files. `rg` showed every changed symbol is used only inside the explorer crate, plus `Explorer::new` and `PullRequestView::new` in `shell.rs`, which did not change.
- Screen checks: DEFERRED (UI checker), after TASK-231 shows the toasts.

## Lessons

- A struct of menu inputs with `..ALL` in the tests keeps old test literals short when a new gate is added.
