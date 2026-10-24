# TASK-300 Outcome

## Status

DONE

## Changed

- `packages/desktop-gpui/crates/decider-probe/RESULTS.md` (new): host, model, question version, dates, go/no-go table, summary numbers. No command text.
- `watchtower/NEXT.md`, `watchtower/QUESTIONS.md`: TASK-300 DONE, Q-190 ANSWERED, handoff.
- Data (ignored, not staged): `.dev/decider-probe/labels.jsonl`, `report.md`.

## Result

- Decider (`pplx-decider-flash-gguf-v1.2`, MacBook Air M4 24 GB): criterion 1 PASS (60.0%, 0.989), 2 PASS (90 vs 1), 3 FAIL (server p95 2897 ms for <= 400 tokens). No-go, latency only, so the Mac mini rerun is next.
- Jev: criteria 1 and 2 PASS (54.0%, 1.000; 81 vs 1); criterion 3 does not apply.

## Verified

- `results-decider.jsonl` 300 lines, `results-jev.jsonl` 150 lines; errored lines 0 and 0.
- `report` -> 150 scored events per backend, 0 skipped ids, 0 missing labels.
- `rg -F -f <(jq -r .state.command .dev/decider-probe/events.jsonl) RESULTS.md` -> no match.
- `npm run format:check` -> clean after `npm run format:files` on `RESULTS.md`.
- Human check: PASS. On 2026-10-25 Hiep agreed with the no-go call.

## Anti-goal

Decider lines with `client_ms - server_ms > 200`: 0 of 300 (limit 15). Read from `results-decider.jsonl` on 2026-10-25. PASS.

## Lessons

- Both runs finished on 2026-10-23, before labeling. Only `report` needs labels, so TASK-300 needed no new requests and no Jev cost.
- `labels.jsonl` equals `labels.suggested.jsonl`, an agent draft that Hiep reviewed without edits. The ground truth is agent-assisted, not labeled from scratch.
