# TASK-222 Explorer toasts, Discard gate, and Add to chat gate

Group: E (shared files in the explorer crate: [explorer.rs](packages/desktop-gpui/crates/explorer/src/explorer.rs), [lib.rs](packages/desktop-gpui/crates/explorer/src/lib.rs), [branch_switcher.rs](packages/desktop-gpui/crates/explorer/src/branch_switcher.rs), [pull_request_view.rs](packages/desktop-gpui/crates/explorer/src/pull_request_view.rs), [file_menu.rs](packages/desktop-gpui/crates/explorer/src/file_menu.rs))
Class: code

## Brief

Goal: Explorer errors and notes reach the app toast as in Electron, Discard shows only on hosts that support it, and Add to chat shows only when a chat tab can take the file.

Change: errors show inline in the branch switcher and the PR view, Discard always shows, and Add to chat always shows -> events for the app toast, and two gates.

Boundaries:

- Branch switcher: `Confirm::Notice(String)` (`branch_switcher.rs` lines 134, 344, 372, 448) is what Electron shows as an error toast. Emit it as a notice instead; also emit Electron's success note "Stashed changes restored". The only event today is `BranchSwitched` (line 165). Q-122.
- PR view: the failed Refresh banner (`pull_request_view.rs` lines 554-555, 1502-1514) becomes a notice. Keep the first-load error state with Retry (lines 464-465, 1315), as Electron does. Q-124.
- File menu errors: the red line under the tab rail (TASK-213) becomes a notice. Q-132.
- Discard: show only when `Client::discard_changes_supported()` is true and the mode is Uncommitted, as [packages/app/src/git/diff-pane.tsx](packages/app/src/git/diff-pane.tsx) lines 138-141 and 174 do. Q-123.
- Add to chat: hidden, not disabled, when no chat tab is focused ([packages/app/src/panels/use-add-file-to-chat.ts](packages/app/src/panels/use-add-file-to-chat.ts) line 33, [files-panel.tsx](packages/app/src/panels/files-panel.tsx) line 49). The explorer does not know the tabs; the app tells it through `set_chat_target`. Q-136.
- `ExplorerEvent` and `ExplorerFileEvent` stay unchanged; add new event types, as CONTEXT says.
- Fix the `PrCheck` test literal at `pull_request_view.rs` line 1578 for the two fields TASK-220 adds, so the explorer tests compile again.

How:

- Add `ExplorerNoticeEvent` on `Explorer` and a separate `PullRequestNoticeEvent` on `PullRequestView`. Planner call: `shell.rs` lines 1866-1868 match `PullRequestEvent` exhaustively, so a new variant would break the app build until TASK-231; a new type keeps it building, as CONTEXT says.
- Gate the two menu items in the pure menu model in `file_menu.rs`, with tests.

Files:

- [packages/desktop-gpui/crates/explorer/src/explorer.rs](packages/desktop-gpui/crates/explorer/src/explorer.rs)
- [packages/desktop-gpui/crates/explorer/src/lib.rs](packages/desktop-gpui/crates/explorer/src/lib.rs) (exports)
- [packages/desktop-gpui/crates/explorer/src/branch_switcher.rs](packages/desktop-gpui/crates/explorer/src/branch_switcher.rs)
- [packages/desktop-gpui/crates/explorer/src/pull_request_view.rs](packages/desktop-gpui/crates/explorer/src/pull_request_view.rs)
- [packages/desktop-gpui/crates/explorer/src/file_menu.rs](packages/desktop-gpui/crates/explorer/src/file_menu.rs)

Interfaces:

- Consumes: `Client::discard_changes_supported(&self) -> bool` from TASK-220.
- Consumes: `PrCheck { name: String, status: String, url: Option<String>, workflow: Option<String>, duration: Option<String>, traits: Vec<String> }` from TASK-220.
- Produces: `ExplorerNoticeEvent { ok: bool, message: String }` with `impl EventEmitter<ExplorerNoticeEvent> for Explorer`, exported from `bachuc_explorer`, for TASK-231.
- Produces: `PullRequestNoticeEvent { ok: bool, message: String }` with `impl EventEmitter<PullRequestNoticeEvent> for PullRequestView`, exported from `bachuc_explorer`, for TASK-231. `PullRequestEvent` stays unchanged.
- Produces: `Explorer::set_chat_target(&mut self, available: bool, cx: &mut Context<Explorer>)` for TASK-231.

Expected result:

- A failed branch switch shows no inline notice and emits `ExplorerNoticeEvent { ok: false }`.
- A host without `checkoutDiscardChanges` shows no Discard item.
- With `set_chat_target(false)`, the file menu has no Add to chat item.

Anti-goal: the explorer crate must not depend on the app crate; tripwire; `./cargo.sh tree -p bachuc-explorer | rg bachuc-app` has no match before and after.

## Verify

- `cd packages/desktop-gpui && CARGO_BUILD_JOBS=4 ./cargo.sh test -p bachuc-explorer` -> passes, with the Discard gate and the Add to chat gate cases in the `file_menu` tests, and a pure test that a switch error and a restore success map to notices with `ok` false and true.
- `CARGO_BUILD_JOBS=4 ./cargo.sh build -p bachuc-app` -> builds.
- Anti-goal: `./cargo.sh tree -p bachuc-explorer | rg bachuc-app` -> no match at both reads.
