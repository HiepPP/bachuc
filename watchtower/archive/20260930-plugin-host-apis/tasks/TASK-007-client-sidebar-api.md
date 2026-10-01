# TASK-007 Sidebar API for project filters, menus, and sections

Group: B (client plugin host files)
Class: risky

## Brief

Goal: Let a plugin filter sidebar projects, add project menu items, and render a sidebar section.

Change: workspace-spaces edits the sidebar DOM -> three sidebar contributions.

Boundaries:

- `addSidebarProjectFilter({ id, subscribe(listener), isVisible(project, { activeServerId }) })`. Filters apply together with the user's project filter. Pickers keep reading all projects.
- `addSidebarProjectMenuItems({ id, getItems(project, { activeServerId }) })` returns `PluginButtonMenuEntry[]`. Items render in the kebab and right-click project menus, before Remove. Submenus follow [docs/menus.md](docs/menus.md).
- `addSidebarSection({ id, Component })` renders above the sidebar footer on desktop and mobile. Props add `activeServerId` to the host props. It is wrapped in the plugin error boundary.
- The project value is `{ viewKey, name, serverIds }`.
- Follows [ADR-0001](watchtower/decisions/ADR-0001-active-host-over-host-filters.md): the active host still decides which host shows. The plugin filter only narrows that list.
- Only the installation on one host renders a section, picked the same way as `sidebar-items.tsx`.

How:

- Add the types to [packages/plugin/src/client/contracts.ts](packages/plugin/src/client/contracts.ts).
- Collect them in [packages/app/src/plugins/evaluate.ts](packages/app/src/plugins/evaluate.ts).
- Apply filters in `filteredProjects` in [packages/app/src/components/sidebar/sidebar-model.tsx](packages/app/src/components/sidebar/sidebar-model.tsx).
- Render menu items in `ProjectMenuItems` in [packages/app/src/components/sidebar-workspace-list.tsx](packages/app/src/components/sidebar-workspace-list.tsx).
- Render sections in [packages/app/src/components/left-sidebar.tsx](packages/app/src/components/left-sidebar.tsx).
- Add tests for the filter logic.

Files:

- [packages/plugin/src/client/contracts.ts](packages/plugin/src/client/contracts.ts) (types)
- [packages/app/src/plugins/evaluate.ts](packages/app/src/plugins/evaluate.ts) (registration)
- [packages/app/src/plugins/types.ts](packages/app/src/plugins/types.ts) (collected fields)
- [packages/app/src/plugins/registry.ts](packages/app/src/plugins/registry.ts) (defaults)
- [packages/app/src/components/sidebar/sidebar-model.tsx](packages/app/src/components/sidebar/sidebar-model.tsx) (filter)
- [packages/app/src/components/sidebar-workspace-list.tsx](packages/app/src/components/sidebar-workspace-list.tsx) (menu items)
- [packages/app/src/components/left-sidebar.tsx](packages/app/src/components/left-sidebar.tsx) (section)
- A new sidebar plugin module and test under [packages/app/src/plugins](packages/app/src/plugins)

Expected result:

- A filter that hides a project removes it from the list, and the list updates when the plugin calls its listener.
- A plugin menu item shows in both project menus and runs its handler.
- A section renders above the footer.

Anti-goal: Sidebar store behavior; tripwire; limit `SIDEBAR_VIEW_STORE_VERSION` stays 6 and [packages/app/src/stores/sidebar-view-store.test.ts](packages/app/src/stores/sidebar-view-store.test.ts) passes; read before changes and at completion.

## Verify

- `npx vitest run <new sidebar plugin test> --bail=1` -> pass.
- `npx vitest run packages/app/src/stores/sidebar-view-store.test.ts --bail=1` -> pass.
- `npm run typecheck` -> exit 0.
- Anti-goal: `rg -n "SIDEBAR_VIEW_STORE_VERSION = 6" packages/app/src/stores/sidebar-view-store.ts` -> one match.
