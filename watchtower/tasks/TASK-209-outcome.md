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
