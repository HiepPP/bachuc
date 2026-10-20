# NEXT

## Current Active Plan

- Title: GPUI parity: background card, running indicators, step list look, subagent tabs, and running-turn CPU
- Slug: 20261019-gpui-background-process-card
- Status: ACTIVE
- Updated: 2026-10-19

## Tracker

One row per TASK. Group ties together items that ship as one transaction.

| Order | TASK                                                    | Group | Status  | Spec                                                                                                                                                       | Deps                         | Context | Notes                                                                                                     |
| ----- | ------------------------------------------------------- | ----- | ------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------- | ------- | --------------------------------------------------------------------------------------------------------- |
| 1     | TASK-264 Background processes in the GPUI client        | A     | DONE    | [watchtower/tasks/TASK-264-background-processes-in-the-gpui-client.md](watchtower/tasks/TASK-264-background-processes-in-the-gpui-client.md)               | -                            | -       | Client data, feature gate, and stop request.                                                              |
| 2     | TASK-265 Background card in the GPUI timeline           | B     | DONE    | [watchtower/tasks/TASK-265-background-card-in-the-gpui-timeline.md](watchtower/tasks/TASK-265-background-card-in-the-gpui-timeline.md)                     | TASK-264                     | -       | Committed 1f0c960f6; UI checks pass. implement: manual checks pending (owner hover).                      |
| 3     | TASK-266 Running turn footer in the GPUI timeline       | B     | BLOCKED | [watchtower/tasks/TASK-266-running-turn-footer-in-the-gpui-timeline.md](watchtower/tasks/TASK-266-running-turn-footer-in-the-gpui-timeline.md)             | TASK-265, TASK-276           | -       | Blocked: 10% cap fails on main too (Q-175: base 42.6%, check 44.1%). Footer checks pass.                  |
| 4     | TASK-267 Running pulse and shimmer in the GPUI timeline | B     | BLOCKED | [watchtower/tasks/TASK-267-running-pulse-and-shimmer-in-the-gpui-timeline.md](watchtower/tasks/TASK-267-running-pulse-and-shimmer-in-the-gpui-timeline.md) | TASK-266, TASK-276           | -       | Blocked: 10% cap fails on main too (Q-175, Q-173). Shimmer and pulse pass after fix 083620d10.            |
| 5     | TASK-268 Running ring on GPUI composer tracks           | C1    | DONE    | [watchtower/tasks/TASK-268-running-ring-on-gpui-composer-tracks.md](watchtower/tasks/TASK-268-running-ring-on-gpui-composer-tracks.md)                     | -                            | -       | Committed 3a017cc5c; ring passes. implement: manual checks pending (CPU drift, Q-175).                    |
| 6     | TASK-269 Step list and markdown look as Electron        | B     | BLOCKED | [watchtower/tasks/TASK-269-step-list-and-markdown-look-as-electron.md](watchtower/tasks/TASK-269-step-list-and-markdown-look-as-electron.md)               | TASK-267                     | -       | Blocked: BUILT, UNVERIFIED (Q-174). Gaps within 1.5 px; note height recheck after 083620d10 not read.     |
| 7     | TASK-270 Provider subagent timelines in the GPUI client | A     | DONE    | [watchtower/tasks/TASK-270-provider-subagent-timelines-in-the-gpui-client.md](watchtower/tasks/TASK-270-provider-subagent-timelines-in-the-gpui-client.md) | TASK-264                     | -       | Shares the client files with TASK-264.                                                                    |
| 8     | TASK-271 Read-only timeline for a provider subagent     | B     | DONE    | [watchtower/tasks/TASK-271-read-only-timeline-for-a-provider-subagent.md](watchtower/tasks/TASK-271-read-only-timeline-for-a-provider-subagent.md)         | TASK-269, TASK-270           | -       | Committed 1f0c960f6; read-only view passes through TASK-272.                                              |
| 9     | TASK-273 Status badge on subagent rows and tabs         | C1    | DONE    | [watchtower/tasks/TASK-273-status-badge-on-subagent-rows-and-tabs.md](watchtower/tasks/TASK-273-status-badge-on-subagent-rows-and-tabs.md)                 | TASK-268                     | -       | Committed 3a017cc5c; badges pass (attention dot per spec). implement: manual checks pending (CPU, Q-175). |
| 10    | TASK-272 Open a provider subagent from its row          | C2    | DONE    | [watchtower/tasks/TASK-272-open-a-provider-subagent-from-its-row.md](watchtower/tasks/TASK-272-open-a-provider-subagent-from-its-row.md)                   | TASK-273, TASK-270, TASK-271 | -       | Committed 7b8048b4d; 4 UI lines pass. implement: manual checks pending (Paseo row press).                 |

| 11 | TASK-274 Find the GPUI repaint sources | D | DONE | [watchtower/tasks/TASK-274-find-the-gpui-repaint-sources.md](watchtower/tasks/TASK-274-find-the-gpui-repaint-sources.md) | - | - | BASE 42.4%: orb 17.4, sidebar ring 22.1. CHECK 31.3%. |
| 12 | TASK-275 One shared 66 ms frame ticker | E | BLOCKED | [watchtower/tasks/TASK-275-one-shared-frame-ticker.md](watchtower/tasks/TASK-275-one-shared-frame-ticker.md) | TASK-274 | - | Blocked: CPU 14.9% (was 31.3), cap 10%; per-frame cost goes to TASK-276. Screen checks pass. |
| 13 | TASK-276 Cut the per-frame whole-window cost | F | TODO | [watchtower/tasks/TASK-276-cut-the-per-frame-whole-window-cost.md](watchtower/tasks/TASK-276-cut-the-per-frame-whole-window-cost.md) | TASK-275 | - | Profile per subtree, then cut per-frame copies; no `.cached` (ADR-0053). |
TASK Status labels: TODO, IN PROGRESS, BLOCKED, DONE.
Plan-level Status header: ACTIVE while any row is open, DONE when all rows DONE, ARCHIVED after archive.

## Plan Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-client -p bachuc-timeline -p bachuc-composer -p bachuc-sidebar -p bachuc-workspace -p bachuc-app` -> all tests pass, in one command so gpui-kit builds once.
- `cd packages/desktop-gpui && ./cargo.sh build` -> the workspace builds.
- UI checker on live: an agent runs a background `sleep 300`; the GPUI timeline shows the running card, Stop ends it, and the row moves to the finished card with "stopped".
- UI checker on live: while a turn runs, the last row is the running footer with a moving loader, fork, and a growing clock; the running step pulses; the running tool label shimmers.
- UI checker on live: a click on a Claude subagent row opens its read-only tab; the row and the agent tab show the turning ring while it runs.
- Owner hand check: one thread side by side in Electron and GPUI shows the same step list fonts, bold, links, and block gaps.
- `measure.sh 60` on the check bundle during a live turn, 1200x800 in front, release plugin set -> median of three reads at most 10% CPU, per [ADR-0039](watchtower/decisions/ADR-0039-incremental-timeline-sync-over-throttle-or-budget.md) and [ADR-0052](watchtower/decisions/ADR-0052-one-shared-66ms-frame-ticker-over-per-mark-timers.md).

## Review Focus

- A host without `features.agentBackgroundProcesses` or `projectedSubagentTimeline` -> no Stop button, and a subagent row opens nothing; owner TASK-264, TASK-265, TASK-270, and TASK-272.
- The newest turn has no step card -> the running card shows at the stream bottom, under the running footer and above permission cards; owner TASK-265 and TASK-266.
- A turn ends or a step stops while its mark animates -> the mark stops, and no `Motion` view stays alive; owner TASK-267.
- A subagent tab while the child keeps running, then finishes -> new steps land live, and the running footer goes; owner TASK-271 and TASK-272.
- A theme change or a status change on an open tab -> the badge follows; owner TASK-273.
- A turn ends, or the stop button is pressed (cancelling), while the orb and rings tick -> they stop, and the ticker sleeps back to the idle read; owner TASK-275.
- Reduce motion on -> every mark draws still and requests no tick; owner TASK-275.
- A running turn while Hiep types in the composer and text streams -> caret, typed text, and streamed text all show; owner TASK-276.
- A marked view closes while its tick is pending, or the window is covered and shown again -> no notify to a dead view, and the ticker restarts; owner TASK-275.

## Handoff

- Next action: run `/watchtower implement` for TASK-276 (profile the per-frame cost by subtree, then cut per-frame copies; no `.cached`, ADR-0053). TASK-275 reads 14.9% against the 10% cap; TASK-266, TASK-267, and TASK-275 wait on TASK-276. Q-174 stays open.
- Implement run 2026-10-19: all 10 TASKs built on `feat/gpui-background-process-card`, not pushed. Builders and UI checkers ran deepseek `deepseek-flash[1m]` thinking max (owner override).

## Archive

- None.
