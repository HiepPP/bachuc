# TASK-083 Outcome

## Outcome

Status: DONE

Run: `implement` in session, 2026-10-13.

Changed:

- `packages/app/src/tool-calls/detail-level/turn/model.ts`: `splitSteps` starts a step at each new note; tool calls join the current step. Only Markdown blocks of the same note join a step that has no work yet. A non-edit call still splits off an edit-only step with no note. `buildTurnGroup` starts a step at its note time; a step with no note starts where the step before it ended (the turn start for the first step).
- `packages/app/src/tool-calls/detail-level/turn/view.tsx` (required dependency fix, not in the spec file list): the step body draws the note above its edit rows, and the running step's note streams only until the step's first tool call. Without this, the note sat under its edits and showed as streaming while a tool call ran.
- `packages/app/src/tool-calls/detail-level/turn/model.test.ts`: 2 new cases (the live sample turn; a turn that starts with a tool call). 8 cases changed only in which step holds an item, labels, step counts, and times.
- `packages/app/src/tool-calls/detail-level/turn/view.test.tsx`: 1 new case (note above edits, streaming only before the first call). 4 cases changed only in step count, dimmed count, and step times.

Contract:

- ADR-0021: a note opens its step. ADR-0018 answer rule and `grouping.ts` unchanged.

Impact (GitNexus upstream): `buildTurnGroup` MEDIUM, 6 direct callers, all in the detail-level module. No HIGH or CRITICAL.

Anti-goal (item count: notes + edit rows + counted calls equal `run.items` minus thoughts):

- Before: view test "draws every note, edit row, and counted tool call of the model once" -> pass, count 10. 2026-10-12T19:03:36Z.
- After the split change: same test -> pass, count 10. 2026-10-12T19:05:23Z.
- At completion: same test -> pass, count 10; the count assertion lines have 0 changed lines. 2026-10-12T19:06:55Z.
- Result: PASS.

Verified:

- RED first: the 2 new model cases failed before the split change.
- `npx vitest run src/tool-calls/detail-level/turn/model.test.ts --bail=1` -> 16 passed.
- `npx vitest run src/tool-calls/detail-level/turn/view.test.tsx --bail=1` -> 14 passed.
- `npx vitest run src/tool-calls/detail-level/projection.test.ts --bail=1` -> 35 passed; 0 changed lines in that file.
- `npm run typecheck` -> exit 0.
- `npm run lint -- packages/app/src/tool-calls/detail-level/turn` -> exit 0, 0 warnings.
- Manual on live (notes above their work) -> PASS, 2026-10-13: at Summary in the turn-step-check thread, each note sits above the work it announced, for example Step 4 above the first edit.
