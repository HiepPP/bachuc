# Learn 20261018-gpui-polish-and-fixes

## Summary

Discrepancy: 6 found. All 11 TASKs shipped. Two needed a fix round after the UI checker, and three needed files outside their spec.

## Per TASK

- TASK-232: match. The vertical overlap root cause is not proven from source. The fix (`min_w_0` on the row column) passed the UI check. The CPU anti-goal was not read, because the UI checker does not run `measure.sh`.
- TASK-233: plan "measure Electron from source, then apply" -> shipped after one fix round. Mistake: the first source read said 0 px difference, but the paragraph gap was converted with `FONT_BASE` (14) while the window rem is 16, so it drew 27.4 px. Fix: check a source table against a screenshot before you trust it. The list start and end gaps need a vendor option (Q-144).
- TASK-234: plan "whole-UI zoom" -> shipped rem size plus a theme multiplier. Mistake: the spec assumed one zoom method would cover all sizes, but fixed px paddings and icons in the view crates do not scale. Fix: convert them with `zoomed` (Q-151).
- TASK-235: plan "edit `composer.rs` for an insert entry point" -> the lead added `Composer::insert_text` first, because groups S and C both wrote `composer.rs`. Mistake: the spec grouping missed that file conflict. Fix: check every TASK `Files:` line for overlap before the run.
- TASK-236: match.
- TASK-237: match. The tab-bar menu root cause is not proven; a controlled menu replaced it.
- TASK-238: plan Files `crates/composer` -> shipped also `crates/client` (Q-145). Mistake: the spec did not see that `SlashCommand` dropped the wire `kind` and that drafts sent no `draftConfig`. Fix: trace the data path to the client crate when you plan a client feature.
- TASK-239: match, except the incoming side of a swipe is empty, because a peek needs a plugin-host API (Q-149, keep).
- TASK-240: match. The 900 px check was not done, because AX cannot set the window width.
- TASK-241: plan "fix in thread-branch" -> shipped a one-line client capability fix (Q-147). Mistake: the spec listed only plugin layers. Fix: check the client hello capabilities first when a daemon item is missing. Side effect: html-reply rows show an unavailable note (Q-148, hide later).
- TASK-242: plan "fix every gap" -> shipped after one fix round. Mistake: run 1 measured the gpui-kit menu gap from the row, not from the kebab button. Fix: measure a gap from the trigger's own bounds.

## Plan-Level

- File overlap: TASK-235 and TASK-238 both wrote `composer.rs`, but they were in groups S and C, which ran in parallel. The lead solved it with an interface commit.
- Scope: three TASKs (238, 241, and the 233 vendor part) needed files outside their `Files:`. The plan traced only the first layer of each bug.
- Disk: the shared dev target reached about 19 GB during six parallel builders and left 12 GB free. A mid-run `cargo clean --profile dev` was needed before the release bundle build.
- New finding, not in scope: at a 1200 px window, the floating Remove and "Remove & New Thread" buttons overlap message text.

## Lessons

- Before a parallel run, check `Files:` overlap for every group pair, including files a TASK only "may" touch.
- When a bug crosses the client, trace the wire path (hello, RPC fields) before you assign Files.
- Check free disk between groups when six builders share one cargo target.
- ADR candidates were not asked at archive (owner rule). Candidates for the next plan: the zoom method (rem plus `zoomed`, over a gpui-pre scale patch, Q-151), a vendor `TextViewStyle` list option (Q-144), and hiding plugin items with no renderer (Q-148).
