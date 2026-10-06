# TASK-006 Host API for assistant selection actions and plugin chips

Group: B (shared files with TASK-004, TASK-008, and TASK-010: [packages/app/src/composer/index.tsx](packages/app/src/composer/index.tsx), [packages/plugin/src/client/contracts.ts](packages/plugin/src/client/contracts.ts), [docs/plugins.md](docs/plugins.md), [public-docs/plugins/reference.md](public-docs/plugins/reference.md), and the locale files)
Class: risky

## Brief

Goal: A plugin can add an action to a small toolbar that shows over selected assistant text on web and Electron. A plugin can also add a composer chip from code, and the user can write an optional comment on that chip.

Change: plugins cannot see a selection and can add chips only through the attachment picker -> `addAssistantSelectionAction`, `addComposerAttachment`, and an optional comment on plugin chips.

Design: T3 Code `apps/web/src/components/chat/AssistantSelectionToolbar.tsx:21` and `packages/shared/src/composerContextReferences.ts:265`.

Boundaries:

- Q-007 answered: a composer chip with an optional comment. A click on the chip jumps back to the source; TASK-010 owns the jump. The selection toolbar is web and Electron only.
- Selection API: `client.addAssistantSelectionAction({ id, title, icon?, onSelect })`. `onSelect` gets `{ serverId, workspaceId, agentId, messageId?, text }`, where `text` is Markdown.
- Chip API: `client.addComposerAttachment({ agentId, sourceId, item })`. It adds a `plugin_resource` chip ([packages/app/src/plugins/attachments/model.ts](packages/app/src/plugins/attachments/model.ts)) from code. Reuse `PluginAttachmentItemSchema` and the existing submit path, which sends the chip as a text attachment.
- Comment: a plugin chip can declare that it takes a comment. The user edits it from the chip. Submit adds the comment to the text the agent gets. Reuse the comment editor of browser element chips (`BrowserElementAttachment.comment` in [packages/app/src/attachments/types.ts](packages/app/src/attachments/types.ts)) if it can be shared; else add the smallest generic one.
- Put the toolbar in `.web.tsx` files under `packages/app/src/assistant-selection-copy/`. Do not change the native `surface.tsx`.
- Reuse `createAssistantSelectionClipboardContent` in [packages/app/src/assistant-selection-copy/content.web.ts](packages/app/src/assistant-selection-copy/content.web.ts) (line 62) to get Markdown from the DOM selection.
- Use `selectionchange` and `getBoundingClientRect`. Render the toolbar in a portal per [docs/floating-panels.md](docs/floating-panels.md). Do not use `onPointerEnter`.
- Register the action like `addComposerInterceptor` in [packages/app/src/plugins/evaluate.ts](packages/app/src/plugins/evaluate.ts) (line 425).
- Keep the host thin. No cite logic in the host; TASK-007 owns it.

How:

- Run GitNexus `impact` on `PluginClientContext`, `PluginResourceComposerAttachmentSchema`, and `AssistantSelectionCopySurface`.
- Add the types in [packages/plugin/src/client/contracts.ts](packages/plugin/src/client/contracts.ts) and [packages/app/src/plugins/types.ts](packages/app/src/plugins/types.ts).
- Register selection actions in `evaluate.ts`. Wire `addComposerAttachment` in [packages/app/src/plugins/client-runtime.ts](packages/app/src/plugins/client-runtime.ts).
- Add the optional comment to plugin chips: schema, chip UI in [packages/app/src/composer/index.tsx](packages/app/src/composer/index.tsx) or the chip component it uses, and submit text.
- Add `packages/app/src/assistant-selection-copy/selection-actions.web.tsx` and mount it from [packages/app/src/assistant-selection-copy/surface.web.tsx](packages/app/src/assistant-selection-copy/surface.web.tsx).
- Add new strings to every locale file. Document the APIs in [docs/plugins.md](docs/plugins.md) and [public-docs/plugins/reference.md](public-docs/plugins/reference.md).

Files:

- [packages/plugin/src/client/contracts.ts](packages/plugin/src/client/contracts.ts) (API types)
- [packages/app/src/plugins/types.ts](packages/app/src/plugins/types.ts), [packages/app/src/plugins/evaluate.ts](packages/app/src/plugins/evaluate.ts), [packages/app/src/plugins/client-runtime.ts](packages/app/src/plugins/client-runtime.ts) (registration and wiring)
- [packages/app/src/plugins/attachments/model.ts](packages/app/src/plugins/attachments/model.ts) (comment field, submit text)
- [packages/app/src/composer/index.tsx](packages/app/src/composer/index.tsx) or the plugin chip component (comment editor)
- `packages/app/src/assistant-selection-copy/selection-actions.web.tsx` (new toolbar) and [packages/app/src/assistant-selection-copy/surface.web.tsx](packages/app/src/assistant-selection-copy/surface.web.tsx) (mount)
- [packages/app/src/plugins/attachments/model.test.ts](packages/app/src/plugins/attachments/model.test.ts), [packages/app/src/plugins/evaluate.test.ts](packages/app/src/plugins/evaluate.test.ts) (tests)
- `packages/app/src/i18n/resources/*.ts` (new labels)
- [docs/plugins.md](docs/plugins.md), [public-docs/plugins/reference.md](public-docs/plugins/reference.md) (docs)

Expected result:

- A plugin registers one action. Selecting text in an assistant message on web shows the toolbar with that action.
- Clicking the action calls `onSelect` with Markdown text and the agent id.
- No toolbar when no action is registered, when the selection is outside assistant messages, or on native.
- `addComposerAttachment` adds a chip to that agent's draft. The draft text stays.
- The user writes a comment on a chip that takes one. The sent text attachment holds the item text and the comment.
- A chip with no comment sends the same text as a picker chip today.
- Unloading the plugin removes its action.

Anti-goal: The native file [packages/app/src/assistant-selection-copy/surface.tsx](packages/app/src/assistant-selection-copy/surface.tsx) stays unchanged; tripwire: `git diff --stat -- packages/app/src/assistant-selection-copy/surface.tsx` shows any change; read before changes, after the toolbar change, and at completion.

## Verify

- `npx vitest run packages/app/src/plugins/attachments/model.test.ts --bail=1` -> pass, with code-added chip, comment, and no-comment cases.
- `npx vitest run packages/app/src/plugins/evaluate.test.ts --bail=1` -> pass, with register and unload cases.
- From `packages/app`: `npx vitest run --project browser src/assistant-selection-copy/content.browser.test.ts --bail=1` -> pass.
- `git diff --stat -- packages/app/src/assistant-selection-copy/surface.tsx` -> empty (anti-goal read).
- `npm run typecheck` -> exit 0.
- `npm run lint` on every changed file -> exit 0.
- Browser check (verifier or human, needs live Electron or web): with a test action registered, select assistant text -> the toolbar shows. The action adds a chip, and the chip takes a comment.
