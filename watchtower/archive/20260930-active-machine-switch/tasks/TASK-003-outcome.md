# TASK-003 Outcome

## Outcome

Status: DONE

Changed:

- Added `useEffectiveSidebarHostFilters` and `useHasActiveSidebarHostFilter` in [packages/app/src/hooks/use-sidebar-workspaces-list.ts](packages/app/src/hooks/use-sidebar-workspaces-list.ts). The active host wins over `hostFilters`.
- The display menu hides the host filter while a host is active. Row badges and the skeleton flag follow the effective filter.

Contract:

- The `hostFilters` persisted shape did not change, and there is no migration.

Verified:

- `npx vitest run src/stores/sidebar-view-store.test.ts --bail=1` -> pass.
- `npm run typecheck` -> exit 0. `npm run lint` -> 0 errors.
- Verify 2026-09-30T17:40: `npm run typecheck` (all workspaces) -> exit 0; `npm run lint` on 27 changed files -> 0 errors; 8 test files -> 81 passed.

Anti-goal:

- Before changes: `SIDEBAR_VIEW_STORE_VERSION = 6`; 2026-09-30.
- Final: 6, and sidebar-view-store tests pass; 2026-09-30.
- Verification read: same value; checks run 2026-09-30T17:40.
- Result: PASS.
