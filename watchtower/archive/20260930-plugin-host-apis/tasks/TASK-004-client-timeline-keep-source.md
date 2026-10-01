# TASK-004 Timeline transformers can keep the source item

Group: B (client plugin host files)
Class: code

## Brief

Goal: Let a timeline transformer keep the native message and add plugin items next to it.

Change: a transform result always replaces the source -> an optional `source` field keeps it, with optional replaced text.

Boundaries:

- Result: `{ items, source?: { placement: "first" | "last"; text?: string } }`.
- `text` works only for `user_message` and `assistant_message`. The kept item keeps its images, copy, and rewind actions.
- The wire protocol does not change.

How:

- Extend `PluginTimelineTransformResult` in [packages/plugin/src/contracts.ts](packages/plugin/src/contracts.ts).
- Parse `source` in [packages/app/src/plugins/timeline/model.ts](packages/app/src/plugins/timeline/model.ts).
- Emit the kept source item in [packages/app/src/plugins/timeline/projection.ts](packages/app/src/plugins/timeline/projection.ts).
- Add tests in the existing timeline model and projection tests.

Files:

- [packages/plugin/src/contracts.ts](packages/plugin/src/contracts.ts) (result type)
- [packages/app/src/plugins/timeline/model.ts](packages/app/src/plugins/timeline/model.ts) (parse)
- [packages/app/src/plugins/timeline/projection.ts](packages/app/src/plugins/timeline/projection.ts) (keep source)
- Existing tests next to those files

Expected result:

- `source: { placement: "first" }` renders the native message, then the plugin items.
- `source.text` replaces the rendered text of the native message.
- A result without `source` still replaces the item.

Anti-goal: Existing transformer behavior; tripwire; limit every existing test in [packages/app/src/plugins/timeline](packages/app/src/plugins/timeline) passes unchanged; read before changes and at completion.

## Verify

- `npx vitest run packages/app/src/plugins/timeline --bail=1` -> all pass, including the new `source` cases.
- `npx vitest run packages/app/src/agent-stream/presentation.test.ts --bail=1` -> pass.
- `npm run typecheck` -> exit 0.
- Anti-goal: the pre-change timeline tests pass without edits.
