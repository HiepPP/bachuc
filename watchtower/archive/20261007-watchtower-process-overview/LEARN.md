# Learn 20261007-watchtower-process-overview

## Summary

Discrepancy: 9 found. All 10 TASKs shipped in commit `6c7871a25`, but specs had model gaps, live checks never ran, and stopped-run fixes came after the plan.

## Per TASK

- TASK-001: plan `watchtower.overview.read` with plan meta, deps, Plan Verify, and manual checks -> shipped as planned, 6 tests. Mistake: an out-of-spec fix to the plugin `lint` script (`--disable-nested-config`), and `inside` was exported too. Likely cause: the lint script was already broken before the plan, and nobody checked it at plan time. Fix: run the plugin checks once while you write the plan.
- TASK-002: plan run log, questions, ADRs, and history, with "the board task fields" and `nowMinute` to the read time -> shipped, but overview tasks drop `brief`, and `nowMinute` stops at `Finished:`. Mistake: the spec did not size the payload, and did not cover a finished run. Likely cause: the 32 KiB anti-goal was the first size check. Fix: name the exact task fields in the schema, and state the axis end for a finished run.
- TASK-003: match.
- TASK-004: plan timeline rules from [CONTEXT.md](watchtower/archive/20261007-watchtower-process-overview/CONTEXT.md) -> shipped with three rule changes. A running bar over the average holds its lane until `now`, planned bars never start before `now`, and `finishMinute` is set when every open task runs. Mistake: the rules missed tasks that run late. Likely cause: the rules were written from the mid-run mock only. Fix: list edge cases (late, all running, all done, stopped) in the rules.
- TASK-005: plan `timelineGeometry(model, width)` -> shipped `timelineGeometry(model, width, nowMinute)`, and `BranchTimeline` takes `nowMinute` and `started`. Mistake: the TASK-004 model does not carry `nowMinute` or `started`. Likely cause: no check of the interface between TASK-004 and TASK-005. Fix: write the model output type in the producer spec and read it in the consumer spec. Render check skipped (live down).
- TASK-006: match for code. Render check skipped (live down).
- TASK-007: plan props `PluginSurfaceProps` -> shipped `PluginHostProps` plus `workspaceId` and `projectName`, and an `answer by HH:MM` meta. Mistake: small, the spec named a props type that the page does not pass. Fix: check the real prop type before you name it. Render check skipped (live down).
- TASK-008: match for code. Live check skipped: `localDaemon: stopped` at 13:07.
- TASK-009: match. `pillLabel` moved to [client/pill-label.ts](hiep-plugins/plugins/watchtower-board/client/pill-label.ts), which the spec allowed. Live pill check skipped (live down).
- TASK-010: match, 23 added lines. It does not cover the later stopped-run rules (see Plan-Level).

## Plan-Level

- Every visible check (TASK-005 to TASK-009) moved to Plan Verify and Handoff, and live was down for the whole run. The four manual checks in the archived [NEXT.md](watchtower/archive/20261007-watchtower-process-overview/NEXT.md) Handoff are still pending. The plan files show no proof that anyone saw the UI before merge.
- Scope after the plan: PR #24 also carries #26 (a run with no `Finished:` and no log for over two hours is stopped) and #27 (`IN PROGRESS` tasks read `stalled`). No TASK owns this, CONTEXT lifecycle rules do not name a stopped run, and the [README.md](hiep-plugins/plugins/watchtower-board/README.md) does not describe it.
- The TASK-002 `brief` drop changed the TASK-001 contract after TASK-001 was DONE. The spec of TASK-001 was not updated.
- Plan text misdescribes reality: the archived Tracker Spec and Context links still point to `watchtower/tasks/` and `watchtower/CONTEXT.md`, which no longer exist.
- The squash message of `6c7871a25` lists the t3code-port commits (#2 to #13) and other work, but the diff holds only the plugin and the plan files. Readers of the log get a wrong picture of the change.

## Lessons

- Start live, or plan a screenshot step, before an autorun with UI TASKs. Else block the UI TASKs, not only the final check.
- In each producer spec, write the exact output type that the next TASK reads.
- For time models, list the edge cases: late task, all running, all done, finished run, and stopped run.
- When a TASK changes an earlier TASK's contract, update the earlier spec or outcome in the same commit.
- Rewrite plan links to archive paths at archive time.
- Write the squash commit message from the plan, not from the branch history.
