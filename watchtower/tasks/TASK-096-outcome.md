# TASK-096 Outcome

## Status

DONE

## Changed

- `hiep-plugins/plugins/watchtower-board/client/builders.ts` (new)
- `hiep-plugins/plugins/watchtower-board/client/run-agents.ts` (new)
- `hiep-plugins/plugins/watchtower-board/tests/builders.test.ts` (new)

## Contract

- `matchBuilders(agents, tasks, now = Date.now())` returns `{ byTask, lead }`.
  - `byTask[taskId]` is `{ id, title, state, activeAgo }`. `title` falls back to the agent id. `state` is `running | waiting | failed | idle`. `activeAgo` is milliseconds, or null when the time is unreadable.
  - A label `TASK-089` maps to every TASK of that TASK's Group (to that TASK only when it has no group). A label `D` maps to every TASK of Group D. An unknown label maps to nothing.
  - Archived and mock-provider agents never match. Two agents on one TASK keep the newest by `updatedAt`.
  - `lead` is the parent named by most winning builders (each agent votes once), as `{ id, title, state, activeAgo }`. With no parent it is `null` (Q-036 default). A parent missing from the list keeps its id with a null title.
- `useRunAgents(hostId, workspaceId)` returns the workspace's agents (`BuilderCandidate[]`, empty until the first lookup). The caller passes them and the overview tasks to `matchBuilders`.

## Verified

- `npx tsx --test tests/builders.test.ts` -> 13 pass, 0 fail.
- `npm run typecheck` -> exit 0.
- `npx oxlint --disable-nested-config index.server.ts index.client.tsx server client shared tests` (the `lint` script body) -> 0 warnings, 0 errors. `npm run lint` itself is intercepted in this shell by an ESLint wrapper that finds no config and exits 2; this is not a code failure.
- `npm run format` -> changed only untracked new files (mine and the other builder's).
- Anti-goal: `rg -n "MAX_PAGES|REFRESH_MS|refetchInterval" client/run-agents.ts` -> `MAX_PAGES = 3`, `REFRESH_MS = 15_000`, `refetchInterval: REFRESH_MS`.

## Anti-goal

Tripwire: at most 3 list calls per 15 s refresh. Checkpoint: `MAX_PAGES` = 3 (`client/run-agents.ts:8`), `REFRESH_MS` = 15000 (`:9`), source read from code at the end of the TASK, 2026-10-14. Not crossed. `useRunAgents` has no early exit on a match, so a workspace with more than 600 agents uses all 3 calls; it never uses more.

## Lessons

- The agent snapshot (`AgentSnapshotPayload`) has no `parentAgentId` or `lastActivityAt` field. The parent is the label `paseo.parent-agent-id`, and activity time is `updatedAt`. The code reads the direct fields first, then falls back to those.
- `waiting` takes precedence over `running`: an agent blocked on a permission still reports status `running`, so the spec's order would hide every waiting builder.
- `useRunAgents` takes no `enabled` flag and `matchBuilders` needs the overview tasks, so the page that calls both owns the tasks.
- In this shell `npm run lint` exits 2 through an ESLint wrapper; run the oxlint command from the script body to check.
