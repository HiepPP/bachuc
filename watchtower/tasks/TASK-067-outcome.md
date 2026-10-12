# TASK-067 Outcome

## Outcome

Status: DONE

Changed:

- `client/page.tsx`: `RunColumn` is back; each host lane shows Needs you, then Running and Just finished. Cards and subagent rows show a TASK chip (`onOpenTask` flows through `RunCluster` to `RunCard`). One plan line per plan project renders above its first card.
- New `client/task-chip.tsx` and `client/plan-line.tsx`.
- `client/lanes.tsx` deleted (with `rm`, a single untracked file this plan created). `shared/lanes.ts` keeps `needsYouItems` and adds `filterTrees`, `columnTrees`, and `planLineRoots`.
- `tests/host-lanes.test.ts` asserts the two columns again; `tests/lanes.test.ts` covers plan-line placement and Needs you.
- `README.md`: "Task lanes" became "Watchtower on Board", linking ADR-0012; column wording restored.
- Compact fix found on live: stacked columns kept `flex: 1` and split the height, leaving a gap under Running. They now take their content height on compact layouts.

Contract:

- Server fields from TASK-063, Needs you, `openWatchtower`, and `isShown` are unchanged.
- A chip whose TASK is in the active plan opens Watchtower at that TASK on the plan's workspace; otherwise it opens the thread's own workspace without a TASK.

Verified:

- `npm run typecheck` -> exit 0; `npm run-script lint` -> 0 warnings, 0 errors; `npm test` -> 127 pass, 0 fail.
- `rg -n "laneModel|LANE_COLUMNS|TaskLanes" hiep-plugins/plugins/board` -> no match.
- Live check, 2026-10-12, Expo web on daemon 6768, light theme, with a mock agent labelled TASK-067: Needs you (Q-024), Running with the bachuc plan line "Repo-wide Watchtower IDs and Board task lanes · 8/11 · Watchtower" above "Columns check" (TASK-067 chip), Just finished with "Lanes check B" (TASK-066 chip). Screenshot `/tmp/cols-1.png`.
- The TASK-067 chip opened `...?presentation=overlay&p.workspaceId=wks_68a64f9b580b7c27&p.task=TASK-067` with TASK-067 selected and its brief shown; Escape returned to Board.
- Compact (420 px) after the fix: Needs you, Running, Just finished stacked with no gap. Screenshot `/tmp/cols-compact2.png`.

Anti-goal:

- Before: bachuc block about 560 px in the lane grid; `/tmp/lanes-1.png`, 2026-10-12.
- After restoring the columns: plan line plus one card, about 106 px; `/tmp/cols-1.png`.
- Final: about 106 px at 1440 px; `/tmp/cols-desktop2.png`, 2026-10-12.
- Result: PASS against 250 px.

Lessons:

- On compact layouts, a column stacked inside a `flex: 1` parent must drop `flex: 1`, or siblings split the height.
