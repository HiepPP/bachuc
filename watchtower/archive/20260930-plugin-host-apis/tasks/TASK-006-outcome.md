# TASK-006 Outcome

## Outcome

Status: DONE

Changed:

- `PluginComposerPillContribution` has optional `workspaceId` and `agentId`, and action `onPress(context)` gets `PluginButtonContext` in [packages/plugin/src/client/buttons.ts](packages/plugin/src/client/buttons.ts).
- [packages/app/src/plugins/buttons/model.ts](packages/app/src/plugins/buttons/model.ts) stores an omitted field as a wildcard, matches it, passes the context to `run`, and keeps the open state per agent through `openContextKey`. `resolveButtonForContext` gives a shared pill the rendering composer's context.
- [packages/app/src/plugins/buttons/view.tsx](packages/app/src/plugins/buttons/view.tsx) resolves shared pills per composer and passes that context to presses and open state.

Contract:

- Old `onPress()` handlers keep working; the extra argument is ignored.
- Header buttons still need a workspace. Pills still render only in the agent panel.

Verified:

- `cd packages/app && npx vitest run --project unit src/plugins/buttons --bail=1` -> 6 passed (4 old, 2 new).
- `cd packages/app && npx vitest run --project unit src/plugins --bail=1` -> all passed.
- App typecheck -> exit 0. Lint on 4 files -> 0 errors.

Anti-goal:

- Final: the 4 old button tests pass without edits; 2026-09-30T23:57.
- Result: PASS.
