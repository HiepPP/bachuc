# TASK-062 Surface params in openSurface

Group: H (`packages/plugin/src/client/contracts.ts` and the app's plugin navigation)
Class: code

## Brief

Goal: A plugin can open another plugin's surface with a few string params, and the surface reads them as a prop.

Change: `openSurface(id, { pluginId, serverId, presentation })` -> the same plus `params?: Record<string, string>`, delivered as `props.params`.

Boundaries:

- Q-023 default: add the field. The client plugin API changes only; there is no wire protocol change.
- Keep the host change thin: carry the record through, and add no workspace logic in the host.
- Older surfaces ignore the prop. Tag nothing as COMPAT, because the field is optional and new.

How:

- Add `params` to the `openSurface` options and to `PluginSurfaceProps`.
- Carry it through the app's surface navigation to the mounted component. Find the path with `rg -n "openSurface" packages/app/src/plugins`.
- Add a test next to the existing navigation tests.
- Update [docs/plugins.md](docs/plugins.md) where it documents `openSurface`.

Files:

- [packages/plugin/src/client/contracts.ts](packages/plugin/src/client/contracts.ts) (options and props)
- [packages/app/src/plugins/navigation.ts](packages/app/src/plugins/navigation.ts) (carry params)
- [packages/app/src/plugins/actions.ts](packages/app/src/plugins/actions.ts) (carry params, if it builds the call)
- [docs/plugins.md](docs/plugins.md) (one line)

Expected result:

- `openSurface("watchtower", { pluginId: "watchtower-board", params: { workspaceId: "w1" } })` mounts the surface with `props.params.workspaceId === "w1"`.
- A call without `params` behaves as today.

Anti-goal: existing plugin navigation tests keep passing, 0 new failures; tripwire; read from `npx vitest run packages/app/src/plugins/navigation*.test.ts --bail=1` before and after.

## Verify

- `npx vitest run <the navigation test file> --bail=1` -> the new params test and the old tests pass.
- `npm run typecheck` -> exit 0.
- `npm run lint -- packages/plugin/src/client/contracts.ts packages/app/src/plugins/navigation.ts` -> no errors.
