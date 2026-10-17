# TASK-148 Workspace shell with the Electron tab bar and splits

Group: D (writes the new `crates/workspace`)
Class: risky

## Brief

Goal: Add the crate `bachuc-workspace` with a pane container that has the Electron tab bar and split panes, so the app can show agents, terminals, files, and diffs side by side.

Change: one tab bar with one pane -> tab groups in resizable splits, with the Electron tab look, drag reorder, and the new-tab menu.

Design: [packages/app/src/screens/workspace/workspace-desktop-tabs-row.tsx](packages/app/src/screens/workspace/workspace-desktop-tabs-row.tsx), [packages/app/src/components/split-container.tsx](packages/app/src/components/split-container.tsx), [packages/app/src/screens/workspace/workspace-screen.tsx](packages/app/src/screens/workspace/workspace-screen.tsx)

Boundaries:

- Owner answer (2026-10-16): split right and split down from a menu and shortcuts, with resizable dividers. No drag-to-split drop zones.
- Tab sizes from `workspace-desktop-tabs-row.tsx` lines 102-111: min width 96, max width 160, chip padding 8, row padding 4, icon 14, modified dot 8, button height `CONTROL_XS` 28; the row is `TAB_BAR_HEIGHT` 36.
- The crate does not know pane kinds. A pane item carries its own view; the app owns what each view is.
- Drag reorder moves a tab inside its group, and to another group's tab bar.
- Closing the last tab of a split group removes the group; the divider goes away.
- Keep pane views alive when their tab is not active, so a terminal or a timeline keeps its state.
- Layout reentrancy: never update a pane view inside the container's render. Defer changes to the next frame.

How:

- Write the split tree model (groups, axis, ratios) with tests.
- Write the tab bar, the new-tab menu hook, drag reorder, and the dividers.
- Add `examples/workspace.rs` with three plain text panes.

Files:

- [packages/desktop-gpui/crates/workspace](packages/desktop-gpui/crates/workspace) (new crate `bachuc-workspace`: `Cargo.toml`, `src`, `tests`, `examples/workspace.rs`)

Interfaces:

- Consumes: `bachuc_theme::tokens(cx: &gpui_kit::App) -> Tokens` from TASK-145
- Consumes: `bachuc_theme::metrics` (module of `pub const` `f32` values: `SPACE_0_5`, `SPACE_1`, `SPACE_1_5`, `SPACE_2`, `SPACE_3`, `SPACE_4`, `SPACE_6`, `SPACE_8`, `RADIUS_SM`, `RADIUS_BASE`, `RADIUS_MD`, `RADIUS_LG`, `RADIUS_XL`, `RADIUS_2XL`, `FONT_CODE`, `FONT_SM`, `FONT_BASE`, `FONT_CONTENT`, `FONT_LG`, `FONT_XL`, `FONT_2XL`, `ICON_XS`, `ICON_SM`, `ICON_MD`, `ICON_LG`, `CONTROL_XS`, `CONTROL_SM`, `CONTROL_MD`, `HEADER_HEIGHT`, `TAB_BAR_HEIGHT`, `MAX_CONTENT_WIDTH`, `SIDEBAR_WIDTH`, `SIDEBAR_MIN_WIDTH`, `SIDEBAR_MAX_WIDTH`, `TRAFFIC_LIGHT_WIDTH`, `TRAFFIC_LIGHT_HEIGHT`, `SETTINGS_SIDEBAR_WIDTH`, `SETTINGS_DETAIL_MIN_WIDTH`, `SETTINGS_CONTENT_WIDTH`) from TASK-145
- Consumes: `bachuc_icons::IconName` (enum with one variant per generated icon, named as the lucide import, for example `FolderPlus`, `GitBranch`, `X`), `bachuc_icons::icon(name: IconName) -> gpui_kit::component::Icon` (sized with `ICON_MD`; change it with `with_size` and `text_color`), and `bachuc_icons::Assets` (a GPUI `AssetSource` that serves the SVGs and falls through to the gpui-kit assets) from TASK-145
- Produces: `bachuc_workspace::Workspace` (a GPUI view) with `Workspace::new(window: &mut gpui_kit::Window, cx: &mut gpui_kit::Context<Workspace>) -> Workspace`, `Workspace::open(&mut self, item: PaneItem, window: &mut gpui_kit::Window, cx: &mut gpui_kit::Context<Workspace>)`, `Workspace::close_active(&mut self, window: &mut gpui_kit::Window, cx: &mut gpui_kit::Context<Workspace>)`, `Workspace::close(&mut self, key: &str, window: &mut gpui_kit::Window, cx: &mut gpui_kit::Context<Workspace>)`, `Workspace::split(&mut self, axis: SplitAxis, window: &mut gpui_kit::Window, cx: &mut gpui_kit::Context<Workspace>)`, `Workspace::activate_index(&mut self, index: usize, window: &mut gpui_kit::Window, cx: &mut gpui_kit::Context<Workspace>)`, and `Workspace::active_key(&self) -> Option<&str>` for TASK-155
- Produces: `bachuc_workspace::PaneItem { key: String, title: gpui_kit::SharedString, icon: bachuc_icons::IconName, modified: bool, view: gpui_kit::AnyView }` and `bachuc_workspace::SplitAxis` (enum `Right`, `Down`) for TASK-155
- Produces: `bachuc_workspace::PaneEvent` (enum `Closed { key: String }`, `Activated { key: String }`, `NewTab`) emitted by `Workspace` for TASK-155

Expected result:

- The example shows tabs that match the Electron tab bar, and split right and down with dividers that resize.
- Drag reorder works inside a group and across groups.
- Cases:
  - Closing the active tab activates its neighbor, as Electron does.
  - Closing the last tab of a group removes the group.
  - Many tabs shrink to the min width, then scroll.

Anti-goal: the workspace example stays at or below 1% CPU while idle with three groups open; tripwire; read with `measure.sh 60` after splits work and at completion.

## Verify

- `packages/desktop-gpui/cargo.sh test -p bachuc-workspace` -> split model tests pass, including close-last-tab, neighbor activation, ratio clamping, and move across groups
- Agent check: bundle `packages/desktop-gpui/bundle.sh example workspace BachucWorkspace io.github.hieppp.bachuc-workspace`; compare the tab bar with the reference app on 6770 with `cua_repl` screenshots -> the parity list in the outcome shows match for tab sizes, colors, icons, close button, active state, and the new-tab button, or records each mismatch
- Agent check: split right, split down, drag a divider, and drag a tab to the other group with `cua_repl` -> the layout follows each action
- Agent check: `cua_repl` `getAXState` -> tabs, close buttons, and dividers have roles and labels
- Anti-goal: `measure.sh 60 ws="pids:<pid>"` idle with three groups -> CPU at most 1.0
