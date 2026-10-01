# TASK-008 Plugin navigation to other surfaces and new workspaces

Group: B (client plugin host files)
Class: code

## Brief

Goal: Let a plugin open another plugin's surface and open the new workspace screen for a project.

Change: plugins click DOM buttons or push routes -> `client.openSurface(id, options)` and `client.openNewWorkspace(input)`.

Boundaries:

- `client.openSurface(surfaceId, { pluginId?, serverId? })`. It defaults to the calling plugin and the active host.
- `client.openNewWorkspace({ serverId?, cwd, projectId?, name? })` routes to `buildNewWorkspaceRoute`.
- An unknown plugin or surface throws a clear error.

How:

- Extend the actions in [packages/app/src/plugins/actions.ts](packages/app/src/plugins/actions.ts) and [packages/app/src/plugins/navigation.ts](packages/app/src/plugins/navigation.ts).
- Update the types in [packages/plugin/src/client/contracts.ts](packages/plugin/src/client/contracts.ts).
- Add tests next to the actions.

Files:

- [packages/plugin/src/client/contracts.ts](packages/plugin/src/client/contracts.ts) (types)
- [packages/app/src/plugins/actions.ts](packages/app/src/plugins/actions.ts) (actions)
- [packages/app/src/plugins/navigation.ts](packages/app/src/plugins/navigation.ts) (routes)
- A test next to [packages/app/src/plugins/actions.ts](packages/app/src/plugins/actions.ts)

Expected result:

- `openSurface("board", { pluginId: "board" })` from another plugin opens the Board route on the active host.
- `openNewWorkspace` opens `/new` with the project.

Anti-goal: Plugin host navigation contract; tripwire; limit [packages/app/src/plugins/host-navigation.test.ts](packages/app/src/plugins/host-navigation.test.ts) passes unchanged; read before changes and at completion.

## Verify

- `npx vitest run <new actions test> packages/app/src/plugins/host-navigation.test.ts --bail=1` -> pass.
- `npm run typecheck` -> exit 0.
- Anti-goal: host-navigation tests pass without edits.
