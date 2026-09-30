# TASK-007 Delegation broker

Group: J (shared orchestrator engine, contracts, tests, and documentation)
Class: risky

## Brief

Goal: Create scoped jobs through MCP and run real Paseo children with saved profiles.

Change: Add this capability to the standalone Jev orchestrator plugin without changing Paseo source.

How:

- Use installed Paseo 0.8.0 APIs and copy saved profile settings exactly.
- Keep tasks inside the parent workspace and preserve unrelated changes.
- Keep authority and deterministic checks in code; Jev ranks allowed decisions only.
- Preserve errors, unknown evidence, and bounded runtime budgets.

Files:

- [Plugin](../../../../plugins/jev-orchestrator) contains server logic, shared contracts, MCP, panel, and tests.
- [Catalog](../../../../README.md) describes installation and capabilities.

Expected result: Create scoped jobs through MCP and run real Paseo children with saved profiles.

## Verify

- Run `npm --prefix plugins/jev-orchestrator run typecheck`, `run lint`, and `test`; all pass.
- Test profile setting preservation, invalid requests, duplicate submission, and scoped MCP access.
- Install the plugin and verify a real child result reaches its parent or remains retrievable.
