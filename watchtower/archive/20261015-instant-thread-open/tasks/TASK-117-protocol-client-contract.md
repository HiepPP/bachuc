# TASK-117 Protocol, client, and plugin contract

Group: A
Class: mechanical

## Brief

Goal: Give the app and plugins the wire field and types that phases 3 and 4 need.

Change: add optional `includeLatestPrompt` to `fetch_agent_timeline_request`, optional `timelineLatestPromptTail` to `server_info.features`, the client passthrough, and optional `navigation.prefetchAgent` in the plugin contract.

Boundaries:

- Follow [docs/protocol-compatibility.md](docs/protocol-compatibility.md): optional fields only, pure schemas, a COMPAT tag at each gate.

Files:

- [packages/protocol/src/messages.ts](packages/protocol/src/messages.ts)
- [packages/client/src/daemon-client.ts](packages/client/src/daemon-client.ts)
- [packages/plugin/src/client/contracts.ts](packages/plugin/src/client/contracts.ts)
- [packages/protocol/src/messages.timeline-latest-prompt.test.ts](packages/protocol/src/messages.timeline-latest-prompt.test.ts) (new)

Expected result:

- An old client and a new daemon still parse each other's messages.

## Verify

- `npx vitest run src/messages.timeline-latest-prompt.test.ts --bail=1` in `packages/protocol` -> pass.
- `npm run build:client` -> exit 0.
- `npm run typecheck` -> exit 0.
