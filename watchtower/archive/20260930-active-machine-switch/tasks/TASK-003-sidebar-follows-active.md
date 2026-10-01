# TASK-003 Sidebar follows the active machine

Group: B (shares [packages/app/src/components/left-sidebar.tsx](packages/app/src/components/left-sidebar.tsx) with TASK-002)
Class: code

## Brief

Goal: The sidebar workspace list shows only the active machine, with no second conflicting filter.

Change: sidebar filtered only by `hostFilters` -> active machine first, `hostFilters` only in "All machines" mode.

Boundaries:

- Active machine wins. When a machine is active, the effective host filter is `[activeServerId]`.
- Keep `hostFilters` and its persisted shape. Do not bump `SIDEBAR_VIEW_STORE_VERSION`; no migration.
- Hide the host filter section in the display preferences menu while a machine is active.
- Host badges on sidebar rows follow the visible host count. One visible host means no badges.

How:

- In [packages/app/src/hooks/use-sidebar-workspaces-list.ts](packages/app/src/hooks/use-sidebar-workspaces-list.ts), derive the effective host filter from `activeServerId` and `hostFilters`.
- In [packages/app/src/components/sidebar/display-preferences/menu.tsx](packages/app/src/components/sidebar/display-preferences/menu.tsx), hide the host filter section when a machine is active.
- In [packages/app/src/components/sidebar-workspace-list.tsx](packages/app/src/components/sidebar-workspace-list.tsx), use `useVisibleHosts()` for badge visibility.
- In [packages/app/src/components/left-sidebar.tsx](packages/app/src/components/left-sidebar.tsx), make `hasActiveHostFilter` true when a machine is active, so the skeleton logic matches.

Files:

- [packages/app/src/hooks/use-sidebar-workspaces-list.ts](packages/app/src/hooks/use-sidebar-workspaces-list.ts) (effective host filter)
- [packages/app/src/components/sidebar/display-preferences/menu.tsx](packages/app/src/components/sidebar/display-preferences/menu.tsx) (hide host section)
- [packages/app/src/components/sidebar-workspace-list.tsx](packages/app/src/components/sidebar-workspace-list.tsx) (badge host count)
- [packages/app/src/components/left-sidebar.tsx](packages/app/src/components/left-sidebar.tsx) (skeleton filter flag)

Expected result:

- Machine A active: sidebar shows only A workspaces, even when `hostFilters` names B.
- "All machines" active: sidebar applies `hostFilters` as before.
- Machine A active: the display menu has no host filter section.

Anti-goal: Existing sidebar view store behavior; tripwire; limit `SIDEBAR_VIEW_STORE_VERSION` stays 6 and [packages/app/src/stores/sidebar-view-store.test.ts](packages/app/src/stores/sidebar-view-store.test.ts) passes; read before changes and at completion.

## Verify

- `npx vitest run packages/app/src/stores/sidebar-view-store.test.ts --bail=1` -> pass.
- `rg -n "SIDEBAR_VIEW_STORE_VERSION = 6" packages/app/src/stores/sidebar-view-store.ts` -> one match.
- `npm run typecheck` -> exit 0.
- `npm run lint -- packages/app/src/hooks/use-sidebar-workspaces-list.ts packages/app/src/components/sidebar/display-preferences/menu.tsx packages/app/src/components/sidebar-workspace-list.tsx packages/app/src/components/left-sidebar.tsx` -> exit 0.
