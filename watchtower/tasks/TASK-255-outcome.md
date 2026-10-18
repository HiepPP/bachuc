# TASK-255 Outcome

## Status

BLOCKED: BUILT, UNVERIFIED (Q-169). Code committed in `d15aabdd8`; the hover and sweep feel needs a hand check.

## Changed

- `packages/desktop-gpui/crates/timeline/src/outline.rs`: pure `ease_out`, `BarLook`, `BarEase`, `BAR_EASE`; `OutlineRail` keeps a `BarState` per prompt seq and animates the pill.
- `packages/desktop-gpui/crates/timeline/tests/outline.rs`: 5 new tests.

## Contract

- `ease_out(t)` is CSS `cubic-bezier(0, 0, 0.58, 1)` (bisection on x), exact 0 and 1 at the ends, clamped, NaN -> 0.
- `BarEase::retarget(target, elapsed)` starts from the drawn look (`look_at(elapsed)`), bumps `generation`, and returns false for an unchanged target.
- A bar animates only while a change runs: `with_animation(("outline-pill", generation), Animation::new(140 ms).with_easing(ease_out))`, finite (no `.repeat()`; confirmed in `gpui-pre-0.3.9/src/elements/animation.rs`: `done` stops `request_animation_frame`). A settled bar is a plain div with no animation element, so an idle rail draws no frames.
- Reduce motion: no magnification, and the pill draws `to` at once (no animation element).
- Rail hidden: all `BarState.ease` reset to `None`, so the rail returns settled at the target.
- Bars are keyed by prompt seq and carried over when the prompt index changes.

## Verified

- `./cargo.sh test -p bachuc-timeline --test outline` -> 20 passed (ease-out 0/1/monotonic, size and color lerp, retarget in mid-ease).
- `./cargo.sh test -p bachuc-timeline` -> all suites pass.
- `./cargo.sh build -p bachuc-app` -> builds (one existing `next_theme` dead-code warning, not from this task).
- `./cargo.sh fmt --check -p bachuc-timeline` -> exit 0.
- Impact: GitNexus has no Rust symbols (UNKNOWN); `rg` shows `OutlineRail` used only in `crates/timeline/src/lib.rs` through `new`, `set_prompts`, `set_anchor`; `tick` is private.

## Anti-goal

DEFERRED (UI checker):

- Screen check: two screenshots about 70 ms apart after a hover on `mock TASK-252 outline`.
- Screen check: a fast sweep across 10 bars, then stop -> bars settle.
- Screen check: hide the rail during an ease -> no panic; settled bars on return.
- Anti-goal: `ps -o %cpu`, 10 samples over 10 s, rail showing, no pointer movement, each under 1%.

## Lessons

- `with_animation` keyed by a generation in the element id restarts on retarget; a settled bar rendered without the animation element requests no frames.
- GPUI `AnimationElement` handles reduce motion itself, but the view still skips the wrapper to keep the settled path simple.

## UI Checks

Lead, computer use, 2026-10-19. Screenshots in `/tmp/wt-ui/`.

- Two screenshots about 70 ms apart after a change: UNVERIFIED (unattended). A `cua_repl` screenshot takes 95 to 460 ms, so the first frame after a jump (460 ms) is already settled (`255-crop-a.png`). The ease math is covered by the crate tests.
- Fast sweep across 10 bars: UNVERIFIED (unattended): needs pointer. Q-169.
- Hide the rail during an ease: PASS with a limit. A jump to prompt 7, then the right panel opened about 300 ms later: the rail hid (no "Chat outline" in the AX tree), the app did not panic, and the rail came back settled with bar 7 lit (`255-returned.png`). The hide came after the 140 ms ease, not inside it.
- Anti-goal CPU, rail showing, no pointer movement: PASS on the second run. Run 1, 10 samples: 0.4 to 0.7%, one 2.4% sample right after the jump and panel toggles. Run 2, 20 samples: 0.4 to 0.7%.

Blocked: BUILT, UNVERIFIED (Q-169)
