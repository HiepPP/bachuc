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

## Fix round 1

### Status

DONE (code). This status replaces the first-build status above. The screen checks are DEFERRED (UI checker).

### Cause

- Star and pencil past the card edge. The star had `mr(-6)` inside the `flex_none` actions row (pencil plus star). Taffy 0.13 computes that row's max-content width in `compute/flexbox.rs` (intrinsic main size, about lines 1022-1180). For the star, the content contribution is 28 - 6 = 22, so `content_flex_fraction` is -6 / max(1, 0 _ 28) = -6. Then the size is 28 + max(1, 0) _ 28 \* -6 = -140. The row sum is 28 - 140 < 0, so the row measured 0 wide. `justify_between` put the 0-wide row at the content's right edge, and the pencil and star drew from there to the right, under the card's `overflow_hidden`. This matches the run 1 crop: the pencil glyph is cut about 6 pt inside the clip, and the star is fully outside. The bug is in the code since the first port (`f46da3a49`), not from `72fefd064`.
- AX press on star, pencil, and Remove. With no `AccessibleAction::Click` listener, GPUI replays a mouse click at the node's centre (`gpui-pre-0.3.9/src/window.rs:7054-7075`). The star's centre was outside the card, so the click hit the next card or empty space. Remove worked because its centre was inside the card, on its own hitbox.

### Changed

- `packages/desktop-gpui/plugins/board/src/card.rs`:
  - The pure `actions_offset(scale) -> f32` (6 times the Board scale) holds the star's `marginRight: -6` from `page.tsx`. The repo row's actions row takes it as `relative().left(..)`, which moves it without a change to any flex size. The star keeps only `my(-6)`, which is on the cross axis. The new test `the_repo_actions_move_6_px_into_the_padding_and_scale_with_the_board` covers it.
  - The new `Paint::press` attaches the click (`act`, with `stop_propagation`) and an `AccessibleAction::Click` listener that runs the same `CardAction`. Star, pencil, and Remove use it. An AX action goes only to its target node (`window.rs:7037-7051`), so it never reaches the card's Open. The listener also calls `stop_propagation` to match the click.
  - Not changed: Remove keeps `mr(-8)`. It is a direct child of the badges row, whose width is definite, so taffy's definite path is used and the UI check saw Remove in place.

### Pencil box (static read)

- With the fix, the actions row ends 6 px into the 12 px right padding. The pencil (28 px, `my(-6)`) is the first item, so its box is inside the content box on the x axis. On the y axis it reaches 6 px above the 16 px repo row, inside the 16 px top padding. The card's `overflow_hidden` does not clip it. The hover rule (opacity 0, `group_hover` 1) is not changed.

### Contract

- Produces `pub(crate) fn actions_offset(scale: f32) -> f32` in `card.rs`. `CardPart` and `part_opacity` do not change.
- An AX press on star, pencil, or Remove runs the same `CardAction` as a click, and it never opens the agent.

### Verified

- GitNexus impact (index 45 commits stale, callers confirmed with `rg`): `star_button` LOW, `card_body` LOW, `Paint::act` MEDIUM (7 callers, all in `card.rs`; `act` is not changed).
- `CARGO_BUILD_JOBS=4 ./cargo.sh test -p bachuc-plugin-board` -> pass, 94 + 1 tests (rerun after the formatter). Log: `/tmp/task209-fix1-test.txt`.
- `./cargo.sh fmt --check -p bachuc-plugin-board` -> exit 0.
- `./cargo.sh metadata --format-version 1 > /dev/null` -> exit 0.
- UI check: the star shows at 0.35 opacity inside each card, at 100% and 160% Board size -> DEFERRED (UI checker).
- UI check: an AX press on star, pencil, and Remove runs the action and opens no agent tab -> DEFERRED (UI checker).
- UI check: hover a card with diffs on -> the pencil shows whole -> DEFERRED (UI checker).

### Lessons

- Never put a negative main-axis margin on a child of a GPUI flex container that is sized by its content (`flex_none` or basis auto). Taffy 0.13 then measures the container as 0 wide. Use `relative()` with `left`/`right`, or put the margin on a child of a row whose width is definite.
- A GPUI control that an AX press must reach needs an explicit `AccessibleAction::Click` listener. The fallback is a mouse click at the node's centre, so it fails when the centre is clipped or covered.

## UI Checks Run 4

Run 2026-10-18 by the UI checker (run 4). Bundle `Bachuc GPUI Check` built from HEAD `60fd3a593`, PID 88032, launched with `open -n -g --env BACHUC_NO_ACTIVATE=1` and connected only to live `127.0.0.1:6768` (`lsof`). The checker forced each frame with the AX "zoom the window" action and sent keys with cua `pressKey`/`typeText` to the check app only. Scratch repo `/tmp/t202-repo` with disposable agents "r4 board card", "r4 remove card", "r4 fork src" (mock ten-second-stream), "r4 fork ok" (Haiku 5.5, one-word reply), and the fork child "Continue." (`af3218a`). After the run, PID 88032 exited, all five agents and the five t202-repo workspaces were archived, and the repo stayed as before (`feature-x`, ` M a.txt`).

- Star inside each card at 0.35 opacity, 100% Board size: PASS (recheck after `ca0ac2d90`). The star draws inside the card's top right on every card. Darkest star pixel 191-192 on a 249-251 background; a full-opacity star (t198r2 card) is 110. With antialiasing, (251-191)/(251-110) = 0.43, which fits 0.35 on thin strokes. Evidence: `/tmp/r4-209-01-board-100.png`.
- Same at 160%: PASS. Star darkest 191 vs full 105; the card border is at x 3371 and the star at x 3295-3350 (device px), so it is inside the card. Evidence: `/tmp/r4-209-03-board-160.png`.
- AX press on the star toggles it and keeps the Board open with no tab: PASS. On "r4 remove card" the label went from "Star" to "Unstar", the star drew filled, the Board stayed open, and no agent tab opened. Evidence: `/tmp/r4-209-04-after-star.png`, crop `/tmp/r4-209-05-after-star-crop.png`.
- Remove by AX: PASS. On the same card: the card left the Board ("11 finished" -> "10 finished"), the Board stayed open, no tab opened. Evidence: `/tmp/r4-209-06-after-remove.png`. Board size was reset to 100%.
- Observation: the card "t198r2 running" (not created in this run) shows its pencil and a full-opacity star with no hover; its label is still "Star", so it is not starred. Cause not checked.
- Pencil whole on hover: UNVERIFIED (unattended): needs pointer.

## Owner Hand Test

- 2026-10-18: Hiep ran the Q-133 checks of this TASK by hand on the "Bachuc GPUI Test" bundle built from `fc70abf5a`, on live: PASS.
