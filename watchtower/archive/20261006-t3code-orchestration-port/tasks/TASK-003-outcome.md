# TASK-003 Outcome

## Outcome

Status: DONE

Changed:

- [packages/protocol/src/messages.ts](packages/protocol/src/messages.ts): queue summary and queued message schemas, optional `queue` on the agent snapshot, six `agent.queue.*` request and response pairs in both unions, `server_info.features.agentMessageQueue`, all tagged `COMPAT(agentMessageQueue)`.
- [packages/server/src/server/agent/agent-message-queue.ts](packages/server/src/server/agent/agent-message-queue.ts) (new): per-agent JSON store in `$PASEO_HOME/agent-queues/`, a per-agent lock, drain on a normal run end, hold, resume, promote, a cap of 50, and `holdAgentMessageQueues`.
- [packages/server/src/server/session.ts](packages/server/src/server/session.ts): the six RPC handlers, and the queue summary on live and stored agent payloads.
- [packages/server/src/server/websocket-server.ts](packages/server/src/server/websocket-server.ts) creates the queue and sets the feature flag. [packages/server/src/server/authorization/operation-permissions.ts](packages/server/src/server/authorization/operation-permissions.ts) has the permission entries.
- [packages/server/src/server/agent/lifecycle-command.ts](packages/server/src/server/agent/lifecycle-command.ts): Stop holds the queues of the stopped subtree before any cancel. Archive holds the queue before its cancel. Stop now reads the agent once (TASK-001 regression fix).
- [packages/server/src/server/agent/agent-prompt.ts](packages/server/src/server/agent/agent-prompt.ts): `sendPromptToAgent` takes an optional `replaceRunning` (default `true`).
- [packages/client/src/daemon-client.ts](packages/client/src/daemon-client.ts): six queue methods.
- Tests: [packages/server/src/server/agent/agent-message-queue.test.ts](packages/server/src/server/agent/agent-message-queue.test.ts) (new), [packages/protocol/src/messages.wire-compat.test.ts](packages/protocol/src/messages.wire-compat.test.ts), [packages/client/src/daemon-client.test.ts](packages/client/src/daemon-client.test.ts).
- Docs: [docs/agent-lifecycle.md](docs/agent-lifecycle.md) (Message queue) and [docs/data-model.md](docs/data-model.md) (6a).

Contract:

- Q-002 answered: Stop holds a non-empty queue; resume or promote releases it. Q-003: an error end does not drain. Q-004: 50 items.
- The queue drains only on a `running` to `idle` change with no in-flight run, never on boot or a fresh load. Enqueue and resume on an idle agent start at once.
- A drain never replaces a run: it starts with `replaceRunning: false`, and a failed start keeps the item at the head.
- An archived agent rejects enqueue, and a send to it fails, so no item is dropped silently.
- Spec deviations, accepted by the reviewer:
  - Queued images stay inline as base64 in the queue file, not as daemon files.
  - `cancel` returns the full item for the app's Edit; there is no take RPC.
  - The hold applies only to a non-empty queue.
  - A direct send does not take the queue lock.
  - Protocol tests went into the existing wire-compat suite, per the repo rule.
- Known nits, not changed:
  - The enqueue schema reuses `AgentAttachmentsSchema` (an existing inbound transform) for parity with `send_agent_message_request`.
  - Each queue change calls `notifyAgentState`, which bumps `updatedAt`.

Verified:

- `npx vitest run packages/server/src/server/agent/agent-message-queue.test.ts` -> 13 passed, 3 runs in a row (drain, error end, restart, idle enqueue, Stop hold, subagent hold, empty hold, cap, edit, reorder, cancel, promote, promote-versus-drain race, failed send, archived agent, archive hold).
- `npx vitest run packages/protocol/src/messages.wire-compat.test.ts` -> 15 passed. `messages.active-turn-behavior.test.ts` -> 3 passed.
- `npx vitest run packages/client/src/daemon-client.test.ts` -> 147 passed. `packages/server/src/server/session.test.ts` -> 151 passed. `authorization/index.test.ts` -> 7 passed. `lifecycle-command.test.ts` -> 13 passed. `agent-prompt.test.ts` -> 28 passed. `mcp-server.test.ts` -> 121 passed.
- `npm run build:client`, `npm run typecheck`, `npm run lint`, and `npm run format:check:files` -> exit 0. The reviewer ran `generate:validators`: the new outbound schemas compile.
- Reviewer agent -> `VERDICT: ISSUES`. Fixed: a lost item after a failed send (blocker), archived agents losing items, and a drain replacing a run that just started.
- Manual live check -> UNVERIFIED (autonomous run). The app does not use the queue until TASK-004.

Anti-goal:

- Before changes: active-turn test 3 passed; old snapshot shape is the current schema; 2026-10-06 23:34.
- After the protocol edit: 3 passed, and the wire-compat case "agent snapshots parse with and without the daemon message queue" passed (a snapshot without `queue` parses, and a pre-queue schema parses one with it); 23:37.
- Final: 3 passed and 15 passed; 2026-10-06 23:57.
- Result: PASS against the tripwire. Existing wire schemas gained 0 required fields.

Lessons:

- `session.test.ts` mocks count `getAgent` calls. Run it after any change to [packages/server/src/server/agent/lifecycle-command.ts](packages/server/src/server/agent/lifecycle-command.ts). It caught a TASK-001 regression here.
- In queue tests, wait on the per-agent lock before an absence assertion. An unchanged `edit` waits without side effects; `hold` holds a non-empty queue.
- TASK-004 reads the queue from `snapshot.queue` (absent means empty) and loads an edited item from the `cancel` response.
