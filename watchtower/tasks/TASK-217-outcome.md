# TASK-217 Outcome

## Status

DONE

## Changed

- `packages/desktop-gpui/crates/command/src/center.rs`: the Files scope (`Scope`, `scope_after_key`, `file_query_delay`, `file_row`, `files_layout`), `CommandFileEvent` and its `EventEmitter` impl, `open_files_scope`, `set_file_results`, the Files chip, the file rows, the loading spinner, and 4 scope tests.
- `packages/desktop-gpui/crates/command/src/shortcuts.rs`: the 5 rows, `ShortcutSection` (moved here from settings), a section on every row, and `shortcut_section`.
- `packages/desktop-gpui/crates/command/src/shortcuts_dialog.rs` (new): `ShortcutsDialog`, `ShortcutsDialogEvent`, and 3 tests.
- `packages/desktop-gpui/crates/command/src/chord.rs`: `Shift+?` binds as `?`, plus 1 test.
- `packages/desktop-gpui/crates/command/src/lib.rs`: the module line and the exports.
- `packages/desktop-gpui/crates/settings/src/bindings.rs`: `Section` re-exports `bachuc_command::ShortcutSection`, and `section_of` reads the table. Added a test for the 5 new ids.

## Contract

- Produces: `CommandFileEvent::FileQuery { query: String }` and `CommandFileEvent::OpenFile { path: String }`. They are exported from `lib.rs`. `CommandEvent` is unchanged.
- Produces: `CommandCenter::open_files_scope(&mut self, window, cx)` and `CommandCenter::set_file_results(&mut self, paths: Vec<String>, cx)`.
- Produces: `ShortcutsDialog::new(window, cx) -> ShortcutsDialog` and `ShortcutsDialogEvent::Dismissed`.
- Produces: the action ids `workspace.new` (Cmd+N), `agent.new` (Cmd+O, "Open project"), `command-center.files` (Cmd+P, "Search files"), `workspace.pin` (Cmd+Shift+P, "Pin chat"), and `shortcuts.dialog.toggle` (`Shift+?`, "Show keyboard shortcuts").
- Sections follow `keyboard-shortcuts.ts`. General is command center, Search files, Show keyboard shortcuts, then the old rows. Projects & Workspaces is Open project, New workspace, the 2 navigation rows, Pin chat, then Archive. This is Electron's `SHORTCUT_HELP_ROW_ORDER`. The settings page and the dialog read one mapping, `shortcut_section`.
- Files scope: the first `FileQuery` is sent on the next tick, so a subscription made right after `open_files_scope` still gets it. An empty query is sent at once. Other queries wait 100 ms, and a newer change drops the pending query. The list shows the paths in the daemon's order under one "Files" section. While a search runs, a spinner shows and "No matches" is hidden. Enter or a click emits `OpenFile` with the path, with `\` changed to `/`. The app removes the view after `OpenFile`. Backspace in an empty field or a press on the chip goes back to the full list.
- `Shift+?`: macOS reports Shift+/ to GPUI as key `?` with no shift (`gpui-pre-macos` `events.rs`, the `chars_with_shift` branch). `gpui_keystrokes("Shift+?")` therefore gives `?`, and the display text is `?`.
- The dialog reads `load_shortcuts()`, so an unassigned row shows "Not set". Its search matches the label, the note, and the chord as `cmd k`, `command+k`, or `⌘K`. A section whose title matches keeps all its rows. Escape, the close button, or a backdrop click emits `Dismissed`.
- Safe default: only the Files scope searches files. Electron also adds a Files section to the full list when the query is not empty. The spec scopes `FileQuery` to the Files scope, and this keeps Cmd+K open time unchanged.
- Safe default: file rows use the generic `File` icon. The Material file icons are in `bachuc-explorer`, and that crate depends on `bachuc-client`.
- Safe default: there is no error line in the Files scope, because `set_file_results` has no error input. On a failed search, the app sends an empty list.
- Impact: GitNexus rates `default_shortcuts` and `keystroke_for` CRITICAL (index 52 commits stale). Both keep their signatures. The table only adds rows, and the shift rule changes only `?`. App, settings, and command tests pass.

## Verified

- `./cargo.sh test -p bachuc-command` -> exit 0: 39 lib, 5 search, 9 shortcuts. The new scope and dialog tests are in lib. The first run failed one doctest with "can't find crate for `bachuc_plugin_host`". It passed when run alone with `--doc` and on the next full run. This was a transient shared-target failure, not a code failure.
- `./cargo.sh test -p bachuc-command chord` -> 6 passed, including `chord_shift_question_mark_binds_the_key_gpui_reports`.
- `./cargo.sh test -p bachuc-settings` -> 50 + 4 passed, including `the_new_rows_land_in_the_electron_sections`.
- `./cargo.sh test -p bachuc-app` -> 74 passed.
- `./cargo.sh build -p bachuc-app` -> exit 0. One warning existed before this TASK, in `crates/app/src/state.rs:424`.
- `./cargo.sh fmt --check -p bachuc-command` and `-p bachuc-settings` -> exit 0.
- `./cargo.sh clippy -p bachuc-command -p bachuc-settings` -> no warnings in changed code. Two warnings existed before, in `search.rs:261` and `pane_appearance.rs:370`.
- `./cargo.sh metadata --format-version 1` -> exit 0.
- Anti-goal: Cmd+K to the first row under 50 ms, from log timestamps -> DEFERRED (UI checker). `CommandCenter::new` adds 4 field inits and no extra work.
- Files scope, chip, spinner, and dialog look -> DEFERRED (UI checker), after TASK-203 wires them.

## Lessons

- TASK-203: the app binds every table row in `SHELL_CONTEXT` (`actions.rs` `binding_specs`). Until TASK-203 puts `shortcuts.dialog.toggle` in a context that excludes text inputs and the terminal, a `?` typed in the composer runs that action and does not type. Ship TASK-203 with this TASK.
- TASK-203: until it runs them, the 5 new ids show as Actions rows in the command center and fall through `run_action`. Cmd+N, Cmd+O, Cmd+P, and Cmd+Shift+P do nothing until then.
- TASK-203: subscribe with `cx.subscribe(&center, |shell, _, event: &CommandFileEvent, cx| ...)`. Answer `FileQuery` with `Client::directory_suggestions(cwd, query, include_files: true, include_directories: false, limit: 100)`. `set_file_results` takes no query, so drop answers to older queries in the app.
- The section of a shortcut now lives in `bachuc_command::shortcut_section`. A new row needs only its `DEFAULTS` entry. Settings `Section` is the same type.
- `on_a11y_action` takes `FnMut(Option<&ActionData>, &mut Window, &mut App)`, not a `cx.listener`. Capture `cx.weak_entity()` and write the closure inline at the call site, or type inference fails ("FnMut is not general enough").
