# Learn 20261019-gpui-background-process-card

## Summary

Discrepancy: 4 found. All 14 TASKs shipped code. The 10% CPU cap blocked TASK-266, TASK-267, and TASK-275 until TASK-276 cut the per-frame copies (median 7.2%). TASK-269 and TASK-277 wait on owner hand checks.

## Per TASK

- TASK-264: match.
- TASK-265: match. The owner hover check stays manual.
- TASK-266: plan "footer at most 10% CPU" -> shipped the footer, but the cap failed on main too (Q-175). Mistake: the plan had no pre-plan CPU base, so a whole-app cost looked like a TASK cost. Fix: measure the base app before the plan sets a CPU cap. Closed after TASK-276.
- TASK-267: same as TASK-266. The 66 ms frame (Q-173) took the default.
- TASK-268: match.
- TASK-269: plan "same look as Electron" -> shipped. Mistake: the look check needs the owner's eye (Q-174), so the TASK cannot close unattended. Fix: plan a look match as an owner check from the start.
- TASK-270: match.
- TASK-271: match.
- TASK-272: match. The Paseo row press stays manual.
- TASK-273: match.
- TASK-274: match. The profile named the stop orb and the sidebar ring.
- TASK-275: plan "one ticker reaches 10%" -> shipped the ticker at 14.9%. Mistake: the plan cut the frame count, but each frame cost the whole window. Fix: profile the cost of one frame before you plan the cut. Closed after TASK-276.
- TASK-276: match. Step 1 used the TASK-275 sample of the same binary, not a fresh one. Three `render` copy cuts gave a median of 7.2%, with AX nodes unchanged. The drop is larger than the cuts predict, and no BASE read used the same procedure.
- TASK-277: plan "button with UI checks" -> shipped the button, code checks pass. The UI checks did not run: the owner stopped the checker (Q-178).

## Plan-Level

- Plan misdescribes reality: the CPU cap was set before anyone measured the base app. Three TASKs blocked on a cost that main already had (Q-175).
- Dep gap: TASK-266, TASK-267, and TASK-275 got a dependency on TASK-276 late, after the first CPU reads failed.
- Open: Q-179, the check window stopped drawing for about 4 minutes during a CPU read until a click.

## Lessons

- Measure the base app at plan time before you write a CPU cap into a TASK.
- Profile one frame by the nearest `bachuc_*` frame; cut copies in `render` before you add a view boundary.
- Give each CPU read its own BASE bundle id, so `open` cannot reach the wrong copy.
- Put owner look and feel checks in their own TASK, so code TASKs can close unattended.
