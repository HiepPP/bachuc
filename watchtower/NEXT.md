# NEXT

## Current Active Plan

- Title: GPUI parity follow-ups: client fields, explorer diffs, fonts, and polish
- Slug: 20261016-gpui-parity-follow-ups
- Status: ACTIVE
- Updated: 2026-10-16

## Tracker

One row per TASK. Group ties together items that ship as one transaction. Each group writes one crate, so groups share no file except the generated `Cargo.lock`.

| Order | TASK                                                   | Group | Status      | Spec                                                                                                                 | Deps                                                                           | Context                                        | Notes                                                   |
| ----- | ------------------------------------------------------ | ----- | ----------- | -------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------ | ---------------------------------------------- | ------------------------------------------------------- |
| 1     | TASK-157 Client fields and requests for the follow-ups | A     | DONE        | [watchtower/tasks/TASK-157-client-fields-and-requests.md](watchtower/tasks/TASK-157-client-fields-and-requests.md)   | -                                                                              | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Add-only API; Q-074, Q-075, Q-077, Q-078, Q-079, Q-081. |
| 2     | TASK-158 Provider logos in bachuc-icons                | B     | DONE        | [watchtower/tasks/TASK-158-provider-logos.md](watchtower/tasks/TASK-158-provider-logos.md)                           | -                                                                              | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Replaces the composer's local `claude.svg`.             |
| 3     | TASK-159 Theme font and syntax API                     | C     | DONE        | [watchtower/tasks/TASK-159-theme-font-and-syntax-api.md](watchtower/tasks/TASK-159-theme-font-and-syntax-api.md)     | -                                                                              | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Q-083.                                                  |
| 4     | TASK-160 Settings Fonts and Syntax rows                | D     | DONE        | [watchtower/tasks/TASK-160-settings-fonts-and-syntax.md](watchtower/tasks/TASK-160-settings-fonts-and-syntax.md)     | TASK-159                                                                       | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Font picker from installed fonts (owner).               |
| 5     | TASK-161 Sidebar on the new client fields              | E     | DONE        | [watchtower/tasks/TASK-161-sidebar-client-fields.md](watchtower/tasks/TASK-161-sidebar-client-fields.md)             | TASK-157                                                                       | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Attention, letter colors, diff stat, pinned order.      |
| 6     | TASK-162 Explorer multi-file diff and commits          | F     | DONE        | [watchtower/tasks/TASK-162-explorer-diff-and-commits.md](watchtower/tasks/TASK-162-explorer-diff-and-commits.md)     | TASK-157, TASK-159                                                             | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Q-082; one request returns every file.                  |
| 7     | TASK-163 Composer features, diff row, and logos        | G     | DONE        | [watchtower/tasks/TASK-163-composer-features-and-logos.md](watchtower/tasks/TASK-163-composer-features-and-logos.md) | TASK-157, TASK-158                                                             | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Q-079, Q-078.                                           |
| 8     | TASK-164 Timeline drift, find ring, and syntax theme   | H     | IN PROGRESS | [watchtower/tasks/TASK-164-timeline-drift-ring-syntax.md](watchtower/tasks/TASK-164-timeline-drift-ring-syntax.md)   | TASK-159                                                                       | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Keeps ADR-0039.                                         |
| 9     | TASK-165 Tab tooltip, menu, and logo                   | I     | DONE        | [watchtower/tasks/TASK-165-tab-tooltip-menu-logo.md](watchtower/tasks/TASK-165-tab-tooltip-menu-logo.md)             | TASK-158                                                                       | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | No `PaneItem` field change.                             |
| 10    | TASK-166 Vendored gpui-base with an input role         | J     | DONE        | [watchtower/tasks/TASK-166-vendor-gpui-base.md](watchtower/tasks/TASK-166-vendor-gpui-base.md)                       | -                                                                              | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | ADR-0044; Q-087.                                        |
| 11    | TASK-167 App wiring for the follow-ups                 | K     | TODO        | [watchtower/tasks/TASK-167-app-wiring.md](watchtower/tasks/TASK-167-app-wiring.md)                                   | TASK-158, TASK-159, TASK-160, TASK-161, TASK-162, TASK-163, TASK-164, TASK-165 | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Only TASK that edits `crates/app`.                      |

TASK Status labels: TODO, IN PROGRESS, BLOCKED, DONE.
Plan-level Status header: ACTIVE while any row is open, DONE when all rows DONE, ARCHIVED after archive.

## Plan Verify

- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test --workspace` -> all crates pass
- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh build --release -p bachuc-app` -> exit 0
- `git diff --stat main -- packages/protocol packages/server packages/app packages/desktop` -> empty; the plan does not touch the existing packages
- `packages/desktop-gpui/cargo.sh fmt --check` over every crate this plan changed -> exit 0

## Review Focus

- A daemon that sends `diffStat: null` or no `projectKey` -> the sidebar shows no count and falls back to the `[serverId, projectId]` color key, with no crash; owner TASK-161
- An agent feature of an unknown `type`, or a select feature with a null value -> the composer skips the unknown one and shows the select with no current value; owner TASK-163
- A commit list response with an `error` or an empty `commits` array -> the Commits footer shows the Electron error or empty state; owner TASK-162
- A stored font family that is not installed, or an unknown stored syntax theme id -> the app falls back to the default font and to `one`; owner TASK-159
- A diff file with `status: too_large` or `binary` inside the multi-file view -> that file shows the Electron placeholder and the others still render; owner TASK-162

## Handoff

- Next action: run `/watchtower implement` on branch `feat/bachuc-gpui` with at most 2 builders. Groups A, B, C, and J have no deps.
- The owner measures the CPU of the previous plan's TASK-155 by hand (Q-088 in the archived plan).

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
- [watchtower/archive/20261015-gpui-desktop-client](watchtower/archive/20261015-gpui-desktop-client)
- [watchtower/archive/20261015-instant-thread-open](watchtower/archive/20261015-instant-thread-open)
- [watchtower/archive/20261015-sqlite-ram-cpu-cuts](watchtower/archive/20261015-sqlite-ram-cpu-cuts)
- [watchtower/archive/20261016-gpui-electron-parity](watchtower/archive/20261016-gpui-electron-parity)
