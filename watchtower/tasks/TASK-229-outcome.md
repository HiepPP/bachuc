# TASK-229 outcome

## Status

DONE (code). The screen check is DEFERRED (UI checker).

## Changed

- `packages/desktop-gpui/plugins/board/src/card.rs`:
  - The new pure `card_active(hovered, focused, keyboard) -> bool` holds `active = hovered || ... || focus?.keyboard` (`page.tsx:563-565`).
  - `run_card` gets one `FocusHandle` per rendered card with `window.use_keyed_state("board-focus-<agent id>")` and tracks it on the card root. The handle is not a tab stop (GPUI default), so Tab order does not change. GPUI drops the handle when the card is not rendered, so a card that leaves the list drops its handle.
  - `run_card` sets the new `Card.focused` to `card_active(false, last_input_was_keyboard && handle.contains_focused, last_input_was_keyboard)`. The dispatch lookup runs only after keyboard input.
  - `dim()` takes `focused`: at rest it uses `part_opacity(part, seen, focused)`, and `group_hover` still gives `part_opacity(part, seen, true)`. Only opacity changes, never `display`. `CardPart` and `part_opacity` do not change.
- `packages/desktop-gpui/plugins/board/src/page.rs`: not changed (see the deviation below).

## Contract

- Produces `pub(crate) fn card_active(hovered: bool, focused: bool, keyboard: bool) -> bool` and `pub(crate) focused: bool` on `Card` in `card.rs`. Callers that build a `Card` set `focused: false`; `run_card` overwrites it.
- A seen card shows its repo, avatar and title, tags, and elbow at the unseen opacity while keyboard focus is on any of its focusable parts (open, repo, star, pencil, Remove, TASK chip). Mouse focus does not un-dim it.

## Deviation from the planner call

- The spec puts the handles in the Board page state, keyed by agent id. `Paint` and the list items are built in `runs.rs` (`render_item`), which is not in Files, so page state cannot reach `run_card` without a `runs.rs` change. The handles live in GPUI keyed element state instead, still one per rendered card and keyed by agent id, and GPUI drops them when the card is not rendered.

## Verified

- GitNexus impact (index stale; callers confirmed with `rg`): `run_card` LOW (4 impacted). `dim` ambiguous, risk UNKNOWN in the graph; it is private and all 13 callers are in `card.rs`.
- `CARGO_BUILD_JOBS=4 ./cargo.sh test -p bachuc-plugin-board` -> pass, 95 + 1 tests, with the new `card_active_on_hover_or_keyboard_focus_but_not_mouse_focus`. Log: `/tmp/task229-test.txt`.
- `./cargo.sh fmt --check -p bachuc-plugin-board` -> exit 0.
- `./cargo.sh metadata --format-version 1 > /dev/null` -> exit 0.
- Static read: `Window::focus` calls `refresh()`, which sets `refreshing` and re-renders every view, so the card opacity follows a Tab move. `use_keyed_state` needs a draw phase; `run_card` runs only inside the Runs list items, which GPUI renders during layout and prepaint.
- UI check on live: Tab onto a seen card's star -> the card un-dims; Tab away -> it dims -> DEFERRED (UI checker).
- Anti-goal: scroll CPU with 30 cards, at most 1 point over the TASK-209 reading -> DEFERRED (UI checker). Per card and frame the change adds one keyed-state lookup and one focus node; the focus lookup runs only after keyboard input.

## Lessons

- `window.use_keyed_state(key, cx, |_, cx| cx.focus_handle())` gives a stateless GPUI builder a focus handle per list item with no view state; tracked on a container, `contains_focused` tells whether any focusable child has focus.
- A new `FocusHandle` is not a tab stop by default, so tracking it on a container does not add a Tab stop. A mouse down that hits no focusable child focuses the container; `last_input_was_keyboard` keeps that from un-dimming the card.

## UI Checks

Run 2026-10-18 by the UI checker (run B). Bundle `Bachuc GPUI Check` built from HEAD `800604233`, PID 46300, launched with `open -n -g --env BACHUC_NO_ACTIVATE=1` and connected only to live `127.0.0.1:6768` (`lsof`). Frames were forced with the AX "zoom the window" action; keys went to the check app only. The seen card was the disposable mock agent "rb seen card" (`1bf369cc`) in the scratch workspace "rb check". Opening it in GPUI did not mark it seen, because GPUI never sends `clear_agent_attention` (no caller in `packages/desktop-gpui`); the checker cleared its attention with the client library `clearAgentAttention`, and the Board then showed "rb seen card, Completed, seen". After the run, the agent and the workspace were archived and PID 46300 exited.

- Tab onto a seen card's star -> the whole card shows at full opacity: FAIL. After 45 Tab presses no focus ring drew anywhere on the Board, and the card stayed dim. Darkest title pixel, seen card before -> after: 130 -> 131; an unseen card title in the same frames: 20 -> 21. Cause (static read): the card parts are `.focusable()` with no `tab_stop`/`tab_index` (`card.rs:176-182`, the star at `card.rs:497-513`). In `gpui-pre` 0.3.9 `Interactivity::tab_stop` defaults to false, and `Root`'s Tab runs `focus_next`, which skips nodes that are not tab stops (`src/tab_stop.rs:114-177`). So Tab can never reach a card part, in the background or the foreground. Evidence: `/tmp/rb-229-02-seen-before.png`, `/tmp/rb-229-03-after-45-tabs.png`.
- Tab away -> the card dims again: UNVERIFIED. Focus never reached the card.
- AX focus is not readable for this check: the AX "focused UI element" stayed the window on every press (background window).
- Anti-goal, scroll CPU with 30 cards at most 1 point over the TASK-209 reading: UNVERIFIED (unattended): needs pointer. The Board has no keyboard or AX scroll path, live has only 10 finished cards, and TASK-209 has no earlier reading. Idle reading with the Board open: 0.0% (`top -l 6 -s 1`, load 5.8).

### Run C

Run 2026-10-18 by the UI checker (run C), after fix round 1. Bundle `Bachuc GPUI Check` built from HEAD `e220a915d`, PID 13872, launched with `open -n -g --env BACHUC_NO_ACTIVATE=1` and connected only to live `127.0.0.1:6768` (`lsof`). The Board opened by AX press; keys went to PID 13872 only; frames were window screenshots, and every Tab press drew a new frame. Disposable mock agents in scratch repo `/tmp/rc-229`: "rc seen card" (`aad84208`, attention cleared with the client library `clearAgentAttention`), its subagent "rc sub row" (`c694bb99`, label `paseo.parent-agent-id`), and "rc other card" (`0b563feb`, unseen). The darkest title pixel is the minimum luminance in the title box (`/tmp/rc-229-scripts/dark2.py`). After the run, PID 13872 exited, and the 3 agents and 3 scratch workspaces were archived. Hiep's test app was not running at any time.

- Tab moves a focus ring through the card parts in order; Shift+Tab goes back: FAIL. The order is correct: "rc other card" open target, repo, pencil, star, Remove (no TASK chip: no linked task), then "rc seen card" open, repo, pencil, star, Remove all, the "1 subagent" toggle row, and "rc sub row" open, star, Remove. Six Shift+Tab presses walked the same stops back. The indicator is wrong on every part with a non-inset ring: the top-level open target fills the whole card with solid accent green, and the repo chip, pencil, star, Remove, and Remove all show a solid green box, not a 2 px ring. On the open target the fill hides the status line and Remove. Parts with an inset ring (the cluster root's open target, the toggle row, the subagent open target) draw a correct ring. Cause (static read): `ring(_, false)` is a spread `BoxShadow` (`card.rs:155-163`), and GPUI paints the shadow quad under the element, so a transparent part shows the full quad. This is new to users, because fix round 1 made these parts reachable by Tab. Evidence: `/tmp/rc-229-02-tab1-crop.png`, `/tmp/rc-229-03-grid.png`, `/tmp/rc-229-04-zoom.png`, `/tmp/rc-229-09-grid.png`.
- Tab onto a part of a seen card -> the whole card shows at full opacity; Tab away -> it dims again: PASS. Darkest title pixel of "rc seen card": 132 at rest and with focus on the other card (Tab 1-5), 18-21 with focus on its open, repo, pencil, star, and Remove all (Tab 6-10), 132 again when focus moved to the toggle row (Tab 11) and to the subagent row (Tab 12-14). Unseen "rc other card" stayed 18-21. Shift+Tab back onto Remove all and the star: 18 again. The subagent row is its own card: 132 at rest, 21 while focus is on its open, star, or Remove. Frames 4-9 had the stale notice below, so their boxes were moved down 54 px. Evidence: `/tmp/rc-229-01-board.png`, `/tmp/rc-229-03-tab6.png`, `/tmp/rc-229-06-tab11.png`, `/tmp/rc-229-06-tab12.png`.
- A subagent row: Tab onto its star -> its actions show: FAIL. Tab 13 and 14 (star, Remove) kept focus in the row (title 21), but the star and Remove stayed invisible, with no ring, in Tab and in Shift+Tab frames. Cause (static read): the actions container shows on `in_focus` (`card.rs:964-968`), and in `gpui-pre` 0.3.9 `in_focus` tests `focus_handle.within_focused` (`elements/div.rs:3417-3421`), which is true when the container is inside the focused element (`window.rs:521-526`), not when the focused star is inside the container. Electron's `actionFocused` needs `contains_focused`. Evidence: `/tmp/rc-229-08-subactions.png` (the first frame shows the star and X; the Tab 12-14 and Shift+Tab 2 frames do not), `/tmp/rc-229-07-subrow.png`.
- Not a TASK-229 result: from Tab 4 on, the Board showed "live, Stale" and "Updates are delayed. Showing a snapshot older than 10 seconds." with Retry, which pushed the list down 54 px; it was gone by Tab 10. Evidence: `/tmp/rc-229-11-notice.png`.

### Run D

Run 2026-10-18 by the UI checker (run D), after fix round 2. Bundle `Bachuc GPUI Check` built from HEAD `f10da33e6` (includes `7d8765ed0`), PID 68984, launched with `open -n -g --env BACHUC_NO_ACTIVATE=1` and connected only to live `127.0.0.1:6768` (`lsof`). The Board opened by AX press; the AX "zoom the window" action forced the first frame and sized the window to Run C's 3420x2024 frames. Keys went to PID 68984 only; frames were window screenshots, and every Tab press drew a new frame. Disposable mock agents in scratch repo `/tmp/rd-229`: "rd seen card" (`bd56210d`, stopped, attention cleared with the client library `clearAgentAttention`), its subagent "rd sub row" (`a40c5551`, label `paseo.parent-agent-id`), and "rd other card" (`109b50db`, unseen). The seen card is first in the list this time, so its stops come first. The darkest title pixel is the minimum luminance in the title box (`/tmp/rd-229-scripts/dark2.py`); the ring position comes from accent pixels (`/tmp/rd-229-scripts/ring.py`). After the run, PID 68984 exited, and the 3 agents and 3 scratch workspaces were archived. Hiep's test app was not running at any time.

- Tab through the card parts -> each part shows a 2 px accent ring with no fill; the open target keeps its status line and Remove visible; nothing shifts: PASS. Every stop draws an outline only: the open targets of "rd seen card" (Tab 1) and "rd other card" (Tab 10) draw an inset ring inside the card, and the status line, the model chip, and "Remove all · 2" or "Remove" stay visible. The repo chip, pencil, star, Remove all, Remove, toggle row, and subagent open target show a ring around the part, not the solid green box of Run C. Star ring at Tab 4: 4 device px wide at 2x (2 px), inner pixel (251, 249, 250) against card background (250, 250, 250). The card boundary rows at x = 2047 are the same in all 14 Tab frames (`642, 836, 1030, ...`). Evidence: `/tmp/rd-229-03-grid.png` (frame 0 at rest, then Tab 1-14; compare `/tmp/rc-229-03-grid.png`), `/tmp/rd-229-05-tab1-crop.png`, `/tmp/rd-229-05-star-zoom.png`.
- Tab onto a part of a seen card -> the card un-dims; Tab away -> it dims again: PASS. Darkest title pixel of "rd seen card": 132 at rest, 19 with focus on its open, repo, pencil, star, and Remove all (Tab 1-5), 131 with focus on the toggle row (Tab 6), the subagent row (Tab 7-9), and the other card (Tab 10-14). Shift+Tab back onto Remove all, star, pencil, repo, and open: 20, 20, 20, 20, 19. Unseen "rd other card" stayed 17-20. The subagent row: 131-132 at rest, 20 while focus is on its open, star, or Remove. Evidence: `/tmp/rd-229-02-tab1.png` to `/tmp/rd-229-02-tab14.png`, `/tmp/rd-229-06-stab1.png` to `/tmp/rd-229-06-stab13.png`.
- Tab onto a subagent row's star and Remove -> the actions show: PASS. Tab 8 (star): the star and X are visible, and the star has the ring. Tab 9 (Remove): both stay visible, and the X has the ring. The Shift+Tab frames show the same (Shift+Tab 5 and 6). At rest and at Tab 7 (row open target) the actions stay hidden. Evidence: `/tmp/rd-229-05-tab8-sub.png`, `/tmp/rd-229-05-tab9-sub.png`, `/tmp/rd-229-07-shift-grid.png`.
- Shift+Tab walks the same stops back: PASS. From "rd other card" Remove (Tab 14), 13 Shift+Tab presses went star, pencil, repo, open of the other card, then the subagent Remove, star, and open, the "1 subagent" toggle row, Remove all, star, pencil, repo, and the open target of "rd seen card": the exact reverse of Tab 1-13. Evidence: `/tmp/rd-229-07-shift-grid.png`.
- Not a TASK-229 result: the stale notice ("live, Stale", "Updates are delayed...", Retry) showed in the first Board frame and again in Shift+Tab 4-12, and moved the list down 62-66 px. The pixel boxes were moved with it. Evidence: `/tmp/rd-229-04-top.png`.

## Fix round 1

Status: DONE (code). The screen checks are DEFERRED (UI checker).

### Cause

The card parts were `.focusable()` but no tab stops. In `gpui-pre` 0.3.9 `Interactivity::tab_stop` defaults to false, and `focus_next`/`focus_prev` skip nodes that are not tab stops (`src/tab_stop.rs:114-177`). So Tab never reached a card part.

### Changed

- `packages/desktop-gpui/plugins/board/src/card.rs` only:
  - `.tab_stop(true)` on every part Electron reaches by Tab: the open target, repo, pencil (diffs), star, linked TASK chip, Remove, and the cluster toggle row. `button()` does not change, because `runs.rs` and `page.rs` also call it.
  - The subagent actions (star, Remove) hide by `opacity(0.)` instead of `invisible()`. GPUI skips the tab-stop insert of a `Visibility::Hidden` element (`elements/div.rs:2536-2538`), so a hidden star could never take focus. They show on `group_hover`, and on `in_focus` of the actions container (now `.id(...).focusable()`, no tab stop), which matches Electron's `actionsVisible = ... || actionFocused` (`page.tsx:386-394`, opacity 0 at `page.tsx:683-693`). Only opacity changes, never `display`.
- The focus handles stay where the first build put them: one keyed handle per rendered card on the card root, no tab stop.

### Tab order

- No order rule is computed, so there is no new pure function. Every part keeps the default `tab_index` 0, and `TabStopMap` sorts by tab path, then by insertion (paint) order (`src/tab_stop.rs:52-58`). So Tab follows the element tree order.
- The GPUI tree order equals Electron's DOM order. Top-level card: open, repo, pencil, star, TASK chip, Remove (`page.tsx:591-860`; `card.rs` `run_card`, `card_body`). Subagent row: open, star, Remove, TASK chip. Cluster: root card, then the toggle row, then the child rows.
- A per-card `tab_group` would break the order: every card's group would share index 0, so the parts of different cards would interleave by part index.

### Many cards

- Electron also puts every card part in the Tab order, so a Board with many cards has the same number of stops per rendered card (at most 7). The Runs list is virtual (`runs.rs:48`, `ListState` with overdraw), so only rendered cards register tab stops. After the last rendered card, Tab leaves the list or wraps; it does not scroll the list. Electron scrolls the focused card into view. A scroll-to-focus needs a `runs.rs` change; it is out of scope.
- If a focused card leaves the rendered range, GPUI drops its handle, and the next Tab starts from the first tab stop (`TabStopMap::next` with an unknown id).

### Verified

- GitNexus impact (index stale; callers confirmed with `rg`): `run_card`, `star_button`, `diffs_button`, `remove_button`, `card_body`, `subagent_body`, `toggle_row` LOW. `task_chip` ambiguous (a second `task_chip` in `timeline.rs`), UNKNOWN in the graph; the card one is private, with 2 callers in `card.rs`.
- `CARGO_BUILD_JOBS=4 ./cargo.sh test -p bachuc-plugin-board` -> pass, 95 + 1 tests. Log: `/tmp/task229-fix1-test.txt`.
- `./cargo.sh fmt --check -p bachuc-plugin-board` -> exit 0.
- `./cargo.sh metadata --format-version 1 > /dev/null` -> exit 0.
- UI check on live: Tab moves through the card parts with the ring; Tab onto a seen card's star -> the card un-dims; Tab away -> it dims -> DEFERRED (UI checker).
- UI check on live: Tab onto a subagent row's star -> its actions show -> DEFERRED (UI checker).

### Lessons

- In `gpui-pre`, `.focusable()` alone is not reachable by Tab. Add `.tab_stop(true)`, or `.tab_index(n)`, which also sets it.
- A `Visibility::Hidden` element registers no tab stop. Hide a focusable control with opacity, and show it with `in_focus` on a focusable container.

## Fix round 2

Status: DONE (code). The screen checks are DEFERRED (UI checker).

### Cause

- Focus indicator: `ring(_, false)` is a spread `BoxShadow`. GPUI paints the whole shadow quad under the element, so a transparent part showed a solid accent box, and the open target filled the card.
- Subagent actions: `in_focus` tests `within_focused`, which is true when the container is inside the focused element. The focused star is inside the container, so the actions stayed at opacity 0.

### Changed

- `packages/desktop-gpui/plugins/board/src/card.rs` only:
  - New `Parts`: one keyed focus handle per focusable card part (open, repo, pencil, star, TASK chip, Remove), created with `use_keyed_state` under `board-part-<agent id>` and tracked on the part with `track_focus`. The render knows which part has focus (`Parts.focused`) and whether the last input was the keyboard.
  - New `part()` replaces `button()` for the card parts: no shadow ring. `button()` and `ring()` stay for the toggle row (inset, draws correctly), `runs.rs`, and `page.rs`.
  - New `focus_ring()`: an absolutely positioned 2 px border with no fill, added as a child only while the part has keyboard focus. It is absolute, so the layout never shifts. The open target's ring is the card root's last child, inset, so the card clip keeps it and it draws over the attention border.
  - The star and pencil keep `focus_visible` for their opacity only.
  - The subagent actions container lost its `id` and `focusable`; it shows at opacity 1 when `subagent_actions_shown(pinned, focused)` is true. Only opacity changes, never `display`.
- New pure `ring_box(radius, inset) -> (outset, radius)`: Electron's 2 px outline at offset 1 (outer edge 3 px out), or inset at offset -2 where a parent clips (`focus.ts`). New pure `subagent_actions_shown(pinned, focused)`: Electron's `actionFocused` (`page.tsx:386-394`), any focus on the star or Remove, a click too.

### Deviation

- The top-level open target draws its ring inside the card, not 2 px outside as Electron does. The card root clips its overflow, and the Runs list items have no side padding, so an outer ring would be cut off.

### Verified

- GitNexus impact (index 91 commits stale; callers confirmed with `rg`): `run_card`, `star_button`, `subagent_body` LOW. `diffs_button`, `remove_button`, `card_body`, `task_chip` have only `card.rs` callers (`task_chip` in `timeline.rs` is a different method).
- `CARGO_BUILD_JOBS=4 ./cargo.sh clippy -p bachuc-plugin-board --all-targets` -> no board warnings. Log: `/tmp/task229-fix2-clippy.txt`.
- `CARGO_BUILD_JOBS=4 ./cargo.sh test -p bachuc-plugin-board` -> pass, 97 + 1 tests, with the new `the_focus_ring_sits_1_px_outside_the_part_or_inside_a_clipping_surface` and `focus_on_the_star_or_remove_shows_a_subagent_rows_actions`. Log: `/tmp/task229-fix2-test.txt`.
- `./cargo.sh fmt --check -p bachuc-plugin-board` -> exit 0.
- `./cargo.sh metadata --format-version 1 > /dev/null` -> exit 0.
- UI check on live: Tab through a card's parts -> each shows a 2 px ring with no fill; the open target keeps the status line and Remove visible; nothing shifts -> DEFERRED (UI checker).
- UI check on live: Tab onto a subagent row's star and Remove -> the actions show with the inset ring -> DEFERRED (UI checker).
- UI check on live: the Tab order and the keyboard un-dim still pass as in Run C -> DEFERRED (UI checker).
- Anti-goal, scroll CPU: per card and frame the change adds 6 keyed-state lookups, 6 `is_focused` checks, and one key string -> DEFERRED (UI checker).

### Lessons

- In `gpui-pre`, a spread `BoxShadow` is a filled quad under the element, not a ring. Draw a ring on a transparent element as an absolute child with a border and no fill.
- With `track_focus`, the element's `.tab_stop(true)` is ignored; set it on the handle, `cx.focus_handle().tab_stop(true)` (`div.rs:2251-2269`).
- `in_focus` means "inside the focused element". For "holds the focused element", check a handle at render.

### Open

- `button()` in `runs.rs` (Retry) and `page.rs` (size control) and the view tabs (`page.rs:188`) still use the spread-shadow ring. They are not tab stops, so Tab does not reach them now; they are outside this TASK's files.
