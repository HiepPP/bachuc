# TASK-230 Composer drop overlay

Group: M (standalone: [packages/desktop-gpui/crates/composer/src/composer.rs](packages/desktop-gpui/crates/composer/src/composer.rs))
Class: risky

## Brief

Goal: Dragging files over the composer shows Electron's "Drop files here" overlay.

Change: no feedback while dragging -> an overlay that fades in on drag over and out on drop or leave. Q-128.

Boundaries:

- Electron: [packages/app/src/components/file-drop/file-drop-backdrop.tsx](packages/app/src/components/file-drop/file-drop-backdrop.tsx) lines 17-43 (text from `composer.attachments.dropFilesHere`), drag state in `use-drop-listeners.ts` lines 123-135 and 198-240, 150 ms fade.
- GPUI: the drop handler is `.on_drop::<ExternalPaths>(...)` at `composer.rs` lines 2919-2921, calling `drop_paths` (line 1243).
- Drag state: `on_drag_move` fires on every window mouse move in the capture phase (gpui-pre `div.rs` lines 360, 1048), so bounds-check `event.bounds`. GPUI has no per-element leave event; `FileDropEvent::Exited` clears the active drag and refreshes (`window.rs` line 5695), so clear the state in render when `cx.has_active_drag()` is false. Planner call.
- The overlay must not take the drop: the existing `on_drop` still handles it. Never toggle `display` from hover; fade by opacity.
- No overlay while the composer submits, as `drop_paths` already ignores a drop then.

Files:

- [packages/desktop-gpui/crates/composer/src/composer.rs](packages/desktop-gpui/crates/composer/src/composer.rs)

Expected result:

- Drag a file over the composer -> "Drop files here" fades in; drop -> it fades out and the file attaches.
- Drag over, then away without dropping -> the overlay goes away.

Anti-goal: typing stays as cheap as before; tripwire; no extra render per keystroke (the drag state changes only on drag events; read the code before and after).

## Verify

- `cd packages/desktop-gpui && CARGO_BUILD_JOBS=4 ./cargo.sh test -p bachuc-composer` -> passes, with a pure drag-state test (inside, outside, drop).
- Owner hand test: drag a file over the composer and away, then drag and drop.
