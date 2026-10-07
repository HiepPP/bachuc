# TASK-003 Daemon-side message queue

Group: A (shared file with TASK-001: [packages/server/src/server/agent/lifecycle-command.ts](packages/server/src/server/agent/lifecycle-command.ts), and [docs/agent-lifecycle.md](docs/agent-lifecycle.md))
Class: risky

## Brief

Goal: The daemon keeps a queue of messages per agent. The queue survives a client close and a daemon restart, every client sees the same queue, and the daemon starts the next message when the agent's run ends.

Change: queue in client memory ([packages/app/src/stores/session-store.ts](packages/app/src/stores/session-store.ts), line 420) -> queue in a daemon JSON store, shown to clients in the agent snapshot.

Design: T3 Code dispatch modes in `packages/contracts/src/orchestrationV2.ts:2863-2955`; queue operations in `apps/server/src/orchestration-v2/ThreadManagementService.ts:41,570`; drain in `apps/server/src/orchestration-v2/Orchestrator.ts:10235-10303`.

Boundaries:

- New RPC pairs: `agent.queue.enqueue`, `agent.queue.edit`, `agent.queue.reorder` (with `beforeId`), `agent.queue.cancel`, `agent.queue.promote`, and `agent.queue.resume`, each with `.request` and `.response`. Add a permission entry for each in [packages/server/src/server/authorization/operation-permissions.ts](packages/server/src/server/authorization/operation-permissions.ts).
- The enqueue payload matches `send_agent_message_request`: text, images, and attachments.
- Add an optional queue summary (item id, text, attachment kinds, held flag) to `AgentSnapshotPayloadSchema` in [packages/protocol/src/messages.ts](packages/protocol/src/messages.ts). Never put image bytes in the snapshot.
- Gate on `server_info.features.agentMessageQueue`. Tag each shim with `COMPAT(agentMessageQueue)`.
- Store the queue in `$PASEO_HOME/agent-queues/<agentId>.json` with atomic writes ([docs/data-model.md](docs/data-model.md)). Do not add it to `STORED_AGENT_SCHEMA`: `toStoredAgentRecord` rebuilds the record on every flush and drops unknown fields.
- Drain only on a running-to-idle change. Do not drain on error (Q-003 answered, default) or on daemon boot. A finalize with `pendingReplacement` must not drain.
- One per-agent lock covers drain, promote, and direct sends. Re-check `hasInFlightRun` under the lock.
- Address every mutation by item id. A missing id returns an "already sent" error.
- Stop holds the queue (Q-002 answered: yes). Hook this into `cancelAgentRunCommand`. `agent.queue.resume` or a promote releases it.
- At most 50 items per agent (Q-004 answered, default).
- Store queued images as daemon files. Edit returns them as file attachments.
- MCP `send_agent_prompt` gets no queue option in this TASK.

How:

- Run GitNexus `impact` on `sendPromptToAgent`, `cancelAgentRunCommand`, and `AgentSnapshotPayloadSchema`.
- Add the protocol schemas and the feature flag.
- Add [packages/server/src/server/agent/agent-message-queue.ts](packages/server/src/server/agent/agent-message-queue.ts): the store, the lock, and a drain that subscribes to `agent_state` like `setupFinishNotification` does.
- Add handlers in [packages/server/src/server/session.ts](packages/server/src/server/session.ts). Set the flag in [packages/server/src/server/websocket-server.ts](packages/server/src/server/websocket-server.ts) (near line 1823).
- Hold the queue in `cancelAgentRunCommand`.
- Add client methods in [packages/client/src/daemon-client.ts](packages/client/src/daemon-client.ts).
- Add tests. Document the new file in [docs/data-model.md](docs/data-model.md) and the drain rules in [docs/agent-lifecycle.md](docs/agent-lifecycle.md).

Files:

- [packages/protocol/src/messages.ts](packages/protocol/src/messages.ts) (queue RPCs, snapshot summary, feature flag)
- [packages/protocol/src/messages.agent-queue.test.ts](packages/protocol/src/messages.agent-queue.test.ts) (new: schema and old-snapshot tests)
- [packages/server/src/server/agent/agent-message-queue.ts](packages/server/src/server/agent/agent-message-queue.ts) (new: store, lock, drain)
- [packages/server/src/server/agent/agent-message-queue.test.ts](packages/server/src/server/agent/agent-message-queue.test.ts) (new)
- [packages/server/src/server/session.ts](packages/server/src/server/session.ts) (RPC handlers)
- [packages/server/src/server/authorization/operation-permissions.ts](packages/server/src/server/authorization/operation-permissions.ts) (permission entries)
- [packages/server/src/server/websocket-server.ts](packages/server/src/server/websocket-server.ts) (feature flag)
- [packages/server/src/server/agent/lifecycle-command.ts](packages/server/src/server/agent/lifecycle-command.ts) (hold on Stop)
- [packages/client/src/daemon-client.ts](packages/client/src/daemon-client.ts) (client methods)
- [packages/client/src/daemon-client.test.ts](packages/client/src/daemon-client.test.ts) (client tests)
- [docs/data-model.md](docs/data-model.md) and [docs/agent-lifecycle.md](docs/agent-lifecycle.md)

Expected result:

- Enqueue while the agent runs -> the item shows in the snapshot for every client. After the run ends, the daemon sends the head item and removes it.
- Daemon restart -> the queue is still there, and nothing is sent at boot.
- Promote -> the item is sent at once as a steer.
- Edit, reorder, and cancel work by id. A missing id -> "already sent" error.
- Stop -> the queue is held. Resume -> the next idle change drains it.
- A run that ends in error -> no drain.
- The 51st enqueue -> rejected with a clear error.
- An old client that never calls the queue RPCs -> `send_agent_message_request` works as before.

Anti-goal: An old-shape agent snapshot (with no queue field) still parses with the new schema, and existing wire schemas gain 0 required fields; tripwire; read from `packages/protocol/src/messages.agent-queue.test.ts` and `npx vitest run packages/protocol/src/messages.active-turn-behavior.test.ts --bail=1` before changes, after the protocol edit, and at completion.

## Verify

- `npx vitest run packages/protocol/src/messages.agent-queue.test.ts --bail=1` -> pass, including the old-snapshot case.
- `npx vitest run packages/protocol/src/messages.active-turn-behavior.test.ts --bail=1` -> pass.
- `npx vitest run packages/server/src/server/agent/agent-message-queue.test.ts --bail=1` -> pass: drain on idle, no drain on error or boot, hold on Stop, lock race, 50-item cap, restart reload.
- `npx vitest run packages/server/src/server/agent/lifecycle-command.test.ts --bail=1` -> pass.
- `npx vitest run packages/client/src/daemon-client.test.ts --bail=1` -> pass.
- `npm run build:client` then `npm run typecheck` -> exit 0.
- `npm run lint` on every changed file -> exit 0.
