# ADR-0020 Scheme label colors over palette tokens for turn steps

Status: accepted
Date: 2026-10-13
Supersedes: -
Scope: `packages/app/src/tool-calls/detail-level/turn/**`, `hiep-plugins/plugins/edit-diffs/client/**`, `docs/design.md`

## Context

- [docs/design.md](docs/design.md) §14 forbids hardcoded hex and new color tokens outside the palette. The identity color table in §13 is the only documented exception.
- The turn step list colors Explore, Edit, and Verify with fixed oklch hues 155, 55, and 300 from the "Agent Response Final" design, and the edit-diffs change bar uses fixed hex `#3D9A4E` and `#C94A4A`.
- The PR #60 review found that the label text color, oklch(0.52 0.11 h), does not change with the theme and may have low contrast on the dark theme.

## Decision

- Keep the three label hues as a documented exception, like the identity colors in §13. Each color scheme gets its own text and border color for each hue, so label text keeps its contrast on the light and dark themes.
- The edit-diffs change bar uses the palette tokens `statusSuccess` and `statusDanger`, not its own hex.
- The owner chose it on 2026-10-13.

## Consequences

- [docs/design.md](docs/design.md) names a second exception next to the identity colors. It is not a license for other hues.
- A change to the label hues must check contrast in both schemes.

## Options Considered

- Map the labels to existing palette tokens: rejected. The strip would lose the Explore, Edit, and Verify colors that the owner chose in the final design.
- Keep the fixed colors with no scheme change: rejected. The dark-theme contrast risk from the review would stay.

## Revisit If

- The palette gains a categorical color family that can carry three distinct label hues.
- A contrast check shows a label color under 4.5:1 against its background in either scheme.
