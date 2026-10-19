# TASK-262 Outcome

## Status

DONE (code). Screen check and the `measure.sh` scroll sample are DEFERRED (UI checker).

## Changed

- `packages/desktop-gpui/crates/sidebar/src/model.rs`: `WorkspaceRow` gains `pull_request: Option<RowPullRequest>`, `labels`, `service: Option<ServiceSummary>` (copied when rows build; `status_entered_at` came with TASK-261); `Facts.services` and `TreeSource::workspace_scripts`; pure rules `meta_items`, `has_meta_line`, `trailing_slot`, `row_title`, `compact_time_ago`, `label_color`, `select_service_summary`, `workspace_label_for`; `RowShape` has `meta_line`, and `Row::shape_with(items, checks)` sets it.
- `packages/desktop-gpui/crates/sidebar/src/sidebar.rs`: the title source, the trailing slot, and the meta line (`render_meta_row`, `render_meta_item`); `refresh` compares shapes under the Show choices, so a toggle re-measures only the rows that change height; a `GitStatusEvent::ScriptsChanged` subscription refreshes the Services item.
- `packages/desktop-gpui/crates/sidebar/src/prefs.rs` (outside the Files list, one helper): `RowItems::none()`, used for the no-meta shape.
- Tests: `crates/sidebar/tests/model.rs`.

## Contract

- Consumes TASK-261 `SidebarPrefs` fields and TASK-259 `WorkspaceNode.pull_request`, `labels`, `status_entered_at` exactly as named.
- `meta_items(row, &RowItems, ChecksDisplay, leading_project) -> Vec<MetaItem>`; order Branch, Project, PullRequest, Checks, Services, Labels. `trailing_slot(row, Option<Trailing>) -> Option<TrailingSlot>` is one of `Diff` or `LastActivity`. `row_title(row, TitleSource)` falls back to `name`.
- The Project item shows only on a row that leads with the project icon (pinned and status rows); under a project heading it would repeat the heading, as Electron's `leadingProjectName` does (`sidebar-workspace-list.tsx` line 2397, `sidebar-status-list.tsx` line 865).

## Verified

- `./cargo.sh test -p bachuc-sidebar` -> 13 lib, 1, 43, 32 pass; 0 fail.
- New tests: items in Electron order with Checks label on and off; toggles off give no items; Checks follow their own choice; missing data hides each item; the Project item needs a leading row; a meta line changes the row shape and an all-off row equals a bare row; trailing is diff or last activity; branch title falls back to name; service summary rules (running services only, unhealthy wins); compact time (now, m, h, d, date); label color names; rows copy PR, labels, and service when they build.
- `./cargo.sh fmt --check -p bachuc-sidebar` -> exit 0. `./cargo.sh clippy -p bachuc-sidebar --all-targets` -> no new warning (older: `kebab` argument count, `plugin.rs` test `unwrap`). `./cargo.sh build -p bachuc-app` -> builds.
- Screen check (every Show item on -> a PR row shows branch, PR, checks; compare with Electron): DEFERRED (UI checker).

## Anti-goal

Scroll CPU with `measure.sh` (start and end): DEFERRED (UI checker). Code-level checkpoints:

| Checkpoint                                      | Value           | Source                                       | Time  |
| ----------------------------------------------- | --------------- | -------------------------------------------- | ----- |
| `build_rows` 500 workspaces, no PR or labels    | 1.01 ms         | `building_500_workspaces_in_project_mode...` | 11:41 |
| `build_rows` 500 workspaces, each with PR+label | 1.07 to 1.08 ms | `building_500_workspaces_with_meta_data...`  | 11:41 |

The row callback reads only `self.rows` and `self.prefs`: it builds the meta items from the row, makes no request, and reads no `ListState`. `now_ms()` is the only clock read, once per row with a Last activity slot.

## Lessons

- The row height depends on the Show choices, so the choices belong in `RowShape`; a toggle then re-measures only the rows whose shape changed, and an unchanged shape needs no new measure.
- Last activity is drawn from the clock at draw time and does not tick; it updates on the next row redraw. Electron re-renders each minute.
- Checks draw as lucide marks (check, cross, dot), not Electron's SVG pie, because the icon set has no pie glyph.
- Pressing the PR chip opens its URL with `cx.open_url` and stops the click, so the row does not open its workspace.

## UI Checks

Lead, 2026-10-19, check bundle on live (6768), cua_repl:

- Show items on rows: UNVERIFIED (unattended); live has no workspace with a PR or labels. measure.sh scroll CPU: not run.
