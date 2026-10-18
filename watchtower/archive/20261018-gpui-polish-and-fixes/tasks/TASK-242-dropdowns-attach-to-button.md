# TASK-242 Dropdowns attach to their button

Group: H (files of groups S, C, W, and T; runs last, so ordering alone keeps it apart)
Class: code

## Brief

Goal: Every GPUI dropdown and popover opens right under its button, with the standard small gap and no extra space.

Change: dropdowns with an extra gap under the button -> dropdowns attached to the button.

Boundaries:

- Owner screenshot 6: the project picker in New workspace opens with a large gap. Suspect: `.mt(px(BADGE_HEIGHT + SPACE_1))` at [packages/desktop-gpui/crates/app/src/new_workspace.rs](packages/desktop-gpui/crates/app/src/new_workspace.rs) line 1093, added on top of an anchored element that is already below the chip.
- Check every hand-rolled `anchored()` site: `new_workspace.rs` about 1088, `chrome.rs` about 1231, `workspace.rs` about 1248, `timeline/src/lib.rs` about 1759, `explorer/src/explorer.rs` about 2212, `pull_request_view.rs` about 1367, `git_actions_ui.rs` about 442, `branch_switcher.rs` about 810, `workspace-spaces/src/space_menu.rs` about 484. Also check gpui-kit `dropdown_menu` and `Popover` sites for the same gap.
- Use one gap value from `bachuc-theme` for all, matching Electron menus (the open-in menu in owner screenshot 3 sits about 4 px under its button).
- Planner call: fix every site with a gap; list sites with no gap as checked.
- If many sites repeat the same anchor code, add one small helper and use it. Do not build a new popover system.

How:

- Measure the gap at each site in a UI checker screenshot.
- Fix each site that differs; put the offset rule in a pure function with a test.
- List each site and its before and after gap in the outcome.

Files:

- [packages/desktop-gpui/crates/app/src/new_workspace.rs](packages/desktop-gpui/crates/app/src/new_workspace.rs) (project picker)
- The other anchored and popover sites listed above that show a gap

Expected result:

- The project picker opens about 4 px under the chip.
- No dropdown has a gap larger than the theme gap.

Anti-goal: no dropdown opens off-screen or over its own button; read from UI checker screenshots near the window edge.

## Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-theme` -> passes with the offset rule test.
- UI checker: open the New workspace project picker -> gap about 4 px.
- UI checker: open one gpui-kit `dropdown_menu` and one `Popover` site -> gap equal to the theme gap.
- UI checker: open each dropdown in the outcome list -> gap equal to the theme gap; a dropdown near the bottom edge flips up and stays on screen.
