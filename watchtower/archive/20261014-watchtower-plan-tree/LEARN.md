# Learn 20261014-watchtower-plan-tree

## Summary

Discrepancy: 6 found. All nine TASKs shipped and the live checks passed, but the first live check found two text bugs, the second found a wrong waiting state, and three spec facts about the agent data were wrong.

## Per TASK

- TASK-108: plan a pure wave model with time text -> match, then fixed after the live check. Mistake: a done wave said "ready to start", and a row title repeated the TASK ID because the Tracker title starts with it. Fix: test the model with a real Tracker title, and name the header for every wave state (done, running, waiting) in the spec.
- TASK-109: plan builder matching on `parentAgentId`, `lastActivityAt`, and `requiresAttention` -> shipped with fallbacks. Mistake: the plugin agent snapshot has no `parentAgentId` or `lastActivityAt`, and a pending permission does not set `requiresAttention`; it shows in `pendingPermissions` (Q-048, fixed in 985b868a8). Fix: read one real snapshot of each state before writing field names into a spec.
- TASK-110: plan the D v2 tree view -> match, with four extra props (plan title, run state, classes, commits) because the model carries none of them. Mistake: the spec assumed data the view needed but no TASK produced. Fix: list each displayed field and its source in the spec.
- TASK-111: plan to replace the timeline and wire the tree -> match. `clockAt` lived in a deleted file and moved to `run-cells.tsx`. The rail selection used a keyed remount (Q-045), later replaced in TASK-115. 1,267 lines removed.
- TASK-112: plan README and ADR-0029 -> match.
- TASK-113: plan run timing from the git log -> match. Mistake: the spec said any git error warns; the builder kept "not a git repository" and "no commits yet" silent (Q-046) and counted only parenthesised IDs (Q-047). The owner kept both defaults. Fix: decide the error and parsing rules in the spec.
- TASK-114: plan polish toward D v2 -> match. The segment bar also reaches the project panel through the shared `ProgressCard`.
- TASK-115: plan a controlled rail selection -> match.
- TASK-116: plan a live check of running and waiting rows -> blocked on the first run (waiting row missing), then passed after the Q-048 fix.

## Plan-Level

- The first plan (095 to 099) ended DONE before anyone looked at the page. Two later rounds (100 to 103, then the Q-048 fix) came from live checks. A live check inside the first plan would have found the text bugs earlier.
- The pace line could never show for plans run by `implement`, because it read only `RUN.md`. Nobody noticed until the live check; TASK-113 fixed it from the git log.
- The owner asked after the run to keep every wave open (bd0da60e4), so the collapse toggle in the spec was not wanted.
- `npm run lint` in agent shells fails through an rtk ESLint wrapper; builders lost time on it until `rtk proxy npm run lint` was known.
- `paseo run` without `--workspace` creates a new workspace each time, which left extra workspaces on live.

## Lessons

- Put one live check of each UI state into the plan that builds the UI, not into a later plan.
- Before a spec names fields of an external object, read a real instance of it in each state the UI shows.
- For a view, list each value on screen and the TASK that produces it.
- Use `paseo run --workspace <id>` for test agents, and Claude Haiku in mode `default` with a Bash prompt to make an agent wait for a permission.
