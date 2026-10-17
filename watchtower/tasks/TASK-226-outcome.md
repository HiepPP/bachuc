# TASK-226 Outcome

## Status

DONE. The UI check is DEFERRED (UI checker).

## Changed

- `packages/desktop-gpui/crates/app/src/toast.rs`:
  - `toast_duration(ok)` is a pure rule: 2.2 s for a success note (Electron's `DEFAULT_DURATION_MS`) and 3.2 s for an error note (Electron's `toast.error`). `TOAST_DURATION` is gone.
  - `Toast` has a `duration` field. `ToastStack::push` returns `Option<&Toast>` instead of the id.
  - `MAX_TOASTS` is 1. Electron's host holds one toast, so a new note replaces the shown one.
  - The host sits at the top center: top `HEADER_HEIGHT + SPACE_2` (36 + 8, as Electron's app-shell `topOffset + spacing[2]`), left and right `SPACE_4`, a 92 % width boundary with a 480 px maximum width, and notes centered.
  - Tests: `a_new_toast_replaces_the_shown_one_and_leaves_by_id` (replaces the old 3-note test) and `a_success_toast_lasts_2_2_s_and_an_error_toast_3_2_s`.
- `packages/desktop-gpui/crates/app/src/shell.rs`: `show_toast` reads the pushed toast's id and duration and waits for that duration. Its doc line names the top center. The import drops `TOAST_DURATION`.

## Contract

- `Shell::show_toast(&mut self, ok: bool, message: String, cx: &mut Context<Self>)` keeps its signature.
- Produces: `toast::toast_duration(ok: bool) -> Duration`, `Toast::duration`, and `ToastStack::push(..) -> Option<&Toast>`. `rg` finds the only `push` caller in `shell.rs`.
- Anti-goal: a timer starts only in `show_toast` after a non-blank push. `render_toasts` returns `None` for an empty stack, so an empty list adds no element, timer, or repaint. A replaced note's timer still fires once, finds no id, and calls no `cx.notify()`. This is the same as before the change.

## Verified

- `CARGO_BUILD_JOBS=4 ./cargo.sh test -p bachuc-app toast` -> 2 passed, including the duration rule.
- `CARGO_BUILD_JOBS=4 ./cargo.sh test -p bachuc-app` -> 80 passed.
- `./cargo.sh fmt --check -p bachuc-app` -> exit 0.
- `CARGO_BUILD_JOBS=4 ./cargo.sh clippy -p bachuc-app --tests` -> no warning in `toast.rs` or `show_toast`.
- `./cargo.sh metadata --format-version 1 > /dev/null` -> exit 0.
- GitNexus impact: `show_toast` and `render_toasts` are not in the stale index, and `push` is ambiguous (21 symbols). `rg` shows that every toast symbol is used only in `shell.rs`.
- UI check on live: a commit in the scratch repo shows the toast at the top center -> DEFERRED (UI checker).

## Lessons

- The first test run failed in `bachuc-explorer` (`has_branch` missing) while the other builder edited that crate. A later run compiled. A shared-checkout build failure in a crate outside your Files is not yours to fix; wait and retry.
- Electron's toast pauses its timer on hover (`pauseDismiss`/`resumeDismiss`). GPUI notes do not pause; that is outside this TASK.
