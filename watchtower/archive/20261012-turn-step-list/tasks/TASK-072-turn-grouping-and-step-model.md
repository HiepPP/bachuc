# TASK-072 Turn grouping and step model

Group: A (writes the grouping, projection, presentation, and plugin timeline projection files; standalone, no file shared with other TASKs)
Class: risky

## Brief

Goal: In the `overview` level on a non-compact screen, group a whole turn into one run and build a step model for it. The model feeds the step list view in TASK-073.

Change: a run ends at every assistant note -> one run per turn, with notes, tool calls, and edit plugin items inside, split into labeled steps.

Design: [watchtower/CONTEXT.md](watchtower/CONTEXT.md) (design link and step rules)

Boundaries:

- Follows [ADR-0016](watchtower/decisions/ADR-0016-host-turn-step-list-over-plugin-group-renderer.md): the host groups the turn; no plugin API for group rendering.
- Compact form factor and the `detailed` level must produce exactly the same output as today.
- Do not check a plugin ID in the host. Mark plugin items that replaced a tool call with a generic app-local field, for example `sourceKind: "tool_call"` on `PluginTimelineStreamItem`. Do not change `packages/protocol`.
- Thought items join the run and are hidden from the steps (Q-026 default).
- The last assistant message of a finished turn stays outside the run as the answer. While the turn runs, it stays inside as a step note.
- The run ends at a user message, a todo list, a notification, a compaction item, or a plugin item without the tool call marker.

How:

- Add the marker in [packages/app/src/plugins/timeline/projection.ts](packages/app/src/plugins/timeline/projection.ts) where a transform replaces a `tool_call` item, and the optional field in [packages/app/src/types/stream.ts](packages/app/src/types/stream.ts).
- Add a turn mode to [packages/app/src/tool-calls/detail-level/grouping.ts](packages/app/src/tool-calls/detail-level/grouping.ts): absorb assistant notes, thoughts, and marked plugin items; keep live and history paths in step.
- Add [packages/app/src/tool-calls/detail-level/turn/model.ts](packages/app/src/tool-calls/detail-level/turn/model.ts): build steps (note, tool counts, edit items, label, end time, duration), the turn summary (step count, changed files), and the running step.
- Pass a non-compact flag through [packages/app/src/agent-stream/presentation.ts](packages/app/src/agent-stream/presentation.ts) into [packages/app/src/tool-calls/detail-level/projection.ts](packages/app/src/tool-calls/detail-level/projection.ts).
- Write tests first in [packages/app/src/tool-calls/detail-level/turn/model.test.ts](packages/app/src/tool-calls/detail-level/turn/model.test.ts) and extend [packages/app/src/tool-calls/detail-level/projection.test.ts](packages/app/src/tool-calls/detail-level/projection.test.ts).

Files:

- [packages/app/src/tool-calls/detail-level/grouping.ts](packages/app/src/tool-calls/detail-level/grouping.ts) (turn run mode)
- [packages/app/src/tool-calls/detail-level/projection.ts](packages/app/src/tool-calls/detail-level/projection.ts) (choose turn mode when non-compact overview)
- [packages/app/src/tool-calls/detail-level/projection.test.ts](packages/app/src/tool-calls/detail-level/projection.test.ts) (turn mode cases)
- [packages/app/src/tool-calls/detail-level/turn/model.ts](packages/app/src/tool-calls/detail-level/turn/model.ts) (new step model)
- [packages/app/src/tool-calls/detail-level/turn/model.test.ts](packages/app/src/tool-calls/detail-level/turn/model.test.ts) (new tests)
- [packages/app/src/agent-stream/presentation.ts](packages/app/src/agent-stream/presentation.ts) (non-compact flag)
- [packages/app/src/plugins/timeline/projection.ts](packages/app/src/plugins/timeline/projection.ts) (tool call marker)
- [packages/app/src/types/stream.ts](packages/app/src/types/stream.ts) (optional marker field)

Expected result:

- A turn of tools, 3 notes, 2 edits, more tools, and a final message gives one run with 5 steps and the final message outside it.
- Labels: steps before the first edit are Explore, the edit step is Edit, steps after the last edit are Verify. A turn with no edit is all Explore.
- Each step has an end time and a duration from item timestamps. The running step has no end time.
- The summary lists the step count and the changed file paths.
- Cases: compact overview -> today's runs; `detailed` -> no groups; a user message or an unmarked plugin item ends the run; a turn that is still running keeps its last note inside.

Anti-goal: existing assertions in [packages/app/src/tool-calls/detail-level/projection.test.ts](packages/app/src/tool-calls/detail-level/projection.test.ts) and [packages/app/src/agent-stream/presentation.test.ts](packages/app/src/agent-stream/presentation.test.ts) stay unchanged and pass, 0 edited; tripwire; read with `git diff` and the test run before changes, after the grouping change, and at completion.

## Verify

- `cd packages/app && npx vitest run src/tool-calls/detail-level/turn/model.test.ts --bail=1` -> all cases in Expected result pass.
- `cd packages/app && npx vitest run src/tool-calls/detail-level/projection.test.ts --bail=1` -> pass, including new turn mode cases.
- `cd packages/app && npx vitest run src/agent-stream/presentation.test.ts --bail=1` -> pass.
- `cd packages/app && npx vitest run src/plugins/timeline/projection.test.ts --bail=1` -> pass, with a case that a transformed tool call item carries the marker.
- `npm run typecheck` -> exit 0.
- `npm run lint -- packages/app/src/tool-calls packages/app/src/agent-stream/presentation.ts packages/app/src/plugins/timeline/projection.ts packages/app/src/types/stream.ts` -> exit 0.
- Anti-goal: `git diff -U0 -- packages/app/src/tool-calls/detail-level/projection.test.ts packages/app/src/agent-stream/presentation.test.ts` -> only added lines, no removed or changed assertion.
