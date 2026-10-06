# TASK-004 Outcome

## Outcome

Status: DONE

Changed:

- [packages/app/src/composer/daemon-queue.ts](packages/app/src/composer/daemon-queue.ts) (new): pure helpers for rows from the snapshot summary, edit eligibility, the move target, enqueue, and take back through `cancel`.
- [packages/app/src/composer/index.tsx](packages/app/src/composer/index.tsx): on a host with `agentMessageQueue`, the queued list comes from the agent snapshot and every write goes to the daemon RPCs. The list adds Cancel, move up, move down, and a "Queue paused after Stop" line with Resume. A host without the flag keeps the client queue.
- [packages/app/src/stores/session-store.ts](packages/app/src/stores/session-store.ts) and [packages/app/src/utils/agent-snapshots.ts](packages/app/src/utils/agent-snapshots.ts): `Agent.queue` is carried from the snapshot.
- [packages/app/src/runtime/host-runtime.ts](packages/app/src/runtime/host-runtime.ts): the client drain skips a host with the flag.
- Labels in all 9 locale files under `packages/app/src/i18n/resources/`.
- Tests: [packages/app/src/composer/daemon-queue.test.ts](packages/app/src/composer/daemon-queue.test.ts) (new) and a gated no-drain case in [packages/app/src/runtime/host-runtime.test.ts](packages/app/src/runtime/host-runtime.test.ts).

Contract:

- The gate is read once in the composer (`useHostFeature(serverId, "agentMessageQueue")`) and once in the drain. Both are tagged `COMPAT(agentMessageQueue)`.
- A queued message clears the composer at once, as a send does. If the enqueue fails, the text and attachments come back, unless the user already typed a new message. Images stay retained against the draft GC until they are encoded.
- Edit removes the item on the daemon and loads its text and images. One image that fails to save does not lose the text or the other images; the count shows as an error. Edit is hidden for items with non-image attachments, because those do not round-trip.
- Spec deviation: there is no separate client queue migration. Old client-only items were memory only.
- Known nit, not changed: a row can show up to 5 icon buttons, and move buttons hide at the ends.

Verified:

- From `packages/app`: `npx vitest run src/composer/daemon-queue.test.ts` -> 6 passed. `src/composer/submit.test.ts` -> 6 passed. `src/composer/actions.test.ts` -> 46 passed. `src/runtime/host-runtime.test.ts` -> 82 passed, with "leaves the queue to a daemon that drains its own queue". `src/utils/agent-snapshots.test.ts` -> 5 passed.
- `npm run typecheck`, `npm run lint`, and `npm run format:check:files` -> exit 0.
- GitNexus impact: `normalizeAgentSnapshot` CRITICAL (7 direct callers); the change only copies the optional `queue`. `renderQueueTrack` and `QueuedMessageRow` LOW.
- Reviewer agent -> `VERDICT: ISSUES`. Fixed: a failed enqueue lost the message and an image could be collected before encode (blocker), a double press could queue twice, a failed image save lost the whole Edit, and the drain was not gated.
- Manual live check with two clients -> UNVERIFIED (autonomous run). The live daemon on port 6768 was stopped, and the run does not start it.

Anti-goal:

- Before changes: `submit.test.ts` 6 passed; its idle-send cases assert that `queueMessage` is not called; 2026-10-07 00:04.
- After the routing change: 6 passed; 00:10.
- Final: 6 passed; 2026-10-07 00:20.
- Result: PASS against the tripwire, through a proxy. On a gated host the daemon enqueue happens only inside `queueMessage`, so an idle send makes 0 queue RPC calls.

Lessons:

- App tests that load Expo modules must run from `packages/app`. From the repo root they fail with `__DEV__ is not defined`.
- A cleared draft schedules a GC run. Retain image ids with `retainAttachmentForGarbageCollection` while an async step still reads them.
