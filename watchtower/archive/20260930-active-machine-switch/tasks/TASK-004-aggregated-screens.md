# TASK-004 Aggregated screens follow the active machine

Group: C (shared hooks and screen files listed below)
Class: risky

## Brief

Goal: Command center, schedules, history, and the label manager show only the active machine.

Change: these screens read every host -> they read `useVisibleHosts()`.

Boundaries:

- `useProjects` and `useAggregatedAgents` are shared. Add an option such as `scope: "visible" | "all"` with default `"all"`. Only the listed screens pass `"visible"`.
- The favicon keeps `"all"` (Q1). [packages/app/src/screens/project-settings-screen.tsx](packages/app/src/screens/project-settings-screen.tsx) and [packages/app/src/screens/projects-screen.tsx](packages/app/src/screens/projects-screen.tsx) keep `"all"`, because both are Settings pages.
- History: when a machine is active, pass that `serverId` to `useAgentHistory` and hide the local host filter. In "All machines" mode keep the local filter.
- Schedules: the list and the create form offer only visible hosts.

How:

- Add the scope option to [packages/app/src/hooks/use-projects.ts](packages/app/src/hooks/use-projects.ts) and [packages/app/src/hooks/use-aggregated-agents.ts](packages/app/src/hooks/use-aggregated-agents.ts).
- Switch [packages/app/src/hooks/use-schedules.ts](packages/app/src/hooks/use-schedules.ts) to `useVisibleHosts()`.
- Pass `"visible"` in command center, schedules screen, and schedule form.
- Use the visible host count for `showHost` in the command center.
- Update [packages/app/src/screens/sessions-screen.tsx](packages/app/src/screens/sessions-screen.tsx) for History.
- Use `useVisibleHosts()` in [packages/app/src/workspace-labels/manager-modal.tsx](packages/app/src/workspace-labels/manager-modal.tsx).

Files:

- [packages/app/src/hooks/use-projects.ts](packages/app/src/hooks/use-projects.ts) (scope option)
- [packages/app/src/hooks/use-aggregated-agents.ts](packages/app/src/hooks/use-aggregated-agents.ts) (scope option)
- [packages/app/src/hooks/use-schedules.ts](packages/app/src/hooks/use-schedules.ts) (visible hosts)
- [packages/app/src/command-center/command-center.tsx](packages/app/src/command-center/command-center.tsx) (visible scope and host count)
- [packages/app/src/screens/schedules-screen.tsx](packages/app/src/screens/schedules-screen.tsx) (visible scope)
- [packages/app/src/components/schedules/schedule-form-sheet.tsx](packages/app/src/components/schedules/schedule-form-sheet.tsx) (visible scope)
- [packages/app/src/screens/sessions-screen.tsx](packages/app/src/screens/sessions-screen.tsx) (History host)
- [packages/app/src/workspace-labels/manager-modal.tsx](packages/app/src/workspace-labels/manager-modal.tsx) (visible hosts)

Expected result:

- Machine A active: Search lists only A workspaces and agents.
- Machine A active: Schedules lists only A schedules. The form offers only A.
- Machine A active: History shows only A sessions and no host filter.
- "All machines" active: every screen behaves as before.
- The favicon still reflects agents on every host.

Anti-goal: Callers that must see every host stay on the default; tripwire; limit [packages/app/src/hooks/use-favicon-status.ts](packages/app/src/hooks/use-favicon-status.ts) and [packages/app/src/screens/project-settings-screen.tsx](packages/app/src/screens/project-settings-screen.tsx) pass no `"visible"` scope; read with `rg -n "hostScope" <those files>` before changes, after the hook change, and at completion.

## Verify

- `npx vitest run packages/app/src/hooks/use-projects.test.ts --bail=1` -> pass.
- `npx vitest run packages/app/src/hooks/use-schedules.test.ts --bail=1` -> pass.
- `rg -n "hostScope" packages/app/src/hooks/use-favicon-status.ts packages/app/src/screens/project-settings-screen.tsx packages/app/src/screens/projects-screen.tsx` -> no match.
- `npm run typecheck` -> exit 0.
- `npm run lint -- <files above>` -> exit 0.
