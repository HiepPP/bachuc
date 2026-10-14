# TASK-086 Fleet map layout function

Group: A (writes only the layout files; no shared file with other TASKs)
Class: code

## Brief

Goal: Turn a list of fleet nodes into stable 3D positions and edges. The function is pure and does not import `three`.

Change: No fleet-map layout exists -> `computeFleetLayout` returns positions, cluster rings, edges, and a bounding radius.

Boundaries:

- Deterministic. The same input gives the same output. No random numbers, no physics. See Decisions in [watchtower/CONTEXT.md](watchtower/CONTEXT.md).
- Stable. A new agent must not move existing nodes in its own workspace. Order nodes in a workspace by `createdAt`, then `id`.
- Workspace clusters sit on a spiral ordered by the earliest `createdAt` in the workspace, then workspace id. A new older workspace may shift later clusters. This is accepted.
- Inside a cluster, root agents use a phyllotaxis (sunflower) spread on the plane. A subagent sits above its parent and orbits it. Depth of the chain adds height.
- A node whose parent is missing from the list is a root.
- Do not import `three`. Positions are plain `{ x, y, z }` numbers.

How:

- Define `FleetNode` as `{ id, workspaceId, parentId, createdAt }` in a small type the layout needs. TASK-087 produces a superset.
- Write `computeFleetLayout(nodes)` returning `{ positions: Map<string, {x,y,z}>, edges: Array<[string,string]>, clusters: Array<{workspaceId, x, z, radius}>, boundingRadius }`.
- Guard cycles in parent links. A node in a cycle becomes a root.
- Write tests first.

Files:

- [packages/app/src/fleet-map/layout.ts](packages/app/src/fleet-map/layout.ts) (new: the pure layout function)
- [packages/app/src/fleet-map/layout.test.ts](packages/app/src/fleet-map/layout.test.ts) (new: unit tests)

Expected result:

- Empty input gives empty positions and a bounding radius of 0.
- Same input twice gives deep-equal output.
- Adding a node to a workspace leaves every existing position in that workspace unchanged.
- A subagent has a larger `y` than its parent. An edge exists for each parent link.
- Two workspaces get cluster centers farther apart than the sum of their radii.
- A parent cycle and a missing parent do not throw.

Anti-goal: Layout cost stays low. Tripwire: `computeFleetLayout` for 500 nodes in 20 workspaces takes under 25 ms in the test; read at the end of the TASK from the `performance.now()` difference in `layout.test.ts`.

## Verify

- `cd packages/app && npx vitest run src/fleet-map/layout.test.ts --bail=1` -> all cases in Expected result pass.
- `npm run typecheck` -> exit 0.
- `npm run lint -- packages/app/src/fleet-map/layout.ts packages/app/src/fleet-map/layout.test.ts` -> exit 0.
- `rg -n "from \"three\"" packages/app/src/fleet-map/layout.ts` -> no match.
- Anti-goal: the 500-node test asserts under 25 ms and passes.
