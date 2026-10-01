# TASK-008 Outcome

## Outcome

Status: DONE

Changed:

- `openSurface(id, { pluginId?, serverId? })` and `openNewWorkspace({ cwd, projectId?, name?, serverId? })` in [packages/plugin/src/client/contracts.ts](packages/plugin/src/client/contracts.ts).
- [packages/app/src/plugins/actions.ts](packages/app/src/plugins/actions.ts) opens another plugin's surface through [packages/app/src/plugins/navigation.ts](packages/app/src/plugins/navigation.ts). Its own surface is still checked.
- [packages/app/src/plugins/client-runtime.ts](packages/app/src/plugins/client-runtime.ts) routes `openNewWorkspace` to `buildNewWorkspaceRoute`. [packages/app/src/plugins/evaluate.ts](packages/app/src/plugins/evaluate.ts) exposes it.
- Added [packages/app/src/plugins/actions.test.ts](packages/app/src/plugins/actions.test.ts).

Contract:

- The default host is the active host, else the calling plugin's host. Changed during TASK-016, because Board installed on two hosts must open one Board, on the active host.
- Another plugin's surface is not checked up front. The surface route shows "This plugin surface is unavailable." when it is missing.

Verified:

- `cd packages/app && npx vitest run --project unit src/plugins/actions.test.ts src/plugins/host-navigation.test.ts --bail=1` -> 12 passed. After the TASK-016 change, `actions.test.ts` -> 3 passed, including the active host case.
- `cd packages/app && npx vitest run --project unit src/plugins --bail=1` -> 142 passed.
- App typecheck -> exit 0. Lint on 5 files -> 0 errors.

Anti-goal:

- Final: host-navigation tests pass without edits; 2026-10-01T00:04.
- Result: PASS.
