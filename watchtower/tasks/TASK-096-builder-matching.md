# TASK-096 Builder matching

Group: B (writes only the matching files and a hook)
Class: code

## Brief

Goal: Find the builder agent of each TASK and the lead agent of the run, so the tree shows who works on what and who waits for the owner.

Change: The page ignores builder agents -> `matchBuilders` maps agents to TASK IDs and finds the lead, and `useRunAgents` loads them.

Boundaries:

- Load agents of the page's workspace with `paseo.agents.list`, the same way [client/agent-target.ts](hiep-plugins/plugins/watchtower-board/client/agent-target.ts) does: not archived, newest first, at most 3 pages, refresh every 15 s.
- A builder carries the label `watchtower.task`. Its value is a TASK ID (`TASK-089`) or a Group tag (`D`). A Group tag maps to every TASK of that group from the overview. See Shared Context in [watchtower/CONTEXT.md](watchtower/CONTEXT.md).
- From 2026-10-14 the skill labels each builder with the first TASK ID of its group ([Q-037](watchtower/QUESTIONS.md)). Such a builder works on every TASK of that group, so map a TASK ID label to all TASKs that share its Group. Keep Group tag matching for older runs.
- When two agents match one TASK, keep the newest by `updatedAt`.
- Builder state: `running`; `waiting` when `requiresAttention` is true and `attentionReason` is `permission`; `failed` when status is `error`; else `idle`. Add `activeAgo` from `lastActivityAt` or `updatedAt`.
- Lead: the agent that most matched builders name as `parentAgentId`. If none, follow the default of [Q-036](watchtower/QUESTIONS.md): no lead agent.
- Skip mock-provider agents, as `chooseAgent` in [client/actions.ts](hiep-plugins/plugins/watchtower-board/client/actions.ts) does.
- No new RPC and no host change.

How:

- Write `tests/builders.test.ts` first, with plain agent objects.
- Write the pure `matchBuilders(agents, tasks)` in `client/builders.ts`.
- Write the thin hook `useRunAgents(hostId, workspaceId)` in `client/run-agents.ts`.

Files:

- [hiep-plugins/plugins/watchtower-board/client/builders.ts](hiep-plugins/plugins/watchtower-board/client/builders.ts) (new)
- [hiep-plugins/plugins/watchtower-board/client/run-agents.ts](hiep-plugins/plugins/watchtower-board/client/run-agents.ts) (new)
- [hiep-plugins/plugins/watchtower-board/tests/builders.test.ts](hiep-plugins/plugins/watchtower-board/tests/builders.test.ts) (new)

Expected result:

- A label `TASK-089` maps to TASK-089. A label `D` maps to every TASK of Group D.
- A builder waiting for permission is `waiting`; an errored one is `failed`.
- The lead is the shared parent of the builders; with no parent there is no lead.
- Archived and mock agents never match.

Anti-goal: The agent lookup stays light. Tripwire: `useRunAgents` makes at most 3 list calls per 15 s refresh, the same as `agent-target.ts`; read from the code at the end of the TASK.

## Verify

- `cd hiep-plugins/plugins/watchtower-board && npx tsx --test tests/builders.test.ts` -> all cases pass.
- `cd hiep-plugins/plugins/watchtower-board && npm run typecheck` -> exit 0.
- `cd hiep-plugins/plugins/watchtower-board && npm run lint` -> exit 0.
- Anti-goal: `rg -n "MAX_PAGES|REFRESH_MS|refetchInterval" hiep-plugins/plugins/watchtower-board/client/run-agents.ts` -> at most 3 pages and a 15000 ms refresh.
