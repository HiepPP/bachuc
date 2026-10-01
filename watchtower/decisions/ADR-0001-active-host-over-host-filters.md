# ADR-0001 Active host over sidebar host filters

Status: accepted
Date: 2026-09-30
Supersedes: -
Scope: packages/app/src/hooks/use-sidebar-workspaces-list.ts, packages/app/src/stores/sidebar-view-store.ts, packages/app/src/components/left-sidebar.tsx

## Context

- The app has a persisted active host selection in [packages/app/src/stores/active-host-store.ts](packages/app/src/stores/active-host-store.ts). `activeServerId: null` means all hosts.
- The sidebar already had a multi-host filter `hostFilters` in [packages/app/src/stores/sidebar-view-store.ts](packages/app/src/stores/sidebar-view-store.ts).
- Two filters on the same list can disagree and show an empty sidebar with no clear cause.

## Decision

- When a host is active, the sidebar shows only that host and hides the host filter section.
- `hostFilters` applies only in the "All hosts" mode.
- The user chose this on 2026-09-30 while archiving plan `20260930-active-machine-switch`.

## Consequences

- `useEffectiveSidebarHostFilters` in [packages/app/src/hooks/use-sidebar-workspaces-list.ts](packages/app/src/hooks/use-sidebar-workspaces-list.ts) decides the filter. New sidebar code must read it, not `hostFilters` directly.
- The persisted `hostFilters` shape stays the same, so no migration is needed.

## Options Considered

- Run both filters together: rejected. The two filters can disagree, and the user cannot see which one hides a project.

## Revisit If

- The user asks to show several chosen hosts at once while one host is active.
