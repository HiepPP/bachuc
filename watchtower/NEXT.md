# NEXT

## Current Active Plan

- Title: GPUI parity: background card, running indicators, step list look, and subagent tabs
- Slug: 20261019-gpui-background-process-card
- Status: ACTIVE
- Updated: 2026-10-19

## Tracker

One row per TASK. Group ties together items that ship as one transaction.

| Order | TASK                                                    | Group | Status      | Spec                                                                                                                                                       | Deps                         | Context | Notes                                                      |
| ----- | ------------------------------------------------------- | ----- | ----------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------- | ------- | ---------------------------------------------------------- |
| 1     | TASK-264 Background processes in the GPUI client        | A     | IN PROGRESS | [watchtower/tasks/TASK-264-background-processes-in-the-gpui-client.md](watchtower/tasks/TASK-264-background-processes-in-the-gpui-client.md)               | -                            | -       | Client data, feature gate, and stop request.               |
| 2     | TASK-265 Background card in the GPUI timeline           | B     | TODO        | [watchtower/tasks/TASK-265-background-card-in-the-gpui-timeline.md](watchtower/tasks/TASK-265-background-card-in-the-gpui-timeline.md)                     | TASK-264                     | -       | Start order only on TASK-264.                              |
| 3     | TASK-266 Running turn footer in the GPUI timeline       | B     | TODO        | [watchtower/tasks/TASK-266-running-turn-footer-in-the-gpui-timeline.md](watchtower/tasks/TASK-266-running-turn-footer-in-the-gpui-timeline.md)             | TASK-265                     | -       | Shares `lib.rs`; adds `motion.rs`.                         |
| 4     | TASK-267 Running pulse and shimmer in the GPUI timeline | B     | TODO        | [watchtower/tasks/TASK-267-running-pulse-and-shimmer-in-the-gpui-timeline.md](watchtower/tasks/TASK-267-running-pulse-and-shimmer-in-the-gpui-timeline.md) | TASK-266                     | -       | Uses the TASK-266 `Motion` view.                           |
| 5     | TASK-268 Running ring on GPUI composer tracks           | C1    | IN PROGRESS | [watchtower/tasks/TASK-268-running-ring-on-gpui-composer-tracks.md](watchtower/tasks/TASK-268-running-ring-on-gpui-composer-tracks.md)                     | -                            | -       | Reuses the sidebar `RunningRing`.                          |
| 6     | TASK-269 Step list and markdown look as Electron        | B     | TODO        | [watchtower/tasks/TASK-269-step-list-and-markdown-look-as-electron.md](watchtower/tasks/TASK-269-step-list-and-markdown-look-as-electron.md)               | TASK-267                     | -       | Vendor patch per ADR-0050; shares `lib.rs`.                |
| 7     | TASK-270 Provider subagent timelines in the GPUI client | A     | IN PROGRESS | [watchtower/tasks/TASK-270-provider-subagent-timelines-in-the-gpui-client.md](watchtower/tasks/TASK-270-provider-subagent-timelines-in-the-gpui-client.md) | TASK-264                     | -       | Shares the client files with TASK-264.                     |
| 8     | TASK-271 Read-only timeline for a provider subagent     | B     | TODO        | [watchtower/tasks/TASK-271-read-only-timeline-for-a-provider-subagent.md](watchtower/tasks/TASK-271-read-only-timeline-for-a-provider-subagent.md)         | TASK-269, TASK-270           | -       | Shares `lib.rs`; uses the TASK-270 key.                    |
| 9     | TASK-273 Status badge on subagent rows and tabs         | C1    | IN PROGRESS | [watchtower/tasks/TASK-273-status-badge-on-subagent-rows-and-tabs.md](watchtower/tasks/TASK-273-status-badge-on-subagent-rows-and-tabs.md)                 | TASK-268                     | -       | Shares composer and sidebar files.                         |
| 10    | TASK-272 Open a provider subagent from its row          | C2    | TODO        | [watchtower/tasks/TASK-272-open-a-provider-subagent-from-its-row.md](watchtower/tasks/TASK-272-open-a-provider-subagent-from-its-row.md)                   | TASK-273, TASK-270, TASK-271 | -       | Shares composer and shell files; needs the read-only view. |

TASK Status labels: TODO, IN PROGRESS, BLOCKED, DONE.
Plan-level Status header: ACTIVE while any row is open, DONE when all rows DONE, ARCHIVED after archive.

## Plan Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-client -p bachuc-timeline -p bachuc-composer -p bachuc-sidebar -p bachuc-workspace -p bachuc-app` -> all tests pass, in one command so gpui-kit builds once.
- `cd packages/desktop-gpui && ./cargo.sh build` -> the workspace builds.
- UI checker on live: an agent runs a background `sleep 300`; the GPUI timeline shows the running card, Stop ends it, and the row moves to the finished card with "stopped".
- UI checker on live: while a turn runs, the last row is the running footer with a moving loader, fork, and a growing clock; the running step pulses; the running tool label shimmers.
- UI checker on live: a click on a Claude subagent row opens its read-only tab; the row and the agent tab show the turning ring while it runs.
- Owner hand check: one thread side by side in Electron and GPUI shows the same step list fonts, bold, links, and block gaps.
- `measure.sh 60` on the check bundle during a live turn -> at most 10% CPU, per [ADR-0039](watchtower/decisions/ADR-0039-incremental-timeline-sync-over-throttle-or-budget.md).

## Review Focus

- A host without `features.agentBackgroundProcesses` or `projectedSubagentTimeline` -> no Stop button, and a subagent row opens nothing; owner TASK-264, TASK-265, TASK-270, and TASK-272.
- The newest turn has no step card -> the running card shows at the stream bottom, under the running footer and above permission cards; owner TASK-265 and TASK-266.
- A turn ends or a step stops while its mark animates -> the mark stops, and no `Motion` view stays alive; owner TASK-267.
- A subagent tab while the child keeps running, then finishes -> new steps land live, and the running footer goes; owner TASK-271 and TASK-272.
- A theme change or a status change on an open tab -> the badge follows; owner TASK-273.

## Handoff

- Next action: run `/watchtower implement`. Group A (TASK-264, TASK-270) and Group C (TASK-268, TASK-273) can start at once. Group B starts after TASK-264. TASK-271 also waits for TASK-270, and TASK-272 waits for TASK-271.

## Archive

- None.
