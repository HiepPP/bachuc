# TASK-008 Project Explorer tab

Group: A (shares the Watchtower board client)
Class: code

## Brief

Goal: Show Watchtower beside Files and Changes for the active project.

Change: Open the board only in Explorer instead of the center workspace tab area.

How:

- Restrict the panel location to Explorer and target Explorer from the open command.
- Show the project name in the board header.
- Keep data reads and query caches scoped to the exact current workspace and host.
- Preserve read-only behavior, refresh, task details, and attachments.
- Close the obsolete center tab through the UI after reload.

Files:

- [Client entry](../../../../plugins/watchtower-board/index.client.tsx): Set the panel location and command target.
- [Board](../../../../plugins/watchtower-board/client/board.tsx): Show project identity.
- [README](../../../../plugins/watchtower-board/README.md): Explain where to open the board.

Expected result:

- Watchtower appears beside Files and Changes in Explorer.
- Switching projects displays only that workspace's plan without mixing task data.

## Verify

- Run the plugin formatter, typecheck, lint, and tests.
- Reload the plugin and confirm running with no load error.
- Open Watchtower through Command Center and verify Explorer placement.
- Switch between two existing projects and check each plan, project label, and task list.
- Confirm the obsolete center tab is closed and the current project remains selected.
