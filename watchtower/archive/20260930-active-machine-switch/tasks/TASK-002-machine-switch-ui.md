# TASK-002 Machine switch in the sidebar

Group: B (shares [packages/app/src/components/left-sidebar.tsx](packages/app/src/components/left-sidebar.tsx) with TASK-003)
Class: code

## Brief

Goal: Add one visible control at the top of the sidebar that picks a machine or "All machines".

Change: no machine control -> a switch row above the sidebar nav rows on desktop and mobile.

Boundaries:

- Reuse `HostPicker` with `includeAllHost`. Do not build a new picker.
- The row shows the active machine label, or "All machines", so the user always sees which machine the UI shows.
- Show the switch only when there are 2 or more hosts.
- Q1: show an attention badge when a hidden machine has an agent that needs attention or permission. Use `requiresAttention` and `pendingPermissionCount`, as in [packages/app/src/hooks/use-favicon-status.ts](packages/app/src/hooks/use-favicon-status.ts).
- On select: set `activeServerId`. For a machine, open its remembered workspace, else `/h/<serverId>`. For "All machines", stay on the current route.
- The footer `SidebarHostPicker` keeps `useHosts()`. It opens host settings for every host.
- Follow [docs/design.md](docs/design.md) and [docs/hover.md](docs/hover.md). Do not use `useUnistyles()` in new code; see [docs/unistyles.md](docs/unistyles.md).

How:

- Create the switch component with `HostPicker`, `useHosts()`, `useActiveServerId()`, and the aggregated agents for the badge.
- Render it above `SidebarNavRows` in `DesktopSidebar` and `MobileSidebar`.
- Add a stable `testID="sidebar-active-host-switch"`.

Files:

- [packages/app/src/components/sidebar/active-host-switch.tsx](packages/app/src/components/sidebar/active-host-switch.tsx) (new switch component)
- [packages/app/src/components/left-sidebar.tsx](packages/app/src/components/left-sidebar.tsx) (render the switch)

Expected result:

- The sidebar shows the switch with the current label when 2 or more hosts exist.
- Picking a machine sets the active machine and opens that machine's workspace or host index.
- Picking "All machines" sets `activeServerId` to `null`.
- A hidden machine with an agent that needs attention shows a badge on the switch.
- With one host, no switch renders.

Anti-goal: The footer host picker still lists every host; tripwire; limit `useHosts()` stays in `SidebarHostPicker`; read with `rg -n "useHosts\(" packages/app/src/components/left-sidebar.tsx` before changes and at completion.

## Verify

- `npm run typecheck` -> exit 0.
- `npm run lint -- packages/app/src/components/sidebar/active-host-switch.tsx packages/app/src/components/left-sidebar.tsx` -> exit 0.
- `rg -n "useHosts\(" packages/app/src/components/left-sidebar.tsx` -> still one match inside `SidebarHostPicker`.
- Manual check in the Paseo Dev app (human): the switch shows, changes machine, and shows a badge for a hidden machine that needs attention.
