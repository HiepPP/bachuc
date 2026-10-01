# TASK-001 Outcome

## Outcome

Status: DONE

Changed:

- Added [packages/app/src/stores/active-host-store.ts](packages/app/src/stores/active-host-store.ts): persisted `activeServerId` (null means all hosts) and `lastWorkspaceIdByServerId`. Storage errors still finish hydration.
- Added [packages/app/src/hosts/use-visible-hosts.ts](packages/app/src/hosts/use-visible-hosts.ts): `useActiveServerId`, `useVisibleHosts`, `useScopedHosts`, `useActiveHostHydrated`, `useActiveHostRouteSync`.
- Mounted `ActiveHostRouteSync` in [packages/app/src/app/\_layout.tsx](packages/app/src/app/_layout.tsx).

Contract:

- Q4: the route sync runs only on a route change or after hydration, so picking a host and then navigating is not undone.
- Reconcile and remember both wait for hydration.

Verified:

- `npx vitest run src/stores/active-host-store.test.ts --bail=1` -> 6 passed.
- `npm run typecheck` -> exit 0. `npm run lint` on changed files -> 0 errors.
- Verify 2026-09-30T17:40: `npm run typecheck` (all workspaces) -> exit 0; `npm run lint` on 27 changed files -> 0 errors; 8 test files -> 81 passed.

Anti-goal:

- Before changes: Settings and pairing `useHosts()` total 8; Plan Verify rg command, 2026-09-30.
- Final: total 8; same command, 2026-09-30.
- Verification read: same value; checks run 2026-09-30T17:40.
- Result: PASS against a limit of 8.
