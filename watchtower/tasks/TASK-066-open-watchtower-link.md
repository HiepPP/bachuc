# TASK-066 Open Watchtower at a workspace

Group: B2 (Board client files and the watchtower-board client entry and page)
Class: code

## Brief

Goal: A TASK chip, a plan header, or a Needs you question opens the Watchtower page on the workspace that owns it.

Change: no link -> `openSurface("watchtower", { pluginId: "watchtower-board", serverId, presentation: "overlay", params: { workspaceId, task } })`.

Boundaries:

- Needs TASK-062. Without `params`, the page would open the most recently active workspace, which can be wrong.
- watchtower-board keeps its rule: a named workspace never falls back to another one.
- `task` preselects that TASK in the rail when it exists; otherwise the page opens normally.
- If watchtower-board is not installed on that host, Board shows no link.

How:

- In watchtower-board, read `props.params.workspaceId` and `props.params.task` in [client/page.tsx](hiep-plugins/plugins/watchtower-board/client/page.tsx); keep `preselectWorkspace` for its own openers.
- In Board, add the link on TASK chips, the plan header, and question and check items in Needs you.
- Update both READMEs.

Files:

- [hiep-plugins/plugins/watchtower-board/client/page.tsx](hiep-plugins/plugins/watchtower-board/client/page.tsx) (read params)
- [hiep-plugins/plugins/watchtower-board/README.md](hiep-plugins/plugins/watchtower-board/README.md) (opener line)
- [hiep-plugins/plugins/board/client/lanes.tsx](hiep-plugins/plugins/board/client/lanes.tsx) (chip and header links)
- [hiep-plugins/plugins/board/client/needs-you.tsx](hiep-plugins/plugins/board/client/needs-you.tsx) (item links)
- [hiep-plugins/plugins/board/README.md](hiep-plugins/plugins/board/README.md) (links)

Expected result:

- Clicking TASK-064 on a bachuc card opens Watchtower on that card's workspace with TASK-064 selected.
- Opening Watchtower from its pill or Cmd+G behaves as before.

Anti-goal: watchtower-board tests keep passing, 0 new failures; tripwire; read from `npm test` in `hiep-plugins/plugins/watchtower-board` before and at completion.

## Verify

- `cd hiep-plugins/plugins/watchtower-board && npm run typecheck && npm test` -> pass.
- `cd hiep-plugins/plugins/board && npm run typecheck && npm run lint` -> pass.
- Human check on live with two workspaces of bachuc: a chip opens the card's own workspace, not the most recent one.
