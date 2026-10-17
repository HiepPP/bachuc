# TASK-203 Outcome

## Status

DONE. Every UI check and the owner hand test are DEFERRED.

## Changed

- `packages/desktop-gpui/crates/app/src/actions.rs`: `binding_context(action)` and `outside_text_context()`. `shortcuts.dialog.toggle` binds in `BachucShell && !Input && !Terminal && !NumberInput && !TimeField && !Combobox`. Every other id keeps its old context. New test `question_mark_runs_only_outside_text_fields_and_the_terminal`.
- `packages/desktop-gpui/crates/app/src/shell.rs`: the subscriptions and `run_action` arms below, plus the new fields `file_query`, `shortcuts_dialog`, and `side_file`. `command` now holds a `Vec<Subscription>`.

## Contract

- Every "Consumes:" line is used by its exact name: `SidebarNavEvent::NewWorkspace { project_id }`, `ExplorerFileEvent::{OpenToSide, AddToChat}`, `TerminalEvent::OpenFile { cwd, path, line }`, `Composer::insert_path_mention`, `CommandFileEvent::{FileQuery, OpenFile}`, `CommandCenter::open_files_scope`, `CommandCenter::set_file_results`, `ShortcutsDialog::new`, `ShortcutsDialogEvent::Dismissed`, `Client::directory_suggestions(cwd, query, true, false, 100, cx)` with `DirectorySuggestion`, `Shell::show_toast`, and the 5 action ids.
- `SidebarNavEvent::NewWorkspace` and `workspace.new` run `Shell::new_workspace(project_id)`. It calls `open_new_workspace` with `NewWorkspacePreset { project_id, .. }`, then closes a page surface.
- `agent.new` runs `Shell::open_project`: a folder dialog with the prompt "Open project", then `add_project`, then the New workspace screen on the added project. This is how Electron's add project flow ends. A failure shows `show_toast(false, ..)`. `sidebar.rs` is unchanged.
- `command-center.files` opens the command center if it is closed, then calls `open_files_scope`.
- `FileQuery` sends `directory_suggestions` for the selected workspace cwd. It keeps only files and calls `set_file_results`. A `file_query` generation drops the answer to an older query, and an answer to a closed or replaced center is dropped too. With no cwd, the list is empty. An error gives a toast and an empty list. `OpenFile` removes the center and opens the file tab in the selected cwd.
- `workspace.pin` reads `WorkspaceNode::pinned` of the selected workspace and sends `pin_workspace(id, !pinned)`. In the home view it does nothing.
- `shortcuts.dialog.toggle` shows `ShortcutsDialog` over the shell, or hides it. It also renders over Settings. `Dismissed` and the hide put the focus back on the active pane.
- `TerminalEvent::OpenFile` runs `open_file(cwd, path, line)`, as `TimelineEvent::OpenFile` does. `Exited` does nothing, as before.
- `OpenToSide`: if the file tab is open, the app shows it. Else, if a group lies to the right of the focused group, the app focuses that group and opens the file there. Else the app splits right, and the split's `PaneEvent::NewTab` opens the file (`side_file`) in place of a New tab.
- `AddToChat` makes the path absolute (`absolute_path(cwd, path)`), and `insert_path_mention` makes it relative to the composer folder. It acts only on the active agent tab.
- `on_client_event` and `create_workspace` are unchanged.

## Verified

- `CARGO_BUILD_JOBS=4 ./cargo.sh test -p bachuc-app` -> 76 passed, including `question_mark_runs_only_outside_text_fields_and_the_terminal`. That test checks the composer (`Input mode=editor` under the agent pane), the terminal, the command center input, and settings `Input` and `NumberInput`: none match. The shell root and a focused timeline match.
- `CARGO_BUILD_JOBS=4 ./cargo.sh build -p bachuc-app` -> exit 0. One warning existed before this TASK (`next_theme`, `state.rs:424`).
- `./cargo.sh fmt --check -p bachuc-app` -> exit 0.
- `./cargo.sh clippy -p bachuc-app` -> no warnings in `shell.rs` or `actions.rs`. Two warnings existed before: `state.rs:424` and `plugins.rs:10`.
- `./cargo.sh metadata --format-version 1 > /dev/null` -> exit 0.
- GitNexus impact (index 58 commits stale): `binding_specs` is CRITICAL with 44 dependents. Its signature is unchanged, and only the context of `shortcuts.dialog.toggle` changed. `run_action` and `open_command_center` are LOW. `on_pane_event` is UNKNOWN; rg shows one caller, the `create_workspace` subscription.
- UI check: Cmd+N, Cmd+P with a query and Enter, Cmd+Shift+P, and `?` -> DEFERRED (UI checker).
- UI check: `?` in the composer, a terminal, the command center input, and a settings field -> DEFERRED (UI checker). The unit test above covers this rule by key context.
- UI check: explorer "Open to side" and "Add to chat" -> DEFERRED (UI checker).
- Owner hand test: Cmd-click a file path in a terminal -> DEFERRED (owner).
- Anti-goal: Cmd+K, Cmd+T, and Cmd+W on live -> DEFERRED (UI checker). Their bindings are unchanged; `binding_context` keeps `SHELL_CONTEXT` for them, and the test checks `command-center.toggle`.

## Lessons

- GPUI `!X` in a binding predicate fails when any context of the focus path holds `X` (`gpui-pre-0.3.9/src/keymap/context.rs:291`). Use it to keep a bare-key shortcut out of text fields.
- `Workspace::split` emits `PaneEvent::NewTab`, and the shell handles that event later. To fill the new group with a given tab, set a pending value that the `NewTab` arm reads, as `side_file` does.
- Safe default: "Add to chat" with no agent tab active, for example on a draft tab, does nothing. Electron hides the item when no chat has focus. The GPUI menu (TASK-213) always shows it.
- Safe default: "Open to the side" on a file that is already open shows the open tab and makes no split.

## UI Checks Run 4

Run 2026-10-18 by the UI checker (run 4). Bundle `Bachuc GPUI Check` built from HEAD `60fd3a593`, PID 88032, launched with `open -n -g --env BACHUC_NO_ACTIVATE=1` and connected only to live `127.0.0.1:6768` (`lsof`). The checker forced each frame with the AX "zoom the window" action and sent keys with cua `pressKey`/`typeText` to the check app only. Scratch repo `/tmp/t202-repo` with disposable agents "r4 board card", "r4 remove card", "r4 fork src" (mock ten-second-stream), "r4 fork ok" (Haiku 5.5, one-word reply), and the fork child "Continue." (`af3218a`). After the run, PID 88032 exited, all five agents and the five t202-repo workspaces were archived, and the repo stayed as before (`feature-x`, ` M a.txt`).

- Cmd+P with a query and Enter opens the file: PASS. "main" + Return closed the command center and opened the tab "main.rs" with "Contents of src/main.rs". Evidence: `/tmp/r4-203-05-cmdp-opened.png`.
- Cmd+N opens New workspace: PASS. A "New workspace" tab opened with the heading "New workspace" (project picker on ".claude"). Evidence: `/tmp/r4-203-13-cmdn.png`.
- Cmd+Shift+P pins the workspace: PASS. A "Pinned" group with "t202-repo, feature-x" appeared at the top of the sidebar (`/tmp/r4-203-17-after-pin.png`). A second Cmd+Shift+P removed it (`/tmp/r4-203-18-after-unpin.png`).
- `?` with focus outside text opens the shortcuts dialog, Escape closes it: PASS. Focus on the main.rs file view; `?` opened `dialog Shortcuts` with an empty search field; Escape closed it. Evidence: `/tmp/r4-203-06-qmark-outside.png`, `/tmp/r4-203-07-qmark-escape.png`.
- `?` in the composer: PASS. Value became "?", no dialog (`/tmp/r4-203-10-composer-qmark.png`). Cleared after.
- `?` in a terminal: PASS. The prompt shows `?`, no dialog (`/tmp/r4-203-02-terminal-qmark-crop.png`). Cleared with Ctrl+U.
- `?` in the command center field: PASS. Value "?", the dialog stays "Command center" (`/tmp/r4-203-09-cc-qmark.png`).
- `?` in a settings field: PASS. Interface font size became "14?", no dialog (`/tmp/r4-203-12-settings-qmark.png`). Restored to 14.
- Explorer "Open to the side" and "Add to chat": UNVERIFIED (unattended): needs pointer. The file menu has no AX or keyboard path on macOS (see TASK-213 UI Checks Run 4).
- Anti-goal, Cmd+K, Cmd+T, Cmd+W: PASS. Cmd+K opened the command center (`/tmp/r4-203-08-cmdk.png`). Cmd+T added a "New tab" tab (`/tmp/r4-203-15-cmdt.png`). Cmd+W closed the active tab each time (two New workspace tabs, then the new tab).
- Cmd-click a file path in a terminal: UNVERIFIED (unattended): needs pointer.
