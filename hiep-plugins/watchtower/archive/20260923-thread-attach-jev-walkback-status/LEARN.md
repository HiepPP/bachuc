# Learn 20260923-thread-attach-jev-walkback-status

## Summary

Discrepancy: 3 found. All 4 TASKs shipped and passed checks, but the user rejected the feature after a live test. All code was reverted to the last commit on 2026-09-23.

## Per TASK

- TASK-001: plan said depth 2 or 3 needs one option above 0.5 -> shipped as planned. Mistake: the live Jev answer split the vote (0.23 / 0.48 / 0.29), so walk-back never ran. Cause: the rule was written before seeing a real Jev answer. Fix: the sum rule P(2) + P(3) > 0.5 was added, then reverted with the rest.
- TASK-002: match. The settings switch in the app was never checked by the user before the revert.
- TASK-003: plan said enrich in the background from search -> shipped as planned. Mistake: every picker open called Jev for all 12 listed threads, and running threads were called again after each activity change. 27 calls ran in one short test. Cause: the plan counted calls per thread, not per picker open. Fix: do not call a paid model from a per-keystroke search path.
- TASK-004: match for the README. The live in-app check failed from the user's view: the status did not show, and the user had to reopen the picker.

## Plan-Level

- The plan misdescribed the user need. The user wanted precise, cheap context from other threads, found by the agent while it works. The plan built a picker label that the user must open twice.
- Jev only answers boolean, choice, and score questions. It is a judge, not a search engine. Search needs a search tool.

## Lessons

- Confirm the user flow before planning. Ask who triggers the action (user or agent) and when.
- Run one live model call before fixing thresholds in a plan.
- Count paid calls per user action, not per item.
- For finding the right thread and passage, use local search such as qmd, not a model judge.
