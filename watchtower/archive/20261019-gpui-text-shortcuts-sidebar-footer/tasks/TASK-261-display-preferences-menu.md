# TASK-261 Display preferences menu

Group: B (writes the sidebar crate; shared files with TASK-258, TASK-262, TASK-263)
Class: risky

## Brief

Goal: the GPUI sidebar has Electron's Display preferences menu, and Grouping and the filters work.

Change: the section title has no menu ([packages/desktop-gpui/crates/sidebar/src/sidebar.rs](packages/desktop-gpui/crates/sidebar/src/sidebar.rs) lines 1551-1572 call it out of scope) -> a sliders button with the tooltip "Display preferences" opens the menu.

Design: [packages/app/src/components/sidebar/display-preferences/menu.tsx](packages/app/src/components/sidebar/display-preferences/menu.tsx) and [packages/app/src/components/sidebar/display-preferences/model.ts](packages/app/src/components/sidebar/display-preferences/model.ts)

Boundaries:

- Trigger: lucide `Settings2` at 14 px in a 28x28 hover button, at the right of the section title. `SECTION_TITLE_HEIGHT` already leaves room. Menu width 232.
- Root rows, in Electron order: Grouping (Project, Status), Title (Title, Branch name), Show (sub-page), then Project filter and Label filter. Each submenu row shows its current value at the right, with `submenu_row_label` / `submenu_trailing` ([packages/desktop-gpui/crates/sidebar/src/plugin.rs](packages/desktop-gpui/crates/sidebar/src/plugin.rs) lines 491-500).
- Show page: toggles for Branch, Project, Pull request, Services, Labels; a Checks sub-page (Icon and text, Icon only, Hidden); and one trailing slot, Diff stats or Last activity, where a second click clears it. Defaults follow Electron: branch off, project off, pull request on, services on, labels on ([packages/app/src/components/sidebar/display-preferences/row-items.ts](packages/app/src/components/sidebar/display-preferences/row-items.ts) lines 29-36); checks `IconAndText`. Copy the trailing default from Electron's settings defaults.
- Show the Project filter only with more than one project. Show the Label filter when labels exist or a label filter is on. These are Electron's rules (menu.tsx lines 187-193).
- Host row item and Host filter are out of scope ([watchtower/CONTEXT.md](watchtower/CONTEXT.md)).
- Status grouping: add a grouping mode to `build_rows` ([packages/desktop-gpui/crates/sidebar/src/model.rs](packages/desktop-gpui/crates/sidebar/src/model.rs) lines 436-514) that groups workspaces by `StatusBucket` like [packages/app/src/components/sidebar/sidebar-status-list.tsx](packages/app/src/components/sidebar/sidebar-status-list.tsx). Keep the rule in a pure function with tests.
- Store every choice in `SidebarPrefs` from TASK-258. Strings come from [packages/app/src/i18n/resources/en.ts](packages/app/src/i18n/resources/en.ts) (lines 1104-1144).
- Build the menu with `PopupMenu` and `submenu`, like `add_plugin_items`.
- Class risky: `build_rows` and `RowShape` drive the sidebar list on every update.

Files:

- [packages/desktop-gpui/crates/sidebar/src/sidebar.rs](packages/desktop-gpui/crates/sidebar/src/sidebar.rs) (trigger button and menu)
- [packages/desktop-gpui/crates/sidebar/src/model.rs](packages/desktop-gpui/crates/sidebar/src/model.rs) (grouping mode, filters in `build_rows`)
- [packages/desktop-gpui/crates/sidebar/src/prefs.rs](packages/desktop-gpui/crates/sidebar/src/prefs.rs) (display fields)

Interfaces:

- Consumes: `bachuc_sidebar::prefs::SidebarPrefs` with `load_from(dir: &Path) -> SidebarPrefs` and `save_to(&self, dir: &Path) -> std::io::Result<()>` from TASK-258
- Consumes: `WorkspaceNode.labels: Vec<WorkspaceLabel>` (name, color) from TASK-259
- Consumes: `WorkspaceNode.status_entered_at: Option<i64>` (ms since epoch) from TASK-259
- Produces: `SidebarPrefs` fields `title_source: TitleSource` (`Title`, `Branch`), `row_items: RowItems` (`branch`, `project`, `pull_request`, `services`, `labels`: bool), `checks_display: ChecksDisplay` (`IconAndText`, `IconOnly`, `Hidden`), `trailing: Option<Trailing>` (`DiffStats`, `LastActivity`) for TASK-262

Expected result:

- The menu matches the owner screenshot: Grouping, Title, Show, and Project, with values at the right.
- Status grouping lists workspaces under status headings. The choices survive a relaunch.

Anti-goal: `build_rows` stays under 2 ms for 50 projects with 500 workspaces in Project mode and in Status mode; read from a unit benchmark test at start and end of the TASK.

## Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-sidebar` -> new tests pass: Status grouping order, Project filter, Label filter, prefs round trip of every display field, and the 500-workspace timing under 2 ms.
- Screen check (cua_repl, AX only): press the "Display preferences" button -> the menu shows Grouping, Title, Show, and Project with current values.
- Owner hand check: hover tooltip and submenus match Electron; each choice changes the list and survives a relaunch.
