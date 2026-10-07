# TASK-010 Host API to reveal a quoted passage

Group: B (shared files with TASK-006: [packages/plugin/src/client/contracts.ts](packages/plugin/src/client/contracts.ts), [packages/app/src/plugins/client-runtime.ts](packages/app/src/plugins/client-runtime.ts), [docs/plugins.md](docs/plugins.md), [public-docs/plugins/reference.md](public-docs/plugins/reference.md))
Class: risky

## Brief

Goal: A plugin can open an agent's timeline at a message and highlight a passage in it. A press on a plugin chip can call the plugin, so a quote chip can jump back to its source.

Change: no way for a plugin to scroll the timeline or react to a chip press -> `revealTimelinePassage` and an optional chip `onOpen` handler.

Design: T3 Code jumps back with a highlight after it normalizes whitespace and needs an unambiguous match (`packages/shared/src/assistantCitations.ts:101`).

Boundaries:

- Q-007 answered: a click on the quote chip jumps back to the source passage.
- API: `client.revealTimelinePassage({ serverId, agentId, messageId, text? })`. It opens the agent's tab when needed and scrolls to the message. On web and Electron, it also highlights `text` inside the message when the match is unique after whitespace is normalized. On native, it scrolls to the message only.
- If the message is not loaded or no longer exists, show a short toast and do nothing else.
- Chip press: `addAttachmentSource` gets an optional `onOpen(item)` handler. A press on a `plugin_resource` chip from that source calls it. With no handler, the chip keeps today's behavior.
- The highlight fades after a short time and never changes the stored message.
- Rows carry `data-message-id` on web ([packages/app/src/agent-stream/strategy-web.tsx](packages/app/src/agent-stream/strategy-web.tsx), line 1261). Read [docs/agent-stream-performance.md](docs/agent-stream-performance.md) before you touch the stream view.
- Keep the host thin. No cite logic in the host.

How:

- Run GitNexus `impact` on the agent stream scroll API and `addAttachmentSource`.
- Find how the stream view scrolls to a message today, for example for rewind or fork. Reuse it.
- Add the API types to `contracts.ts`. Wire them in `client-runtime.ts`.
- Add the web highlight in a `.web.tsx` file. Keep native to scroll only.
- Call `onOpen` from the plugin chip press.
- Add tests. Document both APIs in [docs/plugins.md](docs/plugins.md) and [public-docs/plugins/reference.md](public-docs/plugins/reference.md).

Files:

- [packages/plugin/src/client/contracts.ts](packages/plugin/src/client/contracts.ts) (API types)
- [packages/app/src/plugins/client-runtime.ts](packages/app/src/plugins/client-runtime.ts) (wiring)
- The agent stream scroll and highlight code under `packages/app/src/agent-stream/` (scroll to message, web highlight)
- The plugin chip press handler, next to the chip UI that TASK-006 touches
- Tests next to the changed modules, in existing test files where they exist
- [docs/plugins.md](docs/plugins.md), [public-docs/plugins/reference.md](public-docs/plugins/reference.md) (docs)

Expected result:

- A plugin calls `revealTimelinePassage` with a loaded message id -> the timeline scrolls to it. On web, the passage is highlighted.
- The same text appears twice in the message -> the timeline scrolls to the message, with no highlight.
- An unknown message id -> a toast, and no scroll.
- A press on a chip whose source has `onOpen` -> the handler gets the chip item.
- A press on a chip with no `onOpen` -> today's behavior.

Anti-goal: Opening an agent timeline with no reveal request does the same work as before: 0 new effects or subscriptions per rendered row; tripwire; read from a unit test or render count check on the stream row before changes, after the reveal change, and at completion.

## Verify

- `npx vitest run` on each changed or added test file, with `--bail=1` -> pass, including the unique-match and duplicate-text cases.
- `npm run typecheck` -> exit 0.
- `npm run lint` on every changed file -> exit 0.
- Anti-goal read: the row render test shows no new per-row effect or subscription.
- Browser check (verifier or human, needs live Electron or web): a test plugin calls `revealTimelinePassage` on an older message -> the timeline scrolls there and highlights the passage.
