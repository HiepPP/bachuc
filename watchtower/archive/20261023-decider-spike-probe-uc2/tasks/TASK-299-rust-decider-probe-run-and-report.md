# TASK-299 Rust decider-probe run and report

Group: B (writes `packages/desktop-gpui/crates/decider-probe/Cargo.toml`, `src/**`, `README.md`, and `packages/desktop-gpui/Cargo.lock`)
Class: code

## Brief

Goal: A Rust CLI that sends each event to Decider one question at a time, then scores Decider and Jev results against the labels.

Change: no crate -> `decider-probe run` and `decider-probe report`.

Boundaries:

- Follows [ADR-0066](watchtower/decisions/ADR-0066-standalone-rust-decider-probe-over-jev-evaluator-backend.md): no `gpui-kit` dependency and no dependency on other workspace crates.
- Planner call: use `ureq` (blocking) and `serde_json`. Requests are sequential, because the Decider server queues them anyway. Do not add `tokio` or `reqwest`.
- Before the first request, send one warm-up request and write no result line for it. The first requests after idle took about 3 s on 2026-10-23.
- Read `input_tokens` from `usage.input_tokens`. Read a `noul` answer from the `noul` field and write it as `p_true`. Read a `choice` answer from `choice` and `probabilities`.
- Planner call: if a Decider response lacks an expected field, write the line with `error` set to the parse error.
- One question per Decider request. Send `{"model":"pplx-decider-flash-v1.2","state":<event.state>,"questions":{<qid>:<question>}}`.
- `server_ms`: before each request, record the length of `decider.log`. After the response, read the new bytes and parse `ms=` from the last `decided` line. Use `null` when the log is missing or has no new line.
- `run` skips ids and question ids already in `results-decider.jsonl`. It exits non-zero when any request in the run failed.
- `report` replays `fromVerdict` from [hiep-plugins/plugins/jev-permission-gate/server/policy.ts](hiep-plugins/plugins/jev-permission-gate/server/policy.ts) for each threshold. It reports two views per backend: model only on events with `meta.tier0` `null`, and the full gate where a non-null `tier0` wins.
- Trap: `confidence` in a Decider choice answer is `(p_top - 1/n) / (1 - 1/n)`, not `p_top`. Use `probabilities[choice]` as the action probability.
- Trap: `packages/desktop-gpui/cargo.sh` is the only cargo entry. Plain `cargo` builds a second copy of every crate.

How:

- `src/main.rs`: parse `run` and `report` with `--data <dir>`, `--endpoint <url>` (default from [watchtower/CONTEXT.md](watchtower/CONTEXT.md)), `--questions <path>`, and `--log <path>`. Use `std::env::args`; no clap.
- `src/client.rs`: `ask(endpoint, state, qid, question) -> Result<Answer>` and the `decider.log` tail read.
- `src/metrics.rs`: pure functions `percentile`, `ece`, `gate_decision`, `sweep`.
- `src/report.rs`: write `report.md` with six sections: latency p50 and p95 by input-token bucket (<150, 150-400, 400-1000, >1000) with client and server ms; accuracy and per-class precision and recall at t = 0.9; calibration in 5 bins plus ECE for `readOnly` and `action`; threshold sweep from 0.50 to 0.99 with auto-resolve % and precision, plus the lowest t with precision >= 0.95; the 10 most confident wrong answers; and a regex baseline row plus a go/no-go table with the three criteria from [watchtower/CONTEXT.md](watchtower/CONTEXT.md).

Files:

- [packages/desktop-gpui/crates/decider-probe/Cargo.toml](packages/desktop-gpui/crates/decider-probe/Cargo.toml) (new)
- [packages/desktop-gpui/crates/decider-probe/src/main.rs](packages/desktop-gpui/crates/decider-probe/src/main.rs) (new)
- [packages/desktop-gpui/crates/decider-probe/src/client.rs](packages/desktop-gpui/crates/decider-probe/src/client.rs) (new)
- [packages/desktop-gpui/crates/decider-probe/src/metrics.rs](packages/desktop-gpui/crates/decider-probe/src/metrics.rs) (new, with unit tests)
- [packages/desktop-gpui/crates/decider-probe/src/report.rs](packages/desktop-gpui/crates/decider-probe/src/report.rs) (new)
- [packages/desktop-gpui/crates/decider-probe/README.md](packages/desktop-gpui/crates/decider-probe/README.md) (new: the run order of extract, label, run, Jev run, report)
- [packages/desktop-gpui/Cargo.lock](packages/desktop-gpui/Cargo.lock) (cargo adds `ureq`)

Interfaces:

- Consumes: `questions.json` = `{"version":"uc2-v1","policy":string,"questions":{"readOnly":{"type":"noul","instructions":string},"action":{"type":"choice","instructions":string,"criteria":{"allow":string,"deny":string,"escalate":string}}}}` from TASK-298
- Consumes: `.dev/decider-probe/events.jsonl`, one line per event = `{"id":"e001","uc":"uc2","state":{"policy":string,"command":string,"tool":"Bash","cwd_relative":"."},"meta":{"tier0":"allow"|"deny"|"escalate"|null,"agent_id":string,"ts":string}}` from TASK-298
- Consumes: `.dev/decider-probe/labels.jsonl`, one line per event = `{"id":"e001","uc":"uc2","label":"allow"|"deny"|"escalate","read_only":boolean}` from TASK-298
- Consumes: result line = `{"backend":"decider"|"jev","id":"e001","question_id":"readOnly"|"action"|"both","question_version":"uc2-v1","input_tokens":number|null,"client_ms":number,"server_ms":number|null,"answers":{"readOnly"?:{"p_true":number},"action"?:{"choice":"allow"|"deny"|"escalate","probabilities":{"allow":number,"deny":number,"escalate":number}}},"error":string|null}` from TASK-298
- Produces for TASK-300: `.dev/decider-probe/results-decider.jsonl` with the result line shape above, `backend` `"decider"`, and `question_id` `"readOnly"` or `"action"`.
- Produces for TASK-300: `.dev/decider-probe/report.md`.
- Produces for TASK-300: commands `./cargo.sh run -p decider-probe -- run --data <dir>` and `./cargo.sh run -p decider-probe -- report --data <dir>`, run from `packages/desktop-gpui`.

Expected result:

- WHEN `run` gets an event THE probe SHALL send two requests, one per question, and append two result lines.
- WHEN `run` starts and a result line exists for an id and question id THE probe SHALL skip that request.
- IF the endpoint refuses the connection THEN THE probe SHALL write a result line with `error` set, go on, and exit non-zero.
- WHEN `gate_decision` gets an `allow` answer at P >= t and `p_true` >= t THE function SHALL return allow.
- WHEN `gate_decision` gets `p_true` < 0.5 THE function SHALL return escalate for an `allow` answer, at any t.
- WHEN `report` runs on labels and results THE probe SHALL write `report.md` with the six sections in order.
- IF an id is in results but not in labels THEN THE report SHALL skip it and print the skipped count.
- IF no event is auto-resolved at a threshold THEN THE sweep row SHALL show precision as `n/a`.

Red tests:

File: [packages/desktop-gpui/crates/decider-probe/src/metrics.rs](packages/desktop-gpui/crates/decider-probe/src/metrics.rs)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allow_needs_both_answers_over_threshold() {
        assert_eq!(gate_decision(0.95, Decision::Allow, 0.93, 0.9), Decision::Allow);
        assert_eq!(gate_decision(0.85, Decision::Allow, 0.93, 0.9), Decision::Escalate);
        assert_eq!(gate_decision(0.95, Decision::Allow, 0.80, 0.9), Decision::Escalate);
    }

    #[test]
    fn read_only_false_never_allows() {
        assert_eq!(gate_decision(0.02, Decision::Allow, 0.99, 0.5), Decision::Escalate);
    }

    #[test]
    fn deny_needs_only_action_over_threshold() {
        assert_eq!(gate_decision(0.99, Decision::Deny, 0.91, 0.9), Decision::Deny);
        assert_eq!(gate_decision(0.99, Decision::Deny, 0.89, 0.9), Decision::Escalate);
    }

    #[test]
    fn percentile_uses_nearest_rank() {
        let mut v = vec![5.0, 1.0, 3.0, 2.0, 4.0];
        assert_eq!(percentile(&mut v, 50.0), 3.0);
        assert_eq!(percentile(&mut v, 95.0), 5.0);
    }

    #[test]
    fn ece_is_zero_when_calibrated_and_positive_when_not() {
        let calibrated = [(0.9, true); 9]
            .into_iter()
            .chain([(0.9, false)])
            .collect::<Vec<_>>();
        assert!(ece(&calibrated, 5) < 1e-9);
        let over = vec![(0.9, false), (0.9, false)];
        assert!((ece(&over, 5) - 0.9).abs() < 1e-9);
    }

    #[test]
    fn sweep_reports_none_precision_when_nothing_resolves() {
        let row = sweep(&[(Decision::Escalate, Decision::Allow)]);
        assert_eq!(row.auto_resolved, 0);
        assert_eq!(row.precision, None);
    }
}
```

Signatures these tests fix: `enum Decision { Allow, Deny, Escalate }` with `PartialEq, Debug, Clone, Copy`; `fn gate_decision(read_only_p_true: f64, action: Decision, action_p: f64, t: f64) -> Decision`; `fn percentile(values: &mut [f64], p: f64) -> f64` (nearest rank); `fn ece(points: &[(f64, bool)], bins: usize) -> f64`; `fn sweep(pairs: &[(Decision, Decision)]) -> SweepRow` where each pair is (predicted, label) and `SweepRow { auto_resolved: usize, precision: Option<f64> }`.

Anti-goal: `decider-probe` pulls in 0 GPUI crates; tripwire; read from `./cargo.sh tree -p decider-probe | rg -c gpui` before the first commit and at completion.

## Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p decider-probe` -> the 6 metrics tests pass.
- `./cargo.sh clippy -p decider-probe -- -D warnings` -> no warnings.
- With Decider running and a 3-line `events.jsonl` fixture in a temp dir: `./cargo.sh run -p decider-probe -- run --data <tmp>` -> 6 result lines, each with `server_ms` set; a second run adds 0 lines.
- With `--endpoint http://127.0.0.1:9` -> each line has `error` set, and the exit code is non-zero.
- With a fixture of labels and results for both backends: `report --data <tmp>` -> `report.md` has the six section headings in order, and one sweep row shows `n/a`.
- With a fixture where one result id has no label and one label has no result: `report --data <tmp>` -> stdout prints both skipped counts, and `report.md` still writes.
- Anti-goal: `./cargo.sh tree -p decider-probe | tail -n +2 | rg -c gpui` -> no match (the root line holds the repo path, which contains `gpui`).
- Cleanup per [CLAUDE.md](CLAUDE.md): `./cargo.sh clean --profile dev` when no other `cargo` or `rustc` process runs.
