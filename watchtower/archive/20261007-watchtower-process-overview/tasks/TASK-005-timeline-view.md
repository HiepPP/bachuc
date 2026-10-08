# TASK-005 Branch timeline view

Group: D (standalone: `client/timeline-view.tsx`, `client/timeline-geometry.ts`, and `tests/timeline-geometry.test.ts`)
Class: code

## Brief

Goal: Render the timeline model as the branch timeline cell: lanes of bars, orthogonal dependency lines, a now line, an hour axis, and a stats row.

Change: no view -> a `BranchTimeline` component that takes a `buildTimeline` result, a width, and the plugin theme.

Design: the branch timeline cell of option 3f in the mock named in [watchtower/CONTEXT.md](watchtower/CONTEXT.md).

Boundaries:

- Draw with React Native Views only. There is no SVG. See [watchtower/CONTEXT.md](watchtower/CONTEXT.md).
- Put all geometry in `client/timeline-geometry.ts` so `tsx --test` can test it.
- Use `theme.colors` only. Done bars use `statusSuccess`, running bars use `accent`, blocked bars use a `statusDanger` outline, and planned bars use a dashed `foregroundMuted` outline.
- Read the width from `onLayout`. Do not read the window size.

How:

- `timelineGeometry(model, width)` maps minutes to x, lanes to y, and returns bar rects, label positions, connector segments, the now x, and tick positions.
- A connector between two lanes is at most 3 straight segments: across, down or up, then across. A connector in one lane is one segment.
- A label that would touch the previous label in its lane moves below the bar.
- `BranchTimeline` draws each rect and segment as an absolutely positioned View.
- Under the chart, show four stats: average task, longest done task, the running task most over the average, and branches at once.
- Each bar is a Pressable with an accessibility label such as `"TASK-006, running 37 min"`. A press calls an optional `onSelectTask(id)`.

Files:

- [hiep-plugins/plugins/watchtower-board/client/timeline-geometry.ts](hiep-plugins/plugins/watchtower-board/client/timeline-geometry.ts) (new)
- [hiep-plugins/plugins/watchtower-board/client/timeline-view.tsx](hiep-plugins/plugins/watchtower-board/client/timeline-view.tsx) (new)
- [hiep-plugins/plugins/watchtower-board/tests/timeline-geometry.test.ts](hiep-plugins/plugins/watchtower-board/tests/timeline-geometry.test.ts) (new)

Expected result:

- Geometry tests: a fork from lane 0 to lane 1 gives 3 segments, and a same-lane link gives 1.
- Geometry tests: two labels 10 px apart in one lane put the second label below its bar.
- Geometry tests: the now x equals the x of `nowMinute`.
- The view typechecks and lints. It renders on live inside the overview after TASK-007; that check belongs to Plan Verify.

Anti-goal: the view stays light: connector segments per edge stay at or below 3, and positioned Views per timeline stay at or below 4 x tasks + 24; drift gauge; read in a geometry test on the mock data at completion.

## Verify

- In the plugin folder: `npx tsx --test tests/timeline-geometry.test.ts` -> every case above passes.
- In the plugin folder: `npm run typecheck` and `npm run lint` -> exit 0.
- `rg -n "Svg|react-native-svg" hiep-plugins/plugins/watchtower-board/client` -> no match.
- The View count test on the mock data -> 4 x tasks + 24 or less.
