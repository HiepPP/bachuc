# TASK-005 Composer API for plugins

Group: B (client plugin host files)
Class: risky

## Brief

Goal: Let plugins set a composer's text and change or cancel a message before it is sent or queued.

Change: plugins find the composer through DOM selectors -> `client.setComposerText` and `client.addComposerInterceptor`.

Boundaries:

- Composer target: `{ serverId, workspaceId: string | null, agentId: string | null, draftId: string | null }`. The Composer gets an explicit `composerTarget` prop from its 4 callers.
- `addComposerInterceptor({ id, intercept(input) })`. `input` is `{ target, text, action: "send" | "queue", signal }`. It returns `{ text }`, `{ cancel: true }`, or nothing.
- Interceptors run in plugin order after slash-command handling. The input stays locked while they run.
- An interceptor error keeps the draft and shows the error. The message is not sent.
- `setComposerText({ serverId, agentId, text })` replaces the text of a mounted composer and focuses it. Without a mounted composer it writes the stored draft.
- Queued messages are transformed when queued. The drain path does not run interceptors.

How:

- Add the types to [packages/plugin/src/client/contracts.ts](packages/plugin/src/client/contracts.ts).
- Add a small composer registry store in [packages/app/src/plugins](packages/app/src/plugins). Mounted composers register `getText`, `setText`, and `focus`.
- Call interceptors in `handleSubmit` and `handleQueue` in [packages/app/src/composer/index.tsx](packages/app/src/composer/index.tsx).
- Collect interceptors in [packages/app/src/plugins/evaluate.ts](packages/app/src/plugins/evaluate.ts).
- Add unit tests for the interceptor chain and the registry.

Files:

- [packages/plugin/src/client/contracts.ts](packages/plugin/src/client/contracts.ts) (types)
- [packages/plugin/src/client/index.ts](packages/plugin/src/client/index.ts) (exports)
- [packages/app/src/plugins/evaluate.ts](packages/app/src/plugins/evaluate.ts) (registration)
- [packages/app/src/plugins/types.ts](packages/app/src/plugins/types.ts) (collected field)
- [packages/app/src/plugins/registry.ts](packages/app/src/plugins/registry.ts) (default)
- [packages/app/src/composer/index.tsx](packages/app/src/composer/index.tsx) (target, submit, queue)
- Composer callers: [packages/app/src/panels/agent-panel.tsx](packages/app/src/panels/agent-panel.tsx), [packages/app/src/composer/draft/workspace-tab.tsx](packages/app/src/composer/draft/workspace-tab.tsx), [packages/app/src/screens/new-workspace-screen.tsx](packages/app/src/screens/new-workspace-screen.tsx), [packages/app/src/components/workspace-setup-dialog.tsx](packages/app/src/components/workspace-setup-dialog.tsx)
- A new composer registry module and its test under [packages/app/src/plugins](packages/app/src/plugins)
- Test fixtures that build `InstalledPlugin` literals

Expected result:

- An interceptor that returns `{ text }` changes the sent or queued text.
- `{ cancel: true }` keeps the draft and sends nothing.
- `setComposerText` fills the visible composer of that agent.

Anti-goal: Plain sends stay unchanged; tripwire; limit existing composer submit tests pass unchanged; read before changes and at completion.

## Verify

- `npx vitest run <new composer registry test> --bail=1` -> pass.
- `npx vitest run packages/app/src/composer --bail=1` -> the existing composer tests pass.
- `npm run typecheck` -> exit 0.
- Anti-goal: the pre-change composer tests pass without edits.
