# TASK-016 P7 board uses daemon tools and plugin navigation

Group: J (hiep-plugins/plugins/board)
Class: risky

## Brief

Goal: Remove Board's HTTP bridge, its stdio MCP process, and its DOM code.

Change: bridge plus DOM clicks and style injection -> `server.registerTool("board_remove")`, `client.openSurface`, and `client.openNewWorkspace`.

Boundaries:

- `board_remove` checks that the target is the caller or one of its descendants, as the bridge did.
- The remove, parent, and new-thread buttons stay as Paseo header buttons. The pane placement DOM goes away.
- The Board shortcut and next-prompt-actions events use `client.openSurface`.
- Raise `requirements.paseo` to `>=0.10.2-beta.900`.

How:

- Replace [hiep-plugins/plugins/board/server/bridge.ts](hiep-plugins/plugins/board/server/bridge.ts) and [hiep-plugins/plugins/board/server/mcp.ts](hiep-plugins/plugins/board/server/mcp.ts) with a `registerTool` call.
- Replace [hiep-plugins/plugins/board/client/web.ts](hiep-plugins/plugins/board/client/web.ts) and other DOM files with API calls, or remove them.
- Update tests and the README.

Files:

- [hiep-plugins/plugins/board/index.server.ts](hiep-plugins/plugins/board/index.server.ts) (tool)
- [hiep-plugins/plugins/board/index.client.tsx](hiep-plugins/plugins/board/index.client.tsx) (navigation)
- Files under [hiep-plugins/plugins/board/client](hiep-plugins/plugins/board/client), [hiep-plugins/plugins/board/server](hiep-plugins/plugins/board/server), and [hiep-plugins/plugins/board/tests](hiep-plugins/plugins/board/tests)
- [hiep-plugins/plugins/board/paseo-plugin.json](hiep-plugins/plugins/board/paseo-plugin.json), [hiep-plugins/plugins/board/package.json](hiep-plugins/plugins/board/package.json), [hiep-plugins/plugins/board/README.md](hiep-plugins/plugins/board/README.md)

Expected result:

- An agent can remove its own card with `mcp__paseo__board_remove`.
- The new-thread button opens the new workspace screen for the project.

Anti-goal: Board store behavior stays the same; tripwire; limit store and recap tests in [hiep-plugins/plugins/board/tests](hiep-plugins/plugins/board/tests) pass unchanged; read before changes and at completion.

## Verify

- `ls hiep-plugins/plugins/board/client/web.ts hiep-plugins/plugins/board/client/events.ts hiep-plugins/plugins/board/server/bridge.ts hiep-plugins/plugins/board/server/mcp.ts` -> none exist, and `rg -n "createServer" hiep-plugins/plugins/board/server hiep-plugins/plugins/board/index.server.ts` -> no match. Project header colors and the latest-prompt reveal keep DOM code, because no plugin API covers them.
- `cd hiep-plugins/plugins/board && npm run typecheck && npm run lint && npm test` -> exit 0.
- Real host check in Paseo Dev (TASK-017): `board_remove` hides a card.
- Anti-goal: store and recap tests pass without edits.
