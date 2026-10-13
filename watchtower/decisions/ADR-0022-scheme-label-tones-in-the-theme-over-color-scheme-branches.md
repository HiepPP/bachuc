# ADR-0022 Scheme label tones in the theme over color scheme branches

Status: accepted
Date: 2026-10-13
Supersedes: -
Scope: `packages/app/src/styles/theme.ts`, `packages/app/src/styles/turn-label-colors.ts`, `packages/app/src/tool-calls/detail-level/turn/**`

## Context

- [ADR-0020](watchtower/decisions/ADR-0020-scheme-label-colors-over-palette-tokens-for-turn-steps.md) gives each color scheme its own text and border tone for the turn step labels.
- The first build picked the tone with `theme.colorScheme` inside `StyleSheet.create`. On web, Unistyles passes that factory a theme of CSS variables, so `theme.colorScheme` is `var(--color-scheme)`, not `light` or `dark`. Live crashed on start on 2026-10-13. The vitest Unistyles stub passes a plain theme, so the tests passed.

## Decision

- Each theme carries the tones as `theme.turnLabels`, built by `turnLabelTheme(colorScheme)` in `buildDarkTheme` and `buildLightTheme`. The view reads `theme.turnLabels.<label>.text` and `.border` in its style factory.
- The owner chose it on 2026-10-13.

## Consequences

- Built-in and plugin themes get the tones, because both go through the two builders.
- On web each tone becomes a CSS variable per theme, so a theme switch updates the labels with no React render.
- The test stub theme carries `turnLabels` too.

## Options Considered

- Branch on `theme.colorScheme` in the style factory: rejected. It reads a CSS variable string on web.
- `withUnistyles` wrappers on the label text and number: rejected. The wrapper renders again on a theme change, a caller `style` overrides the mapped style, and the vitest stub ignores mappings.
- One fixed tone for both schemes: rejected. Dark text stays under 4.5:1, against ADR-0020.

## Revisit If

- Unistyles passes plain theme values to `StyleSheet.create` on web.
- The palette gains tokens that can carry the three label hues per scheme.
