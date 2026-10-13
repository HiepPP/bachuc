# TASK-084 Outcome

## Outcome

Status: DONE

Run: `implement` in session, 2026-10-13.

Changed:

- `hiep-plugins/plugins/edit-diffs/client/edit-row.tsx`: the row container has `marginHorizontal: -13, marginBottom: 4` again. Only the outer margins came back, as the spec says; the old inner padding of the header (8) and of the diff box (8) did not.
- `packages/app/src/tool-calls/detail-level/turn/view.tsx`: each step edit sits in a wrapper with `testID` `turn-step-edit-<id>`. A `plugin` item gets `styles.pluginEdit` (`marginHorizontal: 13, marginBottom: -4`), with a comment that names the plugin margins it cancels. Paseo edit rows get no offset.
- `packages/app/src/tool-calls/detail-level/turn/view.test.tsx`: new case for the offset wrapper on a plugin edit row and none on a Paseo edit row.
- `hiep-plugins/plugins/edit-diffs/README.md`: one line on the margin contract with the step list.

Contract:

- No new plugin API. The host matches by item kind `plugin`, not by plugin ID (ADR-0016). Pencil and verb label stay removed.

Impact: style-only JSX changes inside `EditDiffRow` and `TurnStepRowView`; no signature changed, so no symbol-level impact applies.

Anti-goal (existing edit-diffs tests keep 0 edited assertions):

- Before: `npm test` in the plugin -> 18 passed; `git diff -U0 -- tests` -> empty. 2026-10-12T19:08:59Z.
- At completion: `npm test` -> 18 passed; `git diff -U0 -- tests` -> empty (0 bytes). 2026-10-12T19:10:11Z.
- Result: PASS.

Verified:

- Plugin `npm run format`, `npm test` (18 passed), `npm run typecheck`, `npm run lint` (0 warnings, 0 errors) -> exit 0.
- `npx vitest run src/tool-calls/detail-level/turn/view.test.tsx --bail=1` -> 15 passed, including the offset wrapper case.
- `npm run typecheck` -> exit 0.
- `npm run cli -- plugin reload edit-diffs` -> live status `running`, no error.
- Manual on live (gap back in the stream at Full detail, steps unchanged at Summary) -> PASS with a limit, 2026-10-13: at Full detail the edit rows bleed past the text column again; at Summary the rows inside steps stay compact. The thread has a note between its two edits, so two adjacent edit rows were not seen.
