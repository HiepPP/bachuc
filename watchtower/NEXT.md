# NEXT

## Current Active Plan

- Title: Decider spike probe for the permission gate (UC2)
- Slug: 20261023-decider-spike-probe-uc2
- Status: ACTIVE
- Updated: 2026-10-23

## Tracker

| Order | TASK                                       | Group | Status | Spec                                                                                                                             | Deps               | Context                                        | Notes                                                                              |
| ----- | ------------------------------------------ | ----- | ------ | -------------------------------------------------------------------------------------------------------------------------------- | ------------------ | ---------------------------------------------- | ---------------------------------------------------------------------------------- |
| 1     | TASK-298 Node extract and Jev bridge       | A     | TODO   | [watchtower/tasks/TASK-298-node-extract-and-jev-bridge.md](watchtower/tasks/TASK-298-node-extract-and-jev-bridge.md)             | -                  | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Writes the event, label, and result formats.                                       |
| 2     | TASK-299 Rust decider-probe run and report | B     | TODO   | [watchtower/tasks/TASK-299-rust-decider-probe-run-and-report.md](watchtower/tasks/TASK-299-rust-decider-probe-run-and-report.md) | TASK-298           | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Start order only; no shared file. Can build from the Interfaces block in parallel. |
| 3     | TASK-300 Run the probe and write results   | C     | TODO   | [watchtower/tasks/TASK-300-run-probe-and-write-results.md](watchtower/tasks/TASK-300-run-probe-and-write-results.md)             | TASK-298, TASK-299 | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Blocked on Q-190 labels by Hiep.                                                   |

TASK Status labels: TODO, IN PROGRESS, BLOCKED, DONE.

## Plan Verify

- `node --import ./hiep-plugins/plugins/jev-permission-gate/node_modules/tsx/dist/loader.mjs packages/desktop-gpui/crates/decider-probe/scripts/extract.ts` then `cd packages/desktop-gpui && ./cargo.sh run -p decider-probe -- run --data ../../.dev/decider-probe` then `... -- report --data ../../.dev/decider-probe` -> `report.md` exists with all six report sections.
- `git status --short` -> no path under `.dev/`, and no committed file holds a command from `events.jsonl`.

## Review Focus

- Decider is not running or the port is closed -> `run` records an error line per request and exits non-zero, with no panic; owner TASK-299.
- A results file has an event missing from labels, or a label missing from results -> `report` skips it and prints the count; owner TASK-299.
- A shell command holds a secret or is longer than 2,000 characters -> `extract` drops it; owner TASK-298.
- `run` is stopped halfway and run again -> it skips ids already in the results file instead of paying twice; owner TASK-299 (Decider) and TASK-298 (Jev).
- Every model answer escalates at one threshold -> precision is reported as `n/a`, not as a divide-by-zero; owner TASK-299.

## Handoff

- Next action: run `/watchtower implement` for TASK-298 and TASK-299. Then Hiep answers Q-190 before TASK-300.

## Archive

- None.
