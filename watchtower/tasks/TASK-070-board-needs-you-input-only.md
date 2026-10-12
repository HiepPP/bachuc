# TASK-070 Board Needs you holds thread input only

Group: B4 (`hiep-plugins/plugins/board/client/**`, `hiep-plugins/plugins/board/shared/lanes.ts`, Board tests and README; shared with TASK-069)
Class: code

## Brief

Goal: Board's Needs you lists only threads that wait for input, and the plan line counts the plan's open questions and pending manual checks.

Change: Needs you with input, questions, and checks -> Needs you with input only, plus `· N question(s) · N check(s)` on the plan line.

Boundaries:

- Follows [ADR-0013](watchtower/decisions/ADR-0013-thread-input-only-in-board-needs-you.md) and [ADR-0012](watchtower/decisions/ADR-0012-status-columns-with-task-chips-over-task-lanes.md).
- Pressing an input item opens its thread, as now.
- The counts use `plan.questions` (OPEN) and `plan.manualChecks`; counts of 0 are left out. The whole plan line still opens Watchtower.
- Keep the Needs you column shape; when it is empty it shows "Nothing waits on you."
- A plan with open questions but no thread on Board shows no line, as now.

How:

- Drop the question and check items from `needsYouItems` in `shared/lanes.ts`, and the `onOpenWatchtower` path in `client/needs-you.tsx` and `client/page.tsx`.
- Add the counts to `client/plan-line.tsx`, in the warning tone when a question blocks a TASK.
- Update `tests/lanes.test.ts` and the README "Watchtower on Board" section.

Files:

- [hiep-plugins/plugins/board/shared/lanes.ts](hiep-plugins/plugins/board/shared/lanes.ts) (input items only)
- [hiep-plugins/plugins/board/client/needs-you.tsx](hiep-plugins/plugins/board/client/needs-you.tsx) (drop the Watchtower action)
- [hiep-plugins/plugins/board/client/plan-line.tsx](hiep-plugins/plugins/board/client/plan-line.tsx) (counts)
- [hiep-plugins/plugins/board/client/page.tsx](hiep-plugins/plugins/board/client/page.tsx) (wiring)
- [hiep-plugins/plugins/board/tests/lanes.test.ts](hiep-plugins/plugins/board/tests/lanes.test.ts)
- [hiep-plugins/plugins/board/README.md](hiep-plugins/plugins/board/README.md)

Expected result:

- With Q-024 open and no thread waiting for input, Needs you reads "Nothing waits on you." and the bachuc plan line ends with "· 1 question".
- A running thread that needs input appears in Needs you; pressing it opens that thread.
- Cases: 2 questions and 3 checks -> "· 2 questions · 3 checks"; a blocking question uses the warning tone; no questions and no checks -> no count.

Anti-goal: Needs you on Board holds 0 items that are not threads; tripwire; read from `needsYouItems` in `tests/lanes.test.ts` with a plan that has questions and checks, at completion.

## Verify

- `cd hiep-plugins/plugins/board && npm run typecheck && npm run-script lint && npm test` -> all pass.
- The `tests/lanes.test.ts` case with 1 question and 1 check -> `needsYouItems` returns only the input item.
- Human check on live (Expo web on daemon 6768, light theme): Needs you shows only waiting threads; the bachuc plan line shows the question count and opens Watchtower.
