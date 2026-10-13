# TASK-078 Scheme label colors

Group: A (writes `turn/view.tsx` and `edit-row.tsx` with TASK-077, and `turn/view.test.tsx` with TASK-076)
Class: code

## Brief

Goal: The step list label colors read well in the light and dark themes, and the design doc names them as an exception.

Change: one fixed text and border color per label for both themes -> one per label per color scheme; the change bar uses palette status tokens.

Design: [ADR-0020](watchtower/decisions/ADR-0020-scheme-label-colors-over-palette-tokens-for-turn-steps.md)

Boundaries:

- Follows [ADR-0020](watchtower/decisions/ADR-0020-scheme-label-colors-over-palette-tokens-for-turn-steps.md). Keep hues 155, 55, and 300 in one table.
- Follow the per-scheme pattern of `identityForeground(name, colorScheme)` in [packages/app/src/styles/identity-colors.ts](packages/app/src/styles/identity-colors.ts). Read the scheme from `theme.colorScheme` inside `StyleSheet.create`; `useUnistyles()` is forbidden ([docs/unistyles.md](docs/unistyles.md)).
- Label text and number text reach at least 4.5:1 against the step list background in each scheme. The strip segment and the running color #256ABF are not text and keep their values.
- The change bar in edit-diffs uses the plugin theme's success and danger colors, not `#3D9A4E` and `#C94A4A`.
- Do not add other hues or tokens.

How:

- Write a failing test first: for each label and each scheme, the contrast of the text color against that scheme's surface is at least 4.5.
- Change `turnLabelColors` in [packages/app/src/tool-calls/detail-level/turn/view.tsx](packages/app/src/tool-calls/detail-level/turn/view.tsx) to take the color scheme, and pick the lightness per scheme.
- Switch the change bar colors in [hiep-plugins/plugins/edit-diffs/client/edit-row.tsx](hiep-plugins/plugins/edit-diffs/client/edit-row.tsx) to the theme status colors.
- Add the exception to §13 and §14 of [docs/design.md](docs/design.md), next to the identity colors, with a link to ADR-0020.

Files:

- [packages/app/src/tool-calls/detail-level/turn/view.tsx](packages/app/src/tool-calls/detail-level/turn/view.tsx) (per-scheme label colors)
- [packages/app/src/tool-calls/detail-level/turn/view.test.tsx](packages/app/src/tool-calls/detail-level/turn/view.test.tsx) (contrast cases; update the palette test)
- [hiep-plugins/plugins/edit-diffs/client/edit-row.tsx](hiep-plugins/plugins/edit-diffs/client/edit-row.tsx) (change bar colors from the theme)
- [docs/design.md](docs/design.md) (documented exception)

Expected result:

- Each label text color clears 4.5:1 in both schemes.
- The design doc lists the turn label hues as a second exception, with the same limits as the identity colors.

Anti-goal: the three label hues stay 155, 55, and 300 and no new hue or token appears; drift gauge; read from the hue table in `turn/view.tsx` and `rg -n "#[0-9A-Fa-f]{6}" hiep-plugins/plugins/edit-diffs/client` before the change and at completion.

## Verify

- `cd packages/app && npx vitest run src/tool-calls/detail-level/turn/view.test.tsx --bail=1` -> pass, including the contrast cases.
- `cd hiep-plugins/plugins/edit-diffs && npm test && npm run typecheck && npm run lint` -> exit 0.
- `npm run typecheck` -> exit 0.
- `npm run lint -- packages/app/src/tool-calls/detail-level/turn` -> exit 0.
- Manual on live (needs a screen): the step list in the light and the dark theme -> labels and numbers read clearly.
- Anti-goal: `rg -n "#3D9A4E|#C94A4A" hiep-plugins/plugins/edit-diffs/client` -> no match, and the hue table still holds 155, 55, and 300.
