# TASK-175 Outcome

## Status

BLOCKED

Blocked: BUILT, UNVERIFIED (Q-109). The code and its tests pass; the swipe gesture and the slide need the owner's hand check on screen.

## Changed

- `packages/desktop-gpui/crates/sidebar/Cargo.toml`: adds `bachuc-plugin-host`.
- `packages/desktop-gpui/crates/sidebar/src/plugin.rs` (new, `pub mod plugin`): the pure rules.
  - `nav_rows`, `NavRow`, `nav_press`, `NavPress`.
  - `sidebar_project`, `added_sidebar_project`, `visible_projects`, `filtered_rows`, `pick_title`.
  - `WheelGesture`, `wheel_deltas`, `is_horizontal`.
  - `slide_frame`, `SlideFrame`.
  - `submenu_trailing`, `submenu_row_label`, `run_in_order`.
- `packages/desktop-gpui/crates/sidebar/src/sidebar.rs`: the plugin parts of the view.
  - Reads contributions only from `PluginHost::index`, and rebuilds on `ContributionsChanged`, `PluginStateChanged`, and `SidebarFilterChanged`.
  - Plugin nav rows after Search, plugin sections between the list and the footer, the filter title, the swipe handler, the slide, plugin items in the project menu, and the Add project button.
- `packages/desktop-gpui/crates/sidebar/src/lib.rs`: exports `plugin` and `SidebarPluginEvent`.
- `packages/desktop-gpui/crates/sidebar/tests/plugin.rs` (new): 24 tests.

## Contract

- Produces `bachuc_sidebar::SidebarPluginEvent` (`OpenSurface { plugin_id, surface_id }`, `ProjectAdded { project_id, cwd }`, `Clone, Debug, PartialEq, Eq`) through a second `EventEmitter` impl, and `Sidebar::set_active_surface(&mut self, Option<(String, String)>, cx)`.
- `SidebarEvent`, `Sidebar::new`, and `set_selected_workspace` are unchanged. `model::build_rows` and every other public model item are unchanged; `tests/model.rs` passes as it was.
- The host is found with `bachuc_plugin_host::host(cx)` in `Sidebar::new` and again in each `render` until it exists, so the app may create the host before or after the sidebar. With no host, the sidebar looks as today plus the Add project button.
- Defaults taken where the spec left a gap:
  - A GPUI scroll event moves the content, and the web event moves the page, so `wheel_deltas` flips both signs. A finger moving left gives a positive `x`, and `x > 0` is `Next`, as in Electron.
  - GPUI's `ScrollWheelEvent` has no pressed-button field. "A mouse button held" is `cx.has_active_drag()` and the `ctrl` modifier (Electron skips `ctrlKey`).
  - The wheel handler is on each list row, not on the list. GPUI runs the list's own scroll handler before a parent handler, and a row handler runs before the list, so a horizontal-dominant scroll stops there and the list does not scroll.
  - A swipe is read only when some filter has `on_swipe`; with none, the scroll goes to the list as before.
  - The gpui-kit submenu row has no trailing slot. The value follows the title as `Title: value` (`submenu_row_label`).
  - A disabled item with children shows as a plain disabled item without a submenu (the kit `submenu` call takes no disabled flag).
  - A checked item uses `check_side(Side::Right)` on the project menu and its submenus, so the check is trailing.
  - When filters hide every project, the list keeps the title row and shows no onboarding row.
  - A failed `add_project` shows its error text with `push_notification` and emits nothing.
  - The disabled nav row has no `aria_disabled` in this GPUI; it carries the description "Disabled: no workspace is open".
  - `new_workspace_panel` on a sidebar action is ignored (Planner call in the spec).
- The slide applies the filter change 65 ms after the event with a timer task. A second change during a slide applies the first one at once. `render` asks for animation frames only while a slide runs.
- The sidebar never writes the selection when a filter hides the selected workspace. `refresh` touches only `rows`, so the app's selection and open tab stay valid.
- `Cargo.lock` gained the `bachuc-plugin-host` entry for `bachuc-sidebar`; the app builder also edits that file in this checkout.

## Verified

- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test -p bachuc-sidebar` -> exit 0; `tests/model.rs` 21 passed unchanged, `tests/fake_daemon.rs` 1 passed, `tests/plugin.rs` 24 passed, 0 failed. `tests/plugin.rs` covers nav order after Search, the disabled rule, the active-surface rule, hidden projects with their workspaces (pinned too), two filters, the view key fallback, the no-empty-row rule, the title pick, the wheel cases (`(10,0),(10,2),(10,1)` gives one `Next`; a 130 ms gap restarts; `(-30,0)` gives `Previous`; `(20,20)` gives none; reversal; decaying tail), the sign flip, the slide at 0, 65, and 170 ms, the submenu trailing value, the Add project sequence with an `Err("full")` that stops the later filters, a hidden selected workspace, and the 500-project timing test.
- `packages/desktop-gpui/cargo.sh fmt --check -p bachuc-sidebar` -> exit 0.
- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh build -p bachuc-app` -> Finished, no error. The only warnings are 3 `dead_code` warnings in `crates/app` (group D files). No edit outside `crates/sidebar` and this sidecar.
- `rg -n "SidebarEvent::" packages/desktop-gpui/crates/app/src/shell.rs` -> 3 arms (`OpenWorkspace`, `OpenSettings`, `OpenCommandCenter`), as before.
- GitNexus `impact Sidebar --direction upstream`: the struct in `crates/sidebar/src/sidebar.rs` is LOW (15 impacted); the change only adds fields and methods. The 3 `Sidebar::new` and `set_selected_workspace` uses in `crates/app/src/shell.rs` are confirmed by `rg`.
- UI check: with live running, open the GPUI build with `open -g`, background screenshot of the sidebar, compare the footer with the Electron 6770 build -> DEFERRED (UI checker).
- Plugin rows, sections, the slide, the swipe, and the project menu items on screen -> UNVERIFIED (unattended): code tests only. No plugin crate is registered yet (TASK-191), so there is nothing to draw. Live daemon 6768 is down.

## Anti-goal

- Checkpoint 0, before any change: `build_rows` for 500 projects (3 workspaces each, 10 pinned) took 5.148 ms in a debug test, no filters; source `tests/plugin.rs` baseline run; 2026-10-16 23:12.
- Checkpoint 1, after filters landed (`visible_projects` + `filtered_rows`, two filters): 3.693 ms, under the 10 ms limit; source `cargo test -p bachuc-sidebar -- --nocapture`; 23:18. This run was warm; the test calls the rebuild twice and times the second call. The 5.148 ms figure was a cold first call.
- Checkpoint 2, at completion: the timing test passed again (limit 10 ms) in the final `test -p bachuc-sidebar` run; 23:19.
- A rebuild with filters that answer the same gives an equal row list (`a_filter_change_that_hides_nothing_new_leaves_the_rows_equal`), so `refresh` skips the splice and the redraw.
- Tripwire not hit.

## Lessons

- GPUI runs a list's own scroll handler before a parent's handler, because the list registers its window listener during paint, after the parent. To stop a list from scrolling, put the handler on a row (a child painted after the list registers).
- `AsyncWindowContext::update` takes `&mut self`. A chain of `FnOnce` steps that each call it needs one clone of the context per step.
- A gpui-kit `PopupMenuItem::checked` shows on the left by default; set `check_side(Side::Right)` for a trailing check.
