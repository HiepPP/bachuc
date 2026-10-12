# NEXT

## Current Active Plan

- Title: Deeper plugin host APIs and plugin moves
- Slug: 20260930-plugin-host-apis
- Status: ARCHIVED
- Updated: 2026-10-01

## Tracker

One row per TASK. Group ties together items that write the same files.

| Order | TASK                                                                       | Group | Status  | Spec                                                                                                                   | Deps                                                                           | Context                                        | Notes                                           |
| ----- | -------------------------------------------------------------------------- | ----- | ------- | ---------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------ | ---------------------------------------------- | ----------------------------------------------- |
| 1     | TASK-008 Server hook before agent.prompt                                   | A     | DONE    | [watchtower/tasks/TASK-008-server-prompt-hook.md](watchtower/tasks/TASK-008-server-prompt-hook.md)                     | -                                                                              | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Shares server files with TASK-009 and TASK-010. |
| 2     | TASK-009 Server hook before agent.permission                               | A     | DONE    | [watchtower/tasks/TASK-009-server-permission-hook.md](watchtower/tasks/TASK-009-server-permission-hook.md)             | TASK-008                                                                       | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Order only after TASK-008; same files.          |
| 3     | TASK-010 Plugin MCP tools served by the daemon                             | A     | DONE    | [watchtower/tasks/TASK-010-server-plugin-tools.md](watchtower/tasks/TASK-010-server-plugin-tools.md)                   | -                                                                              | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Same group as TASK-008 for shared files.        |
| 4     | TASK-011 Timeline transformers can keep the source item                    | B     | DONE    | [watchtower/tasks/TASK-011-client-timeline-keep-source.md](watchtower/tasks/TASK-011-client-timeline-keep-source.md)   | -                                                                              | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Client host files.                              |
| 5     | TASK-012 Composer API for plugins                                          | B     | DONE    | [watchtower/tasks/TASK-012-client-composer-api.md](watchtower/tasks/TASK-012-client-composer-api.md)                   | TASK-011                                                                       | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Shares client contracts with TASK-011.          |
| 6     | TASK-013 Composer pills for every composer                                 | B     | DONE    | [watchtower/tasks/TASK-013-client-shared-composer-pills.md](watchtower/tasks/TASK-013-client-shared-composer-pills.md) | TASK-012                                                                       | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Shares client files.                            |
| 7     | TASK-014 Sidebar API for project filters, menus, and sections              | B     | DONE    | [watchtower/tasks/TASK-014-client-sidebar-api.md](watchtower/tasks/TASK-014-client-sidebar-api.md)                     | TASK-013                                                                       | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Shares client files.                            |
| 8     | TASK-015 Plugin navigation to other surfaces and new workspaces            | B     | DONE    | [watchtower/tasks/TASK-015-client-navigation.md](watchtower/tasks/TASK-015-client-navigation.md)                       | TASK-014                                                                       | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Shares client contracts.                        |
| 9     | TASK-016 Fork version and plugin API docs                                  | C     | DONE    | [watchtower/tasks/TASK-016-version-and-docs.md](watchtower/tasks/TASK-016-version-and-docs.md)                         | TASK-008, TASK-009, TASK-010, TASK-011, TASK-012, TASK-013, TASK-014, TASK-015 | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Version sync and docs.                          |
| 10    | TASK-017 P4 jev-permission-gate uses before agent.permission               | D     | DONE    | [watchtower/tasks/TASK-017-jev-permission-gate.md](watchtower/tasks/TASK-017-jev-permission-gate.md)                   | TASK-009, TASK-016                                                             | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Plugin P4.                                      |
| 11    | TASK-018 P3 skill-pins uses the prompt hook and a shared pill              | E     | DONE    | [watchtower/tasks/TASK-018-skill-pins.md](watchtower/tasks/TASK-018-skill-pins.md)                                     | TASK-008, TASK-013, TASK-016                                                   | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Plugin P3.                                      |
| 12    | TASK-019 P2 prompt-translate uses composer, timeline, and prompt APIs      | F     | DONE    | [watchtower/tasks/TASK-019-prompt-translate.md](watchtower/tasks/TASK-019-prompt-translate.md)                         | TASK-008, TASK-011, TASK-012, TASK-013, TASK-016                               | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Plugin P2.                                      |
| 13    | TASK-020 P1 next-prompt-actions renders its panel through the timeline API | G     | DONE    | [watchtower/tasks/TASK-020-next-prompt-actions.md](watchtower/tasks/TASK-020-next-prompt-actions.md)                   | TASK-011, TASK-012, TASK-015, TASK-016                                         | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Plugin P1.                                      |
| 14    | TASK-021 P5 workspace-spaces uses the sidebar API                          | H     | DONE    | [watchtower/tasks/TASK-021-workspace-spaces.md](watchtower/tasks/TASK-021-workspace-spaces.md)                         | TASK-014, TASK-016                                                             | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Plugin P5. Keep uncommitted work.               |
| 15    | TASK-022 P6 jev-orchestrator serves its tools through the daemon           | I     | BLOCKED | [watchtower/tasks/TASK-022-jev-orchestrator.md](watchtower/tasks/TASK-022-jev-orchestrator.md)                         | TASK-010, TASK-016                                                             | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Plugin P6.                                      |
| 16    | TASK-023 P7 board uses daemon tools and plugin navigation                  | J     | DONE    | [watchtower/tasks/TASK-023-board.md](watchtower/tasks/TASK-023-board.md)                                               | TASK-010, TASK-015, TASK-016                                                   | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Plugin P7.                                      |
| 17    | TASK-024 Build Paseo Dev and check the plugins on a real host              | K     | DONE    | [watchtower/tasks/TASK-024-build-and-check.md](watchtower/tasks/TASK-024-build-and-check.md)                           | TASK-017 to TASK-023                                                           | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Never touch the 6767 daemon.                    |

TASK Status labels: TODO, IN PROGRESS, BLOCKED, DONE.
Plan-level Status header: ACTIVE while any row is open, DONE when all rows DONE, ARCHIVED after archive.

## Plan Verify

- `npm run typecheck` -> exit 0.
- `npm run lint -- <changed host files>` -> exit 0.
- Each changed plugin: `npm run typecheck && npm run lint && npm test` in its folder -> exit 0.
- `rg -n "querySelector|MutationObserver" hiep-plugins/plugins/{next-prompt-actions,prompt-translate,skill-pins,workspace-spaces,board}/client` -> no match.
- `lsof -nP -iTCP:6767 -sTCP:LISTEN -t` -> the same PID as at plan start.

## Handoff

- 16 of 17 TASKs are DONE. TASK-022 (jev-orchestrator) is BLOCKED: its native Codex and Claude hooks need the HTTP bridge and the old MCP tool name in global hook configs. See [watchtower/tasks/TASK-022-outcome.md](watchtower/tasks/TASK-022-outcome.md) for the three options.
- The fork version is `0.10.2-beta.900`; plugins require `>=0.10.2-beta.900`, so the stable 0.10.1 host refuses them.
- Paseo Dev at `packages/desktop/release/mac-arm64/Paseo.app` runs the final build with all plugins. The stable daemon on 6767 kept PID 1188.
- Not verified on a real device: iOS and Android. The new APIs avoid DOM code, but mobile was not run.
- Not covered: the plugin prompt hook on steer turns has no e2e test, because the fake provider has no steer support.
- Uncommitted work includes the user's earlier workspace-spaces edits; `client/web.ts` was replaced, and its old diff is saved in [watchtower/tasks/TASK-021-web-ts-uncommitted.patch](watchtower/tasks/TASK-021-web-ts-uncommitted.patch).
- Next action: decide TASK-022, then commit and push when the user asks. Run `detect_changes` before committing.

## Archive

- Archived: 2026-09-30 -> watchtower/archive/20260930-active-machine-switch/
- Archived: 2026-10-01 -> watchtower/archive/20260930-plugin-host-apis/
