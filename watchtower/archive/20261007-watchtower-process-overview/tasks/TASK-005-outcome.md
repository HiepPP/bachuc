# TASK-005 Outcome

## Outcome

Status: DONE

Changed:

- Added [hiep-plugins/plugins/watchtower-board/client/timeline-geometry.ts](hiep-plugins/plugins/watchtower-board/client/timeline-geometry.ts) with `timelineGeometry`, `connector`, `viewCount`, `clockAt`, `shortId`, and the lane and bar size constants.
- Added [hiep-plugins/plugins/watchtower-board/client/timeline-view.tsx](hiep-plugins/plugins/watchtower-board/client/timeline-view.tsx) with `BranchTimeline`. It draws segments, bars, the dashed expected part of a running bar, labels, the now line, the axis, hour ticks, and the four stats, all with Views and `theme.colors`.
- Added [hiep-plugins/plugins/watchtower-board/tests/timeline-geometry.test.ts](hiep-plugins/plugins/watchtower-board/tests/timeline-geometry.test.ts) with 5 tests.
- Spec refinement: `timelineGeometry(model, width, nowMinute)` takes `nowMinute` as a third argument, because the timeline model does not carry it. `BranchTimeline` takes `nowMinute` and `started` props for the same reason.

Contract:

- `BranchTimeline` props: `model`, `nowMinute`, `started`, `theme`, and an optional `onSelectTask(id)`. Each bar is a Pressable with a label such as `"TASK-006, running 37 min"`.
- The view reads its width from `onLayout`. With no bars, it shows "No timeline yet." and the stats row.
- A connector in one lane is 1 segment. A connector between lanes is 3 segments.

Verified:

- `npx tsx --test tests/timeline-geometry.test.ts` -> 5 pass, 0 fail.
- `npm test` in the plugin folder -> 46 pass, 0 fail.
- `npm run typecheck` -> exit 0. `npm run lint` -> exit 0, 0 warnings.
- `rg -n "Svg|react-native-svg" client` -> no match.
- The rendered view: UNVERIFIED (autonomous run). It mounts in the overview after TASK-007, and Plan Verify checks it on live.

Anti-goal:

- Final: 33 positioned Views on the mock data, limit 4 x 10 tasks + 24 = 64. Every edge used 3 segments or fewer. `t.diagnostic` in the geometry test, 2026-10-07 12:55.
- Result: PASS against 3 segments per edge and 64 Views.

Lessons:

- Import `PluginTheme` from `@getpaseo/plugin`, not from the client entry. Source: [packages/plugin/src/contracts.ts](packages/plugin/src/contracts.ts).
- Theme colors are opaque strings, so the view uses `opacity` instead of alpha hex suffixes. Source: [hiep-plugins/plugins/watchtower-board/client/timeline-view.tsx](hiep-plugins/plugins/watchtower-board/client/timeline-view.tsx).
