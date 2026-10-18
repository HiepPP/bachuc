# TASK-230 Outcome

## Status

DONE

## Changed

- `packages/desktop-gpui/crates/composer/src/composer.rs`: the `DropOverlay` drag state, the pure `drag_over` rule, `Composer::drag_move`, `Composer::drop_backdrop`, and an `on_drag_move::<ExternalPaths>` listener on the composer root. It adds the test `the_drop_overlay_fades_in_over_the_composer_and_out_on_leave_or_drop`.

## Contract

- A Finder drag over the composer root shows `FileDropBackdrop`: `surface0` at 0.7 over the whole root, a 32 px Upload glyph, and "Drop files here" (base size, medium weight, line height 1.2), 8 px apart. Electron colors the glyph `primary`; the GPUI tokens have no `primary`, so the glyph uses `foreground`, which is the dark `primary` and near the light `#18181b`.
- The overlay fades by opacity over 150 ms, from its current opacity, as `withTiming` does. It never toggles `display`. It is in the tree only while it is on or fading.
- Drag state: `on_drag_move` fires on every window mouse move during a drag, so `drag_move` checks `event.bounds` against the pointer. A move outside the root clears it. The `on_drop` listener clears it before `drop_paths` runs. When `cx.has_active_drag()` is false in render (the drag left the window: `FileDropEvent::Exited` clears the drag and refreshes), render clears it.
- No overlay while interceptors run or a draft submits (`processing` or `locked`), as `drop_paths` ignores a drop then.
- The overlay has no listeners, so it takes no hitbox. The root `on_drop` still handles the drop; `drop_paths` did not change.
- Anti-goal: `DropOverlay::set_over` returns true only on a change, and only then does the composer call `cx.notify()`. A keystroke render adds one `Option` and one `bool` check in `drop_backdrop`, and no notify. Animation frames run only during a 150 ms fade.

## Verified

- `CARGO_BUILD_JOBS=4 ./cargo.sh test -p bachuc-composer` -> 114 passed (113 before, plus the drag-state test: inside, outside, a drop that is not accepted, fade in, fade out on leave, fade out on drop from mid-fade).
- `./cargo.sh fmt --check -p bachuc-composer` -> exit 0.
- `./cargo.sh clippy -p bachuc-composer --tests` -> exit 0, no warnings in `composer.rs`. Its other warnings are in other files and existed before.
- `./cargo.sh metadata --format-version 1 > /dev/null` -> exit 0.
- `./cargo.sh build -p bachuc-app` -> exit 0.
- Anti-goal (no extra render per keystroke) -> PASS by code read, before and after: the drag state changes only in `drag_move`, the `on_drop` listener, and the render clear when no drag is active.
- GitNexus impact on `drop_paths` -> "Target not found", risk UNKNOWN (stale index). `rg` shows one caller, the root `on_drop`; `drop_paths` did not change.
- Owner hand test (drag a file over the composer and away, then drag and drop) -> DEFERRED (owner).

## Lessons

- In taffy, an absolute child positions against its direct parent, so `.relative()` on the root only documents intent and moves no other absolute child.
- A fade that must end needs state outside `with_animation`: `with_animation` restarts on a new id and plays once on mount, so a fade-out id would flash at mount. A stored start `Instant` plus `window.request_animation_frame()` during the fade is simpler and testable.
