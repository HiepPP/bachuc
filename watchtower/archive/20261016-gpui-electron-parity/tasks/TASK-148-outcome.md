# TASK-148 Outcome

## Status

DONE. Screen checks and the CPU read are deferred to the UI checker.

## Changed

- `packages/desktop-gpui/crates/workspace/` (new crate `bachuc-workspace`):
  - `Cargo.toml`
  - `src/lib.rs`: `PaneItem`, `PaneEvent`, `SplitAxis`, `actions`, `KEY_CONTEXT`
  - `src/model.rs`: the split tree `Layout<T>`, `child_rects`, `resize_pair`
  - `src/tab_layout.rs`: the Electron tab width and drop slot math
  - `src/workspace.rs`: the `Workspace` view
  - `tests/model.rs`: 19 tests
  - `examples/workspace.rs`: three text panes
- `packages/desktop-gpui/Cargo.lock` (generated: the new package)

## Contract

- `bachuc_workspace::Workspace` (GPUI view, `EventEmitter<PaneEvent>`, `Focusable`):
  - `new(window, cx) -> Workspace`: one empty group.
  - `open(&mut self, item: PaneItem, window, cx)`: appends to the focused group and activates it. An open key is replaced in place and activated, as Electron's `insertTabIntoPane` does.
  - `close_active(&mut self, window, cx)` and `close(&mut self, key: &str, window, cx)`: emit `Closed { key }`, then `Activated` for the neighbor that became active.
  - `split(&mut self, axis: SplitAxis, window, cx)`: adds an empty group right of or below the focused group, focuses it, and emits `PaneEvent::NewTab`.
  - `activate_index(&mut self, index: usize, window, cx)`: 0-based, in the focused group. Electron's digit shortcut is 1-based, so pass `digit - 1`.
  - `active_key(&self) -> Option<&str>`: the active key of the focused group.
- Extras that TASK-155 can use:
  - `update(&mut self, item, cx) -> bool`: replaces an open item (title, icon, modified, view) without activating it.
  - `layout(&self) -> &model::Layout<PaneItem>`: read the groups and tabs.
  - `set_new_tab_menu(builder, cx)`: makes the new-tab button open a gpui-kit dropdown menu built by `builder`.
  - `actions::{SplitRight, SplitDown, CloseActiveTab, NewTab}`: handled on the workspace root under the key context `KEY_CONTEXT` (`"Workspace"`). The crate binds no keys; the example binds `cmd-\`, `cmd-shift-\`, `cmd-w`, and `cmd-t`.
- `bachuc_workspace::PaneItem { key: String, title: SharedString, icon: IconName, modified: bool, view: AnyView }` (`Clone`).
- `bachuc_workspace::SplitAxis { Right, Down }`.
- `bachuc_workspace::PaneEvent { Closed { key: String }, Activated { key: String }, NewTab }`.

## Verified

- `packages/desktop-gpui/cargo.sh test -p bachuc-workspace` -> PASS: 19 tests in `tests/model.rs`, including `closing_the_last_tab_of_a_split_group_removes_the_group_and_its_divider`, `closing_the_active_tab_activates_the_next_then_the_previous`, `resize_clamps_each_side_to_the_min_split_size`, `move_across_groups_activates_and_focuses_the_target`, and `many_tabs_shrink_to_the_min_width_then_scroll`.
- `cargo.sh clippy -p bachuc-workspace --all-targets` -> PASS: no warnings from this crate.
- `cargo.sh fmt --check -p bachuc-workspace` -> PASS.
- `bundle.sh example workspace BachucWorkspace io.github.hieppp.bachuc-workspace` -> PASS: the release build is at `packages/desktop-gpui/target/bundles/BachucWorkspace.app`. It was not opened.
- Agent check: tab bar against the reference app on 6770 with `cua_repl` screenshots -> DEFERRED (UI checker).
- Agent check: split right, split down, drag a divider, drag a tab to the other group -> DEFERRED (UI checker).
- Agent check: `getAXState` roles and labels of tabs, close buttons, and dividers -> DEFERRED (UI checker).
- Impact analysis: the TASK adds a new crate and edits no existing symbol, so no GitNexus `impact` run applied.

## Anti-goal

- Tripwire: the example stays at or below 1.0% CPU idle with three groups (`measure.sh 60 ws="pids:<pid>"`).
- Checkpoint `after-splits`: DEFERRED (UI checker). Measured: none.
- Checkpoint `completion`: DEFERRED (UI checker). Measured: none.
- Idle work by design: nothing animates or polls. The root canvas re-renders once when the workspace size changes (deferred to the next frame), and hover changes re-render once each.
- How to get three groups in the example: press `cmd-\` twice, or use "More actions" > Split right twice. Each split opens one text pane.

## Parity list

Built from `workspace-desktop-tabs-row.tsx`, `resize-handle.tsx`, and `trailing-action-scrim.tsx`. Screen comparison is DEFERRED (UI checker); the column below states the source match only.

| Element                   | Layout and spacing                                     | Color and icon                                                      | State                                                                               | Source match                                     |
| ------------------------- | ------------------------------------------------------ | ------------------------------------------------------------------- | ----------------------------------------------------------------------------------- | ------------------------------------------------ |
| Tab bar                   | height 36, border bottom 1, row padding 4              | `surface0`, border `border`                                         | -                                                                                   | match                                            |
| Tab chip                  | height 28, padding 8, gap 4, radius 6, width 96 to 160 | label 14 px                                                         | -                                                                                   | match                                            |
| Tab active, focused group | -                                                      | fill `surface2`, label and icon `foreground`                        | `aria_selected`                                                                     | match                                            |
| Tab active, other group   | -                                                      | fill `surface1`, label `foreground_muted`                           | -                                                                                   | match                                            |
| Tab hover                 | -                                                      | fill `surface1`, label `foreground`                                 | -                                                                                   | match                                            |
| Tab icon                  | 14 px                                                  | `foreground` when highlighted, else `foreground_muted`              | -                                                                                   | match                                            |
| Modified dot              | 8 px, after the label                                  | `foreground_muted`                                                  | -                                                                                   | match                                            |
| Close button              | 18 px at right 4 in a 48 px overlay                    | X 12 px, `foreground_muted`, `foreground` on hover; scrim to 55%    | shown on hover                                                                      | match                                            |
| New-tab button inline     | 28 px square in a 4 px padded slot                     | Plus 14 px, `foreground_extra_muted`, hover `interaction_highlight` | moves to the toolbar at 20 px when tabs scroll                                      | match                                            |
| More actions button       | 20 px, padding 2, margin right 3                       | Ellipsis 14 px, `foreground_extra_muted`                            | menu with Split right (Columns2) and Split down (Rows2), 220 px wide, right aligned | match                                            |
| Many tabs                 | shrink toward 96 by spare width, then 96 and scroll    | -                                                                   | -                                                                                   | match                                            |
| Drop indicator            | 4 px pill in the gap, inset 2                          | `accent`                                                            | while a tab drags over a bar                                                        | match                                            |
| Divider                   | 1 px line, 10 px pointer band                          | `border`; 3 px `accent` line on hover or drag                       | -                                                                                   | match                                            |
| Tab tooltip               | gpui-kit tooltip                                       | gpui-kit colors                                                     | -                                                                                   | mismatch: no agent id or activity line           |
| Tab context menu          | -                                                      | -                                                                   | -                                                                                   | mismatch: not built (rename, close others, copy) |
| Maximize pane button      | -                                                      | -                                                                   | -                                                                                   | mismatch: not built (out of the spec)            |

## Lessons

- An inactive tab's view stays alive (the workspace holds its `AnyView`) but is not rendered, so GPUI drops its element state, such as an untracked `overflow_y_scroll` offset. Keep scroll positions in the view, in a `ScrollHandle` or `ListState` field. Rendering hidden views would cost CPU on every frame of a streaming timeline.
- `PaneEvent::NewTab` comes from the new-tab button, the `NewTab` action, and every split. The group that should get the tab has the focus, so a plain `Workspace::open` puts it there. Electron's split also opens a new-tab tab.
- The app binds keys to `bachuc_workspace::actions::*` with the context `bachuc_workspace::KEY_CONTEXT`. The actions reach the workspace only while focus is inside it; the example focuses `Workspace::focus_handle` at start.
- `Workspace::activate_index` is 0-based.
- Closing the last tab of the only group leaves one empty group, as Electron keeps the last pane.
- `#[derive(IntoElement)]` works through gpui-kit: its macros rewrite `gpui::` paths to the facade.
- A gpui-kit `DropdownMenu` trigger must be `Styled + Selectable + InteractiveElement`; `ChromeButton` in `src/workspace.rs` wraps a `Stateful<Div>` for that, so a lean icon button can open a menu without the ten-element `Button`.

## UI Checks (earlier partial run, superseded by the section at the end)

Run by the UI checker on commit b449e19f0, 2026-10-16, light theme. Screenshots are in `/tmp/gpui-ui-check/D/` (JPEG data with a `.png` name): `ref-7.png`, `gpui-1.png`, `gpui-2-hover.png`, `gpui-4-splitright.png`, `gpui-6-three.png`, `gpui-6d.png`, `gpui-7b.png`, `gpui-10b.png`, `gpui-11-tabdrag.png`.

### Agent check: tab bar against the reference (6770)

Sampled from screenshots at the same scale. The example window is 1200x832 and the reference window is 1200x800; the reference has a 36 px toolbar above its tab bar and the example has a 32 px title bar, so only the bar and chips are compared. The reference showed a workspace with one agent tab (`/caveman ultra`); the example showed three text tabs.

| Element                         | GPUI example                                   | Reference                              | Result                                    |
| ------------------------------- | ---------------------------------------------- | -------------------------------------- | ----------------------------------------- |
| Bar height with border          | 35 rows (y 33 to 67), border `#e4e4e6`         | 36 rows (y 36 to 71), border `#e4e4e6` | match (1 px)                              |
| Bar background                  | `#ffffff`                                      | `#ffffff`                              | match                                     |
| Active chip fill, height        | `#f3f3f5`/`#f4f4f6`, 28 rows                   | `#f4f4f6`, 28 rows                     | match                                     |
| Chip left offset                | chip starts 6 px from the bar edge             | chip starts 6 px from the pane edge    | match                                     |
| Icon offset in chip             | 9 px from the chip left edge                   | 9 px                                   | match                                     |
| Label offset in chip            | 26 px from the chip left edge                  | 26 px                                  | match                                     |
| Active label, icon              | `#161618`, `#18181a`                           | `#1a1a1c`, `#161618`                   | match                                     |
| Close X on hover                | 13 px from the chip right edge, `#7c7b80`      | 13 px, `#706f74`                       | match                                     |
| New-tab plus                    | center 20 px right of the last chip, `#aeadb3` | 20 px, `#b6b6b8`                       | match                                     |
| More actions ellipsis           | right edge, `#a2a2a4`                          | right edge, `#9f9f9f`                  | match                                     |
| Inactive tab color              | label `#6c6b71`, icon `#6f6e74`                | not captured (one tab only)            | NOT COMPARED                              |
| Chip width                      | 111 px for `Text pane 1` with X                | 133 px for `/caveman ultra` with X     | NOT COMPARED (different labels)           |
| Tooltip, context menu, maximize | not built                                      | agent id and date tooltip              | mismatch, as the source list above states |

Notes:

- The reference window repainted only after the app came to the front with `open`; before that the screenshots were one step old.
- Dark theme not run.

### Agent check: split right, split down, drag a divider, drag a tab

- Split right (`cmd-\`): a second group right of the first with `Text pane 4`, a vertical divider. PASS.
- Split down (`cmd-shift-\`): the focused right group split top and bottom (`Text pane 4` over `Text pane 5`), a horizontal divider. PASS.
- Drag the vertical divider from x 1279 to 900 (2560 px window): the divider moved to x 900 (35%). PASS.
- Drag the horizontal divider from y 689 to y 500 at x 1100: the divider moved to y 500. PASS. Two earlier drags at x 1500 and 1800 left the divider at y 689 (cause unknown; the same drag at x 1100 worked, and a hover at x 1500 showed the 3 px accent line).
- Drag `Text pane 1` from the left bar to the top-right bar: the tab joined the top-right group after `Text pane 4`, became active, and the left group activated `Text pane 2`. PASS.
- Shot after each action needed a window resize (`zoom` toggle) to repaint; the example's window did not present a new frame while another app was in front.

### Agent check: `getAXState`

- Each group is a `tab group` named `Tabs`. Each tab is a `tab` (settable, boolean) named `Text pane N` with its `button Close Text pane N`; the modified tab has an `image Modified`. Each group has `button New tab` and `button More actions`. The dividers are `splitter Resize panes left and right` and `splitter Resize panes up and down`. PASS.

### Anti-goal: idle CPU with three groups

- `measure.sh 60 ws="pids:<pid>"` with three groups: 0.5, 0.4, 0.4 CPU% (RSS 72, 83, 56 MB). Limit 1.0. PASS with a limit.
- Limit: another app took the front within seconds of every `open` (the checker's machine was in use), so the window was not in front for most of each read. Reads that re-opened the app while it lost the front gave 1.6 and 1.3 CPU%; those include the activation redraws (each re-open cost about 0.1 CPU%) and are not an idle read. No read had the window in front for the full 60 s.

### Cleanup

The example app was quit. The reference app left one terminal in `/tmp/gpui-bench` on 6770 (`Terminal 1`), created by the checker for the J check; it is killed with that check.

## UI Checks

Run by the UI checker on HEAD 5afebcedd plus the working tree, bundle `BachucWorkspace` rebuilt 2026-10-16, light theme (dark not run). Screenshots: `/tmp/gpui-ui-check/D/` (`ref-tabbar.jpg`, `gpui-tabbar.jpg`, `gpui-splitright.jpg`, `gpui-splitdown.jpg`, `gpui-dragdiv-v.jpg`, `gpui-dh-1100-432.jpg`, `gpui-tabdrag.jpg`, `gpui-after-measure.jpg`). Reference: the in-place Electron build on 6770, workspace gpui-bench with one empty "New tab" tab. Both windows are 1200 points wide in the screenshot.

### Tab bar parity -> PASS

| Element                         | GPUI example                                    | Reference                              | Result                                 |
| ------------------------------- | ----------------------------------------------- | -------------------------------------- | -------------------------------------- |
| Bar height incl. bottom border  | 35 rows (y 33 to 67, border `#e4e4e6`)          | 36 rows (y 36 to 71, border `#e4e4e6`) | match (1 px)                           |
| Chip height, fill               | 28 rows, `#f4f4f6`                              | 28 rows, `#f4f4f6`                     | match                                  |
| Chip top offset from bar top    | 2 px                                            | 4 px                                   | match within 2 px                      |
| Chip left offset, icon, label   | icon 9 px, label 26 px from chip left           | icon 9 to 10 px, label 26 px           | match                                  |
| Active label color              | `#161618` family                                | `#1a1a1c` family                       | match                                  |
| Close X on hover                | shows at the chip right on hover, `#7c7b80`     | shows at the chip right on hover       | match                                  |
| New-tab plus, ellipsis          | plus after the last chip, ellipsis at the right | same places                            | match                                  |
| Bar background                  | `#ffffff`                                       | `#ffffff`                              | match                                  |
| Tooltip, context menu, maximize | not built                                       | present                                | known gap, listed in Parity list above |

The reference was not compared for an inactive tab or a different label width (one tab only). The 1 px bar height and 2 px chip offset sit at the limit; the source spec says bar 36, padding 4.

### Split and drag actions -> PASS

- Split right (`cmd-\`): a second group right of the first with `Text pane 4` and a vertical divider.
- Split down (`cmd-shift-\`): the focused right group split into `Text pane 4` over `Text pane 5` with a horizontal divider (three groups).
- Drag the vertical divider from x 599 to 420: the divider moved to x 420 (35%) with the 3 px accent line while hovered.
- Drag the horizontal divider from y 432 to y 250: the divider moved to y 250. Limit: a drag that starts on the line itself (y 431) did nothing in five tries; it started to work 1 px below the line. The first vertical drag at x 600 and the first at x 599 also did nothing before one at 599 to 601 worked, so the grab band sits 1 to 2 px off the drawn line.
- Drag `Text pane 1` from the left bar to the top-right bar: the tab joined the right-top group after `Text pane 4` and became active; the left group activated `Text pane 2`.

### Accessibility tree -> PASS

Each group is a `tab group` named `Tabs`. Each tab is a `tab` (settable, boolean) named `Text pane N` with `Value` on or off and a `button Close Text pane N`; the modified tab has an `image Modified`. Each group has `button New tab` and `button More actions`. Dividers are `splitter Resize panes left and right` and `splitter Resize panes up and down`.

### Idle CPU with three groups -> PASS

`measure.sh 60 ws="pids:58653"` with three groups and the window kept in front by an `open` loop every 3 s: 0.6 CPU% (limit 1.0), 96 MB RSS, 42 MB footprint. A screenshot after the read shows the full window, no cut (Q-069).

### Cleanup

The example and the reference app were quit; the lock was released; 6767 untouched; no terminal or agent left by this check.
