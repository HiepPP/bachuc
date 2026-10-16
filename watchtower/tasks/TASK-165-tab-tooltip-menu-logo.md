# TASK-165 Tab tooltip, menu, and logo

Group: I (writes `crates/workspace`)
Class: code

## Brief

Goal: Give workspace tabs the Electron tooltip, context menu, and maximize action, and let a tab show a provider logo (audit item 8).

Change: tabs have no tooltip, no context menu, and only `IconName` icons; the divider grab band is 1 to 2 px off the drawn line -> the Electron tab behavior.

Design: [packages/app/src/screens/workspace/workspace-desktop-tabs-row.tsx](packages/app/src/screens/workspace/workspace-desktop-tabs-row.tsx) (`ContextMenu` line 834, `Tooltip` with `delayDuration={400}` line 835, maximize line 311 behind `showMaximizeAction`) and [packages/app/src/screens/workspace/workspace-tab-menu.ts](packages/app/src/screens/workspace/workspace-tab-menu.ts) for the menu items and their order.

Boundaries:

- Keep `PaneItem`'s fields and the `PaneEvent` enum unchanged, because `crates/app/src/shell.rs:669-691` matches `PaneEvent` exhaustively and builds `PaneItem` literals. Add the logo through `Workspace::set_item_logo`, and emit menu actions the app owns as a second event type, `TabMenuEvent`, through a second `EventEmitter` impl (Planner call from the Builder Check).
- The context menu opens on right click and on the keyboard menu key; each item has a role and a label. Copy the `workspace-tab-menu.ts` items.
- Center the divider hit band on the drawn line.
- Keep every existing public item and the 19 model tests.

Files:

- [packages/desktop-gpui/crates/workspace](packages/desktop-gpui/crates/workspace) (`src`, `tests`, `examples/workspace.rs`)

Interfaces:

- Consumes: `bachuc_icons::provider_logo_svg(provider: &str) -> Option<&'static str>`, `bachuc_icons::provider_logo(provider: &str, fill: gpui_kit::Hsla) -> Option<std::sync::Arc<gpui_kit::Image>>`, and `bachuc_icons::provider_fallback_icon(provider: &str) -> bachuc_icons::IconName` from TASK-158
- Produces: `bachuc_workspace::Workspace::set_item_logo(&mut self, key: &str, logo: Option<std::sync::Arc<gpui_kit::Image>>, cx: &mut Context<Workspace>)`, `bachuc_workspace::Workspace::toggle_maximize(&mut self, window: &mut Window, cx: &mut Context<Workspace>)`, and `bachuc_workspace::TabMenuEvent` (one variant per `workspace-tab-menu.ts` action that the app owns, each with the tab `key: String`) emitted by `Workspace` through a second `EventEmitter`, for TASK-167

Expected result:

- Hovering a tab shows its full title after 400 ms; right click opens the Electron menu.
- Maximize shows one group alone and restores the layout.

Anti-goal: the layout model stays the single source of truth; tripwire: the existing 19 model tests and a new maximize round-trip test pass; read at completion.

## Verify

- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test -p bachuc-workspace` -> pass, including the 19 existing tests, a maximize round-trip test, maximize with one group, closing the maximized tab, `set_item_logo` on an unknown key, a menu-items test against `workspace-tab-menu.ts`, and a divider hit-band test
- `packages/desktop-gpui/cargo.sh fmt --check -p bachuc-workspace` -> exit 0
- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh build -p bachuc-app` -> exit 0
