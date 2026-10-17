# TASK-203 New workspace, file links, file search, and shortcuts in the app

Group: S (shared file [packages/desktop-gpui/crates/app/src/shell.rs](packages/desktop-gpui/crates/app/src/shell.rs))
Class: code

## Brief

Goal: The app handles the new sidebar rows, explorer and terminal events, the Files search, and the new shortcuts.

Change: the new events and shortcut ids have no handler -> each one runs its Electron action.

Boundaries:

- `SidebarNavEvent::NewWorkspace { project_id }` -> `open_new_workspace` (`shell.rs` line 1300), with the project preset when the id is set. Add this arm to the `SidebarNavEvent` subscription that TASK-200 adds.
- `ExplorerFileEvent::OpenToSide` -> open a file tab in the pane to the right (split if none). `ExplorerFileEvent::AddToChat` -> `Composer::insert_path_mention` on the active agent's composer.
- `TerminalEvent::OpenFile { cwd, path, line }` -> open the file tab at the line, as `TimelineEvent::OpenFile` does today (`shell.rs` line 1787).
- `CommandFileEvent::FileQuery { query }` -> `Client::directory_suggestions` with the active workspace cwd, files only, limit 100; then `set_file_results`. `CommandFileEvent::OpenFile` -> open the file tab.
- Shortcut ids in `run_action` (`shell.rs` line 2304):
  - `"workspace.new"` -> `open_new_workspace`.
  - `"agent.new"` -> the folder picker, then `add_project` (Electron label "Open project"). `Shell` has no such helper; copy the private flow of `sidebar.rs` line 675 (or `new_workspace.rs` line 671) into a shell method. Do not edit `sidebar.rs`.
  - `"command-center.files"` -> open the command center, then `open_files_scope`.
  - `"workspace.pin"` -> toggle pin of the active workspace with `pin_workspace`.
  - `"shortcuts.dialog.toggle"` -> show or hide `ShortcutsDialog`. `binding_specs` (`actions.rs` lines 82-114) binds every table row in `SHELL_CONTEXT`, which would eat `?` in the composer. Bind this id in a context that excludes text inputs and the terminal, as `AGENT_ACTIONS` does (`actions.rs` lines 36, 100).
- Toast errors with `Shell::show_toast`.

How:

- Add the subscriptions and arms.
- Put the "is `?` allowed for this focus" rule in a pure function with a test.

Files:

- [packages/desktop-gpui/crates/app/src/shell.rs](packages/desktop-gpui/crates/app/src/shell.rs) (subscriptions, `run_action` arms)
- [packages/desktop-gpui/crates/app/src/actions.rs](packages/desktop-gpui/crates/app/src/actions.rs) (the `?` binding context)

Interfaces:

- Consumes: `SidebarNavEvent::NewWorkspace { project_id: Option<String> }` from TASK-216.
- Consumes: `ExplorerFileEvent` enum `{ OpenToSide { cwd: String, path: String }, AddToChat { cwd: String, path: String } }` from TASK-213.
- Consumes: `TerminalEvent::OpenFile { cwd: String, path: String, line: Option<u32> }` from TASK-215.
- Consumes: `Composer::insert_path_mention(&mut self, path: &str, window: &mut Window, cx: &mut Context<Composer>)` from TASK-214.
- Consumes: `CommandFileEvent::FileQuery { query: String }` and `CommandFileEvent::OpenFile { path: String }` from TASK-217.
- Consumes: `CommandCenter::open_files_scope(&mut self, window: &mut Window, cx: &mut Context<CommandCenter>)` and `CommandCenter::set_file_results(&mut self, paths: Vec<String>, cx: &mut Context<CommandCenter>)` from TASK-217.
- Consumes: `ShortcutsDialog::new(window: &mut Window, cx: &mut Context<ShortcutsDialog>) -> ShortcutsDialog` and `ShortcutsDialogEvent::Dismissed` from TASK-217.
- Consumes: shortcut action ids `"workspace.new"`, `"agent.new"`, `"command-center.files"`, `"workspace.pin"`, `"shortcuts.dialog.toggle"` from TASK-217.
- Consumes: `DirectorySuggestion { path: String, is_directory: bool }` and `Client::directory_suggestions(&mut self, cwd: &str, query: &str, include_files: bool, include_directories: bool, limit: u32, cx: &mut Context<Client>) -> Task<anyhow::Result<Vec<DirectorySuggestion>>>` from TASK-206.
- Consumes: `Shell::show_toast(&mut self, ok: bool, message: String, cx: &mut Context<Self>)` from TASK-202.
  Expected result:

- Cmd+N and the sidebar New workspace row open the New workspace screen; a project "+" presets that project.
- Cmd+P, type a file name -> matching files; Enter opens the file tab.
- Cmd+Shift+P pins or unpins the active workspace.
- `?` outside a text field shows the shortcuts dialog; inside the composer it types `?`.
- Explorer "Open to side" opens the file in the right pane; "Add to chat" inserts the path in the composer.
- A terminal file link click opens the file at its line.

Anti-goal: existing shortcuts must keep working; tripwire; Cmd+K, Cmd+T, and Cmd+W still run their actions, checked on live before the change and at completion.

## Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-app` -> passes, including the `?` focus rule test.
- UI check on live: Cmd+N, Cmd+P with a query and Enter, Cmd+Shift+P, and `?` -> each runs its action.
- UI check on live: type `?` in the composer, a terminal, the command center input, and a settings field -> it types `?` and no dialog opens.
- UI check on live: explorer "Open to side" and "Add to chat" -> the file opens to the right, and the path appears in the composer.
- Owner hand test: Cmd-click a file path in a terminal -> the file opens at the line.
- Anti-goal: Cmd+K, Cmd+T, and Cmd+W on live -> each still works at both reads.
