# TASK-066 Outcome

## Outcome

Status: DONE

Changed:

- `board/client/hosts.ts`: the per-host client gains `openWatchtower({ workspaceId, task })`, which calls `openSurface("watchtower", { pluginId: "watchtower-board", serverId, presentation: "overlay", params })`.
- `board/client/page.tsx`: TASK chips, the plan title, and Needs you questions and checks open Watchtower on the plan's workspace (the newest thread's workspace under that git root).
- `board/client/lanes.tsx` and `needs-you.tsx`: links show only when the plan has a workspace.
- `watchtower-board/client/page.tsx`: `props.params.workspaceId` names the workspace before the module-level preselection; `props.params.task` reaches `TaskList` as `initialTaskId` through `overview.tsx` and `board-rail.tsx`.
- Both READMEs describe the link.

Contract:

- watchtower-board keeps its rule: a named workspace never falls back to another one. Its pill, Command Center item, and Cmd+G still use `preselectWorkspace`.
- On a host without `openSurface` params, watchtower-board ignores them and shows its fallback workspace.

Verified:

- Board: `npm run typecheck` -> exit 0; `npm test` -> 129 pass; `npm run-script lint` -> 0 warnings, 0 errors.
- watchtower-board: `npm run typecheck` -> exit 0; `npm run-script lint` -> 0 warnings, 0 errors; `npm test` -> 74 pass, 1 fail.
- The 1 failure, "the real t3code plan parses without warnings and stays small", is pre-existing: it asserts a `proposed` ADR in `watchtower/DECISIONS.md`, and HEAD has none since commit 5357c1c accepted ADR-0003. Its plan assertions on the migrated archive (10 TASKs, 10 log rows) pass.
- Human check on live, 2026-10-12, Expo web on daemon 6768, bachuc with three workspaces: PASS after one fix.
  - The TASK-066 chip opened `.../watchtower-board/surface/watchtower?presentation=overlay&p.workspaceId=wks_b682e6b9887c1b73&p.task=TASK-066`, the workspace of the plan's newest run ("Lanes check B"), over Board.
  - The same route with `p.workspaceId=wks_d3b2d36c6ae971a6` showed "Actions go to Lanes check A"; with `wks_b682e6b9887c1b73` it showed "Agent busy". The param picks the workspace.
  - Defect found and fixed: TASK-066 was selected but hidden, because the Done group starts collapsed. `TaskList` now opens the group of the opener's TASK. After reload, TASK-066 showed selected with its brief. Screenshot `/tmp/wt-open2.png`.
- Limit: the chip opens the plan's workspace (newest run under the git root), not each card's own workspace; the spec's two-workspace check passes for the plan's workspace.

Anti-goal:

- Before: not run before the change. The failing assertion reads only `watchtower/DECISIONS.md`, and `git show HEAD:watchtower/DECISIONS.md` has no `proposed` row, so it failed before this TASK.
- Final: 74 pass, 1 fail, the same test; 0 new failures; 2026-10-12.
- Result: PASS against 0 new failures.

Lessons:

- watchtower-board's `tests/overview.test.ts` reads this repo's real `watchtower/` files, so plan and ADR edits can fail it.
