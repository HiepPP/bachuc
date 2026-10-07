# TASK-004 App uses the daemon queue

Group: B (shared files with TASK-006 and TASK-008: [packages/app/src/composer/index.tsx](packages/app/src/composer/index.tsx) and the locale files under `packages/app/src/i18n/resources/`)
Class: risky

## Brief

Goal: When the host has `agentMessageQueue`, the app reads the queue from the agent snapshot and changes it through the queue RPCs. The queued list gets Cancel, move up, move down, and Resume.

Change: client `queuedMessages` map plus client drain -> a list read from the snapshot, written through RPCs, with no client drain on gated hosts.

Design: the daemon side is TASK-003. T3 Code UI reference: `apps/web/src/components/chat/QueuedRunsControl.tsx:50`.

Boundaries:

- Feature contract: check `server_info.features.agentMessageQueue` once. A host without it keeps the current client queue. Tag that branch with `COMPAT(agentMessageQueue)`.
- Drop old client-only queue items on upgrade. They lived in memory only.
- A send to an idle agent does not touch the queue.
- Edit keeps today's behavior: it removes the item and loads it into the composer. Images come back as file attachments (TASK-003).
- Follow [docs/design.md](docs/design.md) for buttons and density. Follow [docs/hover.md](docs/hover.md) for row actions.

How:

- Run GitNexus `impact` on `queueComposerMessage`, `drainQueuedAgentMessage`, and `QueuedMessageRow`.
- Read the queue from the agent snapshot in [packages/app/src/stores/session-store.ts](packages/app/src/stores/session-store.ts) when the host is gated.
- Route queue writes in [packages/app/src/composer/actions.ts](packages/app/src/composer/actions.ts) and [packages/app/src/composer/submit.ts](packages/app/src/composer/submit.ts) to the RPCs.
- Add Cancel, move up, move down, and a held-queue Resume control in [packages/app/src/composer/index.tsx](packages/app/src/composer/index.tsx).
- Skip `drainQueuedAgentMessage` in [packages/app/src/runtime/host-runtime.ts](packages/app/src/runtime/host-runtime.ts) on gated hosts.
- Add new strings to every locale file.

Files:

- [packages/app/src/stores/session-store.ts](packages/app/src/stores/session-store.ts) (snapshot-backed queue)
- [packages/app/src/composer/actions.ts](packages/app/src/composer/actions.ts) (RPC writes)
- [packages/app/src/composer/submit.ts](packages/app/src/composer/submit.ts) (queue routing)
- [packages/app/src/composer/index.tsx](packages/app/src/composer/index.tsx) (row actions, Resume)
- [packages/app/src/runtime/host-runtime.ts](packages/app/src/runtime/host-runtime.ts) (skip client drain)
- [packages/app/src/composer/actions.test.ts](packages/app/src/composer/actions.test.ts), [packages/app/src/composer/submit.test.ts](packages/app/src/composer/submit.test.ts), [packages/app/src/runtime/host-runtime.test.ts](packages/app/src/runtime/host-runtime.test.ts) (tests)
- `packages/app/src/i18n/resources/*.ts` (new labels)

Expected result:

- Gated host: a message queued while the agent runs shows on a second client too.
- Gated host: closing and reopening the app keeps the queue. After the run ends, the daemon sends the item.
- Cancel removes the item. Move up and move down change the order on every client.
- Stop -> the list shows a held state with Resume.
- Host without the flag -> the current client queue works as before.

Anti-goal: A send to an idle agent makes 0 queue RPC calls; tripwire; read from a `submit.test.ts` case before changes, after the routing change, and at completion.

## Verify

- `npx vitest run packages/app/src/composer/actions.test.ts --bail=1` -> pass.
- `npx vitest run packages/app/src/composer/submit.test.ts --bail=1` -> pass, including the idle-send case (anti-goal read).
- `npx vitest run packages/app/src/runtime/host-runtime.test.ts --bail=1` -> pass, including the gated no-drain case.
- `npm run typecheck` -> exit 0.
- `npm run lint` on every changed file -> exit 0.
- Manual live check (needs a human, the live daemon, and two clients): queue a message in Electron while the agent runs -> it shows in the web client, survives an app reload, and sends when the run ends.
