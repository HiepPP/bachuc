# TASK-012 Outcome

## Outcome

Status: DONE

Changed: Persist bounded check-backed metrics, invalidate changed profiles, and use comparable history only for near-tied candidates. Added reproducible live baseline/routed fixtures.

Verified: All 24 tests, format, typecheck and lint pass. Final live pairs passed external checks. Interval: baseline 45.272 s, routed 150.765 s. Cache: baseline 41.840 s, routed 316.600 s with extra discovery/review. Fresh-agent MCP status completed successfully.

Effectiveness: Autonomous dispatch, phase transitions and checks work. Speed/cost improvement is not demonstrated; routed workflows were slower on these small fixtures. Escalation/scheduling branches have offline coverage; production calibration remains unmeasured.

Evidence: [Full verification and preserved failures](../../../../plugins/jev-orchestrator/VERIFICATION.md).

Cleanup: All 17 test agents and nine test-only local workspace records archived without deleting directories. Plugin remains running; ignored synthetic evidence retained.

Commit/push: none. TASK-004 remains separately BLOCKED.

## Whole-workflow accounting follow-up

Replaced lastUsage-based token measurements with native session totals and a per-call Jev ledger.
Counts fresh parent submission/completion, every child and every evaluator call; null means missing.
Fixed late Claude transcript flush so idle alone cannot prove complete accounting.

Verified: 34 tests, typecheck/lint, installed reload, and a fresh two-case benchmark. After closing
all actors, totals remain 90,057 vs 456,093 for intervals and 90,179 vs 1,006,360 for cache.
All arms passed checks. Orchestrator used 5.06× and 11.16× the reported tokens in these samples.
Cache/reasoning breakdown for Jev is unavailable; total input/output coverage is complete.

Eight benchmark agents and four test workspaces archived. No directory removal, commit or push.
See the [verification report](../../../../plugins/jev-orchestrator/VERIFICATION.md) for per-component counts,
cache details, execution/accounting hashes, scope and preserved intermediate measurements.

## Direct model and effort benchmark

The new route removes the parent LLM and extra discovery/review stages.
Two matched fixtures ran twice per arm. Both arms passed 4/4 independent checks.
Baseline used 361,663 tokens; direct routing used 768,645, including all 4,324 Jev tokens.
Mean task times were 45.6115 and 56.97925 seconds. Direct routing used 2.1253 times the tokens.
Jev selected Luna/low every time. Luna used 26 model requests; Astra/low baseline used 12.

All eight native session totals matched after archive. All eight test workspaces were archived.
The source hash stayed unchanged during the run. No answers were discarded or retried.

Conclusion: Direct routing works, but this policy does not meet the user's efficiency objective.
Keep the fixed baseline for comparable tasks. Future ranking needs measured whole-workflow behavior.
See [evidence and limitations](../../../../plugins/jev-orchestrator/VERIFICATION.md#direct-modeleffort-routing--2026-09-21).
