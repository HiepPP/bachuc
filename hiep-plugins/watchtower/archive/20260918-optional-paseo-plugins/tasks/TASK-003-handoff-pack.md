# TASK-003 handoff-pack

Group: A (shared root catalog)
Class: code

## Brief

Goal: Prepare a reviewable handoff when work moves to another agent.

Change: Add an optional handoff-pack plugin after the user selects this task.

How:

- Confirm selection before implementation. Read the shared context and repository rules.
- Check installed host support with the paseo-plugin skill before choosing API signatures.
- Add a handoff action with goals, decisions, files, blockers, verified checks, and the next step.
- Use supported session access and explicit user input. Identify missing context instead of inventing it.
- Show an editable preview before the user copies or sends the handoff.
- Keep sending separate from generation. Never create or message an agent during preview.
- Work without evidence-ledger. Offer evidence reuse only when its documented contract exists.
- Keep the MVP independent from other proposed plugins. Add its entry to the root catalog.

Files:

- [paseo-plugin.json](../../../../plugins/handoff-pack/paseo-plugin.json): Declare the plugin ID and verified host requirements.
- [package.json](../../../../plugins/handoff-pack/package.json): Declare dependencies and validation scripts.
- [package-lock.json](../../../../plugins/handoff-pack/package-lock.json): Lock dependencies.
- [tsconfig.json](../../../../plugins/handoff-pack/tsconfig.json): Configure TypeScript checks.
- [index.server.ts](../../../../plugins/handoff-pack/index.server.ts): Register server contributions and cleanup.
- [index.client.tsx](../../../../plugins/handoff-pack/index.client.tsx): Register the action or workspace panel.
- [server/](../../../../plugins/handoff-pack/server): Keep checks, local data, and service access on the server.
- [client/](../../../../plugins/handoff-pack/client): Render the action and its results.
- [shared/](../../../../plugins/handoff-pack/shared): Define runtime-neutral RPC contracts.
- [tests/](../../../../plugins/handoff-pack/tests): Cover observable success and failure cases.
- [README.md](../../../../plugins/handoff-pack/README.md): Document configuration, permissions, limitations, and checks.
- [Root README](../../../../README.md): Add the plugin to the catalog.

Expected result:

- Prepare a reviewable handoff when work moves to another agent.
- The plugin reports failures clearly and preserves unrelated workspace state.

## Verify

- `cd plugins/handoff-pack && npm run typecheck && npm run lint && npm test` exits with code 0.
- Automated fixture check: A fixture handoff includes the goal, decisions, blockers, files, checks, and next step.
- Automated fixture check: Missing checks appear as unverified, rather than passed.
- Automated fixture check: Editing and copying preserve the final preview. Cancelling sends nothing.
- Runtime check on the authorized target host: reload `handoff-pack` and confirm `running` with no load error.
- UI check on desktop and a compact client: exercise the action, success state, and error state.
- Stop task-owned fixtures and processes. Confirm temporary settings and artifacts are cleaned up.
