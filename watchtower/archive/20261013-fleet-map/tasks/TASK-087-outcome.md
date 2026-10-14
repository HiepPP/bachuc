# TASK-087 Outcome

## Status

DONE (2026-10-13T11:03Z)

## Changed

- `packages/app/src/fleet-map/nodes.ts` (new: `FleetNode`, `selectFleetNodes`)
- `packages/app/src/fleet-map/use-fleet-nodes.ts` (new: `useFleetNodes`)
- `packages/app/src/fleet-map/nodes.test.ts` (new: 8 cases)

## Contract

- `FleetNode extends FleetLayoutNode` with `title: string | null`, `workspaceName: string | null`, `bucket: SidebarStateBucket`, `createdAt: Date`.
- `selectFleetNodes(session | undefined)` skips archived agents. Workspace id is `agent.workspaceId ?? agent.cwd`.
- Bucket uses the sidebar rule, with `turn.phase === "open"` counting as running.
- `useFleetNodes(serverId)` uses `useStoreWithEqualityFn` with `fast-deep-equal`.

## Verified

- Checkpoint `nodes-test`: `npx vitest run src/fleet-map/nodes.test.ts --bail=1` -> PASS (8) FAIL (0), vitest, 2026-10-13T11:03Z.
- Checkpoint `typecheck`: `npm run typecheck` -> exit 0, 2026-10-13T11:02Z.
- Checkpoint `lint`: `npx oxlint` on the 3 files -> 0 warnings, 0 errors (root `npm run lint` has an ESLint config error).
- Checkpoint `rg`: `rg -n "useUnistyles|from \"three\""` on nodes.ts and use-fleet-nodes.ts -> no match.

## Anti-goal

PASS. Reference changes after 100 timeline-only updates: 0 (test asserts `referenceChanges === 0` and render count unchanged). Source: `nodes.test.ts`, case "keeps the output reference across 100 timeline-only updates".

## Lessons

- The sidebar has no cwd fallback for a missing workspace id: `workspace-agent-activity.ts` skips such agents. Default taken: fall back to `agent.cwd`, so the map shows them. Reported in questions.
- The sidebar does not filter pending-archive ids in the node selector; only `archivedAt` is checked (as in `subagents/select.ts`, which also uses a pending-archive hook). Not added: the spec names archived agents only.
- The hook test needs the `@vitest-environment jsdom` docblock, because the unit project runs in node.
