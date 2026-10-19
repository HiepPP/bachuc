# TASK-262 Sidebar row meta items

Group: B (writes [packages/desktop-gpui/crates/sidebar/src/sidebar.rs](packages/desktop-gpui/crates/sidebar/src/sidebar.rs); shared files with TASK-258, TASK-261, TASK-263)
Class: code

## Brief

Goal: each Show choice from TASK-261 draws on the workspace rows, as in Electron.

Change: rows draw status, name, diff, and kebab only ([packages/desktop-gpui/crates/sidebar/src/sidebar.rs](packages/desktop-gpui/crates/sidebar/src/sidebar.rs) lines 1114-1168) -> rows also draw the meta row, the trailing slot, and the title source.

Design: [packages/app/src/components/sidebar/workspace-meta-row/index.tsx](packages/app/src/components/sidebar/workspace-meta-row/index.tsx), [packages/app/src/components/sidebar/workspace-meta-row/meta-items.ts](packages/app/src/components/sidebar/workspace-meta-row/meta-items.ts), and [packages/app/src/components/sidebar/workspace-trailing/index.tsx](packages/app/src/components/sidebar/workspace-trailing/index.tsx)

Boundaries:

- Meta items, in Electron order: Branch, Project, Pull request, Checks, Services, Labels. Each shows only when its toggle is on and its data exists.
- Data: branch from `WorkspaceRow.branch`; project from `project_name`; PR, checks, labels, and last activity from TASK-259; services from `Client::workspace_scripts` ([packages/desktop-gpui/crates/client/src/lib.rs](packages/desktop-gpui/crates/client/src/lib.rs) line 1140). Copy the needed values into `WorkspaceRow` when rows build, not in the row callback.
- Trailing slot: Diff stats or Last activity, never both. Last activity uses `status_entered_at`.
- Title source "Branch name" draws `branch` and falls back to `name`, as in [packages/app/src/components/sidebar/sidebar-workspace-title.ts](packages/app/src/components/sidebar/sidebar-workspace-title.ts).
- Row height follows Electron when the meta row shows. Do not read a `ListState` in the row callback.
- Planner call: the Services item shows on desktop, as Electron does on desktop.

Files:

- [packages/desktop-gpui/crates/sidebar/src/sidebar.rs](packages/desktop-gpui/crates/sidebar/src/sidebar.rs) (meta row, trailing slot, title source)
- [packages/desktop-gpui/crates/sidebar/src/model.rs](packages/desktop-gpui/crates/sidebar/src/model.rs) (row fields, pure "which items show" rule)

Interfaces:

- Consumes: `SidebarPrefs` fields `title_source: TitleSource` (`Title`, `Branch`), `row_items: RowItems` (`branch`, `project`, `pull_request`, `services`, `labels`: bool), `checks_display: ChecksDisplay` (`IconAndText`, `IconOnly`, `Hidden`), `trailing: Option<Trailing>` (`DiffStats`, `LastActivity`) from TASK-261
- Consumes: `WorkspaceNode.pull_request: Option<WorkspacePullRequest>` (number, title, url, state, is_merged, is_draft, forge, `checks: Option<CheckSummary>`) from TASK-259
- Consumes: `WorkspaceNode.labels: Vec<WorkspaceLabel>` (name, color) from TASK-259
- Consumes: `WorkspaceNode.status_entered_at: Option<i64>` (ms since epoch) from TASK-259

Expected result:

- Turning on Branch, Pull request, Checks, Services, or Labels shows that item on rows that have the data, matching Electron.

Anti-goal: sidebar scroll stays smooth; CPU while scrolling 500 rows for 10 s stays within 10% of the value before this TASK; read with `measure.sh` at start and end.

## Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-sidebar` -> new tests pass for the pure rule: toggles off give no items; missing data hides an item; trailing is one of diff or last activity; branch title falls back to name.
- Screen check (cua_repl, AX only): turn on every Show item -> a row with a PR shows branch, PR, and checks; compare with the Electron live screenshot.
- `measure.sh` scroll sample -> CPU within 10% of the start value.
