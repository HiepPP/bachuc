# decider-probe

Measures whether local Decider can back the permission gate (UC2) instead of hosted Jev. The crate
is standalone (ADR-0066): no `gpui-kit`, no other workspace crate. It speaks plain http only.

Run from the repo root unless noted. Data lives in `.dev/decider-probe/`, which git ignores.

1. Extract events: `node packages/desktop-gpui/crates/decider-probe/scripts/extract.ts`
   writes `events.jsonl` and `labels.draft.jsonl`.
2. Label: copy `labels.draft.jsonl` to `labels.jsonl` and fill `label` and `read_only` for every
   event. The human label is ground truth.
3. Run Decider (from `packages/desktop-gpui`):
   `./cargo.sh run -p decider-probe -- run --data ../../.dev/decider-probe`
   Two requests per event, one per question, after one unlogged warm-up request. It skips ids and
   question ids already in `results-decider.jsonl` and exits non-zero if any request failed. To retry
   a failed request, delete its line from the results file.
4. Run Jev: `node packages/desktop-gpui/crates/decider-probe/scripts/jev-run.ts` (each call is billable) writes `results-jev.jsonl`.
5. Report: `./cargo.sh run -p decider-probe -- report --data ../../.dev/decider-probe` writes
   `report.md` and prints how many result ids and labels it skipped.

Flags: `--data <dir>`, `--endpoint <url>` (default `http://127.0.0.1:8080/v1/decisions`),
`--questions <path>` (default `questions.json` in this crate), `--log <path>` (default the Decider
`decider.log`, read for `server_ms`).
