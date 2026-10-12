# TASK-076 Note opens its step

Group: A (shares `turn/view.test.tsx` with TASK-078; TASK-077 and TASK-078 share `turn/view.tsx`)
Class: risky

## Brief

Goal: Each note sits above the work it announces, because agents write a note before they act.

Change: a step is a note plus the tool calls before it -> a step is a note plus the tool calls after it, up to the next note.

Design: [ADR-0021](watchtower/decisions/ADR-0021-note-opens-its-step-over-tools-before-the-note.md)

Boundaries:

- Follows [ADR-0021](watchtower/decisions/ADR-0021-note-opens-its-step-over-tools-before-the-note.md). Tool calls before the first note form a first step with no note.
- The answer rule of [ADR-0018](watchtower/decisions/ADR-0018-all-trailing-messages-over-last-message-as-answer.md) and the turn grouping in [packages/app/src/tool-calls/detail-level/grouping.ts](packages/app/src/tool-calls/detail-level/grouping.ts) do not change.
- Labels keep the position rule: before the first edit Explore, a step with an edit Edit, after the last edit Verify, a non-edit step between two Edit steps Edit.
- A step starts at its note time, or at the turn start for a first step with no note. It ends at its last item. A running step has no end time.
- Keep the split of a non-edit call after an edit-only step with no note.
- Thoughts stay hidden (Q-026 of the archived plan).

How:

- Write the failing tests first in [packages/app/src/tool-calls/detail-level/turn/model.test.ts](packages/app/src/tool-calls/detail-level/turn/model.test.ts): the live sample turn (note, tools, note, tools, note, edit, note, edit, note, shell, answer) gives steps whose notes match the work below them.
- Change `splitSteps` in [packages/app/src/tool-calls/detail-level/turn/model.ts](packages/app/src/tool-calls/detail-level/turn/model.ts) so a new note starts a step, and tool calls join the current step.
- Update the step expectations that the rule changes in `model.test.ts` and [packages/app/src/tool-calls/detail-level/turn/view.test.tsx](packages/app/src/tool-calls/detail-level/turn/view.test.tsx). Change only expectations that describe which step holds an item, its label, or its times.

Files:

- [packages/app/src/tool-calls/detail-level/turn/model.ts](packages/app/src/tool-calls/detail-level/turn/model.ts) (step split)
- [packages/app/src/tool-calls/detail-level/turn/model.test.ts](packages/app/src/tool-calls/detail-level/turn/model.test.ts) (new rule cases, updated step expectations)
- [packages/app/src/tool-calls/detail-level/turn/view.test.tsx](packages/app/src/tool-calls/detail-level/turn/view.test.tsx) (updated step times and labels only)

Expected result:

- Sample turn: "Step 4: fixing add()" sits in the step with the first edit, and "Step 6: running calc.py again" sits in the step with the second shell call.
- A turn whose first item is a tool call has a first step with no note.
- The answer stays below the list, and every note, tool call, and edit item appears once.

Anti-goal: the item count stays exact: notes + edit rows + counted calls equal `run.items` minus thoughts, 0 lost and 0 doubled; tripwire; read from the view test "draws every note, edit row, and counted tool call of the model once" before the change, after the split change, and at completion.

## Verify

- `cd packages/app && npx vitest run src/tool-calls/detail-level/turn/model.test.ts --bail=1` -> pass, including the sample turn case.
- `cd packages/app && npx vitest run src/tool-calls/detail-level/turn/view.test.tsx --bail=1` -> pass.
- `cd packages/app && npx vitest run src/tool-calls/detail-level/projection.test.ts --bail=1` -> pass with 0 changed lines in that file.
- `npm run typecheck` -> exit 0.
- `npm run lint -- packages/app/src/tool-calls/detail-level/turn` -> exit 0.
- Anti-goal: the item count test in `turn/view.test.tsx` -> passes with its count assertion unchanged.
