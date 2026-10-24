# Learn 20261023-decider-spike-probe-uc2

## Summary

Discrepancy: 2 found. All three TASKs shipped, but TASK-300 ran in a different order and with agent-assisted labels.

## Per TASK

- TASK-298: match.
- TASK-299: match.
- TASK-300: plan said both runs start after Q-190 is answered -> shipped: both runs finished on 2026-10-23, before labeling, and TASK-300 only ran `report` on 2026-10-25. Mistake: the spec marked the whole TASK blocked, but only `report` reads `labels.jsonl`. Fix: block only the step that reads the owner input.
- TASK-300: plan said Hiep labels every event -> shipped: `labels.jsonl` is the agent draft `labels.suggested.jsonl`, reviewed by Hiep with no edits. Mistake: none against [CONTEXT.md](CONTEXT.md), which allows pre-filled drafts, but the ground truth is not independent of an agent. Fix: state in the result how the labels were made, as `RESULTS.md` does.

## Plan-Level

- The spike branch sat unmerged for two days with TASK-300 open, and a branch cleanup almost deleted it. A spike that waits on the owner needs a visible open question outside the branch.

## Lessons

- Put an owner gate on the step that needs it, so the runs can go first and the owner input is the last step.
- Result: Decider (`pplx-decider-flash-gguf-v1.2`) is no-go on the MacBook Air on latency only (server p95 2897 ms). The next plan is a rerun on the Mac mini with `--endpoint` on the same events and labels.
