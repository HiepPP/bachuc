# TASK-004 preview-launcher

Group: A (shared root catalog)
Class: risky

## Brief

Goal: Start a configured workspace preview and show its readiness.

Change: Add an optional preview-launcher plugin after the user selects this task.

How:

- Confirm selection before implementation. Read the shared context and repository rules.
- Check installed host support with the paseo-plugin skill before choosing API signatures.
- Read available repository scripts and let the user choose the launch command.
- Use a configured URL and bounded health check before reporting ready.
- Show startup output, failures, and the preview URL in a workspace panel.
- Track process ownership. Stop only processes started by this plugin.
- Reuse a matching healthy preview. Never kill a process to reclaim a port.
- Keep the MVP independent from other proposed plugins. Add its entry to the root catalog.

Files:

- [paseo-plugin.json](../../../../plugins/preview-launcher/paseo-plugin.json): Declare the plugin ID and verified host requirements.
- [package.json](../../../../plugins/preview-launcher/package.json): Declare dependencies and validation scripts.
- [package-lock.json](../../../../plugins/preview-launcher/package-lock.json): Lock dependencies.
- [tsconfig.json](../../../../plugins/preview-launcher/tsconfig.json): Configure TypeScript checks.
- [index.server.ts](../../../../plugins/preview-launcher/index.server.ts): Register server contributions and cleanup.
- [index.client.tsx](../../../../plugins/preview-launcher/index.client.tsx): Register the action or workspace panel.
- [server/](../../../../plugins/preview-launcher/server): Keep checks, local data, and service access on the server.
- [client/](../../../../plugins/preview-launcher/client): Render the action and its results.
- [shared/](../../../../plugins/preview-launcher/shared): Define runtime-neutral RPC contracts.
- [tests/](../../../../plugins/preview-launcher/tests): Cover observable success and failure cases.
- [README.md](../../../../plugins/preview-launcher/README.md): Document configuration, permissions, limitations, and checks.
- [Root README](../../../../README.md): Add the plugin to the catalog.

Expected result:

- Start a configured workspace preview and show its readiness.
- The plugin reports failures clearly and preserves unrelated workspace state.

## Verify

- `cd plugins/preview-launcher && npm run typecheck && npm run lint && npm test` exits with code 0.
- Automated fixture check: A healthy fixture shows its URL only as ready after its health check passes.
- Automated fixture check: A failed process, occupied port, and health timeout each show a clear failure.
- Automated fixture check: Starting twice does not create duplicate owned processes.
- Automated fixture check: Stopping and unloading clean up owned resources while unrelated processes remain running.
- Runtime check on the authorized target host: reload `preview-launcher` and confirm `running` with no load error.
- UI check on desktop and a compact client: exercise the action, success state, and error state.
- Stop task-owned fixtures and processes. Confirm temporary settings and artifacts are cleaned up.
