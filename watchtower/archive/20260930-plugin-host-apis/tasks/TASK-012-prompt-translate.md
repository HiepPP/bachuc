# TASK-012 P2 prompt-translate uses composer, timeline, and prompt APIs

Group: F (hiep-plugins/plugins/prompt-translate)
Class: risky

## Brief

Goal: Keep translation, English rewrite, and Caveman modes with no DOM code and no native provider hook for Paseo agents.

Change: DOM bubble, composer, and mode menu plus native hooks -> a `user_message` transformer, a composer interceptor with a pill toggle, a mode pill, and a `before("agent.prompt")` hook.

Boundaries:

- The bubble transformer keeps the source with `source: { placement: "first" }` and adds a plugin item. That item fetches the translation or the original through the existing RPCs.
- Rewrite to English runs in `addComposerInterceptor` when the per-agent toggle pill is on and the draft is Vietnamese. It replaces Cmd+Enter.
- The Caveman mode pill is a shared composer pill with a popover.
- The prompt hook adds the Caveman and reply-language context in the daemon, using the existing Caveman parser code in-process.
- Store state under `plugin-data/prompt-translate/v2/`, so the old native hook finds no state for Paseo Dev agents. Set `PROMPT_TRANSLATE_CAVEMAN_DEFAULT_MODE` so the old hook stays silent.
- Keep [hiep-plugins/plugins/prompt-translate/scripts/install-hooks.mjs](hiep-plugins/plugins/prompt-translate/scripts/install-hooks.mjs) and the hook file for sessions outside Paseo.
- New-thread composers use the host default mode.
- Raise `requirements.paseo` to `>=0.10.2-beta.900`.

How:

- Remove [hiep-plugins/plugins/prompt-translate/client/dom.ts](hiep-plugins/plugins/prompt-translate/client/dom.ts), [hiep-plugins/plugins/prompt-translate/client/bubble.ts](hiep-plugins/plugins/prompt-translate/client/bubble.ts), [hiep-plugins/plugins/prompt-translate/client/composer.ts](hiep-plugins/plugins/prompt-translate/client/composer.ts), and [hiep-plugins/plugins/prompt-translate/client/mode-menu.ts](hiep-plugins/plugins/prompt-translate/client/mode-menu.ts).
- Add the transformer, renderer, interceptor, and pills.
- Move the hook context builder into a server module called from `before("agent.prompt")`.
- Update tests and the README.

Files:

- [hiep-plugins/plugins/prompt-translate/index.client.tsx](hiep-plugins/plugins/prompt-translate/index.client.tsx) (contributions)
- [hiep-plugins/plugins/prompt-translate/index.server.ts](hiep-plugins/plugins/prompt-translate/index.server.ts) (prompt hook)
- Files under [hiep-plugins/plugins/prompt-translate/client](hiep-plugins/plugins/prompt-translate/client), [hiep-plugins/plugins/prompt-translate/server](hiep-plugins/plugins/prompt-translate/server), [hiep-plugins/plugins/prompt-translate/shared](hiep-plugins/plugins/prompt-translate/shared), and [hiep-plugins/plugins/prompt-translate/tests](hiep-plugins/plugins/prompt-translate/tests)
- [hiep-plugins/plugins/prompt-translate/paseo-plugin.json](hiep-plugins/plugins/prompt-translate/paseo-plugin.json), [hiep-plugins/plugins/prompt-translate/package.json](hiep-plugins/plugins/prompt-translate/package.json), [hiep-plugins/plugins/prompt-translate/README.md](hiep-plugins/plugins/prompt-translate/README.md)

Expected result:

- A Vietnamese user bubble shows an `EN` line under it on desktop and mobile.
- With the toggle on, a Vietnamese draft is sent as English, and the bubble shows the Vietnamese original.
- A selected Caveman mode changes the reply style.

Anti-goal: Translation cost stays bounded; tripwire; limit at most one translate RPC per distinct user message text per session, read from a renderer test at completion.

## Verify

- `rg -n "querySelector|MutationObserver|document\." hiep-plugins/plugins/prompt-translate/client hiep-plugins/plugins/prompt-translate/index.client.tsx` -> no match.
- `cd hiep-plugins/plugins/prompt-translate && npm run typecheck && npm run lint && npm test` -> exit 0.
- Real host check in Paseo Dev (TASK-017): EN line, rewrite toggle, and a Caveman mode all work.
- Anti-goal: the renderer test counts one translate call for repeated renders of one message.
