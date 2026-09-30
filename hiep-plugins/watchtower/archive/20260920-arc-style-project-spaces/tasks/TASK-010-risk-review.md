# TASK-010 Risk-based review

Group: J (shared orchestrator engine, contracts, tests, and documentation)
Class: risky

## Brief

Goal: Run an independent read-only review for risky changes and preserve deterministic verification.

Change: Add this capability to the standalone Jev orchestrator plugin without changing Paseo source.

How:

- Use installed Paseo 0.8.0 APIs and copy saved profile settings exactly.
- Keep tasks inside the parent workspace and preserve unrelated changes.
- Keep authority and deterministic checks in code; Jev ranks allowed decisions only.
- Preserve errors, unknown evidence, and bounded runtime budgets.

Files:

- [Plugin](../../../../plugins/jev-orchestrator) contains server logic, shared contracts, MCP, panel, and tests.
- [Catalog](../../../../README.md) describes installation and capabilities.

Expected result: Run an independent read-only review for risky changes and preserve deterministic verification.

## Verify

- Run `npm --prefix plugins/jev-orchestrator run typecheck`, `run lint`, and `test`; all pass.
- Test risky work requires independent review while low-risk work can skip it.
- Verify failed deterministic checks cannot be overridden by a positive model judgment.
