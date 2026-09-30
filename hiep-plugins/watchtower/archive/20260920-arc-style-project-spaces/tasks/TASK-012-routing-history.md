# TASK-012 Measured routing history

Group: J (shared orchestrator engine, contracts, tests, and documentation)
Class: risky

## Brief

Goal: Persist bounded verified outcome metrics and compare live baseline and routed tasks.

Change: Add this capability to the standalone Jev orchestrator plugin without changing Paseo source.

How:

- Use installed Paseo 0.8.0 APIs and copy saved profile settings exactly.
- Keep tasks inside the parent workspace and preserve unrelated changes.
- Keep authority and deterministic checks in code; Jev ranks allowed decisions only.
- Preserve errors, unknown evidence, and bounded runtime budgets.

Files:

- [Plugin](../../../../plugins/jev-orchestrator) contains server logic, shared contracts, MCP, panel, and tests.
- [Catalog](../../../../README.md) describes installation and capabilities.

Expected result: Persist bounded verified outcome metrics and compare live baseline and routed tasks.

## Verify

- Run `npm --prefix plugins/jev-orchestrator run typecheck`, `run lint`, and `test`; all pass.
- Test metrics exclude self-reported success and require actual configured check results.
- Run matched synthetic coding tasks through a baseline and live routed profiles; record every attempt and failure.
- Report selected provider/model/effort, wall time, usage availability, success, and limits; clean task-owned agents.
