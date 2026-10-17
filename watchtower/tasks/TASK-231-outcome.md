# TASK-231 Outcome

## Status

DONE

## Changed

- `packages/desktop-gpui/crates/app/src/shell.rs`: explorer and PR notices go to toasts; the chat target rule and its sync to the Explorer; Add to chat attaches a workspace-file chip; file queries answer with `set_file_rows` and `set_file_error`; `view_script_terminal`; the `scripts_restart` field and the restart hook on `ScriptsChanged`; 2 new tests.
- `packages/desktop-gpui/crates/app/src/chrome.rs`: the Scripts menu row in Electron's order (View service, View terminal, Restart, Stop or Run), the URL row, `restart_script`, `restart_stopped_scripts`, `script_url`, and 1 new test.
- `packages/desktop-gpui/crates/app/src/state.rs`: the `WorkspaceScript` test literal has the 3 proxy URL fields.

## Contract

- Consumes, by exact name: `ExplorerNoticeEvent` and `PullRequestNoticeEvent` (each calls `Shell::show_toast(ok, message, cx)`), `Explorer::set_chat_target`, `FileResult { path, icon }`, `CommandCenter::set_file_rows`, `CommandCenter::set_file_error`, and the `WorkspaceScript` proxy URL fields. `bachuc_explorer::file_icons::file_icon` gives each file row its icon, from the last path segment.
- `PullRequestNoticeEvent` has its own subscription. The `PullRequestEvent` match stays exhaustive. `create_workspace` and `on_client_event` did not change.
- `chat_target(focused, group_actives, is_chat)` is a pure rule after Electron's `resolveFocusedChatTarget`. It returns the focused tab when it is a chat (agent, lazy agent, or draft). Else it returns the active tab of the first group, in reading order, that is a chat. `sync_chat_target` calls `Explorer::set_chat_target` from `sync_plugin_active` (select, tab activated, tab closed, host switch), from the end of `put_pane` (a tab replaced in place emits no `Activated`), and when the Explorer is made.
- Safe default: GPUI tabs record no parent tab, so Electron's middle step (the chat a file was opened beside) is skipped.
- Add to chat: a lazy agent tab is materialized first. The attachment is `workspace_file_attachment(path)`, with label = the file name and JSON `{"type":"text","mimeType":"text/plain","title":<name>,"text":"Workspace file: <path>"}`. The path is the Explorer's workspace-relative path, as in Electron. Then the tab is activated and focused.
- Safe default: `WorkspaceDraftPane` keeps its composer private, and `workspace_draft.rs` is outside this TASK's files. A draft target therefore gets the chip through `WorkspaceDraftPane::seed_fork(None, label, attachment, ..)`, which only calls `Composer::add_raw_attachment`.
- Files: a failed query calls `set_file_error(Some(..))` and shows no toast. A missing cwd answers `set_file_rows(vec![])`. Answers to older queries, or to a closed center, are still dropped.
- Scripts: `script_url` picks `proxy_url`, then `public_proxy_url`, then `local_proxy_url`, and only for a running service. A URL shows as a "View <name> service" eye button and as a URL row under the header (the label has no protocol). Both call `cx.open_url`. "View <name> terminal" shows only while the script runs and has a `terminal_id`. It activates the open terminal tab with that id, or attaches a new tab with `TerminalView::attach`. "Restart <name>" kills the terminal and records `workspace_id/name`. The next `ScriptsChanged` that lists the script as not running starts it again. Every button has `Role::Button`, an aria label, and an `AccessibleAction::Click` handler (through `pressable`).
- Not done (matches Electron, outside the spec): no Copy URL button, no route chooser, no Direct `localhost:<port>` URL (`WorkspaceScript` has no port).

## Verified

- `CARGO_BUILD_JOBS=4 ./cargo.sh test -p bachuc-app` -> 79 passed. New: `the_chat_target_is_the_focused_chat_else_the_first_group_with_a_chat`, `a_workspace_file_attaches_as_electron_text_attachment`, `a_running_service_opens_its_proxy_url_first`.
- `CARGO_BUILD_JOBS=4 ./cargo.sh build -p bachuc-app` -> builds. The only warning existed before (`next_theme`, `state.rs:424`).
- `./cargo.sh fmt --check -p bachuc-app` -> exit 0.
- `./cargo.sh clippy -p bachuc-app --tests` -> no warning in `shell.rs`, `chrome.rs`, or `state.rs`.
- `./cargo.sh metadata --format-version 1 > /dev/null` -> exit 0.
- GitNexus impact (the index is 71 commits stale): `sync_plugin_active` CRITICAL and `put_pane` CRITICAL. Each got one appended `sync_chat_target` call and kept its signature. `add_file_to_chat`, `query_files`, and `script_row` were not in the index. `rg` shows each of them has callers only in `shell.rs` and `chrome.rs`.
- UI check: a forced branch switch error shows a toast at the top center -> DEFERRED (UI checker). Top-center placement belongs to TASK-226.
- Owner hand test: Add to chat with an agent tab and with a draft tab focused -> DEFERRED (owner hand test).
- UI check: a service script shows its URL row and View terminal -> DEFERRED (UI checker).
- Anti-goal (no new request type after a workspace click) -> DEFERRED (UI checker). By code, `sync_chat_target` reads local state only and sends no request.

## Lessons

- A tab replaced in place keeps its key, so `Workspace::open` emits no `Activated`. Any shell state that depends on what the tabs show must also resync in `put_pane`.
- A private field of a sibling module view (`WorkspaceDraftPane::composer`) is not reachable from `shell.rs`. Reuse a public method that already does the job, or ask for a getter in a TASK that owns that file.
- Closing an attached script terminal tab kills its daemon terminal (`PaneContent::release`), and the kill stops the script. This is the same as for any other terminal tab.
