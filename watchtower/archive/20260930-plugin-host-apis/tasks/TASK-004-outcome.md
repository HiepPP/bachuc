# TASK-004 Outcome

## Outcome

Status: DONE

Changed:

- Added optional `source: { placement, text? }` to `PluginTimelineTransformResult` in [packages/plugin/src/contracts.ts](packages/plugin/src/contracts.ts).
- [packages/app/src/plugins/timeline/model.ts](packages/app/src/plugins/timeline/model.ts) validates `source` and returns it on the item array.
- [packages/app/src/plugins/timeline/projection.ts](packages/app/src/plugins/timeline/projection.ts) keeps the source item first or last, with replaced text for user and assistant messages.
- Tests added in [packages/app/src/plugins/timeline/model.test.ts](packages/app/src/plugins/timeline/model.test.ts) and [packages/app/src/plugins/timeline/projection.test.ts](packages/app/src/plugins/timeline/projection.test.ts).

Contract:

- A result without `source` still replaces the item. The wire protocol is unchanged.

Verified:

- `cd packages/app && npx vitest run src/plugins/timeline src/agent-stream/presentation.test.ts --bail=1` -> 43 passed. App tests must run from `packages/app`; from the repo root they fail with `__DEV__ is not defined`.
- `cd packages/app && npx tsc --noEmit -p tsconfig.json` -> exit 0. Lint on 5 files -> 0 errors.

Anti-goal:

- Final: all old timeline and presentation tests pass without edits; 2026-09-30T23:50.
- Result: PASS.
