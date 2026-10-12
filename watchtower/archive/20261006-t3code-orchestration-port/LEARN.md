# Learn 20261006-t3code-orchestration-port

## Summary

Discrepancy: 9 found. All 10 TASKs shipped on `t3code-port` (#2 to #11), and draft PR #13 to `main` is open. Nine TASKs shipped with a deviation from their spec. One was a real test regression from TASK-025, which TASK-027 fixed.

## Per TASK

- TASK-025: plan Verify did not list `session.test.ts` -> shipped a second `getAgent` read that broke it. TASK-027 found and fixed it. Mistake: Verify named only the edited files, not the caller suites. Fix: list caller test suites in Verify for a lifecycle command change.
- TASK-026: plan said a fixed `messageId` per notice envelope -> shipped without it. Mistake: the spec assumed the timeline dedupe was safe; it can drop a real repeat. Fix: keep the deviation and correct the spec text.
- TASK-027: plan said queued images are stored as daemon files, and named a new `messages.agent-queue.test.ts` -> shipped inline base64 images, and no such test file. Direct sends skip the lock. Mistake: the spec went against the repo rule to add tests to existing suites. Fix: point Verify at `messages.wire-compat.test.ts`.
- TASK-028: plan said edits in `actions.ts` and `submit.ts` -> shipped in a new `daemon-queue.ts` and `index.tsx`. The anti-goal check is a proxy. Mistake: wrong target files. Fix: add a gated idle-send case to `submit.test.ts`.
- TASK-029: plan said tests for diff, store, and poll rate -> shipped diff, `gh`, and watcher tests; no store test. Mistake: missed step. Fix: add `tests/store.test.ts` for watch record read, write, and remove.
- TASK-030: plan said mount the toolbar from `surface.web.tsx` -> shipped in `view.tsx`; the action has no `icon`, and the chip input needs `sourceTitle` and `icon`. Mistake: spec gap, the surface has no ids there. Fix: check which props reach a component before naming it as the mount point.
- TASK-031: plan said a notice for quotes over 4000 characters -> shipped no chip and a logged warning (Q-011). Mistake: the spec assumed a plugin toast API that does not exist. Fix: check the plugin context API while writing the spec.
- TASK-032: plan said `https:` scripts, styles, images, and fetch -> shipped `https:` fonts too, past the ADR-0002 list. Mistake: the ADR did not cover fonts that CDN stylesheets load. Fix: amend ADR-0002 to list fonts.
- TASK-033: match.
- TASK-034: plan said native scrolls to the passage -> shipped native opens the agent only (Q-010). The unknown-id toast opens the tab first, and the anti-goal used static effect counts. Fix: correct the spec text and add a real per-row render count test.

## Plan-Level

- Scope creep: commit `57ef5cf9c` (faster local release signing) has no TASK row, but it is on `t3code-port` and in draft PR #13.
- Dep gap: the TASK-028 spec still says edited images come back as file attachments. That stopped being true after the TASK-027 base64 change.
- Plan misdescribes reality: `CONTEXT.md` says new plugins copy the thread-branch scripts. They also need `file:` links, a zod mapping, and `--disable-nested-config`. App test Verify lines use root paths that fail on `__DEV__`.
- Stale Handoff: `NEXT.md` says the loop runs Finish, but draft PR #13 was already open.
- Skipped checks: the live daemon was down, so the CLI install-and-list Verify lines of TASK-029, TASK-031, and TASK-033 did not run. Every TASK still has a pending manual live check.
- Plan Verify: typecheck, lint, format, and the plugin checks passed per TASK. No single pass ran on the final tip.
- Decisions: the owner chose not to pick ADRs at archive. No new ADR was recorded, and ADR-0003 stays `proposed`. Open questions Q-010 and Q-011 stay `DEFAULTED`.

## Lessons

- Before writing a spec, check that each named API, mount point, and test file exists.
- List the test suites of each changed symbol's callers in Verify, not only the edited files.
- When a TASK changes a contract, update the specs of the TASKs that depend on it in the same commit.
- Record an ADR in the TASK that makes the choice. Do not leave ADR choices for archive.
- Start the live daemon for the run, or mark CLI install checks as blocked, not as manual.
