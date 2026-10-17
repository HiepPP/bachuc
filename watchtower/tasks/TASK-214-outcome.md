# TASK-214 Outcome

## Status

DONE

## Changed

- `packages/desktop-gpui/crates/composer/src/mention.rs` (new): `find_active_file_mention`, `quoted_path`, `apply_mention`, `mention_path`, `insert_mentions`, and the `MentionMenu` state (debounced request decision, latest-answer guard, Escape dismissal, status text), with 17 tests.
- `packages/desktop-gpui/crates/composer/src/composer.rs`: the `@` menu (detection on text change, debounced `directory_suggestions`, keys, Enter and Tab pick, rows with a folder or file glyph), `Composer::insert_path_mention`, and the `ExternalPaths` drop handler on the composer root. The slash menu frame moved into two helpers, `popup_anchor` and `popup_box`, that the `@` menu uses too. The look is the same.
- `packages/desktop-gpui/crates/composer/src/lib.rs`: the `mod mention;` line.

## Contract

- Produces `Composer::insert_path_mention(&mut self, path: &str, window: &mut Window, cx: &mut Context<Composer>)`. It inserts the quoted path at the caret, with a space before it when the text before is not whitespace, and a space after it. A path inside the composer folder becomes relative; any other path stays as given.
- Consumes `Client::directory_suggestions(cwd, query, true, true, 50, cx)` and `DirectorySuggestion { path, is_directory }` from TASK-206, unchanged.
- Folder: a draft uses its `cwd`; an agent composer uses the agent's `cwd`. With no folder, or an empty one, the composer sends no request and shows no menu.
- A request goes out 180 ms after the last text change. A newer keystroke drops the waiting `Task`, so a burst sends one request. An answer for an older request is dropped by generation.
- A file mention at the caret wins over the slash menu, as `resolveAutocompleteMode` picks "file" first. Slash commands at the start of the text work as before.
- A drop is ignored while interceptors run or a draft submits. Images (`mime_for_path`) load in the background through `load_images` and attach. Every other file or folder becomes a quoted path mention at the caret.
- Planner call on the drag backdrop: not added. `drag_over::<ExternalPaths>` only changes the style of the element it sits on. The Electron backdrop is an overlay with an icon and "Drop files here", which needs a drag state (`on_drag_move`, cleared on drop or exit).

## Verified

- `./cargo.sh test -p bachuc-composer mention` -> 17 passed. This includes Electron's detection, quoting, and replacement cases, and `a_mention_without_a_folder_sends_no_request_and_shows_no_menu`.
- `./cargo.sh test -p bachuc-composer` -> 110 passed.
- `./cargo.sh fmt --check -p bachuc-composer` -> exit 0.
- `./cargo.sh clippy -p bachuc-composer --tests` -> exit 0. It gave no warnings in `mention.rs` or `composer.rs`.
- `./cargo.sh metadata --format-version 1` -> exit 0. `./cargo.sh build -p bachuc-app` -> exit 0.
- UI check (popup lists files, Enter inserts the quoted path) -> DEFERRED (UI checker).
- Owner hand test (drag a file and an image from Finder) -> DEFERRED (owner).
- Anti-goal (10 fast characters after `@`, 3 or fewer `directory_suggestions_request` in the live `daemon.log`) -> DEFERRED (UI checker). By code, one request goes out per 180 ms pause.

## Lessons

- In the vendored gpui-base, `InputBaseState::set_value` emits no `InputEvent::Change`. Code that sets the text must update state that depends on the text itself. `set_text`, `clear_input`, and `set_text_at` call `request_mentions` for this reason.
- `InputBaseState::cursor()` and `set_selected_range` use UTF-8 byte offsets. `Position::character` counts chars, not UTF-16 units.
- TASK-219: the new menu state is self-contained in `mention.rs`. In `composer.rs` it adds three fields (`mention`, `mention_scroll`, `mention_fetch`) and a block of methods after `pick`. `draft.rs` was not changed.

## UI Checks

Run 2026-10-18 by the UI checker (run 3). Bundle `Bachuc GPUI Check` built from HEAD `2d5734c11` (includes `e9a78e843`, `77a9730cb`, and `22906f2cb`), PID 31420, launched with `open -n -g --env BACHUC_NO_ACTIVATE=1` and connected only to live `127.0.0.1:6768` (`lsof`). The app stayed in the background. The checker forced each frame with the AX "zoom the window" action and read the AX tree after a no-op F15 key. Scratch repo `/tmp/t202-repo` (branches `main` and `feature-x`, script `hello`) was the live workspace `wks_4a3662191ab77c60` with disposable mock agents "t202 scratch", "t200 archive me", "t200 board card", and "t208 cpu stream". The workspace was archived after the run; PID 31420 quit and exited.

- Popup lists files: PASS. Typing `check @a.t` in the agent composer opened AX `menu Files` with "a.txt" and a file glyph above the composer (`/tmp/r3-28-mention-popup.png`).
- Enter inserts the quoted path: PASS. After Enter the composer value is `check "a.txt"` (AX value) and the menu closed (`/tmp/r3-29-mention-inserted.png`). The message was not sent.
- File drop: UNVERIFIED (unattended): needs pointer.
- Anti-goal, 10 fast characters after `@`: PASS. `@srcmainrsx` typed in 92 ms (18:30:16.837Z to .929Z) gave 1 `directory_suggestions_request` in the metric window 18:30:08Z to 18:30:38Z.
- The draft was cleared after the checks.
