# TASK-209 outcome

## Status

DONE (code). The screen checks are DEFERRED (UI checker).

## Changed

- `packages/desktop-gpui/plugins/board/src/card.rs`:
  - The new `CardPart` enum and the pure `part_opacity(part: CardPart, seen: bool, active: bool) -> f32` hold the Electron dim rule (`page.tsx:565`). The repo, the avatar and title (`Main`), and the tags dim to 0.5, and the elbow goes to `0.6 * dim`. The star, the pencil, and Remove stay at 1.
  - `dim()` is now generic over `Styled + InteractiveElement` and takes a `CardPart`. A card hover (`group_hover`) is the active state. Star, pencil, and Remove call it too, so one function owns the rule. For them it is a no-op.
  - The card title (`line_clamp(2)`) now has `text_ellipsis()`, which matches `numberOfLines={2}`. Before, GPUI clipped the second line with no ellipsis.
  - `act()` is not changed. Star, pencil, Remove, repo, and TASK chip still call `stop_propagation`.

## Contract

- Produces `pub(crate) enum CardPart { Repo, Main, Tags, Elbow, Star, Diffs, Remove }` and `pub(crate) fn part_opacity(part: CardPart, seen: bool, active: bool) -> f32` in `card.rs`.

## Verified

- `CARGO_BUILD_JOBS=4 ./cargo.sh test -p bachuc-plugin-board` -> pass (93 + 1 tests, including the 2 new dim rule tests). Log: `/tmp/task209-test.txt`.
- `./cargo.sh fmt --check -p bachuc-plugin-board` -> exit 0.
- `./cargo.sh metadata --format-version 1 > /dev/null` -> exit 0.
- Static read, pencil: the 3 suspects in the spec are ruled out by source. The `button()` helper adds no size. `pencil.svg` has `viewBox="0 0 24 24"` (checked by the `bachuc-icons` test). The 28 px button with `my(-6)` reaches 6 px above a 16 px row, inside the 16 px top padding, so the root `overflow_hidden` does not clip it. No ancestor in `runs.rs` clips it. The cause of the cut in the screenshot is not found in code.
- Static read, dim: the old code already dimmed the same parts as Electron, and GPUI scopes `with_element_opacity` per element. The code shows no path that dims Remove or the TASK chip. The chip borders inside the model tags dim in Electron too.
- Static read, Vietnamese text: GPUI truncates on `char_indices` (`line_wrapper.rs` `should_truncate_line`, `truncate_wrapped_line`), so a cut ends on a whole code point. The meta highlight range is `0..status.len()` of a prefix, so it is always on a char boundary.
- UI check: pencil shows whole on hover -> DEFERRED (UI checker).
- UI check: seen card next to the Electron seen card -> DEFERRED (UI checker).
- UI check: long Vietnamese title in a narrow Board -> DEFERRED (UI checker).
- Pixel match of padding, chips, and Remove (1 px) -> DEFERRED (UI checker).
- Anti-goal: scroll CPU with 30 cards -> DEFERRED (UI checker). The change adds no per-frame work: it uses the same number of opacity and `group_hover` styles, and the actions call `dim()` as a no-op.

## Lessons

- Electron restores the dim on keyboard focus too (`focus?.keyboard`). GPUI has no group focus style, so the GPUI card restores the dim on hover only. A focus restore needs card focus state in the view.
- A text with `line_clamp(n)` and no `text_ellipsis()` clips with no ellipsis in GPUI, because truncation runs only when `text_overflow` is set.
- If the UI checker still sees a cut pencil, measure it on screen. A Vietnamese title in NFD form (with combining marks) can still split a mark from its base letter, because GPUI cuts on code points, not on graphemes.

## UI Checks

Run 2026-10-18 by the UI checker, bundle `Bachuc GPUI Check` (PID 80366) on live, background, frames forced by the AX zoom action. The Electron reference Board is not available: the reference daemon logs "Plugin failed to load" for `board`, because `~/.paseo-dev/config.json` points to `/Users/hiep/Projects/paseo/hiep-plugins/plugins/board`, which does not exist. The checker did not change that home.

- Pencil shows whole on hover: UNVERIFIED (unattended): needs pointer. The pencil has opacity 0 unless hovered or diffs are off (`card.rs:481-492`), and it did not show in any frame. The star, which should show at 0.35 opacity, is also not drawn inside any card (see TASK-198 UI Checks Run 2).
- Seen card dims the right parts: PASS against the Electron rule (`page.tsx:565`); Electron screen UNVERIFIED (reference Board fails to load). Darkest pixel, unseen -> seen card: title 19 -> 132, repo 88 -> 175, tag 18 -> 130, Remove 109 -> 107. So repo, avatar and title, and tags dim, and Remove does not. Evidence: `/tmp/t209-unseen-card.png`, `/tmp/t209-seen-card.png` (from `/tmp/t198r2-04-board.png`).
- Long Vietnamese title in a narrow Board: PASS. Window 1200 pt, Board size 160%: the title wraps to 2 lines and ends "... cuối cùng ở bảng…" with "bảng" whole. Evidence: `/tmp/t209-narrow-02.png`, crop `/tmp/t209-narrow-03-title-crop.png`. The meta line fit, so meta truncation was not exercised. Board size was reset to 100%.
- Padding, chips, and Remove against Electron within 1 px: UNVERIFIED. No Electron Board to compare.
