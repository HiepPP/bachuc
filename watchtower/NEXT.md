# NEXT

## Current Active Plan

- Title: GPUI unread thread switcher and after-send jump
- Slug: 20261027-unread-thread-switcher
- Status: ACTIVE
- Updated: 2026-10-27

## Tracker

One row per TASK. Group ties together items that ship as one transaction.

| Order | TASK                                        | Group | Status      | Spec                                                                                                               | Deps                         | Context                                        | Notes                                                           |
| ----- | ------------------------------------------- | ----- | ----------- | ------------------------------------------------------------------------------------------------------------------ | ---------------------------- | ---------------------------------------------- | --------------------------------------------------------------- |
| 1     | TASK-309 Client reads attentionTimestamp    | A     | DONE        | [watchtower/tasks/TASK-309-client-attention-timestamp.md](watchtower/tasks/TASK-309-client-attention-timestamp.md) | -                            | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Gives the queue its FIFO order.                                 |
| 2     | TASK-310 Composer Delivered event           | B     | DONE        | [watchtower/tasks/TASK-310-composer-delivered-event.md](watchtower/tasks/TASK-310-composer-delivered-event.md)     | -                            | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Fires after the daemon accepts a send.                          |
| 3     | TASK-311 After send setting                 | C     | DONE        | [watchtower/tasks/TASK-311-after-send-setting.md](watchtower/tasks/TASK-311-after-send-setting.md)                 | -                            | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Stay, Back to Board, Next unread.                               |
| 4     | TASK-312 Unread queue and Mod+J switcher    | D     | BLOCKED     | [watchtower/tasks/TASK-312-unread-queue-and-switcher.md](watchtower/tasks/TASK-312-unread-queue-and-switcher.md)   | TASK-309                     | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | BUILT, UNVERIFIED (Q-196): held-Cmd release needs a hand check. |
| 5     | TASK-313 After send wiring                  | D     | DONE        | [watchtower/tasks/TASK-313-after-send-wiring.md](watchtower/tasks/TASK-313-after-send-wiring.md)                   | TASK-310, TASK-311, TASK-312 | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | implement: manual checks pending (failing send, Board missing). |
| 6     | TASK-314 Unread queue only from Board cards | D     | IN PROGRESS | [watchtower/tasks/TASK-314-board-cards-only-queue.md](watchtower/tasks/TASK-314-board-cards-only-queue.md)         | TASK-312, TASK-313           | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Hiep asked for Board cards only (ADR-0072).                     |

TASK Status labels: TODO, IN PROGRESS, BLOCKED, DONE.
Plan-level Status header: ACTIVE while any row is open, DONE when all rows DONE, ARCHIVED after archive.

## Plan Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-client -p bachuc-composer -p bachuc-settings -p bachuc-command -p bachuc-app` -> all tests pass.
- `cd packages/desktop-gpui && ./cargo.sh clippy -p bachuc-app --no-deps -- -D warnings` -> no warnings.
- Live check by Hiep (needs a human and the live GPUI app): two agents finish on live, press Mod+J in a third thread, and the oldest unread thread opens; send a prompt with After send set to Next unread, and the next unread thread opens.

## Review Focus

- Mod+J with an empty queue -> a "No unread threads" toast, and the view stays; owner TASK-312.
- An agent in the queue is archived or deleted while the switcher is open -> its row drops and Enter never opens a missing agent; owner TASK-312.
- The send fails in Next unread mode -> the thread stays open with its send error; owner TASK-313.
- A host switch while the switcher is open -> the switcher closes; owner TASK-312.
- A Board visit between two threads -> Mod+Shift+J still opens the last thread; owner TASK-312.

## Handoff

- Next action: Hiep checks Q-196 by hand on live: hold Cmd, press J twice, release Cmd. Then answer Q-196 and run `/watchtower verify`.

## Archive

- None.
