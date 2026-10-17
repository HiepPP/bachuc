# NEXT

## Current Active Plan

- Title: GPUI bug fixes and daily Electron gaps
- Slug: 20261017-gpui-daily-gaps-and-fixes
- Status: IMPLEMENTED
- Updated: 2026-10-18

## Tracker

One row per TASK. Group ties together TASKs that write the same files; a group runs its TASKs in order. Deps across groups set the start order.

| Order | TASK                                                   | Group | Status | Spec                                                                                                                               | Deps                                                                 | Context                                        | Notes                                                                  |
| ----- | ------------------------------------------------------ | ----- | ------ | ---------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------- | ---------------------------------------------- | ---------------------------------------------------------------------- |
| 1     | TASK-198 Board card opens its thread                   | S     | DONE   | [watchtower/tasks/TASK-198-board-card-opens-thread.md](watchtower/tasks/TASK-198-board-card-opens-thread.md)                       | -                                                                    | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | 150a36be3, 8819e6d89, 7c066ec26; implement: manual checks pending.     |
| 2     | TASK-209 Board card looks like Electron                | B     | DONE   | [watchtower/tasks/TASK-209-board-card-visual.md](watchtower/tasks/TASK-209-board-card-visual.md)                                   | -                                                                    | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | 72fefd064, ca0ac2d90; implement: manual checks pending (Q-133). Q-116. |
| 3     | TASK-207 Step list matches Electron                    | T     | DONE   | [watchtower/tasks/TASK-207-step-list-matches-electron.md](watchtower/tasks/TASK-207-step-list-matches-electron.md)                 | -                                                                    | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | 8591a60ef, 77a9730cb; UI PASS. Q-117, Q-118.                           |
| 4     | TASK-204 Client git, PR, and script calls              | C     | DONE   | [watchtower/tasks/TASK-204-client-git-pr-script-calls.md](watchtower/tasks/TASK-204-client-git-pr-script-calls.md)                 | -                                                                    | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | 9fa7463d3; no UI checks.                                               |
| 5     | TASK-199 Board closes on a sidebar workspace click     | S     | DONE   | [watchtower/tasks/TASK-199-agent-tab-shows-thread.md](watchtower/tasks/TASK-199-agent-tab-shows-thread.md)                         | TASK-198                                                             | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Fixed by 150a36be3 (TASK-198); manual checks pending (Q-133). Q-127.   |
| 6     | TASK-205 Client history, archived agents, fork context | C     | DONE   | [watchtower/tasks/TASK-205-client-history-archive-fork.md](watchtower/tasks/TASK-205-client-history-archive-fork.md)               | TASK-204                                                             | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | a86c39862; no UI checks. Q-119 defaulted.                              |
| 7     | TASK-206 Client file calls                             | C     | DONE   | [watchtower/tasks/TASK-206-client-file-calls.md](watchtower/tasks/TASK-206-client-file-calls.md)                                   | TASK-205                                                             | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | 3a16c8c36; no UI checks. Q-123.                                        |
| 8     | TASK-216 Sidebar History and New workspace rows        | N     | DONE   | [watchtower/tasks/TASK-216-sidebar-history-new-workspace.md](watchtower/tasks/TASK-216-sidebar-history-new-workspace.md)           | -                                                                    | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | 93fc8d183; implement: manual checks pending (Q-133). Q-126.            |
| 9     | TASK-218 History screen                                | H     | DONE   | [watchtower/tasks/TASK-218-history-screen.md](watchtower/tasks/TASK-218-history-screen.md)                                         | TASK-205                                                             | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | a4295b97b; date groups PASS (run 3); search UNVERIFIED (Q-133). Q-125. |
| 10    | TASK-208 Fork button in the turn footer                | T     | DONE   | [watchtower/tasks/TASK-208-fork-button-turn-footer.md](watchtower/tasks/TASK-208-fork-button-turn-footer.md)                       | TASK-207, TASK-205                                                   | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | 77a9730cb; implement: manual checks pending (Q-133). Q-129.            |
| 11    | TASK-210 Git actions policy and split button           | E     | DONE   | [watchtower/tasks/TASK-210-git-actions-split-button.md](watchtower/tasks/TASK-210-git-actions-split-button.md)                     | TASK-204                                                             | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | cb8dee5fb; look checked with TASK-202. Q-121.                          |
| 12    | TASK-200 History screen and unarchive in the app       | S     | DONE   | [watchtower/tasks/TASK-200-history-and-unarchive-wiring.md](watchtower/tasks/TASK-200-history-and-unarchive-wiring.md)             | TASK-199, TASK-205, TASK-216, TASK-218                               | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | 8687b71db, aaada0a61; implement: manual checks pending (Q-133).        |
| 13    | TASK-211 Branch switcher                               | E     | DONE   | [watchtower/tasks/TASK-211-branch-switcher.md](watchtower/tasks/TASK-211-branch-switcher.md)                                       | TASK-210                                                             | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | da7231026; UI PASS. Q-122.                                             |
| 14    | TASK-212 Pull request view                             | E     | DONE   | [watchtower/tasks/TASK-212-pull-request-view.md](watchtower/tasks/TASK-212-pull-request-view.md)                                   | TASK-211                                                             | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | c542cd8f8; implement: manual checks pending (Q-133). Q-124.            |
| 15    | TASK-213 Explorer file menu                            | E     | DONE   | [watchtower/tasks/TASK-213-explorer-file-menu.md](watchtower/tasks/TASK-213-explorer-file-menu.md)                                 | TASK-212, TASK-206                                                   | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | 4b39e4fbb; implement: manual checks pending (Q-133). Q-132.            |
| 16    | TASK-214 File mentions and file drop                   | M     | DONE   | [watchtower/tasks/TASK-214-file-mentions-and-drop.md](watchtower/tasks/TASK-214-file-mentions-and-drop.md)                         | TASK-206                                                             | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | cd364fc63; implement: manual checks pending (Q-133). Q-128.            |
| 17    | TASK-215 Terminal find and links                       | R     | DONE   | [watchtower/tasks/TASK-215-terminal-find-and-links.md](watchtower/tasks/TASK-215-terminal-find-and-links.md)                       | TASK-206                                                             | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | 45cde7625; implement: manual checks pending (Q-133). Q-134.            |
| 18    | TASK-217 Files scope, shortcut rows, shortcuts dialog  | N     | DONE   | [watchtower/tasks/TASK-217-files-scope-and-shortcuts.md](watchtower/tasks/TASK-217-files-scope-and-shortcuts.md)                   | TASK-216                                                             | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | 359ce49c4; implement: manual checks pending (Q-133). Q-135.            |
| 19    | TASK-219 Composer draft seed and raw attachment        | M     | DONE   | [watchtower/tasks/TASK-219-composer-draft-seed-and-attachment.md](watchtower/tasks/TASK-219-composer-draft-seed-and-attachment.md) | TASK-214                                                             | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | 22906f2cb; UI PASS (with TASK-201).                                    |
| 20    | TASK-202 Git, PR tab, Scripts, Open in editor in app   | S     | DONE   | [watchtower/tasks/TASK-202-git-pr-header-wiring.md](watchtower/tasks/TASK-202-git-pr-header-wiring.md)                             | TASK-200, TASK-204, TASK-210, TASK-212                               | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | e9a78e843; implement: manual checks pending (Q-133). Q-130.            |
| 21    | TASK-201 Fork into a new tab or workspace              | S     | DONE   | [watchtower/tasks/TASK-201-fork-wiring.md](watchtower/tasks/TASK-201-fork-wiring.md)                                               | TASK-202, TASK-205, TASK-208, TASK-219                               | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | 5da20645f; UI PASS. Q-131.                                             |
| 22    | TASK-203 New workspace, files, shortcuts in app        | S     | DONE   | [watchtower/tasks/TASK-203-navigation-files-shortcuts-wiring.md](watchtower/tasks/TASK-203-navigation-files-shortcuts-wiring.md)   | TASK-201, TASK-206, TASK-213, TASK-214, TASK-215, TASK-216, TASK-217 | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | 941432b4a; implement: manual checks pending (Q-133). Q-136.            |

TASK Status labels: TODO, IN PROGRESS, BLOCKED, DONE.

## Plan Verify

- `cd packages/desktop-gpui && ./cargo.sh build -p bachuc-app` -> builds after each group commit.
- `./cargo.sh test` for each crate a TASK touched, one crate at a time -> passes.
- `./cargo.sh metadata --format-version 1 > /dev/null` -> exits 0 before each commit.
- Background UI checker on live (6768): runs every `UI check` line after its TASK lands.
- Owner hand test at the end: Board card click, agent tab click, step list, git header, History, fork, `@` mentions, file drop, terminal links, and the new shortcuts.
- Cleanup: quit the GPUI apps the run launched, then `./cargo.sh clean --profile dev` when no cargo runs.

## Review Focus

- Board card of an archived agent -> opens its tab with the archived callout; owner TASK-200.
- Git status still loading or not a git folder -> every git action is disabled; owner TASK-210.
- History with no agents, or a search with no match, or a failed load -> the right Electron text; owner TASK-218.
- `?` typed in the composer, a terminal, the command center, or a settings field -> it types `?` and no dialog opens; owner TASK-203.
- A burst of git status updates -> one PR refetch per 500 ms, not one per update; owner TASK-212.

## Handoff

- Implement ran on 2026-10-17 and 2026-10-18 on `feat/gpui-daily-gaps-and-fixes`. All 22 TASKs are DONE; nothing is pushed.
- TASK-199 was re-scoped by Q-127: the Board stayed in front after a sidebar workspace click. TASK-198 (`150a36be3`) fixed it.
- Owner hand test: the pointer, foreground, and timing checks in Q-133.
- Deferred gaps (Q-115) and follow-ups: Q-117, Q-122, Q-124, Q-128, Q-129, Q-130, Q-132, Q-134, Q-135.

## Archive

- [watchtower/archive/20261016-gpui-parity-follow-ups/NEXT.md](watchtower/archive/20261016-gpui-parity-follow-ups/NEXT.md)
- [watchtower/archive/20261016-gpui-electron-parity/NEXT.md](watchtower/archive/20261016-gpui-electron-parity/NEXT.md)
