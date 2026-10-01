# TASK-004 Outcome

## Outcome

Status: DONE

Changed:

- `useProjects` and `useAggregatedAgents` take `hostScope` (default "all").
- The command center, schedules screen, and schedule form pass "visible". `useSchedules` and the label manager use `useVisibleHosts()`.
- History pins to the active host and hides its local filter. The logic moved into `useHistoryHost` to stay under the complexity limit.

Contract:

- The favicon, project settings, and Settings projects screen still read every host.

Verified:

- `npx vitest run src/hooks/use-projects.test.ts src/hooks/use-schedules.test.ts --bail=1` -> pass.
- `rg -n "hostScope" <favicon, project-settings, projects-screen>` -> no match.
- `npm run typecheck` -> exit 0. `npm run lint` -> 0 errors.
- Verify 2026-09-30T17:40: `npm run typecheck` (all workspaces) -> exit 0; `npm run lint` on 27 changed files -> 0 errors; 8 test files -> 81 passed.

Anti-goal:

- Before changes: no `hostScope` in those files; 2026-09-30.
- After the hook change: none; 2026-09-30.
- Final: none; 2026-09-30.
- Verification read: same value; checks run 2026-09-30T17:40.
- Result: PASS.
