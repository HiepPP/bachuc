# TASK-006 release-trace

Group: A (shared root catalog)
Class: code

## Brief

Goal: Show which revision reached each release stage.

Change: Add an optional release-trace plugin after the user selects this task.

How:

- Confirm selection before implementation. Read the shared context and repository rules.
- Check installed host support with the paseo-plugin skill before choosing API signatures.
- Add a /release-status action and a report for branch, PR head, CI, and deployed revision.
- Start with GitHub PR and Actions data plus a configured deployment version endpoint.
- Use existing authorized credentials on the server. Never expose tokens to the client.
- Compare exact SHAs. Mark missing access, absent version data, and mismatches separately.
- Keep the action read-only. Do not push, merge, rerun jobs, or deploy.
- Keep the MVP independent from other proposed plugins. Add its entry to the root catalog.

Files:

- [paseo-plugin.json](../../../../plugins/release-trace/paseo-plugin.json): Declare the plugin ID and verified host requirements.
- [package.json](../../../../plugins/release-trace/package.json): Declare dependencies and validation scripts.
- [package-lock.json](../../../../plugins/release-trace/package-lock.json): Lock dependencies.
- [tsconfig.json](../../../../plugins/release-trace/tsconfig.json): Configure TypeScript checks.
- [index.server.ts](../../../../plugins/release-trace/index.server.ts): Register server contributions and cleanup.
- [index.client.tsx](../../../../plugins/release-trace/index.client.tsx): Register the action or workspace panel.
- [server/](../../../../plugins/release-trace/server): Keep checks, local data, and service access on the server.
- [client/](../../../../plugins/release-trace/client): Render the action and its results.
- [shared/](../../../../plugins/release-trace/shared): Define runtime-neutral RPC contracts.
- [tests/](../../../../plugins/release-trace/tests): Cover observable success and failure cases.
- [README.md](../../../../plugins/release-trace/README.md): Document configuration, permissions, limitations, and checks.
- [Root README](../../../../README.md): Add the plugin to the catalog.

Expected result:

- Show which revision reached each release stage.
- The plugin reports failures clearly and preserves unrelated workspace state.

## Verify

- `cd plugins/release-trace && npm run typecheck && npm run lint && npm test` exits with code 0.
- Automated fixture check: Matching fixture SHAs show a verified revision chain.
- Automated fixture check: Successful CI with a different deployed SHA shows a deployment mismatch.
- Automated fixture check: Missing authorization and an endpoint without version data remain unverified.
- Automated fixture check: Requests do not mutate GitHub or deployment state.
- Runtime check on the authorized target host: reload `release-trace` and confirm `running` with no load error.
- UI check on desktop and a compact client: exercise the action, success state, and error state.
- Stop task-owned fixtures and processes. Confirm temporary settings and artifacts are cleaned up.
