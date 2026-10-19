# TASK-257 Native macOS menu bar

Group: A (writes [packages/desktop-gpui/crates/app/src/main.rs](packages/desktop-gpui/crates/app/src/main.rs) and [packages/desktop-gpui/crates/app/src/actions.rs](packages/desktop-gpui/crates/app/src/actions.rs); shared file with TASK-256)
Class: code

## Brief

Goal: standard macOS shortcuts work in GPUI. Cmd+Q, Cmd+H, Cmd+M, and the Edit keys do nothing today.

Change: no menu bar (no `set_menus` call) -> a native menu bar that mirrors Electron's menu in [packages/desktop/src/features/menu.ts](packages/desktop/src/features/menu.ts).

Boundaries:

- Menus: App (About, Settings Cmd+,, Services, Hide Cmd+H, Hide Others Cmd+Alt+H, Show All, Quit Cmd+Q). Edit (Undo, Redo, Cut, Copy, Paste, Select All). View (Zoom In Cmd+=, Zoom Out Cmd+-, Actual Size Cmd+0, Enter Full Screen). Window (Minimize Cmd+M, Zoom).
- Leave out Electron-only items: Reload, Force Reload, Toggle DevTools.
- Edit items use `MenuItem::os_action` with the vendored gpui-base input actions (`input::Undo`, `input::Redo`, `input::Cut`, `input::Copy`, `input::Paste`, `input::SelectAll`, [packages/desktop-gpui/vendor/gpui-base/src/input/base/state.rs](packages/desktop-gpui/vendor/gpui-base/src/input/base/state.rs) lines 74-124). They route to the focused input and grey out when no input has focus.
- Settings and zoom items dispatch the existing shortcut actions, so the shortcut table in [packages/desktop-gpui/crates/command/src/shortcuts.rs](packages/desktop-gpui/crates/command/src/shortcuts.rs) stays the one source of key bindings. Do not bind those keys twice.
- New app actions: `Quit` (`cx.quit()`), `Hide`, `HideOthers`, `ShowAll`, `Minimize` (`window.minimize_window()`), `Zoom` (`window.zoom_window()`), `About`. Bind Cmd+Q, Cmd+H, Cmd+Alt+H, Cmd+M in `fixed_bindings`.
- APIs: `cx.set_menus` (`gpui-pre-0.3.9/src/app.rs:2788`), `Menu`, `MenuItem`, `OsAction` (`gpui-pre-0.3.9/src/platform/app_menu.rs`).

Files:

- [packages/desktop-gpui/crates/app/src/main.rs](packages/desktop-gpui/crates/app/src/main.rs) (call `set_menus`, register app action handlers)
- [packages/desktop-gpui/crates/app/src/actions.rs](packages/desktop-gpui/crates/app/src/actions.rs) (new actions, menu builder, fixed bindings)

Expected result:

- The menu bar shows Bachuc GPUI, Edit, View, and Window menus.
- Cmd+Q quits, Cmd+H hides, Cmd+M minimizes. Cut, Copy, Paste, Select All, and Undo work in the composer through the menu and the keys.

Anti-goal: every existing shortcut still fires once; 0 shortcuts in [packages/desktop-gpui/crates/command/src/shortcuts.rs](packages/desktop-gpui/crates/command/src/shortcuts.rs) are bound a second time; read with a unit test over `fixed_bindings` plus the shortcut table at build time.

## Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-app` -> a new test passes that checks no keystroke in `fixed_bindings` equals a keystroke in the shortcut table.
- Screen check (cua_repl, AX only): read the menu bar AX tree of the check bundle -> the four menus and their items exist.
- Owner hand check: with no input focused, Edit > Copy and Paste are disabled and nothing crashes.
- Owner hand check: Settings, Zoom In, Zoom Out, and Actual Size from the menu do the same as their keys.
- Owner hand check: Cmd+Q, Cmd+H, Cmd+M, and Cmd+C/V/X/A/Z in the composer work; Cmd+K and Cmd+, still work.
