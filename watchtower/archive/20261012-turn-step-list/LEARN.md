# Learn 20261012-turn-step-list

## Summary

Discrepancy: 4 found. All 4 TASKs shipped, but reviews and a live check after the autorun found bugs that the specs and the autorun checks did not catch.

## Per TASK

- TASK-072: plan grouped a turn into one run with the last message as the answer -> shipped that, plus rules the builder chose where the spec was silent. Mistake: the spec did not say what ends a turn besides a user message, and it assumed one answer message. The risky review found the answer pulled into the list when a plugin row or a notification followed it, and the live check later found an answer split over two messages (L1). Fix: describe turn boundaries with real provider output in the spec; see [ADR-0018](watchtower/decisions/ADR-0018-all-trailing-messages-over-last-message-as-answer.md).
- TASK-073: plan drew the step list view -> shipped the view, its wiring, and strings in 9 locale files. Mistake: class `code`, so no review ran in the autorun. The PR #60 review found that chat find could not reach step notes (R1), that a row click scrolled tall rows (R3), that a whole row was one button for VoiceOver (R4), and one reduce-motion listener per row (R5). The live check found header +/- counts from whole old and new strings (L2). The builder also edited a file with a script instead of the Edit tool. Fix: name chat find, accessibility, and the count source in a view spec, and review UI TASKs even when they are class `code`.
- TASK-074: plan changed the edit-diffs row for a step -> shipped it, plus `shared/change-bar.ts` outside `Files:`, accepted because the node test runner cannot load the react-native row. Mistake: the spec did not check the other places that draw the same row, so the restyle also changes the `detailed` level and compact screens (R2, waiting for the owner). Fix: list every caller of a shared component in the spec.
- TASK-075: match.

## Plan-Level

- The Plan Verify manual checks needed live and a screen, so the autorun left them `UNVERIFIED`. The owner review ran them later; the compact-width check is still not done, because the live window filled the screen and could not be resized.
- Design gap: a step is the tool calls before a note, but agents write a note before they act. Each note then describes the next step's work.
- The edit-diffs rule changed during the plan: [ADR-0017](watchtower/decisions/ADR-0017-native-host-features-over-thin-host-changes.md) replaced the thin-host rule while TASK-073 was waiting.
- Infrastructure slowed the run: SSH to GitHub reset on port 22 (the run used HTTPS), the GitNexus index refresh stalled, and the live window kept a stale bundle after a Metro resolve error until a reload.

## Lessons

- Review every TASK that changes a shared view or component, not only `risky` TASKs.
- Write specs from a real provider transcript: split answers, plugin rows, notifications, and notes before actions.
- Reload the live window after a Metro error before you judge a UI result.
- Check that no other agent runs in the checkout before you start an autorun loop.
