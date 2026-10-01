# TASK-007 Outcome

## Outcome

Status: DONE

Changed:

- Added `addSidebarProjectFilter`, `addSidebarProjectMenuItems`, `addSidebarSection`, and `PluginSidebar*` types in [packages/plugin/src/client/contracts.ts](packages/plugin/src/client/contracts.ts).
- [packages/app/src/plugins/evaluate.ts](packages/app/src/plugins/evaluate.ts) collects them. [packages/app/src/plugins/types.ts](packages/app/src/plugins/types.ts) has optional fields.
- Added [packages/app/src/plugins/sidebar/model.ts](packages/app/src/plugins/sidebar/model.ts) (installation choice, hidden view keys, menu items) and [packages/app/src/plugins/sidebar/index.tsx](packages/app/src/plugins/sidebar/index.tsx) (hooks and `PluginSidebarSections`).
- [packages/app/src/components/sidebar/sidebar-model.tsx](packages/app/src/components/sidebar/sidebar-model.tsx) removes hidden projects and their workspaces. It adds `useOptionalSidebarModel`.
- [packages/app/src/components/sidebar-workspace-list.tsx](packages/app/src/components/sidebar-workspace-list.tsx) renders plugin items in both project menus, before Remove.
- [packages/app/src/components/left-sidebar.tsx](packages/app/src/components/left-sidebar.tsx) renders plugin sections above the footer on mobile and desktop.

Contract:

- Fix found in the TASK-017 host check: the sidebar did not re-filter when a plugin reported a change. React Compiler dropped `revision` from the memo inputs of `usePluginHiddenProjectViewKeys`, because the hook only read it with `void`. The hook now starts with `"use no memo"`.

- Deviation from the spec: menu items are flat `{ id, title, disabled?, onSelect }` entries, not `PluginButtonMenuEntry` submenus. The project context and kebab menus use different menu primitives.
- Each plugin contributes through one installation: the active host's, else the first.
- `allProjects` still holds every project, so pickers keep working. A throwing filter hides nothing.

Verified:

- `cd packages/app && npx vitest run --project unit src/plugins/sidebar src/components/sidebar --bail=1` -> 97 passed, including 3 new tests.
- `cd packages/app && npx vitest run --project unit src/stores/sidebar-view-store.test.ts --bail=1` -> pass.
- App typecheck -> exit 0. Lint on 10 files -> 0 errors.

Anti-goal:

- Final: `SIDEBAR_VIEW_STORE_VERSION = 6` and the store test passes; 2026-10-01T00:05.
- Result: PASS.
