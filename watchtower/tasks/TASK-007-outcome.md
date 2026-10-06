# TASK-007 Outcome

## Outcome

Status: DONE

Changed:

- New client-only plugin `hiep-plugins/plugins/assistant-cite/`: [index.client.tsx](hiep-plugins/plugins/assistant-cite/index.client.tsx) registers the Cite selection action and a chip-only `quote` attachment source whose `onOpen` calls `revealTimelinePassage`.
- Pure modules: [shared/format.ts](hiep-plugins/plugins/assistant-cite/shared/format.ts) (lead line, blockquote, 4000-character cap, reverse parse, highlight line) and [shared/item.ts](hiep-plugins/plugins/assistant-cite/shared/item.ts) (chip item with its source, and the passage it opens).
- Tests in `hiep-plugins/plugins/assistant-cite/tests/`: format and item.
- [hiep-plugins/plugins/assistant-cite/README.md](hiep-plugins/plugins/assistant-cite/README.md) and a catalog row in [hiep-plugins/README.md](hiep-plugins/README.md).

Contract:

- Q-007 answered: a chip with an optional comment, and a press jumps back to the source.
- The agent text is `Quoted from your earlier reply:`, a blank line, and the quote as a blockquote. The host appends `Comment: <comment>` (TASK-006).
- The chip keeps its source in `item.url` (`paseo-cite://quote?server=…&agent=…&message=…`) and the quote in `item.text`, so a chip opens its source after a reload and on another client.
- The chip id is the message id plus a hash of the quote, so citing the same passage again replaces the chip and keeps the comment.
- The jump back highlights the first line of the quote with Markdown syntax removed, because the host matches rendered text inside one block (TASK-010).
- A selection across messages uses the first message id; the host reads it from the selection start.
- Q-011 DEFAULTED: plugins have no toast API, so an over-long selection adds no chip and logs a warning.
- Q-010 DEFAULTED: on native, a chip press opens the agent without scrolling.

Verified:

- In `hiep-plugins/plugins/assistant-cite`: `npm test` -> 13 passed. `npm run typecheck` -> exit 0. `npm run lint` -> 0 warnings, 0 errors. `npm run format` -> done; root `npm run format:check:files` on the plugin files -> exit 0.
- The chip item passes the host `PluginAttachmentItemSchema`, including the `paseo-cite://` URL (test "builds a chip item the host accepts").
- Live install (`npm run cli -- plugin install ...`, then `plugin ls`) -> UNVERIFIED (autonomous run). The live daemon on port 6768 was stopped.
- Browser check (select assistant text, click Cite, add a comment, press the chip) -> UNVERIFIED (autonomous run).

Anti-goal:

- Final: test "caps the quote body at the limit" passed: a 4000-character quote is accepted with its body at 4000 characters or fewer, and 4001 characters returns `too_long`; 2026-10-07 01:52.
- Result: PASS.

Lessons:

- A plugin that uses fork-only SDK APIs needs the `file:` links and the `zod` path mapping from pr-watch; a client entry also needs `"jsx"` and the `DOM` lib in `tsconfig.json`.
- Plugins have no toast or notice API on `PluginClientContext`; a callback outside React cannot tell the user anything.
