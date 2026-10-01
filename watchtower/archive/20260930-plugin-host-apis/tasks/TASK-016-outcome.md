# TASK-016 Outcome

## Outcome

Status: DONE

Changed:

- [hiep-plugins/plugins/board/index.server.ts](hiep-plugins/plugins/board/index.server.ts) registers `board_remove` with `server.registerTool`. It checks the daemon-provided caller with the existing `store.removeByAgent`. The `agent.create`, `agent.session_open`, and `agent.archived` bridge hooks are gone.
- Deleted `server/bridge.ts`, `server/mcp.ts`, `client/web.ts`, `client/events.ts`, and `tests/events.test.ts`. The `@modelcontextprotocol/sdk` dependency was dropped; `zod` became a runtime dependency.
- [hiep-plugins/plugins/board/index.client.tsx](hiep-plugins/plugins/board/index.client.tsx) opens the Board with `client.openSurface("board")` and sets [hiep-plugins/plugins/board/client/workspace.ts](hiep-plugins/plugins/board/client/workspace.ts) to `client.openNewWorkspace`.
- [hiep-plugins/plugins/board/client/new-thread.tsx](hiep-plugins/plugins/board/client/new-thread.tsx) no longer needs its helper surface. [hiep-plugins/plugins/board/client/page.tsx](hiep-plugins/plugins/board/client/page.tsx) opens projects through `workspaceActions` and drops the send-result toast that listened for next-prompt-actions DOM events.
- [hiep-plugins/plugins/board/client/shortcut.ts](hiep-plugins/plugins/board/client/shortcut.ts): the first installation owns Cmd+D, so one press opens one Board.
- Host change made for this TASK: `client.openSurface` defaults to the active host ([packages/app/src/plugins/actions.ts](packages/app/src/plugins/actions.ts)), and button `onPress(context?)` is optional in [packages/plugin/src/client/buttons.ts](packages/plugin/src/client/buttons.ts) to keep existing callers compiling.
- Tests updated: bridge and MCP cases removed from [hiep-plugins/plugins/board/tests/board-remove.test.ts](hiep-plugins/plugins/board/tests/board-remove.test.ts); [hiep-plugins/plugins/board/tests/navigation.test.ts](hiep-plugins/plugins/board/tests/navigation.test.ts) rewritten for shortcut ownership; test fakes adjusted.
- Requirements, `file:` SDK links, tsconfig type paths, lint flag, README.

Contract:

- Deviation from the spec: project header colors ([hiep-plugins/plugins/board/client/project-header.ts](hiep-plugins/plugins/board/client/project-header.ts)) and the latest-prompt reveal ([hiep-plugins/plugins/board/client/latest-prompt.ts](hiep-plugins/plugins/board/client/latest-prompt.ts)) keep DOM code; no plugin API covers them. The Verify line was corrected to match.
- The Board no longer shows a warning toast when a next-prompt-actions send fails.

Verified:

- `npx tsc --noEmit` -> exit 0. `npm run lint` -> 0 warnings, 0 errors on 50 files. `npm test` -> 99 passed.
- Deleted files are absent; `rg -n "createServer"` in the server -> no match.
- Real host check, Paseo Dev 2026-10-01T00:33: the daemon MCP route listed `board_remove`; a call as agent `7eca0768` returned `removed` and the Board card disappeared.

Anti-goal:

- Before changes: 106 passed; 2026-10-01T00:53.
- Final: `tests/store.test.ts` and the recap tests are unedited and pass; 2026-10-01T01:05.
- Result: PASS.
