# Learn 20261016-gpui-parity-follow-ups

## Summary

Discrepancy: 9 found. All 41 TASKs (TASK-157 to TASK-197) shipped and are DONE. The gaps came from specs that missed facts the code held, from checks the run could not do without a screen, and from two runtime bugs that code tests did not catch.

## Per TASK

- TASK-157 to TASK-167: match. Proof was code tests only, so every window check stayed UNVERIFIED until the second run's UI checks.
- TASK-166: plan said "role and label" in the spec Change line and ADR-0044, but Boundaries said no label -> shipped `Role::Group` with no label (Q-089). Mistake: two lines of one spec disagreed. Fix: state each rule once, in Boundaries.
- TASK-168, TASK-169, TASK-170: match. TASK-170 needed an empty `plugins/.gitkeep`, because cargo rejects a member glob with no folder.
- TASK-171, TASK-172: match.
- TASK-173: plan matched -> the first UI check failed by 6.5 pt (title line height 1.618 instead of about 1.2). Fixed in fix round 1. Mistake: the spec gave sizes but not line heights. Fix: give line heights in every text spec.
- TASK-174: match. The theme cycle call came from TASK-180 during the run.
- TASK-175: plan matched -> the footer was 2.5 pt too tall, and AX press on its rows did nothing. Fixed in fix round 1 with explicit `AccessibleAction::Click` handlers.
- TASK-176, TASK-177, TASK-178, TASK-180: match.
- TASK-179: shipped a crash: `DiffView::render_row` read `ListState::viewport_bounds` during list layout ("already mutably borrowed"). Code tests passed; the UI checker found it in round 2. Fixed in `11a6592ba`. Mistake: no rule against reading list state in a row callback. Fix: the rule is now in `MEMORY.md`.
- TASK-181: plan said the nav row pitch matched -> it was 33 pt against 30 pt. Fixed in fix round 1.
- TASK-182, TASK-183, TASK-185, TASK-186, TASK-187: match.
- TASK-184: match. It moved the hard-coded edit row out of the timeline.
- TASK-188: plan said 26 pt tabs -> shipped 28 pt. Fixed in fix round 1.
- TASK-189, TASK-193, TASK-190, TASK-194: match. TASK-194 tripped its anti-goal once (643 ms against 200 ms) and fixed it to 153 ms in the same run.
- TASK-191, TASK-192, TASK-195, TASK-196, TASK-197: match.

## Plan-Level

- Wrong order: TASK-191 (app registration) and TASK-184 (edit-diffs) shared files with groups D and H but depended on later groups, which made a group cycle. The run split them into D2 and H2. Fix: when a TASK writes a group's files but waits on later groups, give it its own sequential group at plan time.
- Plan misdescribes reality: the timeline scrolled jerky on long release threads after the run, because `mono_font_family` enumerated every system font on each row draw. Code tests and background UI checks did not show it. Fixed in `c1887709e`.
- Environment: live was down for most of the run, `run_in_background` jobs died at their time limit, and the GPUI app picked release (6767) by default. Many UI checks stayed UNVERIFIED until the owner's hand test.
- Tooling: two builders' cargo runs left a duplicate entry in `Cargo.lock` that broke every cargo command until the lead removed it. Two builders also stopped on an expired OAuth session.
- ADR candidates not recorded at archive (owner rule): the plugin client crate location `packages/desktop-gpui/plugins/<id>` (now in `CLAUDE.md` and `docs/plugins.md`), and the GPUI New agent as the Electron New workspace screen.
- ADR revisit: no `## Revisit If` holds. ADR-0041 comes close if the owner retires Electron; the owner froze it but did not retire it.

## Lessons

- Give line heights, not only sizes, in GPUI UI specs; GPUI's default is 1.618.
- Never read a `ListState` or query the system (`all_font_names`) in a row or render path.
- Every pressable GPUI element needs an explicit `AccessibleAction::Click` handler, or AX press does nothing.
- Start live detached before any UI check or owner test, and keep it running until the owner is done.
- Builders that share a checkout can corrupt `Cargo.lock`; the lead checks it with `cargo metadata` before each commit.
- Plan an owner hand test for gestures (trackpad scroll, swipe) from the start; automation cannot send them.
