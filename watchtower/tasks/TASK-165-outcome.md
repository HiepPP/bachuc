# TASK-165 Outcome

## Status

DONE

## Changed

- `packages/desktop-gpui/crates/workspace/src/model.rs`: `Layout` keeps `maximized`; adds `maximized()`, `toggle_maximize()`, `visible_rects()`, a private `settle()`; adds `DIVIDER_HIT` (11), `DIVIDER_HIGHLIGHT` (3), `divider_inset()`.
- `packages/desktop-gpui/crates/workspace/src/tab_menu.rs` (new): `TabKind`, `TabMenuAction`, `TabMenuEvent`, `TabMenuEntry`, `TabMenuItem`, `menu_entries()`, `close_targets()`, `TabExtras` (logo and kind per key).
- `packages/desktop-gpui/crates/workspace/src/workspace.rs`: tooltip delay 400 ms, right-click and keyboard context menu, maximize toolbar button and render, tab logo, centered divider band, `set_item_logo`, `set_item_kind`, `toggle_maximize`, second `EventEmitter<TabMenuEvent>`.
- `packages/desktop-gpui/crates/workspace/src/lib.rs`: `pub mod tab_menu`, re-exports `TabKind` and `TabMenuEvent`, new action `OpenTabMenu`.
- `packages/desktop-gpui/crates/workspace/tests/model.rs`: 7 new tests (19 old tests unchanged).
- `packages/desktop-gpui/crates/workspace/tests/tab_menu.rs` (new): 8 tests.
- `packages/desktop-gpui/crates/workspace/examples/workspace.rs`: logo and kind demo, menu-key bindings, typed `PaneEvent` subscription (two `EventEmitter` impls make the type ambiguous).

## Contract

- `Workspace::set_item_logo(&mut self, key: &str, logo: Option<Arc<gpui_kit::Image>>, cx: &mut Context<Workspace>)`: unknown key changes nothing.
- `Workspace::toggle_maximize(&mut self, window: &mut Window, cx: &mut Context<Workspace>)`: shows the focused group alone, or restores. No-op with one group.
- `bachuc_workspace::TabMenuEvent`: `CopyResumeCommand`, `CopyAgentId`, `CopyTerminalId`, `CopyFilePath`, `Rename`, `ReloadAgent`, each `{ key: String }`. Emitted by `Workspace` through `EventEmitter<TabMenuEvent>`. The close items are not events: the workspace closes the tabs and emits `PaneEvent::Closed`.
- Added beyond the spec (needed by the menu): `Workspace::set_item_kind(key, TabKind, cx)` with `TabKind::{Agent { id }, Terminal { id }, File, Other}`. `PaneItem` has no kind, so the menu cannot choose its copy and rename items without it. Default is `Other` (close items only).
- Added: action `bachuc_workspace::actions::OpenTabMenu`. The app must bind `menu` and `shift-f10` to it with `KEY_CONTEXT` (the example does). It opens the active tab's menu under the tab.
- `PaneItem` and `PaneEvent` are unchanged. A subscriber that calls `subscribe`/`subscribe_in` on the workspace needs an explicit event type (`event: &PaneEvent`) now that two `EventEmitter` impls exist; `bachuc-app` still builds.
- Maximize rules: the maximized group is always the focused one, or none. Closing the last tab of the maximized group, or a split, restores the layout (as Electron drops the pane). Opening or activating a tab in a hidden group moves the maximize to that group.
- The maximize button shows only with 2 or more groups, or while maximized.

## Verified

- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test -p bachuc-workspace` -> pass: `tests/model.rs` 26 passed (19 old + 7 new: maximize round trip, one group, closing the maximized tab, closing one of several, hidden-group open, split, divider hit band), `tests/tab_menu.rs` 8 passed (menu items against `workspace-tab-menu.ts` and `en.ts`, `set_item_logo` on an unknown key), 0 failed.
- `packages/desktop-gpui/cargo.sh fmt --check -p bachuc-workspace` -> exit 0.
- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh build -p bachuc-app` -> Finished `dev` profile, no errors.
- Tooltip after 400 ms, right click and menu key opening the menu, drawn logo, drawn maximize, and the divider band on screen: UNVERIFIED (unattended): code tests only.

## Anti-goal

- The layout model stays the single source of truth: maximize lives in `Layout` (`maximized`), and the view only reads `maximized()` and `visible_rects()`. Tripwire: the 19 old model tests and the maximize round-trip test pass (`cargo.sh test -p bachuc-workspace`, 26 passed, 2026-10-16, local).

## Lessons

- `gpui` elements already have `tooltip_show_delay(Duration)`; the repo-wide `TooltipDefaults` (500 ms) needs no change.
- `gpui_component::menu::ContextMenuExt` cannot open from the keyboard (`open_menu` is private) and its focus restore setters are `pub(crate)`. The workspace builds its own `PopupMenu` in a `deferred(anchored(...))` and refocuses itself on `DismissEvent`.
- The old divider band (10 px, 4.5 px inset) landed on half pixels. The new 11 px band and 3 px line start on whole pixels, centered on the 1 px line. The Electron band is 10 px; this is 1 px wider on purpose. The off-screen cause is unconfirmed (no screenshot), only the arithmetic is tested.
- A menu item with a hint or tooltip is a custom `PopupMenuItem::element` row, because plain items take only a label and an icon.
- For TASK-167: handle `TabMenuEvent` with a second `subscribe` and call `set_item_kind` and `set_item_logo` after `open`; the keys are `PaneItem::key`.
