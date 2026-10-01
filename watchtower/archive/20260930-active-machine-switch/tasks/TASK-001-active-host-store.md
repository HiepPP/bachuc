# TASK-001 Active host store and visible hosts hook

Group: A (new store and hook files, plus the root layout mount)
Class: code

## Brief

Goal: Add one persisted active machine selection and a hook that returns only the hosts the UI should show.

Change: no app-wide machine selection -> `activeServerId` store, `useVisibleHosts()`, and a route sync hook.

Boundaries:

- `activeServerId: null` means "All machines" and is the default (Q2).
- An `activeServerId` that is not in the host list counts as `null`. Reconcile it to `null` when the host registry has loaded and the host is gone.
- Route sync (Q4): when the route has a host that differs from a non-null `activeServerId`, set `activeServerId` to the route host.
- Remember the last workspace per host in the same store, keyed by `serverId`, so the switch can reopen it.
- Follow the persist pattern of [packages/app/src/stores/sidebar-view-store.ts](packages/app/src/stores/sidebar-view-store.ts): zustand `persist`, `AsyncStorage`, validated storage.

How:

- Create the store with `activeServerId`, `lastWorkspaceIdByServerId`, `setActiveServerId`, `rememberWorkspace`, and `reconcile(serverIds)`.
- Create `useVisibleHosts()`: return `useHosts()` when there is no valid active host, else only the active host. Keep the array reference stable with `useMemo`.
- Create `useActiveHostRouteSync()`: read the route host with `parseServerIdFromPathname`, apply Q4, remember the active workspace, and reconcile on host list change.
- Mount `useActiveHostRouteSync()` once in [packages/app/src/app/\_layout.tsx](packages/app/src/app/_layout.tsx).
- Add unit tests for the pure store logic.

Files:

- [packages/app/src/stores/active-host-store.ts](packages/app/src/stores/active-host-store.ts) (new store)
- [packages/app/src/stores/active-host-store.test.ts](packages/app/src/stores/active-host-store.test.ts) (new tests)
- [packages/app/src/hosts/use-visible-hosts.ts](packages/app/src/hosts/use-visible-hosts.ts) (new hooks `useVisibleHosts`, `useActiveServerId`, `useActiveHostRouteSync`)
- [packages/app/src/app/\_layout.tsx](packages/app/src/app/_layout.tsx) (mount the route sync hook)

Expected result:

- With `activeServerId: null`, `useVisibleHosts()` returns the same hosts as `useHosts()`.
- With a valid `activeServerId`, `useVisibleHosts()` returns one host.
- With an unknown `activeServerId`, `useVisibleHosts()` returns every host, and reconcile sets it to `null`.
- Opening `/h/<other>/...` while machine A is active sets the active machine to `<other>`.
- Opening any route while "All machines" is active keeps `null`.
- The selection survives an app restart.

Anti-goal: Settings and pairing call sites keep `useHosts()`; drift gauge; limit total 8 from the Plan Verify `rg -c` command; read before changes and at completion.

## Verify

- `npx vitest run packages/app/src/stores/active-host-store.test.ts --bail=1` -> pass, covering null, valid, unknown, reconcile, and remember cases.
- `npm run typecheck` -> exit 0.
- `npm run lint -- packages/app/src/stores/active-host-store.ts packages/app/src/hosts/use-visible-hosts.ts packages/app/src/app/_layout.tsx` -> exit 0.
- Plan Verify `rg -c "useHosts\("` command on Settings and pairing files -> total 8.
