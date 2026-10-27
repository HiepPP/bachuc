# Learn 20261027-unread-thread-switcher

## Summary

Discrepancy: 4 found. The first queue rule did not match what Hiep wanted, so two TASKs and two ADRs were added after live use.

## Per TASK

- TASK-309: match.
- TASK-310: plan said clippy keeps 0 warnings -> the crate already had 10 old clippy errors. Mistake: the Verify line assumed a clean baseline. Fix: compare the clippy error list with the baseline.
- TASK-311: match. The "After send" row went into the New agent defaults pane as a planner call.
- TASK-312: plan said the list shows on held Cmd -> shipped as planned, but automation could not hold Cmd. Mistake: a Verify line needed a held modifier. Fix: plan a hand check for held-key input from the start.
- TASK-313: match. A failed send and an inactive Board stayed unit-tested only.
- TASK-314: added after Hiep saw old threads off the Board in the list. The lead dropped a late Board answer after a host switch. Later, the command center item lost its list, because Cmd is up there; fixed after Q-196.
- TASK-315: added so Mod+J cycles read cards too. The builder rotated the full order, which pushed unread cards to the end after a just-read card. The lead kept unread cards first and rotated only read cards.

## Plan-Level

- The plan used every unread agent of the host (ADR-0070). Hiep thinks in Board cards, so ADR-0072 and then ADR-0073 replaced it.
- The UI checker for TASK-315 removed 7 old test cards from the live Board to get a clean Board.

## Lessons

- For a feature that lists threads, ask which list Hiep looks at (the Board) before choosing the data source.
- A screen check that needs a held modifier key goes to Hiep as a hand check in the plan.
- A tap-versus-hold rule must name every entry point, such as the command center, not only the shortcut.
