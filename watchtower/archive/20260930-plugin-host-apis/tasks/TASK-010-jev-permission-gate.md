# TASK-010 P4 jev-permission-gate uses before agent.permission

Group: D (hiep-plugins/plugins/jev-permission-gate)
Class: code

## Brief

Goal: Answer shell permissions before the user sees them.

Change: `server.on("agent.permission_requested")` plus `respondToPermission` -> `server.before("agent.permission")` that sets `decision`.

Boundaries:

- Keep the regex rules, the Jev judge, the ledger, and `HOOK_BUDGET_MS`.
- `escalate` returns no decision.
- Raise `requirements.paseo` to `>=0.10.2-beta.900`.

How:

- Replace the event handler in [hiep-plugins/plugins/jev-permission-gate/index.server.ts](hiep-plugins/plugins/jev-permission-gate/index.server.ts).
- Link the local [packages/plugin](packages/plugin) types for typecheck.
- Update tests and the README.

Files:

- [hiep-plugins/plugins/jev-permission-gate/index.server.ts](hiep-plugins/plugins/jev-permission-gate/index.server.ts) (hook)
- [hiep-plugins/plugins/jev-permission-gate/paseo-plugin.json](hiep-plugins/plugins/jev-permission-gate/paseo-plugin.json) (requirements)
- [hiep-plugins/plugins/jev-permission-gate/package.json](hiep-plugins/plugins/jev-permission-gate/package.json) (local SDK link)
- [hiep-plugins/plugins/jev-permission-gate/README.md](hiep-plugins/plugins/jev-permission-gate/README.md) (behavior)
- Tests under [hiep-plugins/plugins/jev-permission-gate/tests](hiep-plugins/plugins/jev-permission-gate/tests)

Expected result:

- An allowed command is answered with no permission prompt shown.
- An uncertain command shows the normal prompt.

Anti-goal: Gate decisions stay the same; tripwire; limit every existing test in [hiep-plugins/plugins/jev-permission-gate/tests](hiep-plugins/plugins/jev-permission-gate/tests) that covers `gate` passes unchanged; read before changes and at completion.

## Verify

- `cd hiep-plugins/plugins/jev-permission-gate && npm run typecheck && npm run lint && npm test` -> exit 0.
- Real host check in Paseo Dev (TASK-017): a safe shell command runs with no prompt.
- Anti-goal: gate tests pass without edits.
