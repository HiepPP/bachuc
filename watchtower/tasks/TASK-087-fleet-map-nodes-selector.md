# TASK-087 Fleet node selector and hook

Group: B (writes only the nodes files; no shared file with other TASKs)
Class: code

## Brief

Goal: Read the agents of one host from the session store and return fleet nodes. The hook must not change identity when an unrelated field changes.

Change: No fleet data source exists -> `selectFleetNodes` and `useFleetNodes(serverId)` return a stable list of nodes with id, title, workspace, parent, bucket, and creation time.

Boundaries:

- Data source is `useSessionStore` agents of the given `serverId`. Add no RPC and no protocol field. See [docs/protocol-compatibility.md](docs/protocol-compatibility.md).
- Bucket comes from `deriveSidebarStateBucket` in [packages/app/src/utils/sidebar-agent-state.ts](packages/app/src/utils/sidebar-agent-state.ts), so the map matches the sidebar.
- Parent comes from `getParentAgentIdFromLabels(agent.labels)` in `@getpaseo/protocol/agent-labels`.
- Skip the agents the sidebar skips (archived agents). Read how the sidebar filters before copying the rule.
- Workspace id is `agent.workspaceId`. If it is missing, fall back to the sidebar's rule for `cwd`. Workspace name comes from the session `workspaces` map and may be absent.
- Provider subagents (`provider-store`) are out of scope.
- Streaming updates agent objects often. Use `useStoreWithEqualityFn` with `fast-deep-equal`, as in [packages/app/src/subagents/select.ts](packages/app/src/subagents/select.ts). The returned array keeps its reference when no node field changed.
- The app runs React Compiler. Do not rely on `void`-only reads in `useMemo`.

How:

- Write `FleetNode` as `{ id, title, workspaceId, workspaceName, parentId, bucket, createdAt }`. Keep it a superset of the layout input from TASK-086.
- Write the pure `selectFleetNodes(session)` and test it with plain objects.
- Write `useFleetNodes(serverId)` as a thin equality-guarded wrapper.

Files:

- [packages/app/src/fleet-map/nodes.ts](packages/app/src/fleet-map/nodes.ts) (new: `FleetNode`, `selectFleetNodes`)
- [packages/app/src/fleet-map/use-fleet-nodes.ts](packages/app/src/fleet-map/use-fleet-nodes.ts) (new: the hook)
- [packages/app/src/fleet-map/nodes.test.ts](packages/app/src/fleet-map/nodes.test.ts) (new: selector and identity tests)

Expected result:

- Each non-archived agent of the host becomes one node. Other hosts are not read.
- A subagent node has `parentId` set. A root has `null`.
- A needs-input agent has bucket `needs_input`. A running agent has `running`.
- A streaming update that changes only the timeline or `lastActivityAt` does not change the hook output reference.
- A status change makes a new output with the new bucket.

Anti-goal: Re-render count stays at zero for unrelated agent updates. Tripwire: the identity test applies 100 timeline-only updates and the hook output reference changes 0 times; read in `nodes.test.ts` at the end of the TASK.

## Verify

- `cd packages/app && npx vitest run src/fleet-map/nodes.test.ts --bail=1` -> all cases in Expected result pass.
- `npm run typecheck` -> exit 0.
- `npm run lint -- packages/app/src/fleet-map/nodes.ts packages/app/src/fleet-map/use-fleet-nodes.ts packages/app/src/fleet-map/nodes.test.ts` -> exit 0.
- `rg -n "useUnistyles|from \"three\"" packages/app/src/fleet-map/nodes.ts packages/app/src/fleet-map/use-fleet-nodes.ts` -> no match.
- Anti-goal: the 100-update test asserts 0 reference changes and passes.
