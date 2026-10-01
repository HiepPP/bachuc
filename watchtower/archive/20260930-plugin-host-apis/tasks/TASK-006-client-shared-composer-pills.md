# TASK-006 Composer pills for every composer

Group: B (client plugin host files)
Class: code

## Brief

Goal: Let one composer pill show on every agent composer, with the agent passed to its callbacks.

Change: each pill needs one `agentId` and `workspaceId` -> both are optional, and `onPress` gets the button context.

Boundaries:

- A pill without `agentId` matches every agent composer. A pill without `workspaceId` matches every workspace.
- `onPress(context)` and menu item `onPress(context)` get `PluginButtonContext`. Old `onPress()` handlers keep working.
- Pills still render only in the real agent panel.

How:

- Make the fields optional in [packages/plugin/src/client/buttons.ts](packages/plugin/src/client/buttons.ts).
- Update matching in [packages/app/src/plugins/buttons/model.ts](packages/app/src/plugins/buttons/model.ts).
- Pass the context to handlers in [packages/app/src/plugins/buttons/view.tsx](packages/app/src/plugins/buttons/view.tsx).
- Add tests in [packages/app/src/plugins/buttons/model.test.ts](packages/app/src/plugins/buttons/model.test.ts).

Files:

- [packages/plugin/src/client/buttons.ts](packages/plugin/src/client/buttons.ts) (types)
- [packages/app/src/plugins/buttons/model.ts](packages/app/src/plugins/buttons/model.ts) (matching)
- [packages/app/src/plugins/buttons/view.tsx](packages/app/src/plugins/buttons/view.tsx) (context)
- [packages/app/src/plugins/buttons/model.test.ts](packages/app/src/plugins/buttons/model.test.ts) (tests)

Expected result:

- A shared pill shows on two different agents' composers.
- Its handler gets the right `agentId` for each composer.

Anti-goal: Existing pill and header button matching; tripwire; limit every existing test in [packages/app/src/plugins/buttons/model.test.ts](packages/app/src/plugins/buttons/model.test.ts) passes unchanged; read before changes and at completion.

## Verify

- `npx vitest run packages/app/src/plugins/buttons --bail=1` -> all pass, including shared pill cases.
- `npm run typecheck` -> exit 0.
- Anti-goal: the pre-change button tests pass without edits.
