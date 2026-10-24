# decider-probe results

UC2 (permission gate) probe of local Decider against hosted Jev. Numbers come from the ignored
`.dev/decider-probe/report.md`; the most confident wrong answers stay there.

## Run

- Host: MacBook Air M4 (Mac16,12), 24 GB, on power.
- Decider model: `pplx-decider-flash-gguf-v1.2` (from `decider.log`), endpoint `http://127.0.0.1:8080/v1/decisions`.
- Question version: `uc2-v1`.
- Runs: 2026-10-23. Report: 2026-10-25.
- Events: 150, labeled. The labels started from agent suggestions; Hiep reviewed them and changed none.
- Lines: Decider 300 (two questions per event), Jev 150. Errored lines: 0 for both.

## Go/no-go

Threshold t = 0.9, full gate view (tier0 regex wins, the model decides the rest).

| Criterion                                     | Decider               | Jev                   |
| --------------------------------------------- | --------------------- | --------------------- |
| 1. Auto-resolve >= 50% at precision >= 0.95   | PASS (60.0%, 0.989)   | PASS (54.0%, 1.000)   |
| 2. Beats regex alone at precision >= 0.95     | PASS (90 vs 1, 0.989) | PASS (81 vs 1, 1.000) |
| 3. Server p95 <= 2000 ms, state <= 400 tokens | FAIL (2897 ms)        | n/a (hosted)          |

Decider is no-go on the Air. It fails on latency only, so the next step is a rerun on the Mac mini
with `--endpoint`.

## Summary

| Metric                          | Decider       | Jev           |
| ------------------------------- | ------------- | ------------- |
| Accuracy, model only (n = 120)  | 91.7%         | 85.8%         |
| Accuracy, full gate (n = 150)   | 81.3%         | 76.7%         |
| Escalate precision, full gate   | 0.550         | 0.493         |
| ECE readOnly / action           | 0.037 / 0.013 | 0.075 / 0.027 |
| Lowest t with precision >= 0.95 | 0.50          | 0.50          |

Latency in ms, p50 / p95:

| Input tokens                            | Decider client | Decider server | Jev client |
| --------------------------------------- | -------------- | -------------- | ---------- |
| 150-400 (n = 126)                       | 2098 / 2900    | 2094 / 2897    | -          |
| 400-1000 (Decider n = 174, Jev n = 149) | 2571 / 3875    | 2566 / 3860    | 401 / 1217 |

Client minus server latency stays under 200 ms on all 300 Decider lines, so the model, not the
client, sets the latency.
