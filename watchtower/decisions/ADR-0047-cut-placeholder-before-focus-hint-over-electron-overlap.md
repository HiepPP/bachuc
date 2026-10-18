# ADR-0047 Cut the placeholder before the focus hint over Electron's overlap

Status: accepted
Date: 2026-10-19
Supersedes: -
Scope: `packages/desktop-gpui/crates/composer/src/composer.rs`

## Context

- In Electron, the composer placeholder and the "⌘L to focus" hint are two layers over the input, so the glyphs overlap in a narrow composer (`focusHintText` in [packages/app/src/composer/input/input.tsx](packages/app/src/composer/input/input.tsx)).
- The GPUI port copied that shape, and the overlap showed at zoom 150% and with the right panel open.
- The owner chose to cut the placeholder before the hint (Q-163, 2026-10-19).

## Decision

- Draw the placeholder and the hint in one row over the input. The placeholder shrinks and ends in "…"; the hint keeps its width.
- When the hint is hidden, the placeholder takes the full width.

## Consequences

- The GPUI composer differs from Electron here by owner choice.
- A wide composer looks the same as before.

## Options Considered

- Keep Electron's two overlapping layers: rejected. The text overlaps in a narrow composer.

## Revisit If

- Electron changes `focusHintText` so the two no longer overlap, and the owner wants exact parity again.
