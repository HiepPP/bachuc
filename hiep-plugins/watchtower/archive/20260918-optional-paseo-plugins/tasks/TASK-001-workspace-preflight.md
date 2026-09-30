# TASK-001 workspace-preflight

Group: A (shared root catalog)
Class: code

## Brief

Goal: Detect workspace blockers before an agent starts work.

Change: Add an optional workspace-preflight plugin after the user selects this task.

How:

- Confirm selection before implementation. Read the shared context and repository rules.
- Check installed host support with the paseo-plugin skill before choosing API signatures.
- Add a /preflight action with a workspace report.
- Read explicit repository settings for runtime, dependencies, ports, and health endpoints.
- Report pass, blocker, or unknown for each check, with a suggested repair command.
- Use bounded read-only checks. Do not install dependencies, change files, or stop services.
- Keep the MVP independent from other proposed plugins. Add its entry to the root catalog.

Files:

- [paseo-plugin.json](../../../../plugins/workspace-preflight/paseo-plugin.json): Declare the plugin ID and verified host requirements.
- [package.json](../../../../plugins/workspace-preflight/package.json): Declare dependencies and validation scripts.
- [package-lock.json](../../../../plugins/workspace-preflight/package-lock.json): Lock dependencies.
- [tsconfig.json](../../../../plugins/workspace-preflight/tsconfig.json): Configure TypeScript checks.
- [index.server.ts](../../../../plugins/workspace-preflight/index.server.ts): Register server contributions and cleanup.
- [index.client.tsx](../../../../plugins/workspace-preflight/index.client.tsx): Register the action or workspace panel.
- [server/](../../../../plugins/workspace-preflight/server): Keep checks, local data, and service access on the server.
- [client/](../../../../plugins/workspace-preflight/client): Render the action and its results.
- [shared/](../../../../plugins/workspace-preflight/shared): Define runtime-neutral RPC contracts.
- [tests/](../../../../plugins/workspace-preflight/tests): Cover observable success and failure cases.
- [README.md](../../../../plugins/workspace-preflight/README.md): Document configuration, permissions, limitations, and checks.
- [Root README](../../../../README.md): Add the plugin to the catalog.

Expected result:

- Detect workspace blockers before an agent starts work.
- The plugin reports failures clearly and preserves unrelated workspace state.

## Verify

- `cd plugins/workspace-preflight && npm run typecheck && npm run lint && npm test` exits with code 0.
- Automated fixture check: A ready fixture reports pass for every configured check.
- Automated fixture check: Missing dependencies, a wrong runtime, and an unhealthy endpoint each report the correct blocker.
- Automated fixture check: A timeout reports unknown or blocker without hanging. No repair command runs automatically.
- Runtime check on the authorized target host: reload `workspace-preflight` and confirm `running` with no load error.
- UI check on desktop and a compact client: exercise the action, success state, and error state.
- Stop task-owned fixtures and processes. Confirm temporary settings and artifacts are cleaned up.
