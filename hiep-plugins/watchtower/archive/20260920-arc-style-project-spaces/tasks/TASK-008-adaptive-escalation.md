# TASK-008 Adaptive escalation

Group: J (shared orchestrator engine, contracts, tests, and documentation)
Class: risky

## Brief

Goal: Classify failed attempts and select a different valid profile within a bounded attempt budget.

Change: Add this capability to the standalone Jev orchestrator plugin without changing Paseo source.

How:

- Use installed Paseo 0.8.0 APIs and copy saved profile settings exactly.
- Keep tasks inside the parent workspace and preserve unrelated changes.
- Keep authority and deterministic checks in code; Jev ranks allowed decisions only.
- Preserve errors, unknown evidence, and bounded runtime budgets.

Files:

- [Plugin](../../../../plugins/jev-orchestrator) contains server logic, shared contracts, MCP, panel, and tests.
- [Catalog](../../../../README.md) describes installation and capabilities.

Expected result: Classify failed attempts and select a different valid profile within a bounded attempt budget.

## Verify

- Run `npm --prefix plugins/jev-orchestrator run typecheck`, `run lint`, and `test`; all pass.
- Test environment failures never trigger blind model escalation.
- Test attempt limits, explicit profile preservation, unavailable profiles, and cancellation.
