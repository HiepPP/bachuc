# TASK-011 Dependency and ownership scheduler

Group: J (shared orchestrator engine, contracts, tests, and documentation)
Class: risky

## Brief

Goal: Schedule ready jobs and serialize overlapping file ownership and shared resources.

Change: Add this capability to the standalone Jev orchestrator plugin without changing Paseo source.

How:

- Use installed Paseo 0.8.0 APIs and copy saved profile settings exactly.
- Keep tasks inside the parent workspace and preserve unrelated changes.
- Keep authority and deterministic checks in code; Jev ranks allowed decisions only.
- Preserve errors, unknown evidence, and bounded runtime budgets.

Files:

- [Plugin](../../../../plugins/jev-orchestrator) contains server logic, shared contracts, MCP, panel, and tests.
- [Catalog](../../../../README.md) describes installation and capabilities.

Expected result: Schedule ready jobs and serialize overlapping file ownership and shared resources.

## Verify

- Run `npm --prefix plugins/jev-orchestrator run typecheck`, `run lint`, and `test`; all pass.
- Test dependency failure, cycles, exact and nested path overlap, shared resources, and independent concurrency.
- Verify restart and duplicate events cannot silently relaunch an unknown running child.
