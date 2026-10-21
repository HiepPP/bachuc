# NEXT

## Current Active Plan

- Title: GPUI Board moves a What next Send card to Running at once, and the branch pill gets Pull
- Slug: 20261021-gpui-board-instant-running-after-send
- Status: ACTIVE
- Updated: 2026-10-21

## Tracker

One row per TASK. Group ties together items that write the same files.

| Order | TASK                                          | Group | Status | Spec                                                                                                                                   | Deps     | Context | Notes                                             |
| ----- | --------------------------------------------- | ----- | ------ | -------------------------------------------------------------------------------------------------------------------------------------- | -------- | ------- | ------------------------------------------------- |
| 1     | TASK-282 Client pending turn mark             | A     | TODO   | [watchtower/tasks/TASK-282-client-pending-turn-mark.md](watchtower/tasks/TASK-282-client-pending-turn-mark.md)                         | -        | -       | Host API in `bachuc-client`.                      |
| 2     | TASK-283 Board shows pending turns as Running | B     | TODO   | [watchtower/tasks/TASK-283-board-shows-pending-turns-as-running.md](watchtower/tasks/TASK-283-board-shows-pending-turns-as-running.md) | TASK-282 | -       | Start order only; no shared file.                 |
| 3     | TASK-284 What next Send marks the turn        | C     | TODO   | [watchtower/tasks/TASK-284-what-next-send-marks-the-turn.md](watchtower/tasks/TASK-284-what-next-send-marks-the-turn.md)               | TASK-282 | -       | Start order only; runs in parallel with TASK-283. |
| 4     | TASK-285 Branch pill Pull                     | D     | TODO   | [watchtower/tasks/TASK-285-branch-pill-pull.md](watchtower/tasks/TASK-285-branch-pill-pull.md)                                         | -        | -       | Independent; can run with group A.                |

TASK Status labels: TODO, IN PROGRESS, BLOCKED, DONE.

## Plan Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-client -p bachuc-plugin-board -p bachuc-plugin-next-prompt-actions` -> all tests pass.
- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-plugin-thread-branch` -> all tests pass.
- `cd packages/desktop-gpui && ./cargo.sh check -p bachuc-app` -> exit 0.
- Owner check on live GPUI: click Send on a What next card -> the Board opens with that card already in Running, with no jump from Just finished.
- After the checks, quit any check app and run `./cargo.sh clean --profile dev` when no other cargo or rustc process runs.

## Review Focus

- A turn that ends before any poll sees it running -> the card goes to Just finished when a snapshot shows `ended_at` after the mark; owner TASK-283 (`a_run_that_ended_after_the_mark_stays_finished`).
- A marked agent with no card on the Board, for example a removed card -> no card appears; owner TASK-283 (`a_mark_without_a_run_adds_no_card`).
- A failed send, for example a busy agent -> the mark clears and the card goes back; owner TASK-284 (`only_an_acknowledged_send_keeps_the_mark`).
- No active client, for example in All machines mode -> Send works with no mark; owner TASK-284 (no-client Verify line).
- A folder that turned dirty after the last pill read -> the fresh `Force` read blocks the Pull; owner TASK-285 (dirty owner check).

## Handoff

- Next action: run `/watchtower implement`. TASK-282 and TASK-285 go first; then TASK-283 and TASK-284 can run in parallel.

## Archive

- None.
