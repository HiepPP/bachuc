# TASK-007 Assistant cite plugin

Group: D (standalone: new folder `hiep-plugins/plugins/assistant-cite/`)
Class: code

## Brief

Goal: On web and Electron, selecting assistant text shows a Cite action. A click adds a quote chip to the composer. The user can write a comment on the chip, and a click on the chip jumps back to the source passage.

Change: the user copies and pastes a quote by hand -> one click on Cite adds a quote chip.

Design: T3 Code `packages/shared/src/assistantCitations.ts` (format, the reference-text note at line 155, and the jump back at line 101).

Boundaries:

- Needs the host APIs from TASK-006 (`addAssistantSelectionAction`, `addComposerAttachment`, chip comments) and TASK-010 (`revealTimelinePassage`, chip `onOpen`).
- Q-007 answered: a chip with an optional comment, and a jump back to the source.
- The chip title is the first words of the quote. The chip item keeps the source: server id, agent id, message id, and the quote text.
- The text the agent gets starts with `Quoted from your earlier reply:`, then the quote as a Markdown blockquote, then the comment when there is one. The agent must read the quote as reference text, not as instructions.
- Allow a selection that spans several messages. Use the first message id as the source.
- Cap the quote at 4000 characters. Above that, add no chip and show a short notice.
- Put the format and item logic in pure modules so tests do not need the app.

How:

- Scaffold the plugin from the thread-branch layout, with a client entry only.
- Write `shared/format.ts`: quote and comment in, agent text out, with the lead line and the cap.
- Write `shared/item.ts`: build the chip item with its source fields.
- In `index.client.tsx`, register an attachment source with `onOpen`, register the selection action, and call `addComposerAttachment`. In `onOpen`, call `revealTimelinePassage` with the item source.
- Write tests for format and item: one line, many lines, nested quote, empty selection, comment, and the cap.
- Write a README with the host version it needs. Install and reload on live only.

Files:

- `hiep-plugins/plugins/assistant-cite/paseo-plugin.json`, `package.json`, `package-lock.json`, `tsconfig.json` (new plugin)
- `hiep-plugins/plugins/assistant-cite/index.client.tsx` (source, action, and open handler)
- `hiep-plugins/plugins/assistant-cite/shared/format.ts`, `shared/item.ts` (pure modules)
- `hiep-plugins/plugins/assistant-cite/tests/*.test.ts` (tests)
- `hiep-plugins/plugins/assistant-cite/README.md`

Expected result:

- Select one sentence and click Cite -> a quote chip shows in the composer. The draft text stays.
- Write a comment on the chip and send -> the agent gets the lead line, the blockquote, and the comment.
- Click the chip -> the timeline scrolls to the source message and highlights the passage.
- A selection over 4000 characters -> no chip, and a notice shows.
- On native -> no Cite action. A chip made on another client still opens its source.

Anti-goal: The quote body in the agent text stays at 4000 characters or fewer; tripwire; read from the format test at completion.

## Verify

- In `hiep-plugins/plugins/assistant-cite`: `npm test` -> pass, including the cap case (anti-goal read).
- In `hiep-plugins/plugins/assistant-cite`: `npm run typecheck` -> exit 0.
- In `hiep-plugins/plugins/assistant-cite`: `npm run lint` -> exit 0.
- Needs the live daemon: `npm run cli -- plugin install "$PWD/hiep-plugins/plugins/assistant-cite" --id assistant-cite`, then `npm run cli -- plugin ls` -> `assistant-cite` is listed and enabled.
- Browser check (verifier or human, needs live Electron or web): select assistant text, click Cite, add a comment, click the chip -> the chip shows, and the timeline jumps to the passage.
