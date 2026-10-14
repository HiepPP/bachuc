# ADR-0029 Plan tree over branch timeline

Status: accepted
Date: 2026-10-14
Supersedes: -
Scope: `hiep-plugins/plugins/watchtower-board/client/**`, `hiep-plugins/plugins/watchtower-board/README.md`

## Context

- The right side of the Watchtower page showed a branch timeline (lanes, hour axis, now line), **Running now**, and **Up next**.
- The owner compared mockups A to D v2 on 2026-10-14 in Claude Design project `9aea602d` and chose the plan tree.
- The tree groups tasks into waves by their deps, shows each task's state, and links a running task to its builder thread. The timeline showed time on an axis and no builder.
- The tree shipped in TASK-095 to TASK-098: the model, the builder matching, the view, and the replacement of the timeline cell.

## Decision

- The Watchtower page shows a pace line and the plan tree in place of the branch timeline, Running now, and Up next.
- The owner chose this on 2026-10-14.

## Consequences

- The timeline view and its geometry module are deleted. `clockAt` moved to `client/run-cells.tsx`.
- The page has no hour axis. Time shows as a per-row figure and four pace numbers.
- A reader who wants a Gantt of task overlap must read the run log.

## Options Considered

- Keep the timeline: rejected. The owner found the tree reads faster and the timeline has no builder link.
- Tree as a second tab: rejected. The page has no tab bar, and a hidden view would be opened rarely.
- Tree beside the timeline: rejected. Both cells repeat the same tasks and the page already stacks below 900 px.

## Revisit If

- The owner asks for task overlap or idle gaps on a time axis, and the run log is the only place to see them.
- The page gains a tab bar, so a second view costs no extra space.
