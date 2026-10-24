# NEXT

## Current Active Plan

- Title: Decider spike probe for the permission gate (UC2)
- Slug: 20261023-decider-spike-probe-uc2
- Status: ARCHIVED
- Updated: 2026-10-25

## Tracker

| Order | TASK                                       | Group | Status | Spec                                                                                                                             | Deps               | Context                                        | Notes                                                                              |
| ----- | ------------------------------------------ | ----- | ------ | -------------------------------------------------------------------------------------------------------------------------------- | ------------------ | ---------------------------------------------- | ---------------------------------------------------------------------------------- |
| 1     | TASK-298 Node extract and Jev bridge       | A     | DONE   | [watchtower/tasks/TASK-298-node-extract-and-jev-bridge.md](watchtower/tasks/TASK-298-node-extract-and-jev-bridge.md)             | -                  | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Writes the event, label, and result formats.                                       |
| 2     | TASK-299 Rust decider-probe run and report | B     | DONE   | [watchtower/tasks/TASK-299-rust-decider-probe-run-and-report.md](watchtower/tasks/TASK-299-rust-decider-probe-run-and-report.md) | TASK-298           | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Start order only; no shared file. Can build from the Interfaces block in parallel. |
| 3     | TASK-300 Run the probe and write results   | C     | DONE   | [watchtower/tasks/TASK-300-run-probe-and-write-results.md](watchtower/tasks/TASK-300-run-probe-and-write-results.md)             | TASK-298, TASK-299 | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Blocked on Q-190 labels by Hiep.                                                   |

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

- Result: Decider is no-go on the Air. It passes criteria 1 and 2 but fails criterion 3 on latency only (server p95 2897 ms), see [packages/desktop-gpui/crates/decider-probe/RESULTS.md](packages/desktop-gpui/crates/decider-probe/RESULTS.md).
- Next action: Hiep reads the most confident wrong answers in `.dev/decider-probe/report.md` and agrees with the call. Then rerun Decider on the Mac mini with `--endpoint` and the same events and labels. Rerun `extract.ts` only with `PROBE_DB=.dev/decider-probe/timeline-snapshot.sqlite`, because the live timeline changes the sample.

## Archive

- Archived: 2026-10-25 -> watchtower/archive/20261023-decider-spike-probe-uc2/
