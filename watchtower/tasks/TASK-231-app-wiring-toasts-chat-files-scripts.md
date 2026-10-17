# TASK-231 App wiring: toasts, Add to chat chip, Files, Scripts

Group: S (shared files [shell.rs](packages/desktop-gpui/crates/app/src/shell.rs), [chrome.rs](packages/desktop-gpui/crates/app/src/chrome.rs), [state.rs](packages/desktop-gpui/crates/app/src/state.rs))
Class: code

## Brief

Goal: The app shows explorer notices as toasts, attaches files to the focused chat as Electron does, fills the new Files rows, and offers Electron's script actions.

Change: notices are not subscribed, Add to chat inserts a text path into an agent tab only, Files rows have no icon or error, and Scripts has Run and Stop only -> Electron behavior. Q-122, Q-124, Q-130, Q-132, Q-135, Q-136.

Boundaries:

- Notices: subscribe `ExplorerNoticeEvent` and `PullRequestNoticeEvent` and call `Shell::show_toast(ok, message, cx)`.
- Chat target: follow Electron's `resolveFocusedChatTarget` ([packages/app/src/composer/focused-chat-target.ts](packages/app/src/composer/focused-chat-target.ts) line 43): the focused tab when it is a chat (agent or draft; a `LazyAgent` tab counts, and is materialized before the attach), else its parent tab, else Electron's next fallback. Call `Explorer::set_chat_target` whenever that result changes. Put the rule in a pure function with a test.
- Add to chat (owner answer 2026-10-18, Q-136): attach a chip to the focused chat's composer with `Composer::add_raw_attachment(label, attachment, cx)`, as Electron's workspace-file attachment: label = file name; attachment = `{"type":"text","mimeType":"text/plain","title":<file name>,"text":"Workspace file: <path>"}` ([packages/app/src/attachments/workspace-file.ts](packages/app/src/attachments/workspace-file.ts) lines 83-97). Then focus that tab. This replaces the TASK-203 text mention for `ExplorerFileEvent::AddToChat`.
- Files: answer `CommandFileEvent::FileQuery` with `set_file_rows`, using the explorer's file icon for each path (`crates/explorer/src/file_icons.rs`), and `set_file_error` on failure. Keep dropping answers to older queries.
- Scripts (Q-130): for a service with a URL, show URL rows that open with `cx.open_url` (prefer `proxy_url`, as Electron's [workspace-scripts-button.tsx](packages/app/src/screens/workspace/workspace-scripts-button.tsx)); "View terminal" opens the script's terminal tab by `terminal_id`; "Restart" stops, waits for the daemon to report the script stopped, then starts, as Electron does (`workspace-scripts-button.tsx` lines 617-650). Follow Electron's item order and labels.
- Fix the `WorkspaceScript` test literal at `state.rs` line 808 for the TASK-220 fields.

Files:

- [packages/desktop-gpui/crates/app/src/shell.rs](packages/desktop-gpui/crates/app/src/shell.rs)
- [packages/desktop-gpui/crates/app/src/chrome.rs](packages/desktop-gpui/crates/app/src/chrome.rs) (Scripts menu)
- [packages/desktop-gpui/crates/app/src/state.rs](packages/desktop-gpui/crates/app/src/state.rs) (test literal only)

Interfaces:

- Consumes: `ExplorerNoticeEvent { ok: bool, message: String }` from TASK-222.
- Consumes: `PullRequestNoticeEvent { ok: bool, message: String }` from TASK-222.
- Consumes: `Explorer::set_chat_target(&mut self, available: bool, cx: &mut Context<Explorer>)` from TASK-222.
- Consumes: `FileResult { path: String, icon: Arc<Image> }` from TASK-228.
- Consumes: `CommandCenter::set_file_rows(&mut self, rows: Vec<FileResult>, cx: &mut Context<CommandCenter>)` from TASK-228.
- Consumes: `CommandCenter::set_file_error(&mut self, message: Option<String>, cx: &mut Context<CommandCenter>)` from TASK-228.
- Consumes: `WorkspaceScript { name: String, kind: String, lifecycle: String, health: Option<String>, terminal_id: Option<String>, exit_code: Option<i32>, proxy_url: Option<String>, local_proxy_url: Option<String>, public_proxy_url: Option<String> }` from TASK-220.

Expected result:

- A failed branch switch shows an error toast at the top center.
- Add to chat on a file with an agent tab focused adds a chip named after the file; with a draft tab focused it adds the chip to the draft; with neither, the item is hidden.
- The Scripts menu of a running service shows its URL, View terminal, Restart, and Stop.

Anti-goal: a workspace click to the first tab paint stays as before; drift gauge; no new request on a workspace click (read the live `daemon.log` request counts per 30 s window before and after one click).

## Verify

- `cd packages/desktop-gpui && CARGO_BUILD_JOBS=4 ./cargo.sh test -p bachuc-app` -> passes, with a test for the workspace-file attachment JSON and the chat-target rule.
- `CARGO_BUILD_JOBS=4 ./cargo.sh build -p bachuc-app` -> builds.
- UI check on live (scratch repo): a forced branch switch error shows a toast at the top center.
- Owner hand test: Explorer right-click, Add to chat, with an agent tab and with a draft tab focused -> a chip named after the file. The file menu opens only by right-click (Q-133).
- UI check on live: a service script shows its URL row and View terminal.
- Anti-goal: request counts in the live `daemon.log` show no new request type after a workspace click.
