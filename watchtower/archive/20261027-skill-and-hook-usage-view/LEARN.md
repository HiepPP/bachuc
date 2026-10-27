# Learn 20261027-skill-and-hook-usage-view

## Summary

Discrepancy: 3 found. The four TASKs shipped as planned; the gaps were side effects that no spec named, and two follow-ups were added after the run.

## Per TASK

- TASK-305: match. The live probe proved the nesting and showed no Paseo callback hooks. One `Stop` hook reached `cancelled` at turn end, then `success` when its late response came; the last row wins, so the final state is right.
- TASK-306: match. The summary is stored in the agent record and sent on the snapshot. The daemon restart check stayed UNVERIFIED, because the run config forbids a live restart; a unit test covers the reload.
- TASK-307: plan "hook rows stay their own rows" -> shipped as planned, but the step summary counted `hook` rows as "other tools". Mistake: the spec did not list the step counts as a reader of tool call items. Fix: shipped in `d920464e6` (Q-194). Next time, list every reader of a changed item shape in `Boundaries:`.
- TASK-308: match. The chip, the popover, and the 900 px header fit passed the UI checker.

## Plan-Level

- Plan misdescribes reality: the plan assumed tool hooks were the noise risk. The live probe showed non-tool hooks are louder: about 19 `hook` rows for one short turn (Q-192, defaulted to no fold).
- Scope after the run: Hiep found the skill row hard to see in long turns. Claude Design produced four options, Hiep chose 1c, and it shipped in `f460a40d6` ([ADR-0069](watchtower/decisions/ADR-0069-skill-marker-between-steps.md)).
- Test gap: the owner first looked at the daily GPUI app on release, which had none of the new code. Hand tests on branch work need the check app on live, named in the handoff.

## Lessons

- When a change adds rows to the timeline, list every view that counts or groups rows, and give each one a Verify line.
- Run a live probe early in a plan that depends on provider events; real event counts changed the UX picture.
- Say in the handoff which app and host show branch work, so the owner does not test the release app.
