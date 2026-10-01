# TASK-013 Outcome

## Outcome

Status: DONE

Changed:

- Removed the DOM adapter and its tests: `client/web.ts` (989 lines), `client/recap.ts`, `client/selection.ts`, `tests/web.test.ts`, `tests/recap.test.ts`, `tests/selection.test.ts`.
- Added [hiep-plugins/plugins/next-prompt-actions/shared/section.ts](hiep-plugins/plugins/next-prompt-actions/shared/section.ts) (split at the What Next heading) and moved `layout` and `reachable` to [hiep-plugins/plugins/next-prompt-actions/shared/selection.ts](hiep-plugins/plugins/next-prompt-actions/shared/selection.ts).
- Added the React Native panel [hiep-plugins/plugins/next-prompt-actions/client/panel.tsx](hiep-plugins/plugins/next-prompt-actions/client/panel.tsx) and an `assistant_message` transformer in [hiep-plugins/plugins/next-prompt-actions/index.client.tsx](hiep-plugins/plugins/next-prompt-actions/index.client.tsx) that keeps the text before the heading.
- Added `prompts.scope` in [hiep-plugins/plugins/next-prompt-actions/shared/contracts.ts](hiep-plugins/plugins/next-prompt-actions/shared/contracts.ts) and [hiep-plugins/plugins/next-prompt-actions/index.server.ts](hiep-plugins/plugins/next-prompt-actions/index.server.ts), because timeline rows have no workspace ID.
- Edit uses `client.setComposerText`; Back to Board uses `client.openSurface("board", { pluginId: "board" })`.
- Requirements, `file:` SDK links, tsconfig type paths, lint flag, README.

Contract:

- Deviation from the spec: the Recap is no longer folded into the panel; it keeps native rendering. The Cmd-hover swap is replaced by a separate New thread button.
- The server engine, candidate keys, send and start guards, and Jev auto-run are unchanged.

Verified:

- `npx tsc --noEmit` -> exit 0. `npm run lint` -> 0 errors. `npm test` -> 71 passed.
- DOM check -> no match.
- Real host check, Paseo Dev 2026-10-01T00:36: the reply showed the What Next panel with Edit, Send, and New thread under the native Recap; Edit put the suggestion into the composer.

Anti-goal:

- Before changes: 107 passed, including `tests/next-prompts.test.ts`, `tests/prompts.test.ts`, and `tests/engine.test.ts`; 2026-10-01T00:34.
- Final: those three files are unedited and pass inside the 71; 2026-10-01T00:40.
- Result: PASS.
