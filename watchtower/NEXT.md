# NEXT

## Current Active Plan

- Title: GPUI client daily-use parity with the Electron UI
- Slug: 20261016-gpui-electron-parity
- Status: ACTIVE
- Updated: 2026-10-16

## Tracker

One row per TASK. Group ties together items that ship as one transaction. Waves: 1 = A, B; 2 = C, D, E, F, G, H, J; 3 = I; 4 = K; 5 = L.

| Order | TASK                                                          | Group | Status      | Spec                                                                                                                   | Deps                                                                                     | Context                                        | Notes                                                       |
| ----- | ------------------------------------------------------------- | ----- | ----------- | ---------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------- | ---------------------------------------------- | ----------------------------------------------------------- |
| 1     | TASK-145 Design system tokens and lucide icons                | A     | DONE        | [watchtower/tasks/TASK-145-design-system-and-icons.md](watchtower/tasks/TASK-145-design-system-and-icons.md)           | -                                                                                        | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Follows ADR-0043. Q-070, Q-071.                             |
| 2     | TASK-146 Client RPCs for parity and a typed server_info       | B     | IN PROGRESS | [watchtower/tasks/TASK-146-client-rpcs.md](watchtower/tasks/TASK-146-client-rpcs.md)                                   | -                                                                                        | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Reopened for Q-073: skip unknown permissions.               |
| 3     | TASK-147 Sidebar with the project and workspace tree          | C     | DONE        | [watchtower/tasks/TASK-147-sidebar-tree.md](watchtower/tasks/TASK-147-sidebar-tree.md)                                 | TASK-145, TASK-146                                                                       | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | implement: manual checks pending (live down). Q-077, Q-085. |
| 4     | TASK-148 Workspace shell with the Electron tab bar and splits | D     | DONE        | [watchtower/tasks/TASK-148-tabs-and-splits.md](watchtower/tasks/TASK-148-tabs-and-splits.md)                           | TASK-145                                                                                 | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | New crate `bachuc-workspace`.                               |
| 5     | TASK-149 Agent pane parity                                    | E     | IN PROGRESS | [watchtower/tasks/TASK-149-agent-pane-parity.md](watchtower/tasks/TASK-149-agent-pane-parity.md)                       | TASK-145, TASK-146                                                                       | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Third fix round for the heading-to-code gap (Q-086).        |
| 6     | TASK-150 Composer parity                                      | F     | DONE        | [watchtower/tasks/TASK-150-composer-parity.md](watchtower/tasks/TASK-150-composer-parity.md)                           | TASK-145, TASK-146                                                                       | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | implement: manual checks pending (live down). Q-079, Q-080. |
| 7     | TASK-151 Explorer with files, a file viewer, and git changes  | G     | DONE        | [watchtower/tasks/TASK-151-explorer-files-and-diff.md](watchtower/tasks/TASK-151-explorer-files-and-diff.md)           | TASK-145, TASK-146                                                                       | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Known gaps Q-075, Q-081, Q-082.                             |
| 8     | TASK-152 Command center and keyboard shortcuts                | H     | DONE        | [watchtower/tasks/TASK-152-command-center-and-shortcuts.md](watchtower/tasks/TASK-152-command-center-and-shortcuts.md) | TASK-145                                                                                 | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | New crate `bachuc-command`.                                 |
| 9     | TASK-153 Hosts and settings                                   | I     | DONE        | [watchtower/tasks/TASK-153-hosts-and-settings.md](watchtower/tasks/TASK-153-hosts-and-settings.md)                     | TASK-145, TASK-146, TASK-152                                                             | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Known gaps Q-083, Q-084.                                    |
| 10    | TASK-154 Terminal colors and font from the tokens             | J     | DONE        | [watchtower/tasks/TASK-154-terminal-colors-and-font.md](watchtower/tasks/TASK-154-terminal-colors-and-font.md)         | TASK-145                                                                                 | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Colors and font only.                                       |
| 11    | TASK-155 App integration and Electron window chrome           | K     | BLOCKED     | [watchtower/tasks/TASK-155-app-integration.md](watchtower/tasks/TASK-155-app-integration.md)                           | TASK-146, TASK-147, TASK-148, TASK-149, TASK-150, TASK-151, TASK-152, TASK-153, TASK-154 | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | BUILT, UNVERIFIED: CPU reads need live (Q-088).             |
| 12    | TASK-156 Side-by-side parity audit                            | L     | DONE        | [watchtower/tasks/TASK-156-parity-audit.md](watchtower/tasks/TASK-156-parity-audit.md)                                 | TASK-155                                                                                 | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | implement: manual checks pending (VoiceOver, dark pairs).   |

TASK Status labels: TODO, IN PROGRESS, BLOCKED, DONE.
Plan-level Status header: ACTIVE while any row is open, DONE when all rows DONE, ARCHIVED after archive.

## Plan Verify

- `packages/desktop-gpui/cargo.sh test --workspace` -> all crates pass
- `packages/desktop-gpui/cargo.sh build --release -p bachuc-app` -> exit 0
- [packages/desktop-gpui/docs/parity-audit.md](packages/desktop-gpui/docs/parity-audit.md) -> every in-scope surface has a row in both themes, and no high-rank mismatch is left open without a follow-up item
- `measure.sh 60` on the app bundle with one mock stream tab and one terminal tab, at about 2 and about 15 minutes -> CPU at most 10.0 and footprint sum at most 250, WebKit helpers included
- `git diff --stat main -- packages/protocol packages/server packages/app packages/desktop` -> empty; the plan does not touch the existing packages

## Review Focus

- A shortcut override with an empty chord -> it loads as unassigned and nothing crashes or binds; owner TASK-152
- A workspace with more than 10,000 files -> the explorer lists only expanded folders and stays responsive; owner TASK-151
- A theme switch with an agent pane and a terminal open -> every region recolors at once; owner TASK-155
- A 900 px wide window with the sidebar and explorer open -> nothing overlaps and tabs shrink; owner TASK-155
- A workspace folder that is not a git repo, an image file, and a `too_large` diff -> the explorer shows the Electron non-git state, the image, and the placeholder; owner TASK-151

## Handoff

- Run of 2026-10-16: 10 TASKs DONE; TASK-149 and TASK-155 BLOCKED on owner questions Q-086 and Q-088. Nothing pushed.
- Plan Verify: workspace tests PASS (333 passed, 0 failed); release build PASS; existing-package diff against `main` empty PASS; audit written with 145 rows, but most dark-theme rows are UNVERIFIED (needs foreground); `measure.sh` CPU reads UNVERIFIED (live down, needs foreground).
- Run limits: live (6768) was down after the reboot; owner rules forbid taking the cursor and bringing a tested app to the front, so hover, drag, live, CPU, and most dark-theme checks are UNVERIFIED.
- Owner answers (2026-10-16, drill): Q-086 third fix round; Q-073 fix now; Q-088 the agent runs the CPU read and the dark-theme pairs as soon as live listens, with the app in front for that read only.
- Follow-up plan, to create with `/watchtower new` after this plan is archived: client fields (Q-074 `requires_attention`, Q-075 `is_git`, Q-077 `project_key`, Q-078 `diffStat` for sidebar and composer, Q-079 agent features, Q-081 `isNew`/`isDeleted`); explorer multi-file diff, Files toolbar, and Commits footer (Q-082); theme font and syntax API with the Settings Fonts and Syntax rows (Q-083); vendored gpui-base with a role on `input-state` (Q-087); provider icons in `bachuc_icons` for tabs and pills.
- Next action: close the TASK-149 and TASK-146 fixes, wait for live on 6768, run the CPU read and dark pairs, update the audit, then archive.

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
