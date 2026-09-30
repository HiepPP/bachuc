# TASK-005 watchtower-board

Group: A (shared root catalog)
Class: code

## Brief

Goal: Show repository Watchtower tasks inside Paseo.

Change: Add an optional watchtower-board plugin after the user selects this task.

How:

- Confirm selection before implementation. Read the shared context and repository rules.
- Check installed host support with the paseo-plugin skill before choosing API signatures.
- Read the active Watchtower manifest and linked task specs into a workspace panel.
- Show task status, dependencies, and recorded blockers.
- Let the user attach a selected task brief to the composer. Do not submit it automatically.
- Keep the first version read-only. Do not change task status or planning files.
- Handle missing files and unsupported legacy formats with a clear message.
- Keep the MVP independent from other proposed plugins. Add its entry to the root catalog.

Files:

- [paseo-plugin.json](../../../../plugins/watchtower-board/paseo-plugin.json): Declare the plugin ID and verified host requirements.
- [package.json](../../../../plugins/watchtower-board/package.json): Declare dependencies and validation scripts.
- [package-lock.json](../../../../plugins/watchtower-board/package-lock.json): Lock dependencies.
- [tsconfig.json](../../../../plugins/watchtower-board/tsconfig.json): Configure TypeScript checks.
- [index.server.ts](../../../../plugins/watchtower-board/index.server.ts): Register server contributions and cleanup.
- [index.client.tsx](../../../../plugins/watchtower-board/index.client.tsx): Register the action or workspace panel.
- [server/](../../../../plugins/watchtower-board/server): Keep checks, local data, and service access on the server.
- [client/](../../../../plugins/watchtower-board/client): Render the action and its results.
- [shared/](../../../../plugins/watchtower-board/shared): Define runtime-neutral RPC contracts.
- [tests/](../../../../plugins/watchtower-board/tests): Cover observable success and failure cases.
- [README.md](../../../../plugins/watchtower-board/README.md): Document configuration, permissions, limitations, and checks.
- [Root README](../../../../README.md): Add the plugin to the catalog.

Expected result:

- Show repository Watchtower tasks inside Paseo.
- The plugin reports failures clearly and preserves unrelated workspace state.

## Verify

- `cd plugins/watchtower-board && npm run typecheck && npm run lint && npm test` exits with code 0.
- Automated fixture check: A fixture manifest displays every task with its correct status and dependencies.
- Automated fixture check: Selecting a task attaches its correct brief without sending a prompt.
- Automated fixture check: Missing specs and legacy formats show a useful message without crashing.
- Automated fixture check: Opening, refreshing, and selecting tasks leave repository files unchanged.
- Runtime check on the authorized target host: reload `watchtower-board` and confirm `running` with no load error.
- UI check on desktop and a compact client: exercise the action, success state, and error state.
- Stop task-owned fixtures and processes. Confirm temporary settings and artifacts are cleaned up.
