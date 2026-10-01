# TASK-005 Creation defaults use the active machine

Group: D (landing, new workspace, and host chooser files listed below)
Class: code

## Brief

Goal: Landing, new workspace, open project, and add project default to the active machine.

Change: defaults pick from every host -> defaults pick from `useVisibleHosts()`.

Boundaries:

- A host in the route still wins. With Q4 the route host is also the active machine.
- Landing: when a machine is active and the last selection is on another host, open the active machine's remembered workspace, else `/h/<activeServerId>`.
- `useHostChooser` offers only visible hosts. With one visible host it picks that host without a dialog, as it already does for one host.
- The chooser modal at [packages/app/src/hosts/host-chooser.tsx](packages/app/src/hosts/host-chooser.tsx) line 148 looks up by `serverId` and keeps `useHosts()`.

How:

- Pass visible hosts and an active-host-aware selection to `resolveStartupRoute` in [packages/app/src/app/index.tsx](packages/app/src/app/index.tsx).
- Use visible server ids for `resolveNewWorkspaceInitialServerId` in [packages/app/src/screens/new-workspace-screen.tsx](packages/app/src/screens/new-workspace-screen.tsx).
- No change is needed in [packages/app/src/hooks/use-global-new-workspace-action.ts](packages/app/src/hooks/use-global-new-workspace-action.ts) or `SidebarNewWorkspaceRow`. Without a route host they open `/new`, and the new workspace screen then picks from visible hosts.
- In [packages/app/src/app/h/[serverId]/index.tsx](packages/app/src/app/h/[serverId]/index.tsx), restore the host's remembered workspace when the global selection is on another host.
- Use visible hosts in `useHostChooser`, [packages/app/src/components/add-project-flow.tsx](packages/app/src/components/add-project-flow.tsx), and [packages/app/src/screens/open-project-screen.tsx](packages/app/src/screens/open-project-screen.tsx).
- Add a case to [packages/app/src/screens/new-workspace-initial-context.test.ts](packages/app/src/screens/new-workspace-initial-context.test.ts) for one visible host.

Files:

- [packages/app/src/app/index.tsx](packages/app/src/app/index.tsx) (landing)
- [packages/app/src/screens/new-workspace-screen.tsx](packages/app/src/screens/new-workspace-screen.tsx) (default host)
- [packages/app/src/screens/new-workspace-initial-context.test.ts](packages/app/src/screens/new-workspace-initial-context.test.ts) (new case)
- [packages/app/src/app/h/[serverId]/index.tsx](packages/app/src/app/h/[serverId]/index.tsx) (per-host workspace restore)
- [packages/app/src/hosts/host-chooser.tsx](packages/app/src/hosts/host-chooser.tsx) (visible hosts in `useHostChooser`)
- [packages/app/src/components/add-project-flow.tsx](packages/app/src/components/add-project-flow.tsx) (visible hosts)
- [packages/app/src/screens/open-project-screen.tsx](packages/app/src/screens/open-project-screen.tsx) (visible hosts)

Expected result:

- Machine A active: New workspace opens with host A selected.
- Machine A active: Open project and Add project use A without a host dialog.
- Machine A active: app start opens A, not the last workspace on B.
- "All machines" active: defaults behave as before.

Anti-goal: Route host priority; tripwire; limit existing cases in [packages/app/src/screens/new-workspace-initial-context.test.ts](packages/app/src/screens/new-workspace-initial-context.test.ts) and [packages/app/src/navigation/host-runtime-bootstrap.test.ts](packages/app/src/navigation/host-runtime-bootstrap.test.ts) pass unchanged; read before changes and at completion.

## Verify

- `npx vitest run packages/app/src/screens/new-workspace-initial-context.test.ts --bail=1` -> pass, including the new case.
- `npx vitest run packages/app/src/navigation/host-runtime-bootstrap.test.ts --bail=1` -> pass.
- `npm run typecheck` -> exit 0.
- `npm run lint -- <files above>` -> exit 0.
