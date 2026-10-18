# TASK-209 Board card looks like Electron

Group: B (standalone: [packages/desktop-gpui/plugins/board/src/card.rs](packages/desktop-gpui/plugins/board/src/card.rs))
Class: code

## Brief

Goal: The GPUI Board card looks like the Electron card: the pencil is not cut, the seen card dims only the right parts, and spacing, chips, and Remove match.

Change: the owner's screenshot shows a cut pencil at the top right and a fully dimmed second card -> the Electron look.

Boundaries:

- A static read found that every number in `card.rs` matches [hiep-plugins/plugins/board/client/page.tsx](hiep-plugins/plugins/board/client/page.tsx): padding 12 by 16, radius 8, avatar 36, title 14/19, chips 7 by 2 with 12.5/17 text, Remove 8 by 4 with margins -4 and -8. So the fault is at render time. Measure it on screen; do not trust the numbers alone.
- Dim rule in Electron: `dim = seen && !active ? 0.5 : 1` (`page.tsx` line 565), where active is hover or focus. Dim only the repo button (line 729), the avatar and title row (line 760), the model tags (line 854), and the subagent elbow at `0.6 * dim` (line 629). Never dim the star, pencil, or Remove. In the screenshot the whole second card is dim, including Remove and the chips' borders.
- The pencil is the diffs button (`card.rs` lines 428-484). It shows only on hover when diffs are on (`opacity(0.)` plus `group_hover`). It has `my(-6)` in a 16 px row. Suspects: the gpui-kit `button` helper adds size, the `lucide/pencil.svg` asset viewBox, or the card root `overflow_hidden` (`card.rs` line 336).
- Remove sits at the card's right edge with negative margins inside `overflow_hidden`; check it is not cut.
- The meta text uses `.truncate()` on styled text with byte-based highlight ranges (`card.rs` lines 762-768). Check that a non-ASCII title does not cut a character.

How:

- Start live and the GPUI app on 6768. Screenshot a Board with a seen card, an unseen card, and a hovered card. Screenshot Electron with the same data.
- Measure each difference and fix it in `card.rs`.
- `card.rs` lines 243-250 already have a `dim()` helper that dims a seen card and restores it on `group_hover`. Change which parts call it, and move the rule into a pure function, for example `fn part_opacity(part: CardPart, seen: bool, active: bool) -> f32`, with tests.

Files:

- [packages/desktop-gpui/plugins/board/src/card.rs](packages/desktop-gpui/plugins/board/src/card.rs) (layout, dim rule, pencil, Remove)

Expected result:

- The pencil shows whole on hover, 28 px, with no cut.
- A seen card dims the repo name, avatar, title, and tags to 0.5, but not the star, pencil, or Remove. Hover sets all to 1.
- A card with a Vietnamese title truncates on a whole character.
- Side-by-side screenshots of GPUI and Electron cards differ by no more than 1 px in padding, chip size, and Remove position.

Anti-goal: Board page frame time must not grow; drift gauge; with 30 cards, scroll CPU stays at or under the reading before the change plus 1 point, read with `top -l 2 -pid <pid>` before the change and at completion.

## Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-plugin-board` -> passes, including the dim rule tests.
- UI check on live: hover a card with diffs on -> the pencil shows whole.
- UI check on live: a seen card beside an Electron seen card -> the same parts are dim.
- UI check on live: a card whose title is a long Vietnamese sentence in a narrow Board -> the title and meta end on a whole character.
- Owner hand test: look at the Board in daily use -> cards look like Electron.
- Anti-goal: scroll CPU with 30 cards -> at most the earlier reading plus 1 point.
