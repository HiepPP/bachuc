# Learn 20261021-gpui-board-instant-running-after-send

## Summary

Discrepancy: none in what shipped. All 4 TASKs shipped as planned, with one builder per group and no fix rounds.

## Per TASK

- TASK-282: match. The client uses `SystemTime` for the mark clock, because the crate has no `chrono`; the spec allowed this.
- TASK-283: match. A side effect the spec implied but did not name: a `board.snapshot` answer whose JSON changed but decodes to an equal snapshot no longer bumps the store revision (Q-184, defaulted to keep). The first-frame owner check passed (Q-183).
- TASK-284: match. The intended test change replaced `back_to_board_opens_before_send`.
- TASK-285: match. The pull task holds a strong store handle until the pull ends, unlike the weak handles of the poll; this is bounded by the 120 s daemon pull timeout.

## Plan-Level

- The "card in Running on the first frame" result needs a human eye, so TASK-283 stayed `BUILT, UNVERIFIED` until Hiep checked it. Plan such timing results as owner checks from the start, as this plan did.
- Builders wrote outcome status as a `## Status` section, not the template's `Status:` line. The lead had to adapt its edits to that shape.
- TASK-283 was `risky`, but the repo rule ran every builder on Sonnet high. It passed with no fix round.

## Lessons

- When a store keeps a derived view, name in the spec what happens to the revision when the derived view is equal, so the builder does not have to ask.
- Ask builders to keep the outcome template's `Status:` line, so later status edits stay mechanical.
