# NEXT

## Current Active Plan

- Title: GPUI desktop client to replace the Electron UI
- Slug: 20261015-gpui-desktop-client
- Status: ACTIVE
- Updated: 2026-10-15

## Tracker

One row per TASK. Group ties together items that ship as one transaction. Waves: 1 = A; 2 = B, C; 3 = D, E, F; 4 = G; 5 = H, I.

| Order | TASK                                                        | Group | Status      | Spec                                                                                                       | Deps                                   | Context                                        | Notes                                                                                                                 |
| ----- | ----------------------------------------------------------- | ----- | ----------- | ---------------------------------------------------------------------------------------------------------- | -------------------------------------- | ---------------------------------------------- | --------------------------------------------------------------------------------------------------------------------- |
| 1     | TASK-134 Move the prototype into the desktop-gpui workspace | A     | DONE        | [watchtower/tasks/TASK-134-desktop-gpui-workspace.md](watchtower/tasks/TASK-134-desktop-gpui-workspace.md) | -                                      | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Workspace, theme crate, build scripts. Commit f4feab6db.                                                              |
| 2     | TASK-135 Generate Rust protocol types from zod              | A     | DONE        | [watchtower/tasks/TASK-135-protocol-codegen.md](watchtower/tasks/TASK-135-protocol-codegen.md)             | TASK-134                               | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Follows ADR-0035. Commit f4feab6db.                                                                                   |
| 3     | TASK-136 Client core: connection, requests, and sync        | B     | DONE        | [watchtower/tasks/TASK-136-client-core.md](watchtower/tasks/TASK-136-client-core.md)                       | TASK-135                               | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | One host at a time. Commit 3283bf219.                                                                                 |
| 4     | TASK-137 Diagram renderer with one hidden WebView           | C     | DONE        | [watchtower/tasks/TASK-137-diagram-renderer.md](watchtower/tasks/TASK-137-diagram-renderer.md)             | TASK-134                               | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Follows ADR-0037; prove the hidden snapshot first. Commit f975b8352; scroll CPU check changed (Q-058).                |
| 5     | TASK-138 Timeline view with the turn step list              | D     | DONE        | [watchtower/tasks/TASK-138-timeline.md](watchtower/tasks/TASK-138-timeline.md)                             | TASK-136, TASK-137                     | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Follows the turn design ADRs. Commit a1c844922.                                                                       |
| 6     | TASK-139 Composer with mode and model controls              | E     | DONE        | [watchtower/tasks/TASK-139-composer.md](watchtower/tasks/TASK-139-composer.md)                             | TASK-136                               | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Pill wheel: Q-055. Commit 129125b14.                                                                                  |
| 7     | TASK-140 Terminal with resize, scrollback, and selection    | F     | DONE        | [watchtower/tasks/TASK-140-terminal.md](watchtower/tasks/TASK-140-terminal.md)                             | TASK-136                               | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | alacritty_terminal. Commit 4ef7549d1.                                                                                 |
| 8     | TASK-141 App shell with host switcher, sidebar, and panes   | G     | DONE        | [watchtower/tasks/TASK-141-app-shell.md](watchtower/tasks/TASK-141-app-shell.md)                           | TASK-137, TASK-138, TASK-139, TASK-140 | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Joins all parts into one app. Commit fbce39e7c. implement: manual checks pending; CPU over 10% on long turns (Q-065). |
| 9     | TASK-142 Accessibility audit with VoiceOver and the AX tree | H     | IN PROGRESS | [watchtower/tasks/TASK-142-accessibility-audit.md](watchtower/tasks/TASK-142-accessibility-audit.md)       | TASK-141                               | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | VoiceOver pass needs a human.                                                                                         |
| 10    | TASK-143 Design the plugin UI host for the GPUI client      | I     | IN PROGRESS | [watchtower/tasks/TASK-143-plugin-ui-host-design.md](watchtower/tasks/TASK-143-plugin-ui-host-design.md)   | TASK-141                               | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Writes a proposed ADR only.                                                                                           |

TASK Status labels: TODO, IN PROGRESS, BLOCKED, DONE.
Plan-level Status header: ACTIVE while any row is open, DONE when all rows DONE, ARCHIVED after archive.

## Plan Verify

- `packages/desktop-gpui/cargo.sh test --workspace` -> all crates pass
- `packages/desktop-gpui/cargo.sh build --release -p bachuc-app` -> exit 0
- Agent check on live with one mock stream: open the bundled `Bachuc GPUI.app`, open the agent, send a prompt, open a terminal tab -> the stream, the reply, and the terminal work in one window
- `measure.sh 60` on the bundled app with one stream tab and one terminal tab -> CPU at most 10.0 and footprint sum at most 250, WebKit helpers included
- `git diff --stat main -- packages/protocol packages/server packages/app packages/desktop` -> empty; the plan does not touch the existing packages

## Review Focus

- The daemon restarts while a stream tab is open -> the client reconnects, refetches, and shows no duplicate rows; owner TASK-136
- An agent with a timeline longer than one page -> scrolling to the top loads older items without a jump; owner TASK-138
- A broken mermaid block -> the timeline shows the code block, not an empty gap; owner TASK-138
- The selected host is down at app start -> the app shows a disconnected state and retries; owner TASK-141
- A terminal pane resized while output streams -> the emulator and the daemon PTY agree on the size; owner TASK-140

## Handoff

- Next action: run `/watchtower implement` on branch `feat/bachuc-gpui`. Wave 1 is group A (TASK-134, TASK-135).

## Archive

- [watchtower/archive/20260930-active-machine-switch](watchtower/archive/20260930-active-machine-switch)
- [watchtower/archive/20260930-plugin-host-apis](watchtower/archive/20260930-plugin-host-apis)
- [watchtower/archive/20261006-t3code-orchestration-port](watchtower/archive/20261006-t3code-orchestration-port)
- [watchtower/archive/20261007-bachuc-rebrand](watchtower/archive/20261007-bachuc-rebrand)
- [watchtower/archive/20261007-watchtower-process-overview](watchtower/archive/20261007-watchtower-process-overview)
- [watchtower/archive/20261012-repo-wide-ids-and-board-lanes](watchtower/archive/20261012-repo-wide-ids-and-board-lanes)
- [watchtower/archive/20261012-turn-step-list](watchtower/archive/20261012-turn-step-list)
- [watchtower/archive/20261013-composer-pill-wheel-select](watchtower/archive/20261013-composer-pill-wheel-select)
- [watchtower/archive/20261013-fleet-map](watchtower/archive/20261013-fleet-map)
- [watchtower/archive/20261013-turn-step-list-follow-ups](watchtower/archive/20261013-turn-step-list-follow-ups)
- [watchtower/archive/20261014-native-glm-provider](watchtower/archive/20261014-native-glm-provider)
- [watchtower/archive/20261014-watchtower-plan-tree](watchtower/archive/20261014-watchtower-plan-tree)
- [watchtower/archive/20261015-instant-thread-open](watchtower/archive/20261015-instant-thread-open)
- [watchtower/archive/20261015-sqlite-ram-cpu-cuts](watchtower/archive/20261015-sqlite-ram-cpu-cuts)
