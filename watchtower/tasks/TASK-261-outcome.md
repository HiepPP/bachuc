# TASK-261 Outcome

## Status

DONE (code). Screen checks and the owner hand check are DEFERRED (UI checker).

## Changed

- `packages/desktop-gpui/crates/sidebar/src/prefs.rs`: `TitleSource`, `RowItems`, `RowItem`, `ChecksDisplay`, `Trailing` (with `Trailing::toggled`); `SidebarPrefs` fields `title_source`, `row_items`, `checks_display`, `trailing`; `parse`, `to_json`, and `retain_projects` cover them and the new `ViewState` fields; manual `Default` (trailing `Some(DiffStats)`).
- `packages/desktop-gpui/crates/sidebar/src/model.rs`: `Grouping`; `ViewState` gains `grouping`, `collapsed_statuses`, `project_filter`, `label_filter`; `Group::Status`; `Row::StatusHeader`; `WorkspaceRow.status_entered_at`; `status_groups`, `label_key`, `label_filter_matches`, `label_options`, `project_filter_visible`, `label_filter_visible`, `project_filter_value`; `build_rows` takes the grouping and both filters from `ViewState` (same signature); `sorted` now borrows instead of cloning every workspace; `natural_compare` returns early on equal text.
- `packages/desktop-gpui/crates/sidebar/src/sidebar.rs`: the 28 px "Display preferences" button with tooltip (`display_button`, `KebabButton.always_visible`), `display_menu` built with `PopupMenu`/`submenu`, the setters, `render_status_header`, project-icon leading for status rows.
- Tests: `crates/sidebar/tests/model.rs`, `crates/sidebar/tests/plugin.rs` (`row_ids` match arm for `StatusHeader`).

## Contract

- `SidebarPrefs { view: ViewState, title_source: TitleSource, row_items: RowItems, checks_display: ChecksDisplay, trailing: Option<Trailing> }` in `bachuc_sidebar::prefs`, as specified. `RowItems { branch, project, pull_request, services, labels }`. Defaults: branch off, project off, pull request on, services on, labels on; `IconAndText`; `Some(DiffStats)` (Electron `sidebarWorkspaceTrailing: "diff"`).
- Grouping, the two filters, and the collapsed status headings sit in `ViewState` (`grouping`, `project_filter` as project ids, `label_filter` as `label_key`s with `""` for no label, `collapsed_statuses`) because `build_rows` already takes `&ViewState`. `sidebar.json` keys: `grouping`, `projectFilter`, `labelFilter`, `collapsedStatuses`, `titleSource`, `rowItems`, `checksDisplay`, `trailing`.
- Status grouping order is `StatusBucket::GROUP_ORDER` (needs input, failed, ready to review, working, done); a group sorts by `status_entered_at` newest first (none last), then project name, name, id.

## Verified

- `./cargo.sh test -p bachuc-sidebar` -> 13 lib, 1, 31, 32 pass; 0 fail.
- New tests: status grouping order and block ends; status sort; collapsed status group; project filter (kept without workspaces, stale ids filter nothing, status mode); label filter (any label, `""` for none, case and space ignored); label options; filter row rules; prefs round trip of every display field and every choice; defaults; trailing toggle; a bad display field falls back alone; the two 500-workspace timings.
- `./cargo.sh fmt --check -p bachuc-sidebar` -> exit 0. `./cargo.sh clippy -p bachuc-sidebar --all-targets` -> no new warning (`kebab` argument count and the `plugin.rs` test `unwrap` are older).
- Screen check (cua_repl, AX only): press "Display preferences" -> Grouping, Title, Show, Project with values: DEFERRED (UI checker).
- Owner hand check (tooltip, submenus, each choice changes the list, survives a relaunch): DEFERRED (UI checker).

## Anti-goal

| Checkpoint                     | Value                  | Source                                        | Time  |
| ------------------------------ | ---------------------- | --------------------------------------------- | ----- |
| start, Project mode (old code) | 1.81, 1.11, 1.09, 1.13 | `building_500_workspaces_in_project_mode...`  | 11:28 |
| start, Status mode             | not available          | the mode did not exist                        | -     |
| first Status build             | 1.91 to 2.08 ms        | `building_500_workspaces_in_status_mode...`   | 11:36 |
| end, Project mode              | 0.98, 0.98, 1.53 ms    | same tests, `--test-threads=1`, debug, load 5 | 11:37 |
| end, Status mode               | 1.29, 1.31, 1.44 ms    | same                                          | 11:37 |

The first Status build failed the 2 ms bound; `natural_compare` now returns early on equal text, which removed the cost of comparing the same project name on every sort step. Both tests assert under 2 ms on the median of 21 runs of 50 projects with 500 workspaces, in a debug build.

## Lessons

- The gpui-kit `PopupMenu` closes after every item click, so the Show toggles close the menu where Electron keeps it open. A change needs the vendored popup, not this crate.
- A submenu row has no trailing slot, so the value follows the title through `submenu_row_label` ("Grouping: Project"), as the plugin menu does.
- `sorted` cloned every project and workspace on each build; borrowing them paid for the new row work.
- The label catalog is not exposed by the client, only the labels on workspaces, so the Label filter lists labels some workspace carries. A label on no workspace has no row.
- Safe defaults taken: the Project row shows "All projects", the one chosen name, or "N projects"; the Label row shows no value. "Manage labels" and the filter empty state are not ported. A filter that hides everything leaves only the section title.

## UI Checks

Lead, 2026-10-19, check bundle on live (6768), cua_repl:

- The Display preferences button pressed by AX opens the menu: Grouping: Project, Title: Title, Show, Project: All projects. PASS.
- Visible difference: GPUI writes the value after a colon ("Grouping: Project"); Electron shows it right-aligned. Tooltip, submenus, and choices after relaunch: owner hand check Q-171.

## UI Checks 2 (lead, 2026-10-19 13:50)

- Not run: the display did not repaint and scroll did nothing, so the row was not reachable in the virtual list.
