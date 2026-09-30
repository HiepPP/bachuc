# Plan Context

## Shared Context

- Plugin: [plugins/thread-context-attach](plugins/thread-context-attach). It attaches the last assistant reply of another thread, capped at 20,000 characters.
- Jev answers only boolean, choice, and score questions. It cannot summarize or write text.
- The attachment item `text` is built inside the search RPC. Search runs on every keystroke. There is no hook when the user selects an item.
- A Jev call can take up to 45 seconds and costs money. The search RPC must never wait for Jev.
- Snapshots are cached per `thread.id@lastActivityAt` in [plugins/thread-context-attach/server/threads.ts](plugins/thread-context-attach/server/threads.ts). Jev results use the same key.
- The Paseo server bundler cannot import the AI SDK directly. Call Jev from a standalone Node worker, like [plugins/next-prompt-actions/server/jev-worker.mjs](plugins/next-prompt-actions/server/jev-worker.mjs).
- The worker finds the `jev-evaluator` plugin path and the Gateway key in the daemon `config.json`. The key never enters plugin code, prompts, or logs.
- Run lint as `S=lint; npm run $S`. A local hook rewrites a literal `npm run lint`.

## Decisions

- Jev support is opt-in. The setting `jevEnabled` defaults to `false`.
- With the setting off, or when Jev fails or times out, output is exactly the current format.
- One Jev call per thread and activity key. It answers two questions: status and reply depth.
- Status choices: `done`, `blocked`, `in-progress`, `waiting-on-user`.
- Walk-back depth is 1 to 3 assistant replies. There is no setting for the depth.
- Jev sees only the thread title and the last 3 assistant replies. Each reply is cut to its last 4,000 characters for the evaluation. User messages and tool calls are never sent.
- Search returns the plain snapshot at once and starts enrichment in the background. The next search shows the enriched snapshot.
- At most 2 Jev workers run at once. A failed key is not retried until the thread activity changes.

## Open Decisions

- None.

## References

- [plugins/thread-context-attach/server/snapshot.ts](plugins/thread-context-attach/server/snapshot.ts)
- [plugins/thread-context-attach/server/threads.ts](plugins/thread-context-attach/server/threads.ts)
- [plugins/next-prompt-actions/server/jev.ts](plugins/next-prompt-actions/server/jev.ts)
- [plugins/thread-janitor/shared/settings.ts](plugins/thread-janitor/shared/settings.ts) for the `defineSettings` pattern.
- [plugins/board/client/orb-settings.tsx](plugins/board/client/orb-settings.tsx) for a settings screen.
