# TASK-001 Cascade Stop to managed subagents

Group: A (shared files with TASK-002 and TASK-003: [packages/server/src/server/agent/agent-prompt.ts](packages/server/src/server/agent/agent-prompt.ts), [packages/server/src/server/agent/lifecycle-command.ts](packages/server/src/server/agent/lifecycle-command.ts), [docs/agent-lifecycle.md](docs/agent-lifecycle.md))
Class: risky

## Brief

Goal: Stop on a parent agent also stops its running managed subagents, deepest first. No finish notice from the stopped subtree wakes the parent.

Change: Stop interrupts only the parent, and running children later wake it -> Stop cancels the whole managed subtree and disarms its finish notices.

Design: T3 Code cascades Stop in `apps/server/src/orchestration-v2/Orchestrator.ts:10214-10234` (deepest first, only when the queue is held).

Boundaries:

- Change only `cancelAgentRunCommand` in [packages/server/src/server/agent/lifecycle-command.ts](packages/server/src/server/agent/lifecycle-command.ts). Do not change `AgentManager.cancelAgentRun`. Reload, replace, rewind, and archive call it, about 8 callers.
- Find children by the label `paseo.parent-agent-id`, the same way `cascadeArchiveChildren` does in [packages/server/src/server/agent/agent-manager.ts](packages/server/src/server/agent/agent-manager.ts) (line 1786).
- Skip provider subagents. The parent interrupt already stops them through `cancelRunningProviderSubagents`.
- Leave idle, archived, and detached agents alone.
- Q-009 answered (default): a Hub interrupt cascades too, because it calls the same command.
- Finish notices are armed in memory by `setupFinishNotification` in [packages/server/src/server/agent/agent-prompt.ts](packages/server/src/server/agent/agent-prompt.ts) (lines 433-588), keyed by child and caller.

How:

- Run GitNexus `impact` on `cancelAgentRunCommand` and `setupFinishNotification`.
- Export `disarmFinishNotifications(agentManager, callerAgentId)` from `agent-prompt.ts`. It stops every armed entry whose caller is that agent.
- In `cancelAgentRunCommand`, collect live descendants by label, recursively.
- Disarm notices for the parent and for each descendant.
- Cancel the descendants deepest first, then cancel the parent.
- Add tests. Update the Cancellation section of [docs/agent-lifecycle.md](docs/agent-lifecycle.md).

Files:

- [packages/server/src/server/agent/agent-prompt.ts](packages/server/src/server/agent/agent-prompt.ts) (export the disarm function)
- [packages/server/src/server/agent/lifecycle-command.ts](packages/server/src/server/agent/lifecycle-command.ts) (cascade in `cancelAgentRunCommand`)
- [packages/server/src/server/agent/agent-prompt.test.ts](packages/server/src/server/agent/agent-prompt.test.ts) (disarm test)
- [packages/server/src/server/agent/lifecycle-command.test.ts](packages/server/src/server/agent/lifecycle-command.test.ts) (cascade tests)
- [docs/agent-lifecycle.md](docs/agent-lifecycle.md) (Cancellation section)

Expected result:

- A parent has 2 running children and 1 running grandchild. Stop cancels the grandchild, then the children, then the parent.
- After Stop, the parent gets no system-notification prompt from any stopped descendant.
- An idle child, an archived child, and a detached agent are not touched.
- Reload, replace, and rewind of a parent do not cancel its children.
- MCP `cancel_agent` on a parent cascades the same way as the app Stop.

Anti-goal: The `cancelAgentRun` callers for reload, replace, rewind, and archive keep their behavior; tripwire: any failure in `npx vitest run packages/server/src/server/agent/agent-manager.test.ts -t "cascade|cancel" --bail=1`; read before changes, after the cascade change, and at completion.

## Verify

- `npx vitest run packages/server/src/server/agent/lifecycle-command.test.ts --bail=1` -> pass, with the new cascade tests.
- `npx vitest run packages/server/src/server/agent/agent-prompt.test.ts --bail=1` -> pass, with the new disarm test.
- `npx vitest run packages/server/src/server/agent/agent-manager.test.ts -t "cascade|cancel" --bail=1` -> pass (anti-goal read).
- `npm run typecheck` -> exit 0.
- `npm run lint -- packages/server/src/server/agent/agent-prompt.ts packages/server/src/server/agent/lifecycle-command.ts` -> exit 0.
- Manual live check (needs a human and the live daemon): a parent agent starts one subagent through MCP, then the user presses Stop on the parent -> both go idle, and the parent gets no finish notice.
