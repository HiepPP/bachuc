# TASK-229 Board card un-dims on keyboard focus

Group: B (standalone: [packages/desktop-gpui/plugins/board/src/card.rs](packages/desktop-gpui/plugins/board/src/card.rs))
Class: code

## Brief

Goal: A seen Board card un-dims while any of its parts has keyboard focus, as in Electron.

Change: only hover un-dims a seen card -> hover or keyboard focus. Q-116.

Boundaries:

- Electron: `active = hovered || ... || focus?.keyboard` ([hiep-plugins/plugins/board/client/page.tsx](hiep-plugins/plugins/board/client/page.tsx) lines 563-565, focus from lines 343, 398-400).
- GPUI: `part_opacity(part, seen, active)` and `dim()` (`card.rs` lines 283-309) use hover only; the parts use `.focusable().focus_visible(...)` for the ring (lines 180-181, 467-471, 517, 690-691) and hold no `FocusHandle`.
- `Card` (`card.rs` line 312) is a stateless builder and the list is virtual. Planner call: keep one `FocusHandle` per visible card's parts in the Board page state, keyed by agent id, and drop handles of cards that leave the list. Electron counts keyboard focus only: use `Window::last_input_was_keyboard()` with the focus state. Never toggle `display`; change opacity only.
- Keep `part_opacity` and `CardPart`; pass the new active value into them.

Files:

- [packages/desktop-gpui/plugins/board/src/card.rs](packages/desktop-gpui/plugins/board/src/card.rs)
- [packages/desktop-gpui/plugins/board/src/page.rs](packages/desktop-gpui/plugins/board/src/page.rs) (focus handles per card)

Expected result:

- Tab onto the star of a seen card -> the whole card shows at full opacity; Tab away -> it dims again.

Anti-goal: scroll CPU with 30 cards stays as before; drift gauge; `top` CPU while scrolling, at most 1 point over the TASK-209 reading.

## Verify

- `cd packages/desktop-gpui && CARGO_BUILD_JOBS=4 ./cargo.sh test -p bachuc-plugin-board` -> passes, with a pure `card active` test over hover and keyboard focus.
- UI check on live: Tab onto a seen card's star -> the card un-dims.
