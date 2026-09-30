# TASK-002 evidence-ledger

Group: A (shared root catalog)
Class: code

## Brief

Goal: Keep check results tied to the source and environment that produced them.

Change: Add an optional evidence-ledger plugin after the user selects this task.

How:

- Confirm selection before implementation. Read the shared context and repository rules.
- Check installed host support with the paseo-plugin skill before choosing API signatures.
- Add an explicit check runner and a workspace evidence panel.
- Record the command, exit code, time, working directory, HEAD SHA, and relevant diff fingerprint.
- Include relevant untracked source and declared check configuration in the fingerprint.
- Mark evidence stale when relevant source or configuration changes.
- Store records locally outside Git. Bound output and redact secrets.
- Show recorded checks only. Do not imply coverage of every agent tool call.
- Keep the MVP independent from other proposed plugins. Add its entry to the root catalog.

Files:

- [paseo-plugin.json](../../../../plugins/evidence-ledger/paseo-plugin.json): Declare the plugin ID and verified host requirements.
- [package.json](../../../../plugins/evidence-ledger/package.json): Declare dependencies and validation scripts.
- [package-lock.json](../../../../plugins/evidence-ledger/package-lock.json): Lock dependencies.
- [tsconfig.json](../../../../plugins/evidence-ledger/tsconfig.json): Configure TypeScript checks.
- [index.server.ts](../../../../plugins/evidence-ledger/index.server.ts): Register server contributions and cleanup.
- [index.client.tsx](../../../../plugins/evidence-ledger/index.client.tsx): Register the action or workspace panel.
- [server/](../../../../plugins/evidence-ledger/server): Keep checks, local data, and service access on the server.
- [client/](../../../../plugins/evidence-ledger/client): Render the action and its results.
- [shared/](../../../../plugins/evidence-ledger/shared): Define runtime-neutral RPC contracts.
- [tests/](../../../../plugins/evidence-ledger/tests): Cover observable success and failure cases.
- [README.md](../../../../plugins/evidence-ledger/README.md): Document configuration, permissions, limitations, and checks.
- [Root README](../../../../README.md): Add the plugin to the catalog.

Expected result:

- Keep check results tied to the source and environment that produced them.
- The plugin reports failures clearly and preserves unrelated workspace state.

## Verify

- `cd plugins/evidence-ledger && npm run typecheck && npm run lint && npm test` exits with code 0.
- Automated fixture check: Passing and failing checks retain their actual exit codes and source fingerprints.
- Automated fixture check: Tracked edits, relevant untracked edits, and declared configuration changes mark prior evidence stale.
- Automated fixture check: Records survive a plugin reload. Unchanged inputs keep the same evidence validity.
- Automated fixture check: Secret fixtures do not appear in stored output or the panel.
- Runtime check on the authorized target host: reload `evidence-ledger` and confirm `running` with no load error.
- UI check on desktop and a compact client: exercise the action, success state, and error state.
- Stop task-owned fixtures and processes. Confirm temporary settings and artifacts are cleaned up.
