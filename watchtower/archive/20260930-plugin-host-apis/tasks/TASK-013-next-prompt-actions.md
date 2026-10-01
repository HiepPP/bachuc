# TASK-013 P1 next-prompt-actions renders its panel through the timeline API

Group: G (hiep-plugins/plugins/next-prompt-actions)
Class: risky

## Brief

Goal: Show the Recap and What Next panel with Edit and Send on every client, with no DOM code.

Change: a 989-line DOM adapter -> an `assistant_message` transformer that keeps the text before the Recap and adds a plugin panel item.

Boundaries:

- The transformer runs on the `complete` phase. It cuts the Recap, What Next heading, and suggestion fence from `source.text` and adds one `next-prompt-panel` plugin item.
- The panel is a React Native component. It keeps single and multi-select, exclusive groups, allowed combinations, Git actions, `why`, and `thread: new`.
- Send, Start in new thread, and auto-run keep their server RPCs and guards.
- Edit uses `client.setComposerText`.
- Back to Board after Send uses `client.openSurface("board", { pluginId: "board" })`, not DOM events.
- The Cmd hover swap is removed. New thread gets its own button.
- Raise `requirements.paseo` to `>=0.10.2-beta.900`.

How:

- Keep parsing in [hiep-plugins/plugins/next-prompt-actions/shared](hiep-plugins/plugins/next-prompt-actions/shared) and [hiep-plugins/plugins/next-prompt-actions/client/recap.ts](hiep-plugins/plugins/next-prompt-actions/client/recap.ts) where it has no DOM.
- Replace [hiep-plugins/plugins/next-prompt-actions/client/web.ts](hiep-plugins/plugins/next-prompt-actions/client/web.ts) with a transformer and a panel component.
- Replace DOM tests with transformer and selection tests.

Files:

- [hiep-plugins/plugins/next-prompt-actions/index.client.tsx](hiep-plugins/plugins/next-prompt-actions/index.client.tsx) (contributions)
- Files under [hiep-plugins/plugins/next-prompt-actions/client](hiep-plugins/plugins/next-prompt-actions/client), [hiep-plugins/plugins/next-prompt-actions/shared](hiep-plugins/plugins/next-prompt-actions/shared), and [hiep-plugins/plugins/next-prompt-actions/tests](hiep-plugins/plugins/next-prompt-actions/tests)
- [hiep-plugins/plugins/next-prompt-actions/paseo-plugin.json](hiep-plugins/plugins/next-prompt-actions/paseo-plugin.json), [hiep-plugins/plugins/next-prompt-actions/package.json](hiep-plugins/plugins/next-prompt-actions/package.json), [hiep-plugins/plugins/next-prompt-actions/README.md](hiep-plugins/plugins/next-prompt-actions/README.md)

Expected result:

- A reply ending in a What Next block shows the panel with working Edit and Send on desktop and mobile.
- Earlier replies show the panel without buttons.
- The raw fence is not shown.

Anti-goal: Suggestion parsing stays the same; tripwire; limit [hiep-plugins/plugins/next-prompt-actions/tests/next-prompts.test.ts](hiep-plugins/plugins/next-prompt-actions/tests/next-prompts.test.ts), [hiep-plugins/plugins/next-prompt-actions/tests/prompts.test.ts](hiep-plugins/plugins/next-prompt-actions/tests/prompts.test.ts), and [hiep-plugins/plugins/next-prompt-actions/tests/engine.test.ts](hiep-plugins/plugins/next-prompt-actions/tests/engine.test.ts) pass unchanged; read before changes and at completion.

## Verify

- `rg -n "querySelector|MutationObserver|document\." hiep-plugins/plugins/next-prompt-actions/client hiep-plugins/plugins/next-prompt-actions/index.client.tsx` -> no match.
- `cd hiep-plugins/plugins/next-prompt-actions && npm run typecheck && npm run lint && npm test` -> exit 0.
- Real host check in Paseo Dev (TASK-017): Edit fills the composer, Send sends the prompt.
- Anti-goal: the three parsing and engine test files pass without edits.
