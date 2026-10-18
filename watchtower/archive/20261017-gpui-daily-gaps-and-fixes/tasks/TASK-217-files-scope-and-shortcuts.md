# TASK-217 Command center Files scope, shortcut rows, and shortcuts dialog

Group: N (shared files [packages/desktop-gpui/crates/sidebar/src/sidebar.rs](packages/desktop-gpui/crates/sidebar/src/sidebar.rs) and [packages/desktop-gpui/crates/command](packages/desktop-gpui/crates/command))
Class: code

## Brief

Goal: The command center can search files (Cmd+P), the shortcut table has Electron's missing rows, and `?` opens a shortcuts dialog.

Change: the center has only Actions, Workspaces, and Agents, and no Files scope ([packages/desktop-gpui/crates/command/src/center.rs](packages/desktop-gpui/crates/command/src/center.rs)) -> a Files scope, 5 new shortcut rows, and a shortcuts dialog.

Boundaries:

- Files scope: Electron `command-center.tsx` lines 262-294 and `workspace-file-search.ts` lines 91-97: files only, limit 100, debounce 100 ms; a pick opens a file tab.
- The command crate does not depend on `bachuc-client`; keep it that way. In Files scope the center emits `CommandFileEvent::FileQuery`, the app answers with `CommandCenter::set_file_results`, and a pick emits `CommandFileEvent::OpenFile`.
- Use a new event type `CommandFileEvent` with `impl EventEmitter<CommandFileEvent> for CommandCenter`. Do not add variants to `CommandEvent`: the app matches it exhaustively (`shell.rs` lines 2224-2225), so a new variant breaks the app build.
- New shortcut rows in [packages/desktop-gpui/crates/command/src/shortcuts.rs](packages/desktop-gpui/crates/command/src/shortcuts.rs), with Electron ids and chords from [packages/app/src/keyboard/keyboard-shortcuts.ts](packages/app/src/keyboard/keyboard-shortcuts.ts):
  - `workspace.new`, Cmd+N.
  - `agent.new`, Cmd+O, label "Open project" (lines 263-277).
  - `command-center.files`, Cmd+P.
  - `workspace.pin`, Cmd+Shift+P, label "Pin chat" (lines 367-377).
  - `shortcuts.dialog.toggle`, `?`, only when focus is not in a text field or terminal.
- Out of scope: move tab between panes and toggle focus mode (Q-113).
- The settings crate tests read the shortcut table; keep them passing. `section_of` ([packages/desktop-gpui/crates/settings/src/bindings.rs](packages/desktop-gpui/crates/settings/src/bindings.rs) lines 41-53) would put `workspace.new` and `workspace.pin` under Tabs and Panes and `agent.new` under Agent Input. Put them where Electron's Shortcuts screen lists them; read the section of each id in `keyboard-shortcuts.ts`.
- The `?` row: give it a binding context that excludes text inputs and the terminal, so the key still types in the composer. Follow the `AGENT_ACTIONS` context pattern in [packages/desktop-gpui/crates/app/src/actions.rs](packages/desktop-gpui/crates/app/src/actions.rs) lines 36 and 100; TASK-203 registers it there. Check that `chord.rs` line 50 parses Electron's `Shift+?` (line 855); if GPUI reports the key as `shift-/`, map it in `chord.rs`.
- Shortcuts dialog: Electron [packages/app/src/components/keyboard-shortcuts-dialog.tsx](packages/app/src/components/keyboard-shortcuts-dialog.tsx). Build it in the command crate from the same table. The app (TASK-203) shows it and runs the actions.

How:

- Add the scope state and the two events to `center.rs`, with a pure function for the scope switch and tests.
- Add the 5 rows to `shortcuts.rs`.
- Add `shortcuts_dialog.rs`.

Files:

- [packages/desktop-gpui/crates/command/src/center.rs](packages/desktop-gpui/crates/command/src/center.rs) (Files scope, events)
- [packages/desktop-gpui/crates/command/src/shortcuts.rs](packages/desktop-gpui/crates/command/src/shortcuts.rs) (5 rows)
- [packages/desktop-gpui/crates/command/src/shortcuts_dialog.rs](packages/desktop-gpui/crates/command/src/shortcuts_dialog.rs) (new)
- [packages/desktop-gpui/crates/command/src/lib.rs](packages/desktop-gpui/crates/command/src/lib.rs) (module line and exports)
- [packages/desktop-gpui/crates/command/src/chord.rs](packages/desktop-gpui/crates/command/src/chord.rs) (only if `Shift+?` needs a mapping)
- [packages/desktop-gpui/crates/settings/src/bindings.rs](packages/desktop-gpui/crates/settings/src/bindings.rs) (sections of the new ids)

Interfaces:

- Produces: `CommandFileEvent::FileQuery { query: String }` and `CommandFileEvent::OpenFile { path: String }` for TASK-203.
- Produces: `CommandCenter::open_files_scope(&mut self, window: &mut Window, cx: &mut Context<CommandCenter>)` and `CommandCenter::set_file_results(&mut self, paths: Vec<String>, cx: &mut Context<CommandCenter>)` for TASK-203.
- Produces: `ShortcutsDialog::new(window: &mut Window, cx: &mut Context<ShortcutsDialog>) -> ShortcutsDialog` and `ShortcutsDialogEvent::Dismissed` for TASK-203.
- Produces: shortcut action ids `"workspace.new"`, `"agent.new"`, `"command-center.files"`, `"workspace.pin"`, `"shortcuts.dialog.toggle"` for TASK-203.

Expected result:

- In Files scope, typing emits `CommandFileEvent::FileQuery` after a 100 ms pause; `set_file_results` fills the list; Enter emits `CommandFileEvent::OpenFile`.
- The app still builds before TASK-203 subscribes to the new event.
- The shortcut table lists the 5 new rows with Electron chords.
- The dialog lists every shortcut row in groups.

Anti-goal: command center open time must not grow; drift gauge; time from Cmd+K to the first painted row stays under 50 ms, read with a log timestamp before the change and at completion.

## Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-command` -> passes, including the scope tests.
- `./cargo.sh test -p bachuc-settings` -> passes with the new rows, including a test that the 5 new ids land in Electron's sections.
- `./cargo.sh test -p bachuc-command chord` -> `Shift+?` parses to the key GPUI reports.
- `./cargo.sh build -p bachuc-app` -> builds.
- Anti-goal: Cmd+K to first row from log timestamps -> under 50 ms at both reads.
