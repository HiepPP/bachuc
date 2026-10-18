# TASK-242 Outcome

Status: DONE (code checks pass; screen checks DEFERRED to the UI checker).

## Changed

- `packages/desktop-gpui/crates/theme/src/metrics.rs`: `POPUP_GAP = SPACE_1` (4 px).
- `packages/desktop-gpui/crates/theme/src/lib.rs`: pure `popup_offset(trigger_height) -> f32` and its test.
- `packages/desktop-gpui/crates/app/src/new_workspace.rs`: project, isolation, and base picker.
- `packages/desktop-gpui/crates/app/src/chrome.rs`: header actions menu and `header_popover`.
- `packages/desktop-gpui/crates/workspace/src/workspace.rs`: split menu.
- `packages/desktop-gpui/crates/timeline/src/lib.rs`: fork menu.
- `packages/desktop-gpui/crates/explorer/src/pull_request_view.rs`, `git_actions_ui.rs`, `branch_switcher.rs`.
- `packages/desktop-gpui/crates/composer/src/composer.rs`, `plugin_slots.rs`, `agent_controls/mod.rs`: `Popover` offset.
- `packages/desktop-gpui/plugins/workspace-spaces/src/model.rs`: `MENU_GAP`.

## Contract

- `bachuc_theme::metrics::POPUP_GAP: f32` = 4.
- `bachuc_theme::popup_offset(trigger_height: f32) -> f32` = trigger height + `POPUP_GAP`. A menu wrapper sits at this `top` and its anchored child adds no margin.

## Site list (gap from code; pixel measurement is DEFERRED)

| Site                                                                                        | Before                                                                                             | After                                            |
| ------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------- | ------------------------------------------------ |
| New workspace pickers (`new_workspace.rs`)                                                  | `mt(32)` on an anchored child placed after the 28 px chip: 28 + 32 below the chip top, a 32 px gap | wrapper at `popup_offset(28)`, no margin: 4 px   |
| Header actions menu (`chrome.rs`)                                                           | 28 + 4: 4 px                                                                                       | `popup_offset(28)`: 4 px (unchanged)             |
| `header_popover` scripts and editor menus (`chrome.rs`)                                     | 28 + 4: 4 px                                                                                       | `popup_offset(28)`: 4 px (unchanged)             |
| Split menu (`workspace.rs`)                                                                 | 20 + 4: 4 px                                                                                       | `popup_offset(TOOLBAR_BUTTON)`: 4 px (unchanged) |
| Git actions menu (`git_actions_ui.rs`)                                                      | 26 + 4: 4 px                                                                                       | `popup_offset(26)`: 4 px (unchanged)             |
| Branch switcher `popup` (`branch_switcher.rs`)                                              | `top_full` + 4: 4 px                                                                               | `top_full` + `POPUP_GAP`: 4 px (unchanged)       |
| Fork menu (`timeline/src/lib.rs`)                                                           | `top(24)` under a 22 px trigger: 2 px                                                              | `popup_offset(22)`: 4 px                         |
| PR comment kebab (`pull_request_view.rs`)                                                   | `top(22)` under an 18 px trigger: 4 px                                                             | `popup_offset(18)`: 4 px (unchanged)             |
| Space menu (`workspace-spaces` `MENU_GAP`)                                                  | 6 px above the tab                                                                                 | 4 px                                             |
| Composer `Popover` x6 (attach, model, choice pill, features, plugin select, plugin popover) | offset 0                                                                                           | offset 4 px                                      |

Checked, no change:

- `explorer.rs` file menu and `workspace.rs` tab menu: anchored at the pointer position, no trigger.
- `chrome.rs` tab-less `render_header_actions` and `header_popover` use the same wrapper rule as above.
- gpui-kit `dropdown_menu` sites (`sidebar`, `settings`, `explorer/src/ui.rs`, `plugin-host/src/ui.rs`): `gpui-component` builds the `Popover` with the default offset 0 and offers no offset parameter. The gap is 0, below the theme gap. Not changed.
- `composer/src/tracks.rs` `PANEL_OFFSET = 12`: a documented Electron value for the track panel, larger than the theme gap. Left as is (see questions).

## Verified

- `./cargo.sh test -p bachuc-theme -p bachuc-plugin-workspace-spaces -p bachuc-app -p bachuc-explorer -p bachuc-timeline -p bachuc-composer -p bachuc-workspace` -> exit 0, includes `popup_offset_is_the_trigger_height_plus_one_gap` and the updated `menu_bottom` assertion (496).
- `./cargo.sh build -p bachuc-app` -> finished.
- `./cargo.sh fmt --check` on each edited crate -> exit 0.
- DEFERRED (UI checker): New workspace project picker gap about 4 px.
- DEFERRED (UI checker): one gpui-kit `dropdown_menu` and one `Popover` site gap.
- DEFERRED (UI checker): each dropdown in the site list, and a dropdown near the bottom edge keeps on screen.

## Anti-goal

No dropdown opens off-screen or over its own button. Checkpoints (all DEFERRED (UI checker), read from window screenshots near the edge):

- New workspace picker near the window bottom: stays on screen (`snap_to_window_with_margin(8)` unchanged).
- Fork menu, header menus: unchanged `snap_to_window_with_margin(8)`.

## Lessons

- A gpui `anchored()` child without a position lands at the static position of an absolute element. In a block parent that position is after the in-flow siblings, so a margin on the child adds to the chip height. Wrap the `deferred(anchored(...))` in `div().absolute().top(popup_offset(trigger))`, as the other menus do.
- The trigger height can differ from the nearby slot constant (the fork button is 22 px, the footer is 24 px). Compute the offset from the trigger, not the row.

## UI Checks

Window 1710 px, 1 logical px is 2 screenshot px. Gaps read from screenshots.

- Line 41 (New workspace project picker, gap about 4 px): PASS. About 4 px between the chip and the menu (`/tmp/wt-ui/chk-242-newws-project.png`).
- Line 42 (one gpui-kit dropdown_menu and one Popover site, gap equal to the theme gap): FAIL. Popover (composer model menu): 4 px (`/tmp/wt-ui/chk-242-model.png`). gpui-kit `dropdown_menu` (sidebar project actions kebab): about 0 px, the menu top touches the row (`/tmp/wt-ui/chk-242-kebab.png`). This matches the outcome note that gpui-component gives no offset.
- Line 43 (each dropdown in the list, theme gap; one near the bottom edge flips up): FAIL. Measured 4 px: header actions menu (`/tmp/wt-ui/chk-237-header-menu.png`), fork menu (`/tmp/wt-ui/chk-242-fork.png`), New workspace pickers, composer model Popover. Open-in editor chooser (`/tmp/wt-ui/chk-237-open-in.png`): about 6.5 px, not 4 (the trigger group is 26 px tall, the menu offset assumes 28). gpui-kit `dropdown_menu` sites: about 0 px. Flip: the model menu opens above its button near the bottom and stays on screen. Not opened: Git actions menu (its trigger sits beside the Commit button, an AX click could commit), branch switcher (an AX click on the pill showed no popup), PR comment kebab, Space menu (needs right-click), split menu gap (opens, not measured).

### Run 2

Run 2026-10-18 after fix round 1. Check bundle on live (6768), window 1710 px (3420 px screenshot, 2 px per logical px). Screenshots in `/tmp/wt-ui/run2/`.

- Open in editor chooser gap (4 px under the 26 px split button): PASS. Button top y 9, bottom y 61 (52 px, 26 logical). Menu top border y 69. Gap 8 px = 4 logical (`/tmp/wt-ui/run2/chk-242-open-in.jpg`).
- Sidebar project kebab dropdown gap (about 4 px from the kebab button): PASS, with a caveat. Menu top border y 1686. The project row runs y 1620-1692 (36 logical, its bottom 6 px hidden by the menu); the 24 px kebab is centered on the row, so its bounds are y 1632-1680. Gap 6 px = 3 logical, within 1 logical px of 4 (`/tmp/wt-ui/run2/chk-242-kebab.jpg`, crop `/tmp/wt-ui/run2/kebab-crop.png`). The kebab bounds are inferred: the dots are hidden under the AX click cursor overlay, and the button has no AX bounds. Run 1's 0 px came from measuring at the row.

## Fix round 1 (group H)

Status: DONE (code checks pass; screen checks DEFERRED to the UI checker).

Changed:

- `packages/desktop-gpui/crates/app/src/chrome.rs`: `header_popover` takes `trigger_height`. Scripts passes 28; the Open in editor chooser passes `HEADER_CONTROL_HEIGHT` (26). Gap 6 px -> 4 px.
- `packages/desktop-gpui/crates/theme/src/lib.rs`: test `gpui_kit_dropdown_gap_is_the_popup_gap` (`BASE_REM_SIZE * 0.25 == POPUP_GAP`).

gpui-kit `dropdown_menu` finding (no code change needed):

- gpui-component 0.7.3 `src/popover.rs:325` sets `offset = self.offset.unwrap_or(window.rem_size() * 0.25)`. The app rem is 16 (`bachuc-theme` `BASE_REM_SIZE`), so every `dropdown_menu` already opens 4 px from its trigger, equal to `POPUP_GAP`.
- `/tmp/wt-ui/chk-242-kebab.png` (2x): the project row runs y 932-1004, its 24 px kebab sits 6 px in (y 944-992), and the menu border starts at y 998-1000. The gap from the kebab is 3-4 px. The "0 px" reading was from the 36 px row, which the menu overlaps by 2-3 px, and the pointer hid the kebab.
- The new theme test fails if `BASE_REM_SIZE` or `POPUP_GAP` changes apart. At zoom other than 100 %, the rem scales, so this gap scales with it.
- No vendor change is needed. `DropdownMenuPopover` has no offset setter, so a different gap would need a change in gpui-component `src/menu/dropdown_menu.rs:116` (add `.offset(...)` after `.anchor(self.anchor)`).

Verified:

- `CARGO_BUILD_JOBS=4 ./cargo.sh test -p bachuc-theme -p bachuc-app` -> exit 0, both popup tests pass.
- `./cargo.sh build -p bachuc-app` -> finished.
- `./cargo.sh fmt -p bachuc-theme -p bachuc-app --check` -> exit 0.
- DEFERRED (UI checker): Open in editor chooser gap 4 px under the 26 px split button.
- DEFERRED (UI checker): sidebar project kebab menu 4 px under the kebab button (measure from the button, with the pointer moved off it), not from the row.

Lessons:

- Measure a menu gap from the trigger's own bounds, not its row. A small trigger in a tall row makes a correct gap look like zero.
