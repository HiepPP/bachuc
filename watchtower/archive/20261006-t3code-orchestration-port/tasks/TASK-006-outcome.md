# TASK-006 Outcome

## Outcome

Status: DONE

Changed:

- Plugin SDK: `addAssistantSelectionAction` and `addComposerAttachment` on `PluginClientContext`, with `PluginAssistantSelection`, `PluginAssistantSelectionActionContribution`, and `PluginComposerAttachmentInput` in [packages/plugin/src/client/contracts.ts](packages/plugin/src/client/contracts.ts).
- Registration in [packages/app/src/plugins/evaluate.ts](packages/app/src/plugins/evaluate.ts): selection actions are collected like composer interceptors. Chip input is validated with `PluginAttachmentItemSchema` and non-empty `agentId`, `sourceId`, `sourceTitle`, and `icon`.
- Chip path: [packages/app/src/plugins/client-runtime.ts](packages/app/src/plugins/client-runtime.ts) -> [packages/app/src/plugins/composer/index.ts](packages/app/src/plugins/composer/index.ts) -> draft-store `attachPluginResource` in [packages/app/src/stores/draft-store/index.ts](packages/app/src/stores/draft-store/index.ts).
- Comment: `commentable` and `comment` on the chip schema, `upsertPluginResourceAttachment`, and the submit text in [packages/app/src/plugins/attachments/model.ts](packages/app/src/plugins/attachments/model.ts). The comment field is in [packages/app/src/plugins/attachments/pill.tsx](packages/app/src/plugins/attachments/pill.tsx), wired from [packages/app/src/composer/index.tsx](packages/app/src/composer/index.tsx). New label `composer.attachments.addComment` in all 9 locales.
- Toolbar: new [packages/app/src/assistant-selection-copy/selection-actions.web.tsx](packages/app/src/assistant-selection-copy/selection-actions.web.tsx) and a native stub `selection-actions.tsx`, mounted in [packages/app/src/agent-stream/view.tsx](packages/app/src/agent-stream/view.tsx) inside `AssistantSelectionCopySurface`.
- Docs: [docs/plugins.md](docs/plugins.md) and [public-docs/plugins/reference.md](public-docs/plugins/reference.md).

Contract:

- Spec deviations, accepted by the reviewer: actions have no `icon` (a later optional field is safe). The chip API also takes `sourceTitle` and `icon`, because no attachment source is registered. The toolbar mounts in `view.tsx`, because `surface.web.tsx` has no `serverId` or `agentId`.
- `onSelect` gets the selection as Markdown (`createAssistantSelectionClipboardContent`).
- A re-added chip keeps the user's comment. A comment reaches the agent only for a `commentable` chip, as `<item text>\n\nComment: <comment>`. A chip without a comment sends `item.text`, as before.
- The toolbar portals into the overlay root at the floating layer, so modals, toasts, and tooltips paint above it. Scroll and resize only move it; it hides when the selection leaves the pane.

Review (reviewer agent, claude-opus-5-5 xhigh): 1 high, 2 medium, 3 low. All fixed:

- High: invalid chip input broke the draft schema and could empty the draft on the next load. Fixed by validation in `evaluate.ts`, with a test.
- Medium: a re-added chip lost its comment while the field still showed it. Fixed in upsert, with a test.
- Medium: the toolbar portaled into `document.body`. Fixed with `getOverlayRoot()` and `useOverlayLayer("floating")`.
- Low: Markdown conversion on every scroll, a stuck toolbar after scroll-away, and a docs example id. Fixed.
- Not fixed (speculation): streaming can replace the selected nodes without a `selectionchange`, so the toolbar can hold stale text.

Verified:

- From `packages/app`: `npx vitest run src/plugins/attachments/model.test.ts src/plugins/evaluate.test.ts --bail=1` -> 50 passed.
- From `packages/app`: `npx vitest run --project browser src/assistant-selection-copy/content.browser.test.ts --bail=1` -> 40 passed.
- `npm run typecheck` -> exit 0. `npm run lint` on the changed files -> 0 warnings, 0 errors.
- GitNexus detect-changes: risk high, 12 flows. The flows come from `dispatchComposerKeyboardAction` line shifts in `composer/index.tsx`; that function is not edited.
- Browser check (select assistant text with a test action, the action adds a chip, the chip takes a comment) -> UNVERIFIED (autonomous run). The live daemon on port 6768 was stopped.

Anti-goal:

- Before changes: `git diff --stat -- packages/app/src/assistant-selection-copy/surface.tsx` -> empty.
- After the toolbar change: empty; 2026-10-07 00:50.
- Final: empty; 2026-10-07 01:03.
- Result: PASS.

Lessons:

- A chip that fails the draft schema empties the whole draft on hydrate. Validate plugin input before it reaches the draft store.
- `EditingTextInput` reads `initialValue` only on mount. Code that replaces the value from outside must keep the user's text.
- App tests that load Expo need `packages/app` as the working directory; from the repo root they fail with `__DEV__ is not defined`.
- `npm run lint ... | tail` can be rewritten by the shell proxy into a broken ESLint call. Redirect lint output to a file.
