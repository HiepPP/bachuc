# TASK-123 Prefetch host API

Group: D
Class: code

## Brief

Goal: Let a plugin warm a thread before it opens it, so the open paints at once.

Change: `prefetchAgent({ serverId, agentId, anchor })` prepares the route, the replica cache, and a tail fetch. It skips desired agents and agents prefetched in the last 30 s, runs one prefetch at a time per host, and keeps the newest 3 queued. Errors never show UI.

Boundaries:

- A host feature per [ADR-0017](watchtower/decisions/ADR-0017-native-host-features-over-thin-host-changes.md); the plugin only calls it.
- A later replica `prepare` must not overwrite a newer prefetched page (design risk 7).

Files:

- [packages/app/src/timeline/agent-prefetch.ts](packages/app/src/timeline/agent-prefetch.ts) (new)
- [packages/app/src/timeline/agent-prefetch.test.ts](packages/app/src/timeline/agent-prefetch.test.ts) (new)
- [packages/app/src/plugins/host-navigation-model.ts](packages/app/src/plugins/host-navigation-model.ts)
- [packages/app/src/plugins/host-navigation.ts](packages/app/src/plugins/host-navigation.ts)
- [packages/app/src/plugins/host-navigation.test.ts](packages/app/src/plugins/host-navigation.test.ts)

Expected result:

- Fast hover over many cards starts at most one fetch per host at a time.

## Verify

- `npx vitest run src/timeline/agent-prefetch.test.ts --bail=1` in `packages/app` -> pass.
- `npx vitest run src/plugins/host-navigation.test.ts --bail=1` -> pass.
- `npm run typecheck` -> exit 0.
