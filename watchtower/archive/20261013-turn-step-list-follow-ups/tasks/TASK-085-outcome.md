# TASK-085 Outcome

## Outcome

Status: DONE

Run: `implement` in session, 2026-10-13.

Live fix, 2026-10-13 (before commit): the first build crashed live at start with "Cannot read properties of undefined (reading 'explore')". On web, Unistyles passes `StyleSheet.create` a theme of CSS variables, so `theme.colorScheme` was `var(--color-scheme)`. The vitest stub passes a plain theme, so the tests passed. Fix per ADR-0022:

- `packages/app/src/styles/turn-label-colors.ts` (new): `LABEL_HUES`, `LABEL_TONES`, the oklch conversion, `turnLabelColors(label, colorScheme)`, and `turnLabelTheme(colorScheme)`. The hue table moved here from `turn/view.tsx`.
- `packages/app/src/styles/theme.ts`: `buildDarkTheme` and `buildLightTheme` add `turnLabels`. GitNexus impact for both: CRITICAL (319 impacted), because every theme consumer depends on them; the change only adds a field.
- `turn/view.tsx`: the label styles read `theme.turnLabels.<label>.text` and `.border`.
- `packages/app/test-stubs/react-native-unistyles.ts`: the stub theme has `turnLabels`.
- `turn/view.test.tsx`: the contrast test reads `theme.turnLabels` of every built-in theme.
- Live check (cua, live Electron, turn-step-check thread): the app loads; Summary light and dark show readable labels and numbers; settings restored to Full detail and System theme.

Changed:

- `packages/app/src/tool-calls/detail-level/turn/view.tsx`: `turnLabelColors(label, colorScheme)`. `LABEL_TONES` holds the text and border tone per scheme: light text oklch(0.52 0.11), dark text oklch(0.75 0.10), light border oklch(0.80 0.06), dark border oklch(0.45 0.07). The segment stays oklch(0.74 0.09) in both schemes, and `RUNNING_COLOR` stays `#256ABF`. The row reads the scheme through registered styles (`textExplore` and friends, `borderExplore` and friends) built from `theme.colorScheme` in `StyleSheet.create`, as in the workspace label chip; no `useUnistyles()`.
- `packages/app/src/tool-calls/detail-level/turn/view.test.tsx`: the palette test now covers both schemes; a new contrast test checks every label text against `surface0` and `surface2` of the light theme and all 6 dark themes.
- `hiep-plugins/plugins/edit-diffs/client/edit-row.tsx`: the change bar uses `theme.colors.statusSuccess` and `theme.colors.statusDanger`; `BAR_ADD` and `BAR_REMOVE` are gone.
- `docs/design.md`: §13 names the turn step label hues as the second exception, with a link to ADR-0020; §14 names both exceptions.

Contract:

- ADR-0020: hues 155, 55, 300 stay in one table, per-scheme text and border, palette status tokens for the change bar.

Impact: `turnLabelColors` is called only by the view and its test; no other caller.

Anti-goal (hues stay 155, 55, 300; no new hue or token):

- Before: `LABEL_HUES` = explore 155, edit 55, verify 300; `rg -n "#[0-9A-Fa-f]{6}" hiep-plugins/plugins/edit-diffs/client` -> `#3D9A4E`, `#C94A4A`. 2026-10-12T19:10:30Z.
- At completion: `LABEL_HUES` unchanged (155, 55, 300); the same `rg` -> no match; no new hue or theme token. 2026-10-12T19:14:09Z.
- Result: PASS.

Verified:

- RED first: the contrast test failed with explore at 3.32:1 on `#181B1A`.
- Contrast at completion: lowest light value 4.75:1 (explore on `surface2`), lowest dark value 4.80:1 (verify on the Ghostty `surface2`), by a scratch script with the same formula.
- `npx vitest run src/tool-calls/detail-level/turn/view.test.tsx --bail=1` -> 16 passed, including the contrast case.
- Plugin `npm test` (18 passed), `npm run typecheck`, `npm run lint` (0 warnings, 0 errors) -> exit 0.
- `npm run typecheck` -> exit 0.
- `npm run lint -- packages/app/src/tool-calls/detail-level/turn` -> exit 0.
- `rg -n "#3D9A4E|#C94A4A" hiep-plugins/plugins/edit-diffs/client` -> no match.
- Manual on live (labels and numbers in the light and dark themes) -> PASS after the live fix, 2026-10-13: labels and numbers read clearly in the light and dark themes.
