# TASK-280 GPUI sidebar drag to reorder projects, threads, and pinned rows

Group: A (shared files: `crates/sidebar/src/model.rs`, `crates/sidebar/src/sidebar.rs`, with TASK-278 and TASK-279)
Class: risky

## Brief

Goal: The owner drags a project, a thread inside its project, or a pinned row to a new place in the GPUI sidebar. The new order is saved, as in Electron.

Change: no reorder in GPUI -> drag and drop rows, saved to `sidebar.json` with Electron's merge rules.

Design: [watchtower/CONTEXT.md](watchtower/CONTEXT.md)

Boundaries:

- Three drag kinds, as in Electron: a project among projects, a thread among the unpinned threads of its own project, and a pinned row among pinned rows. A drop on a row of another kind or another project does nothing.
- No drag in status grouping, on the section title, on a status heading, on a "show more" row, or on the empty state. Planner call: Electron has no drag in status grouping either.
- The move rule: take the visible keys of the list. Remove the dragged key, then insert it at the index the target had. A drag down lands after the target, and a drag up lands before it.
- Save with Electron's rules. If `has_visible_order_changed(current, reordered)` is false, do nothing. Otherwise, set the saved order to `merge_with_remainder(current, reordered)`. Keys that are hidden by a filter, by "show more", by a collapse, or by a pin stay in the remainder.
- Follow the tab drag in [workspace.rs](packages/desktop-gpui/crates/workspace/src/workspace.rs): `on_drag` with a drag value and a preview view on the tab (around line 1159), and `on_drag_move::<TabDrag>` and `on_drop::<TabDrag>` on the tab bar (around lines 943 and 946). For the target row style, GPUI's `drag_over::<T>` exists on `Div`, but no crate uses it yet.
- Trap: a row opens its thread on click, through `OnPress` in [sidebar.rs](packages/desktop-gpui/crates/sidebar/src/sidebar.rs). Nothing proves that GPUI skips `on_click` after a drag. Check it, and guard the click if it fires. A drag must not open the thread or change the selection.
- Trap: [sidebar.rs](packages/desktop-gpui/crates/sidebar/src/sidebar.rs) already has `ResizeDrag` on the resize handle, and [plugin.rs](packages/desktop-gpui/crates/sidebar/src/plugin.rs) has the Spaces swipe. A row drag must not start a resize or a swipe. A swipe must not start a row drag.
- Drag and drop needs the owner's hand check. The UI checker cannot move the pointer, per [docs/gpui.md](docs/gpui.md) Screen checks.

How:

- In [model.rs](packages/desktop-gpui/crates/sidebar/src/model.rs), add the pure helpers `reorder_visible`, `merge_with_remainder`, and `has_visible_order_changed`.
- In [sidebar.rs](packages/desktop-gpui/crates/sidebar/src/sidebar.rs), add a `RowDrag` value that holds the kind, the key, the project view key for a thread, and the title for the preview. Put `on_drag`, `drag_over`, and `on_drop` on project rows, thread rows, and pinned rows in project grouping.
- On drop, build the visible keys of that list from the current rows. Apply the helpers to `prefs.view.order`, call `save_prefs`, and call `refresh`.
- Keep the preview small: the row title in the row style, as the tab preview does.

Files:

- [packages/desktop-gpui/crates/sidebar/src/model.rs](packages/desktop-gpui/crates/sidebar/src/model.rs) (pure reorder helpers)
- [packages/desktop-gpui/crates/sidebar/src/sidebar.rs](packages/desktop-gpui/crates/sidebar/src/sidebar.rs) (drag, drop target style, save)
- [packages/desktop-gpui/crates/sidebar/tests/model.rs](packages/desktop-gpui/crates/sidebar/tests/model.rs) (red tests)

Interfaces:

- Consumes: `bachuc_sidebar::model::SidebarOrder { pub project_order: Vec<String>, pub workspace_order_by_project: HashMap<String, Vec<String>>, pub pinned_workspace_order: Vec<String> }`, which derives `Clone, Debug, Default, PartialEq, Eq`, with `SidebarOrder::is_empty(&self) -> bool`, from TASK-278
- Consumes: `ViewState.order: SidebarOrder` from TASK-278
- Consumes: `Facts.server_id: Option<String>` and `bachuc_sidebar::model::workspace_key(server_id: &str, workspace_id: &str) -> String`, from TASK-278
- Produces: `bachuc_sidebar::model::reorder_visible(visible: &[String], dragged: &str, target: &str) -> Vec<String>`
- Produces: `bachuc_sidebar::model::merge_with_remainder(current: &[String], reordered_visible: &[String]) -> Vec<String>`
- Produces: `bachuc_sidebar::model::has_visible_order_changed(current: &[String], reordered_visible: &[String]) -> bool`

Expected result:

- WHEN the owner drops a project on another project row THE sidebar SHALL move it to that row's place and save `projectOrder`.
- WHEN the owner drops a thread on another thread row of the same project THE sidebar SHALL move it to that row's place and save that project's `workspaceOrderByProject` entry.
- WHEN the owner drops a pinned row on another pinned row THE sidebar SHALL move it to that row's place and save `pinnedWorkspaceOrder`.
- IF the drop target is another kind, another project, or the dragged row itself THEN THE sidebar SHALL keep the order and SHALL NOT write `sidebar.json`.
- WHILE status grouping is on, THE sidebar SHALL NOT start a row drag.
- WHEN a drag ends THE sidebar SHALL NOT open a thread or change the selection.

Red tests:

File: [packages/desktop-gpui/crates/sidebar/tests/model.rs](packages/desktop-gpui/crates/sidebar/tests/model.rs)

```rust
fn keys(items: &[&str]) -> Vec<String> {
    items.iter().map(|item| item.to_string()).collect()
}

#[test]
fn a_drop_moves_the_dragged_key_to_the_target_index() {
    let visible = keys(&["a", "b", "c"]);
    assert_eq!(reorder_visible(&visible, "a", "c"), keys(&["b", "c", "a"]));
    assert_eq!(reorder_visible(&visible, "c", "a"), keys(&["c", "a", "b"]));
    assert_eq!(reorder_visible(&visible, "b", "b"), visible);
    assert_eq!(
        reorder_visible(&visible, "x", "a"),
        visible,
        "an unknown dragged key changes nothing"
    );
}

#[test]
fn a_merge_puts_the_visible_order_first_and_keeps_hidden_keys() {
    let current = keys(&["hidden", "a", "b", "c"]);
    assert_eq!(
        merge_with_remainder(&current, &keys(&["c", "a", "b"])),
        keys(&["c", "a", "b", "hidden"])
    );
}

#[test]
fn only_a_changed_visible_order_counts_as_a_change() {
    let current = keys(&["hidden", "a", "b"]);
    assert!(!has_visible_order_changed(&current, &keys(&["a", "b"])));
    assert!(has_visible_order_changed(&current, &keys(&["b", "a"])));
    assert!(
        has_visible_order_changed(&current, &keys(&["a", "b", "new"])),
        "a visible key the saved order lacks counts as a change"
    );
}
```

Anti-goal: a click on a row still opens the thread in one press, with no drag preview, in 10 of 10 tries by the owner. Tripwire: a click that shows a drag preview or needs a second press.

## Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-sidebar --test model` -> the three red tests pass, with the TASK-278 tests.
- `cd packages/desktop-gpui && ./cargo.sh clippy -p bachuc-sidebar -- -D warnings` -> no warnings.
- Owner hand check on a check bundle: drag a project, a thread, and a pinned row. Each lands at the target place, and each stays there after a GPUI restart.
- Owner hand check: drop a thread on a row of another project, then on a project row -> nothing moves.
- Owner hand check: in status grouping, try to drag a row -> no drag starts.
- Anti-goal, owner hand check: click 10 rows -> each opens its thread in one press, with no drag preview.
- After the checks, run `./cargo.sh clean --profile dev` when no other `cargo` or `rustc` process runs.
