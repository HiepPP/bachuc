# TASK-257 Outcome

Status: DONE (AX and owner checks DEFERRED)

## Changed

- `packages/desktop-gpui/crates/app/src/actions.rs`: new actions `Quit`, `Hide`, `HideOthers`, `ShowAll`, `Minimize`, `Zoom`, `ToggleFullScreen`, `About`; `app_menus()`; `fixed_bindings` adds `cmd-q`, `cmd-h`, `alt-cmd-h`, `cmd-m` (no context); 4 new tests.
- `packages/desktop-gpui/crates/app/src/main.rs`: `register_app_actions` (global `cx.on_action` handlers), `cx.set_menus(actions::app_menus())`, `show_about_panel()` (NSApplication standard About panel).

## Contract

- `pub fn app_menus() -> Vec<Menu>` in `actions.rs`: menus Bachuc GPUI (About, Settings, Services, Hide, Hide Others, Show All, Quit), Edit (`input::Undo/Redo/Cut/Copy/Paste/SelectAll` as `MenuItem::os_action`), View (Zoom In/Out/Actual Size via `RunAction(view.zoom.*)`, Enter Full Screen), Window (Minimize, Zoom).
- Settings and zoom items dispatch `RunAction("settings.toggle" | "view.zoom.in" | "view.zoom.out" | "view.zoom.reset")`; no key is bound twice.

## Verified

- `./cargo.sh test -p bachuc-app --bin bachuc-desktop` -> 89 passed, 0 failed (14 in `actions::`).
- `./cargo.sh build -p bachuc-app`, `fmt --check -p bachuc-app` (exit 0), `clippy -p bachuc-app` (no warning in `main.rs`/`actions.rs`) -> pass.
- DEFERRED (UI checker): menu bar AX tree shows the four menus and their items.
- DEFERRED (owner): Edit > Copy/Paste disabled with no input focused; Settings and zoom items equal their keys; Cmd+Q/H/M and Cmd+C/V/X/A/Z; Cmd+K and Cmd+, still work.

## Anti-goal

- Checkpoint `fixed_bindings` vs shortcut table, source: unit test `no_fixed_binding_repeats_a_shortcut_table_keystroke_in_the_same_context`, result pass, at build time 2026-10-19. 0 shortcut keystrokes are bound a second time in an overlapping context.
- UNSATISFIABLE as written: "no keystroke in `fixed_bindings` equals a keystroke in the shortcut table". Existing `escape` (Settings context) and the table's `agent.interrupt` `Escape` (agent context) share a keystroke by design. Replacement: the test fails when a fixed binding shares a keystroke with a table binding AND its context predicate matches that binding's context (unscoped fixed bindings match every context).

## Lessons

- A menu item shows the keys of the keymap binding of its action (`partial_eq`), so `RunAction("settings.toggle")` shows Cmd+,. After a user rebind the menu text keeps the old keys until `set_menus` runs again; shell.rs (not in scope) does the rebind.
- `Enter Full Screen` is not in the spec's new-action list; added `ToggleFullScreen` (no key) as the safe default.
- `About` uses `orderFrontStandardAboutPanel`; no custom About window.

## UI Checks

Lead, 2026-10-19, check bundle on live (6768), cua_repl:

- Menu bar AX tree -> Bachuc GPUI Check, Edit, View, Window. PASS.
- Cmd+Q quit the check app (no process left). PASS.
- Cmd+H, Cmd+M, Edit items, and Settings and zoom from the menu: UNVERIFIED, owner hand check Q-170.

## UI Checks 2 (lead, 2026-10-19 13:50, live, cua_repl AX only; the display did not repaint, so no screenshot is evidence)

- Edit menu with no input focused: Undo, Redo, Cut, Copy, Paste, Select All are disabled. PASS.
- In the composer, Edit > Select All then Edit > Copy put the text on the clipboard (`pbpaste` read it). PASS.
- Edit > Cut, Edit > Paste, and Edit > Undo did not change the composer value. The Cmd+X and Cmd+V keys did. FAIL.
- Window > Minimize and Cmd+M left the window on screen. UNVERIFIED: the display did not repaint, so the window state may be stale.
