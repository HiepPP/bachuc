# ADR-0055 GPUI opens the new workspace thread at agent_ready over the create response

Status: accepted
Date: 2026-10-20
Supersedes: -
Scope: `packages/desktop-gpui/crates/client/src/core.rs`, `packages/desktop-gpui/crates/client/src/composer_data.rs`, `packages/desktop-gpui/crates/app/src/new_workspace.rs`

## Context

- Hiep saw a lag after the first prompt on the GPUI New workspace screen: the thread opened 1.5-4.3 s after submit on live.
- The daemon sends the `workspace.create.request` response only after the provider starts the first turn (`waitForAgentRunStartWithTimeout`, `packages/server/src/server/agent/agent-prompt.ts:365`). On live that wait cost 0.93-2.76 s. Workspace and agent creation took about 1 s together.
- The daemon already sends `workspace.create.update` progress, keyed by the request's idempotency key, to a client that sets `subscribe: true`. The GPUI client did not subscribe.

## Decision

- When the daemon has the `workspaceRequestReceipts` and `creationLifecycle` features, the GPUI client sends the create request with an idempotency key and `subscribe: true`.
- The first update with phase `agent_ready`, `prompt_started`, or `completed` resolves the reply, and the New workspace screen opens the agent tab. The later response finds no pending request and is dropped.
- The daemon keeps its wait. Electron and the CLI rely on the response meaning "the first turn started".
- Without those features, the client waits for the response as before.

## Consequences

- Submit to open tab is 0.57 s on live.
- If the first turn fails to start after `agent_ready`, the New workspace screen no longer shows that error. The agent tab must show it. This was not verified on 2026-10-20.
- The early reply has no `setupTerminalId`. No GPUI code reads it today.

## Options Considered

- Remove the daemon wait: rejected. It changes the response contract for Electron and the CLI.
- Open the thread before the daemon replies, with a pending state: rejected. It needs a fake agent tab and its own error path, and the measured cost was the turn-start wait only.

## Revisit If

- A first-turn start error is lost because the agent tab does not show it.
- GPUI starts to use `setupTerminalId` from the create reply.
- The daemon drops `workspace.create.update` or changes its phases.
