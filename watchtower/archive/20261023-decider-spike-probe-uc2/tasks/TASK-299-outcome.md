# TASK-299 outcome

## Status

DONE

## Changed

- `packages/desktop-gpui/crates/decider-probe/Cargo.toml` (serde, serde_json, ureq 3 with default features off)
- `packages/desktop-gpui/crates/decider-probe/src/main.rs`, `client.rs`, `metrics.rs`, `report.rs`
- `packages/desktop-gpui/crates/decider-probe/README.md`
- `packages/desktop-gpui/Cargo.lock` (adds decider-probe, ureq, ureq-proto, utf8-zero)

## Contract

- `decider-probe run|report --data <dir> [--endpoint] [--questions] [--log]`; `std::env::args`, no clap.
- Metrics signatures match the red tests: `Decision`, `gate_decision`, `percentile`, `ece`, `sweep`, `SweepRow`. Extra pure helpers: `calibration_bins`, `class_precision_recall`.
- `run` writes `results-decider.jsonl` lines in the TASK-298 result shape (`backend` `decider`, `question_id` `readOnly` or `action`). Errored lines carry `error` and empty `answers`.
- `report` reads `events.jsonl` (optional), `labels.jsonl`, `results-decider.jsonl`, `results-jev.jsonl` and writes `report.md` with the six sections in order.

## Verified

- Red: with only the test module in `metrics.rs`, `./cargo.sh test -p decider-probe` failed to compile (25 errors: `Decision`, `gate_decision`, `percentile`, `ece`, `sweep` not found).
- Green: `./cargo.sh test -p decider-probe` -> 9 passed (6 metrics, 3 client parse tests).
- `./cargo.sh clippy -p decider-probe --no-deps -- -D warnings` -> clean (after 2 `then_some` fixes).
- `./cargo.sh fmt --check -p decider-probe` -> exit 0.
- Live Decider, 3-event temp fixture: `run` -> 6 lines, all `server_ms` set (1093-1393), all `error` null; second run -> "nothing to do", still 6 lines.
- `--endpoint http://127.0.0.1:9`: 6 lines each with `error` "Connection refused", exit 1.
- Report fixture (5 events, both backends): six `##` headings in order; a Jev sweep row at t = 0.99 shows `n/a` precision.
- Skip fixture (1 result id without label, 1 label without complete result): stdout prints both counts per backend; `report.md` still written.

## Anti-goal

- `./cargo.sh tree -p decider-probe | rg -c gpui`: raw output 1, the root line only, because the path `.../desktop-gpui/...` contains "gpui". With the root line removed (`tail -n +2`), 0 matches (exit 1), measured at completion on 2026-10-23; the dependency tree is serde, serde_json, ureq. Verify line deviates literally; the intent (0 GPUI crates) holds. No tripwire crossed.

## Lessons

- The `gpui` grep matches the root line because the repo path has `gpui` in it; use `tail -n +2`.
- A `ureq` 3 build with default features off has no TLS, so the probe supports http endpoints only.
- `server_ms` needs a short retry loop (up to 3 x 50 ms) because the log line can land just after the response.
- Questions go out in `questions.json` key order, so `action` is sent before `readOnly`.

## Questions

- Safe default taken: `run` skips every id and question id already in the results file, including errored lines; a failed request is retried only after its line is deleted (README says so).
- Safe default taken: `report` treats an event missing from `events.jsonl` as `tier0` null and the command text as empty.
