# TASK-216 Sidebar History and New workspace rows

Group: N (shared files [packages/desktop-gpui/crates/sidebar/src/sidebar.rs](packages/desktop-gpui/crates/sidebar/src/sidebar.rs) and [packages/desktop-gpui/crates/command](packages/desktop-gpui/crates/command))
Class: code

## Brief

Goal: The sidebar has "New workspace" and "History" nav rows and a "+" on each project row, as Electron has.

Change: the nav has only Search and plugin rows (`sidebar.rs` lines 1156-1193) -> New workspace, Search, History, plugin rows; and a hover "+" per project.

Boundaries:

- Electron rows: [packages/app/src/components/sidebar/sidebar-nav-rows.tsx](packages/app/src/components/sidebar/sidebar-nav-rows.tsx) lines 83-108 (New workspace, with the shortcut hint) and lines 112-130 (History, icon `History`, label "History"). Match Electron's order.
- Per-project "+": `NewWorktreeButton` in [packages/app/src/components/sidebar-workspace-list.tsx](packages/app/src/components/sidebar-workspace-list.tsx) lines 442-450. It shows on row hover. Use opacity, never `display`, for the hover swap.
- The app matches `SidebarEvent` exhaustively (`shell.rs` lines 298-304). A new variant there breaks the app build until a group S TASK adds an arm. So emit the new actions as a second event type, `SidebarNavEvent`, as `SidebarPluginEvent` already does (`sidebar.rs` lines 85-89). Do not change `SidebarEvent`.
- The sidebar only emits events. The app opens the screens (TASK-200 for History, TASK-203 for New workspace).
- Match the existing nav row style and pitch.

How:

- Add the two rows and the project "+" button.
- Add `SidebarNavEvent` and `impl EventEmitter<SidebarNavEvent> for Sidebar`.

Files:

- [packages/desktop-gpui/crates/sidebar/src/sidebar.rs](packages/desktop-gpui/crates/sidebar/src/sidebar.rs) (rows, "+" button, new event type)

Interfaces:

- Produces: `SidebarNavEvent::OpenHistory` for TASK-200.
- Produces: `SidebarNavEvent::NewWorkspace { project_id: Option<String> }` for TASK-203.

Expected result:

- The nav shows New workspace, Search, and History rows in Electron's order, each with an icon.
- Click New workspace -> `SidebarNavEvent::NewWorkspace { project_id: None }`. Click a project "+" -> `SidebarNavEvent::NewWorkspace { project_id: Some(id) }`.
- Click History -> `SidebarNavEvent::OpenHistory`.
- The "+" shows only on project row hover, with no layout jump.
- Until TASK-200 and TASK-203 land, the new rows do nothing and the app still builds.

Anti-goal: sidebar row pitch must not change; tripwire; existing rows keep their height, proven by a screenshot measure before the change and at completion.

## Verify

- `cd packages/desktop-gpui && ./cargo.sh build -p bachuc-app` -> builds with the sidebar change.
- UI check on live: the nav rows show in order, and hover on a project shows the "+".
- Anti-goal: measure 3 existing rows on a screenshot -> same height at both reads.
