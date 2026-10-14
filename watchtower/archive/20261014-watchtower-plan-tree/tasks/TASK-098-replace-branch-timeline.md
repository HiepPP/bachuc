# TASK-098 Replace the branch timeline

Group: D (writes the overview, run cells, and removes the timeline view files)
Class: risky

## Brief

Goal: Put the plan tree on the Watchtower page in place of the branch timeline, Running now, and Up next, and show the pace line.

Change: The right side shows an hour-axis Gantt and two cells -> it shows the pace line and the plan tree.

Design: mock D v2, see Shared Context in [watchtower/CONTEXT.md](watchtower/CONTEXT.md).

Boundaries:

- Keep the lifecycle stepper, the Autorun strip, Needs you, Manual checks, Decisions, and Archived plans as they are. Needs you stays on this page per [ADR-0015](watchtower/decisions/ADR-0015-questions-on-the-watchtower-page-over-board.md).
- Pace line: four numbers in monospace with a muted label under each: average task, estimated finish, builders at once, and the slowest TASK over average. Hide a number that has no data.
- With no autorun yet, the tree still shows the waves from the Tracker, with no time text.
- Delete `client/timeline-view.tsx`, `client/timeline-geometry.ts`, and `tests/timeline-geometry.test.ts`. Remove `RunningNowCell` and `UpNextCell` from `client/run-cells.tsx`. Keep `client/timeline.ts` and its test.
- Run impact analysis on every removed symbol before deleting it. A caller outside this plugin is a blocker.
- Wire `useRunAgents` (TASK-096) on the page that renders the overview, and pass `onSelectTask` to the same handler the task list uses.

How:

- Edit `client/overview.tsx` and the page component that owns the workspace and host IDs.
- Delete the files and dead imports. Update `tests/overview.test.ts` where it names removed parts.

Files:

- [hiep-plugins/plugins/watchtower-board/client/overview.tsx](hiep-plugins/plugins/watchtower-board/client/overview.tsx) (tree and pace line in place of the timeline cell and two cells)
- [hiep-plugins/plugins/watchtower-board/client/page.tsx](hiep-plugins/plugins/watchtower-board/client/page.tsx) (pass builders and handlers, only if the overview cannot reach them)
- [hiep-plugins/plugins/watchtower-board/client/run-cells.tsx](hiep-plugins/plugins/watchtower-board/client/run-cells.tsx) (remove two cells)
- [hiep-plugins/plugins/watchtower-board/client/timeline-view.tsx](hiep-plugins/plugins/watchtower-board/client/timeline-view.tsx) (delete)
- [hiep-plugins/plugins/watchtower-board/client/timeline-geometry.ts](hiep-plugins/plugins/watchtower-board/client/timeline-geometry.ts) (delete)
- [hiep-plugins/plugins/watchtower-board/tests/timeline-geometry.test.ts](hiep-plugins/plugins/watchtower-board/tests/timeline-geometry.test.ts) (delete)
- [hiep-plugins/plugins/watchtower-board/tests/overview.test.ts](hiep-plugins/plugins/watchtower-board/tests/overview.test.ts) (update if it names removed parts)

Expected result:

- The Watchtower page shows the pace line and the tree, and no hour axis, Running now, or Up next.
- Pressing a row selects the TASK in the left rail.
- The plugin loads on live after a reload.

Anti-goal: The rest of the page keeps working. Tripwire: `tests/overview.test.ts`, `tests/timeline.test.ts`, `tests/lifecycle.test.ts`, and `tests/actions.test.ts` pass with the same or more cases; read before the first edit and at the end.

## Verify

- `cd hiep-plugins/plugins/watchtower-board && npm test` -> every test file passes.
- `cd hiep-plugins/plugins/watchtower-board && npm run typecheck` -> exit 0.
- `cd hiep-plugins/plugins/watchtower-board && npm run lint` -> exit 0.
- `rg -n "BranchTimeline|TimelineStrip|RunningNowCell|UpNextCell|timeline-geometry" hiep-plugins/plugins/watchtower-board` -> matches only in README.md, which TASK-099 updates.
- Needs a human or a browser session: on live, after `npm run cli -- plugin reload watchtower-board`, the Watchtower page of a workspace with a plan shows the tree and the pace line.
- Anti-goal: record the pass counts of the four test files before and after; the after counts are not lower, except removed timeline-geometry cases.
