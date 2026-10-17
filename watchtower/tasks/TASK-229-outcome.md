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
