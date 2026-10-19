# ADR-0048 Turn off glyph dilation with a preference over patching gpui

Status: accepted
Date: 2026-10-19
Supersedes: -
Scope: `packages/desktop-gpui/crates/app/src/main.rs`

## Context

- GPUI text looked thicker and blurrier than Electron text at the same font and size.
- gpui-pre 0.3.9 thickens glyph strokes by text luminance to copy CoreGraphics font smoothing (`glyph_dilation_for_color` in `gpui-pre-apple-0.3.9/src/text_system.rs`). Electron turns smoothing off with `-webkit-font-smoothing: antialiased` in [packages/app/public/index.html](packages/app/public/index.html).
- gpui skips the dilation when the app preference `AppleFontSmoothing` is `0` (TASK-256).

## Decision

- Write `AppleFontSmoothing = 0` for the current application at the start of `main`, on every launch, before the first glyph.

## Consequences

- GPUI text draws as thin as Electron text. No gpui source changes.
- The value lands in the app's own defaults domain, so a user `defaults` change is overwritten at the next launch.

## Options Considered

- Patch gpui-pre to skip dilation: rejected. It adds a second vendored patch for a one-line preference.
- Bundle a different font: rejected. Both apps already use SF, so the font was not the cause.

## Revisit If

- The GPUI dependency changes its text rasterizer, for example a move to gpui-ce, and stops reading `AppleFontSmoothing`.
- The owner wants CoreGraphics-style thick text again.
