# TASK-280 Outcome

## Status

DONE (code). Every owner hand check is UNVERIFIED (owner hand check): drag, wrong-target drop, status grouping, and the 10-click anti-goal.

## Changed

- `packages/desktop-gpui/crates/sidebar/src/model.rs`: `reorder_visible`, `merge_with_remainder`, `has_visible_order_changed` (the three spec helpers), plus the glue `OrderList`, `visible_keys`, and `apply_drop`.
- `packages/desktop-gpui/crates/sidebar/src/sidebar.rs`: `RowDrag`, `RowPreview`, `DragKeys`, `with_row_drag`; `Sidebar.drag_keys` (filled in `refresh`), `project_drag`, `thread_drag`, `drop_row`; `render_project` and `render_workspace` wrap their press element with `with_row_drag`.
- `packages/desktop-gpui/crates/sidebar/tests/model.rs`: the three red tests.

## Contract

- Produces `reorder_visible(&[String], dragged: &str, target: &str) -> Vec<String>`, `merge_with_remainder(&[String], &[String]) -> Vec<String>`, `has_visible_order_changed(&[String], &[String]) -> bool` in `bachuc_sidebar::model`.
- Consumes `SidebarOrder`, `ViewState.order`, `Facts.server_id`, and `workspace_key` exactly as TASK-278 shipped them.
- Three drag lists, as in Electron: `OrderList::Projects` (key: project view key), `OrderList::Threads(project view key)` (key: `workspace_key`), `OrderList::Pinned` (key: `workspace_key`). A row carries a drag only when its key can be built: a project needs a view key; a thread needs the server id and its project's view key.
- No drag in status grouping (`thread_drag` returns `None`; status groups have no project rows), and none on the section title, a status heading, a "show more" row, the empty state, or a row that is archiving. These never call `with_row_drag`.
- Drop: `drop_row` returns at once when the dragged and target lists differ (another kind, or another project's threads) or the keys are equal. Otherwise it reads the visible keys of the list from `self.rows`, runs `apply_drop` (reorder, `has_visible_order_changed`, `merge_with_remainder`), and calls `save_prefs` and `refresh` only when the order changed. Hidden keys (filter, "show more", collapse, pin) stay in the remainder.
- The target row tints with `surface2` through `drag_over::<RowDrag>` only when the dragged row has the same list and another key.
- Preview: the row title on a `surface2` pill at 85% opacity, as the tab preview does.
- Click guard (read, not run): `on_drag` sits on the same element as `on_press`. In `gpui-pre-0.3.9` `div.rs` (about lines 2981-3010) a started drag runs `pending_mouse_down.take()`, and the click fires only from a pending mouse down captured at mouse up (about lines 3076-3100). So a drag fires no click, and a move below the drag threshold still clicks. No extra guard code was added.
- Resize and swipe: the resize handle is a separate element with its own `ResizeDrag` type, and the tree's `on_drag_move::<ResizeDrag>` ignores a `RowDrag`. The swipe is a wheel gesture, and `on_wheel` already returns while `cx.has_active_drag()`.

## Verified

- Red: before the helpers existed the three tests failed to compile (E0425, `reorder_visible`, `merge_with_remainder`, `has_visible_order_changed` missing). Counts as red.
- `./cargo.sh test -p bachuc-sidebar --test model` -> 55 passed (the three red tests, the TASK-278 tests, and every older test with unchanged expected values), 0 failed.
- `git diff -U0 packages/desktop-gpui/crates/sidebar/tests/model.rs | rg '^-\s'` -> no match: the test diff has only added lines.
- `./cargo.sh clippy -p bachuc-sidebar --no-deps --lib --test model --test electron_order -- -D warnings -A clippy::too_many_arguments` -> clean (the unmodified `-- -D warnings` fails on older code, see TASK-278 outcome).
- Owner hand check on a check bundle (drag a project, a thread, a pinned row; each lands at the target place and stays after a restart): UNVERIFIED (owner hand check).
- Owner hand check (drop on another project's thread, then on a project row -> nothing moves): UNVERIFIED (owner hand check).
- Owner hand check (status grouping: no drag starts): UNVERIFIED (owner hand check).
- The pure helpers in `apply_drop` and `visible_keys` have no test of their own. They are glue over the three tested helpers, and the spec asked for no further test. Their rules were read against `handleProjectDragEnd` and `handleWorkspaceReorder`.

## Anti-goal

Click opens the thread in one press, with no drag preview, in 10 of 10 tries: UNVERIFIED (owner hand check). The code basis is the click guard above. No checkpoint was measured by a builder.

## Lessons

- GPUI never fires a click after a drag on the same element, so a drag source does not need a click guard when `on_drag` and `on_click` are on one element. A drag on a child of a clickable parent would still need one.
- `Div::drag_over::<T>` takes the dragged value, so the target tint can test the kind and the key; the style needs no state in the view.
- `ProjectRow.color_key` falls back to the project id, so it cannot say whether a view key exists; the sidebar keeps `DragKeys` from the last `Facts` instead of adding a string to every row (`build_rows` stays at 1.1 to 1.3 ms for 500 workspaces).

## Group-level run

One run for TASK-278, TASK-279, and TASK-280 together (2026-10-20 14:59 to 15:00, after the last TASK, `CARGO_BUILD_JOBS=4`):

- `./cargo.sh test -p bachuc-sidebar` -> lib 19, `electron_order` 5 (1 ignored), `fake_daemon` 1, `model` 55, `plugin` 32 passed. One run of the `model` suite failed `building_500_workspaces_in_status_mode_stays_under_2_ms` under load (the suite runs on several threads). Alone it passes at 1.32 and 1.35 ms, and a rerun of the whole suite passes. Status grouping code is unchanged by the group (`sorted` returns at once with an empty order).
- `./cargo.sh check -p bachuc-app -p bachuc-composer` -> finished, no error. These crates use `bachuc-sidebar`.
- `./cargo.sh fmt --check -p bachuc-sidebar` -> exit 0.
- No `cargo clean` was run; the lead does it.

## Lead Check

- Same re-check as TASK-278. Committed in `9cc6e3d50`.
- Blocked: BUILT, UNVERIFIED (Q-180). Every drag check and the click anti-goal need the owner's hand, because the UI checker cannot move the pointer.
