# TASK-008 Full-screen surface shows the overview

Group: G (standalone: `client/page.tsx`)
Class: code

## Brief

Goal: Show the overview on the Watchtower surface on wide layouts. Keep the existing board on compact layouts.

Change: the surface always renders `WatchtowerBoard` -> it renders `WatchtowerOverview` when `layout.compact` is false.

Boundaries:

- Keep the workspace chip row and `preselectWorkspace`. TASK-009 relies on `preselectWorkspace` to open the surface on the composer's workspace.
- Do not change [hiep-plugins/plugins/watchtower-board/client/board.tsx](hiep-plugins/plugins/watchtower-board/client/board.tsx).

How:

- In `WatchtowerPage`, render `WatchtowerOverview` with `workspaceId` and `projectName` when `layout.compact` is false.
- Keep the `key={workspace.id}` reset, so each workspace keeps its own state.

Files:

- [hiep-plugins/plugins/watchtower-board/client/page.tsx](hiep-plugins/plugins/watchtower-board/client/page.tsx) (choose the overview or the board)

Expected result:

- A wide surface shows the overview for the selected workspace chip.
- A compact surface shows the existing board, as before.
- Picking another chip switches the overview to that workspace.

Anti-goal: the compact board stays untouched: changed lines in `client/board.tsx` stay at 0; tripwire; read with `git diff --stat -- hiep-plugins/plugins/watchtower-board/client/board.tsx` before the edit and at completion.

## Verify

- In the plugin folder: `npm run typecheck` and `npm run lint` -> exit 0.
- `git diff --stat watchtower-overview -- hiep-plugins/plugins/watchtower-board/client/board.tsx` -> no output.
- Live check, when the live daemon is up: `npm run cli -- plugin reload watchtower-board`, then open Watchtower from the sidebar on a wide desktop window -> the overview shows. Record it as `UNVERIFIED (autonomous run)` when live is down.
