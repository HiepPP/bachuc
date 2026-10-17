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
