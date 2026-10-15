# TASK-108 Outcome

## Status

DONE. Not committed.

## Changed

- `messages.ts`: `includeLatestPrompt` and `timelineLatestPromptTail`, both with `COMPAT(timelineLatestPromptTail)`, removal after 2027-04-15.
- `daemon-client.ts`: sends `includeLatestPrompt` only when it is `true`.
- `contracts.ts`: optional `navigation.prefetchAgent({ agentId, serverId?, anchor? })`.

## Verified

Measured 2026-10-15.

- Protocol test -> 2 passed.
- `npm run generate:validators` -> no change to generated files.
- `npm run build:client`, `npm run typecheck`, lint on the 4 files -> pass.

## Lessons

- A shell wrapper routed bare `npm run lint` to ESLint. `command npm run lint` runs oxlint.
