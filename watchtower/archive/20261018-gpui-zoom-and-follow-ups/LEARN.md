# Learn 20261018-gpui-zoom-and-follow-ups

## Summary

Discrepancy: 6 found. All TASKs shipped; the owner and agent screen checks found five bugs and one missing layout part, and fixed them on the branch.

## Per TASK

- TASK-243: match. List spacing passed the owner hand check.
- TASK-244: plan hid plugin items -> shipped as planned. The check found an older bug in the edit-diffs list height (empty diff strip, also on main). Fixed in `7d60a0f`.
- TASK-245: plan reserved a right gutter for the timeline only -> shipped. Mistake: the composer kept full width, so it did not line up with the text as in Electron; and the gutter had no floor, so at 200% the text wrapped one letter a line. Fix: `67f2c75` (composer takes the same inset) and `d6c855e` (text keeps 320 px). The store-offline check stays unverified, because stopping the daemon is forbidden.
- TASK-246: match after the TASK-245 floor fix.
- TASK-247: plan converted sizes only -> the toolbar density still compared zoomed widths with 100% floors and clipped a pill at 150%. Fix: `132164b`.
- TASK-248: match. The split pane was not opened.
- TASK-249: plan converted sizes only -> History clipped titles and badges at 150%, because its fixed columns scaled too. Fix: `a116b31`.
- TASK-250: match. The Watchtower TASK dialog was not opened.
- TASK-251: match. The Space menu and skill pins menu were not opened.
- TASK-252: added during the run (Chat outline rail). The preview text did not wrap. Fix: `4ce92c6`. All checks passed, including the jump to an unloaded prompt.

## Plan-Level

- The plan treated zoom as a size swap. Layout rules that compare widths (toolbar density, gutter, History columns) also need the zoom; specs should list them.
- The GPUI claude-look port lacked the reading layout (768 px column, 1.5 reply line height, header weight 400). Shipped in `f70f88f` as a plugin reading look.
- The unattended UI checker could not run: a covered GPUI window stops drawing.

## Lessons

- Raise the window with the AX `Raise` action before every screenshot, and navigate with `cmd+k` (MEMORY 2026-10-19).
- Mock UI test data on live first (mock provider threads), then test each screen.
- For each zoom TASK, add a Verify line for every width threshold in the touched crates.
- ADR candidates, not recorded: the pill gutter with a 320 px floor (Q-164); the plugin reading look API in the theme crate and plugin host; paging older history for outline jumps (Q-167).
