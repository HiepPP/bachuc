# TASK-012 Outcome

## Outcome

Status: DONE

Changed:

- Removed the DOM adapters and their tests: `client/dom.ts`, `client/bubble.ts`, `client/composer.ts`, `client/mode-menu.ts`, `client/agent-mode.ts`, `tests/bubble.test.ts`, `tests/composer.test.ts`, `tests/mode-menu.test.ts`.
- [hiep-plugins/plugins/prompt-translate/index.client.tsx](hiep-plugins/plugins/prompt-translate/index.client.tsx): a `user_message` transformer that keeps the bubble and adds a `prompt-note` row, its renderer ([hiep-plugins/plugins/prompt-translate/client/note-view.tsx](hiep-plugins/plugins/prompt-translate/client/note-view.tsx), cache in [hiep-plugins/plugins/prompt-translate/client/notes.ts](hiep-plugins/plugins/prompt-translate/client/notes.ts)), a `rewrite-english` composer interceptor, and a shared `Prompt` pill ([hiep-plugins/plugins/prompt-translate/client/prompt-pill.tsx](hiep-plugins/plugins/prompt-translate/client/prompt-pill.tsx)).
- [hiep-plugins/plugins/prompt-translate/index.server.ts](hiep-plugins/plugins/prompt-translate/index.server.ts): `before("agent.prompt")` appends the Caveman and reply-language context through [hiep-plugins/plugins/prompt-translate/server/turn.ts](hiep-plugins/plugins/prompt-translate/server/turn.ts); session open seeds the new-thread mode.
- [hiep-plugins/plugins/prompt-translate/server/caveman-hook.cjs](hiep-plugins/plugins/prompt-translate/server/caveman-hook.cjs): the context builder is split into `turnContext`, with a `daemonContext` export. The CLI behavior is unchanged.
- [hiep-plugins/plugins/prompt-translate/server/modes.ts](hiep-plugins/plugins/prompt-translate/server/modes.ts): `find`, `prefs`, and a merging `update` with a per-agent `rewrite` flag; snapshot and queue methods removed. RPCs `translate.mode.read` and `write` carry `{ mode, rewrite }`.
- `service.enhance` no longer requires the `enhanceShortcut` setting, which now means the default rewrite choice.
- Requirements `>=0.10.2-beta.900`, `file:` SDK links, tsconfig `paths` for `zod` and `react`, lint with `--disable-nested-config`, README rewritten.

Contract:

- Deviation from the spec: the `Prompt` pill holds both choices; there is no separate mode pill. Cmd+Enter is gone.
- The daemon hook adds context only when `hook-runtime.json` records a Caveman install.

Verified:

- `npx tsc --noEmit` -> exit 0. `npm run lint` -> 0 errors on 32 files. `npm test` -> 41 passed.
- DOM check -> no match.
- Real host check, Paseo Dev 2026-10-01T00:38: with rewrite on, the Vietnamese draft was sent as English, the bubble showed `VI gốc` with the original, the agent answered in Vietnamese, and `last-hook.json` recorded mode `ultra`. A later `daemon restart` exposed a timeout: the Caveman scripts ran through the Electron helper without `ELECTRON_RUN_AS_NODE`. The hook now sets it and catches errors so a turn never fails on Caveman; a `paseo run` turn then passed with mode `ultra`.

Anti-goal:

- Final: `tests/notes.test.ts` shows 1 translate call for 5 renders of one Vietnamese message, and a retry only after a failure; 2026-10-01T00:30.
- Result: PASS against at most one translate RPC per distinct text.
