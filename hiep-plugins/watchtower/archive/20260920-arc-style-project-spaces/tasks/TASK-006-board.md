# TASK-006 Board running and finished tasks

Group: A (shared root catalog with existing plugin tasks)
Class: code

## Brief

Goal: Create a plugin named Board that shows running tasks and recently finished runs.

Change: Add a Board page with its sidebar menu immediately below Schedules.

How:

- Implement the imagegen UI concept approved by the user on 2026-09-20.
- Start with two columns: Running and Just finished. Show title, project, agent, status, and available timing.
- Represent each Paseo agent conversation as one item. New turns move the same item between columns.
- Use the paseo-plugin skill to verify installed host APIs, lifecycle meanings, and sidebar ordering support.
- If exact menu placement requires unsupported integration, report that limit before choosing a workaround.
- Populate from real host state. Update running and finished cards without manual reload; sort finished runs newest first.
- Distinguish successful and failed runs. Do not label an idle agent as a successful task without completion evidence.
- Handle empty, loading, and error states. Keep recent history bounded and document retention and reconnect behavior.
- Allow removing finished cards from Board only. Preserve agents, chat history, and running cards.
- Clean up subscriptions when disabled and preserve existing Spaces behavior.

Files:

- [Plugin manifest](../../../../plugins/board/paseo-plugin.json), [package](../../../../plugins/board/package.json), [lockfile](../../../../plugins/board/package-lock.json), and [TypeScript config](../../../../plugins/board/tsconfig.json).
- [Client entry](../../../../plugins/board/index.client.tsx) and [client modules](../../../../plugins/board/client): page and navigation.
- [Server entry](../../../../plugins/board/index.server.ts), [server modules](../../../../plugins/board/server), and [shared contracts](../../../../plugins/board/shared), only where required by verified APIs.
- [Tests](../../../../plugins/board/tests), [plugin README](../../../../plugins/board/README.md), and [root catalog](../../../../README.md).

Expected result: Approved Board UI shows current runs and recent outcomes, with its menu immediately below Schedules.

## Verify

- Human gate: user approves the generated UI before plugin implementation begins.
- Run plugin format, typecheck, lint, and focused tests; all exit successfully.
- Test running-to-finished transitions, failures, duplicate events, reconnects, and bounded recent history.
- Live UI: Board appears immediately below Schedules and opens the approved layout.
- Live UI: a known run appears in Running, then moves to Just finished with its observed outcome.
- Verify empty, loading, and error states. Verify existing Spaces navigation still works.
- Reload the plugin on the intended host; confirm running status, no load error, and subscription cleanup on disable.
- Record unsupported clients and unverified behavior. Restore task-owned test state.
- Remove a finished card; verify it disappears after refresh while the agent and chat remain available.
