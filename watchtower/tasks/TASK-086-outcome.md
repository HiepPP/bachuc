# TASK-086 Outcome

## Status

DONE

## Changed

- `packages/app/src/fleet-map/layout.ts` (new)
- `packages/app/src/fleet-map/layout.test.ts` (new)

## Contract

`computeFleetLayout(nodes: readonly FleetLayoutNode[])` returns `{ positions, edges, clusters, boundingRadius }`.

- `FleetLayoutNode` is `{ id, workspaceId, parentId, createdAt: Date | number }`. TASK-087 `FleetNode` fits as a superset.
- Pure and deterministic. No `three`. No random numbers. Output does not depend on input order.
- Roots in a workspace sit on a phyllotaxis (spacing 4). A subagent orbits its parent at radius 2.5 and sits 6 units higher per depth level.
- Clusters sit on a golden-angle spiral, ordered by earliest `createdAt`, then workspace id. Each cluster takes the first spiral slot that clears all earlier clusters by more than the sum of radii plus a gap of 4.
- `clusters[].radius` is the largest planar reach of its nodes plus padding. `boundingRadius` is the largest distance from the origin of any node or cluster ring edge.
- A parent in a different workspace: the node is a root for placement, and an edge is still emitted.
- A self-parent or parent-cycle member is a root and gets no edge. A tail node hanging off a cycle keeps its edge.
- A duplicate id keeps the earliest entry.

## Verified

- `cd packages/app && npx vitest run src/fleet-map/layout.test.ts --bail=1` -> PASS (12) FAIL (0)
- `npx oxlint packages/app/src/fleet-map/layout.ts packages/app/src/fleet-map/layout.test.ts` -> 0 warnings, 0 errors
- `npm run format:check:files -- <both files>` -> all matched files use the correct format
- `rg -n 'from "three"' packages/app/src/fleet-map/layout.ts` -> no match
- `npm run typecheck` -> exit 2, but no error is in the layout files. Remaining errors are in sibling-owned `src/screens/workspace/workspace-screen.tsx` and `src/screens/workspace/workspace-tab-menu.ts` (`path` missing on the new `fleet_map` target).

## Anti-goal

Tripwire: 500 nodes in 20 workspaces under 25 ms. The test asserts it (`performance.now()` difference, second warm call) and passes. The exact millisecond value was not printed (vitest reporter in this setup hides console output).

- Builder read: test assertion under 25 ms; source `layout.test.ts`; passed in the builder run.
- Lead read: 2.15 ms for 500 nodes in 20 workspaces, warm second call; source `npx tsx` script in `/tmp/fleet/layout-bench.ts`; time 2026-10-13T11:05:22Z.
- Result: PASS against the 25 ms limit.

Lead re-check at wave close: `npx vitest run src/fleet-map src/panels/panel-manifest.test.ts src/workspace-tabs/identity.test.ts src/stores/workspace-layout-storage.test.ts src/workspace-tabs/launcher/internal/catalog.test.ts src/i18n/resources.test.ts --bail=1` -> 10 files, 104 tests pass. `npm run typecheck` -> exit 0. `npm run lint` on the changed folders -> 0 warnings, 0 errors.

## Lessons

- `npm run lint -- <files>` at the repo root fails here with "ESLint couldn't find an eslint.config" (a wrapper runs ESLint instead of oxlint). `npx oxlint <files>` ran the real lint. Treat this as a tooling issue outside this TASK.
- In zsh, a `$F` variable holding several paths is not word-split. Pass each path as its own argument.
- oxlint rule `no-array-reverse` flags `[...a].reverse()`. Use `toReversed()`.
- Typecheck errors for `fleet_map` in `workspace-screen.tsx` and `workspace-tab-menu.ts` belong to sibling builders.
