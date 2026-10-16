# ADR-0043 Electron tokens and lucide icons over gpui-kit defaults

Status: accepted
Date: 2026-10-16
Supersedes: -
Scope: `packages/desktop-gpui/crates/theme/**`, `packages/desktop-gpui/crates/icons/**`

## Context

- The GPUI client used gpui-kit 0.7.3 default themes, no icons, and a literal "x" for close (GPUI inventory, 2026-10-16).
- The Electron app takes every color, size, space, radius, and control height from [packages/app/src/styles/theme.ts](packages/app/src/styles/theme.ts) and [docs/design.md](docs/design.md), and its icons from `lucide-react-native` 0.546.0.
- The owner wants the GPUI client to look exactly like the Electron app.

## Decision

- Port the `theme.ts` tokens, font sizes, spacing scale, radii, and control heights into `bachuc-theme`, for two themes: `light` and the `claude` dark theme.
- Generate SVG assets from the lucide path data the app imports, and draw icons from them.
- The owner chose this in the parity planning interview on 2026-10-16.

## Consequences

- A token change in `theme.ts` needs the same change in `bachuc-theme`.
- The icon generator runs again when the app imports a new lucide icon.
- Other dark variants (default dark, zinc, midnight, ghostty, pureBlack) are not in the GPUI client.

## Options Considered

- Port the tokens and use the gpui-kit `IconName` set: rejected. The icons differ from Electron, so the look is not exact.
- Port all dark variants now: rejected for this plan. The owner wants light and Claude dark only.
- Keep gpui-kit defaults: rejected. It does not match the Electron look.

## Revisit If

- The owner wants another theme from `THEME_OPTIONS` in the GPUI client.
- The Electron app moves off lucide icons.
