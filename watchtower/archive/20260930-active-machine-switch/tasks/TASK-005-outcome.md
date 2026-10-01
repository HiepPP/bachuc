# TASK-005 Outcome

## Outcome

Status: DONE

Changed:

- The landing page uses visible hosts and redirects to `/h/<activeServerId>` when the last selection is on another host.
- The host index restores the host's remembered workspace.
- The new workspace screen, `useHostChooser` (default "visible"), add project, and open project use visible hosts.
- `OpenProjectListener` passes "all", so a local path from the OS still asks which host to use (review finding P2).

Contract:

- The route host still wins in `resolveNewWorkspaceInitialServerId`.

Verified:

- `npx vitest run src/screens/new-workspace-initial-context.test.ts src/navigation/host-runtime-bootstrap.test.ts --bail=1` -> pass, including the new case.
- `npm run typecheck` -> exit 0. `npm run lint` -> 0 errors.
- Verify 2026-09-30T17:40: `npm run typecheck` (all workspaces) -> exit 0; `npm run lint` on 27 changed files -> 0 errors; 8 test files -> 81 passed.

Anti-goal:

- Before changes: the existing cases pass; 2026-09-30.
- Final: they pass unchanged; 2026-09-30.
- Verification read: same value; checks run 2026-09-30T17:40.
- Result: PASS.
