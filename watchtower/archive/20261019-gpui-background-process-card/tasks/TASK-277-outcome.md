# TASK-277 outcome

Status: BLOCKED

## Changed

- `packages/desktop-gpui/crates/timeline/src/lib.rs`: scroll-to-bottom button, press handler, deferred notify in the scroll handler, two pure helpers, two unit tests.

## Contract

- No new public API. Private items: `shows_scroll_to_bottom(rows: usize, following_tail: bool) -> bool`, `reaches_tail(visible_end: usize, count: usize, following_tail: bool) -> bool`, `TimelineView::render_scroll_to_bottom`, `TimelineView::scroll_to_bottom`.
- Show rule: `render` derives the state from `self.list.is_following_tail()` and `self.rows.len()`; no new field.
- The button is an absolute child of the root `TimelineView::render` div, after the list body and before the outline rail, selection toolbar, find bar, and lightbox. A full-width, 48 px tall wrapper has no id and no handler, so it takes no pointer events; only the circle is a hit target.
- Look: 48 px circle (`rounded_full`), `surface2` fill, `shadow(ShadowSize::Sm)`, lucide `ChevronDown` 24 px in `foreground`, centered, 16 px above the bottom; every size through `zoomed`. No hover change, pointer cursor, no fade.
- Press: `list.set_follow_mode(FollowMode::Tail)`, then `report_current_position`, then `cx.notify()`.
- AX: `Role::Button`, label "Scroll to bottom", explicit `AccessibleAction::Click` that runs the same press.
- Scroll handler: the existing closure now also calls `window.on_next_frame` with one `cx.notify()` when `reaches_tail` holds. `on_next_frame` runs after the current frame is rendered (gpui-pre `window.rs:2710`), so it runs after the list's layout pass re-engaged the tail (`list.rs:1211-1217`) and the next render hides the button.

## Verified

- Group-level run (shared with TASK-276, one command): `CARGO_BUILD_JOBS=4 ./cargo.sh test -p bachuc-app -p bachuc-sidebar -p bachuc-timeline -p bachuc-composer` -> exit 0. `bachuc-timeline` lib 32 passed, including the two new tests `the_scroll_to_bottom_button_shows_only_when_not_following_a_filled_list` (rows 0 hidden; following hidden; not following with rows shown) and `a_scroll_to_the_last_row_asks_for_one_more_render_until_the_tail_follows`.
- `./cargo.sh build` (workspace) -> Finished. One pre-existing warning (`next_theme` never used, `crates/app/src/state.rs:458`).
- `./cargo.sh fmt --check -p bachuc-timeline` -> exit 0.
- Read check: the scroll handler adds no `TimelineView` notify of its own except the one `on_next_frame` notify, and only when the visible range ends at the last row and the list does not follow yet (`reaches_tail`). The press calls `set_follow_mode(FollowMode::Tail)`. The button has `Role::Button`, the label "Scroll to bottom", and an `AccessibleAction::Click` handler that runs the same press.
- `rg -n "\.cached\(" packages/desktop-gpui/crates packages/desktop-gpui/plugins` -> no match (exit 1).
- GitNexus: the index is stale (`TimelineView` listed at line 242, now 344). Known risk LOW, `render` and `build` are private to the crate; my edits add private items and one child in `render`. UNKNOWN for the ambiguous `render` name; confirmed by `rg` that the added code touches no caller outside `lib.rs`.

DEFERRED (UI checker): scrolled up in a long thread, the button shows centered 16 px above the bottom, 48 px at zoom 1; AX press lands on the newest row and hides the button; a short thread shows no button; scroll up then back to the last row hides the button without a press; at zoom 1.25 the button is 60 px and 20 px above the bottom, and switching threads while it shows starts the new thread at the bottom with no button; during a running turn the button stays while rows stream in and a press follows the stream again.

## Anti-goal

- Read check PASS (above): the handler adds one deferred notify at the last row and no other notify.
- DEFERRED (UI checker): one 60 s idle read at the bottom of a thread within 0.3 points of 2.4%. At the bottom the list follows, so `reaches_tail` is false and the handler queues no notify; the button element is not built.

## Lessons

- `ListScrollEvent` carries `is_following_tail`, `count`, and `visible_range`. Use them in the handler instead of reading the `ListState`, which is borrowed during the callback.
- A notify queued in the scroll handler must wait for the next frame (`Window::on_next_frame`), not `cx.defer`: the layout pass that re-engages the tail runs after the render of the same frame.

## UI Checks

Not run. The owner stopped the UI checker on 2026-10-20 before it reached the TASK-277 lines, and the plan was archived.

Blocked: BUILT, UNVERIFIED (Q-178).
