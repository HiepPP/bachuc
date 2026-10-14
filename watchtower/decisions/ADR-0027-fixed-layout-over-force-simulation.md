# ADR-0027 Fixed layout over a force simulation

Status: accepted
Date: 2026-10-14
Supersedes: -
Scope: `packages/app/src/fleet-map/layout.ts`, `packages/app/src/fleet-map/layout.test.ts`, `packages/app/src/fleet-map/view.web.tsx`
Guard: forbid-text (d3-force|forceSimulation|forceManyBody) in packages/app/src/fleet-map/layout.ts packages/app/src/fleet-map/view.web.tsx packages/app/package.json

## Context

- The Fleet map renders on demand. A frame renders only after a data, size, hover, or theme change, and the pulse is the only continuous animation (see the Render Budget in [watchtower/archive/20261013-fleet-map/CONTEXT.md](watchtower/archive/20261013-fleet-map/CONTEXT.md) and [docs/fleet-map.md](docs/fleet-map.md)).
- [packages/app/src/fleet-map/layout.ts](packages/app/src/fleet-map/layout.ts) is a pure function with no random numbers. The same input gives the same output, and the output does not depend on input order.
- A node keeps its position when another agent in its workspace changes or arrives. Workspaces sit on a golden-angle spiral, and subagents orbit their parent.
- Measured on 2026-10-14: 500 nodes in 20 workspaces lay out in 2.15 ms (TASK-086 outcome).
- A force simulation moves nodes over many ticks until it settles, and every data change restarts it.

## Decision

- Keep the layout a pure, fixed function of the node list. No physics, no per-frame tick, no random seed.
- The owner chose this on 2026-10-14. The reason: a force simulation needs continuous ticks and breaks the on-demand render budget.

## Consequences

- An idle map spends no frames, and the layout is easy to test with plain vitest.
- A new agent does not move the nodes the owner already looked at.
- Nodes cannot be dragged or relaxed by hand, and dense graphs may look less balanced than a simulated one.
- A new workspace placed before older ones on the spiral may shift later clusters. This was accepted in TASK-086.

## Options Considered

- Force simulation, for example `d3-force` or a Verlet loop: rejected. It ticks until it settles and restarts on each data change, so the map renders continuously and the budget of zero idle frames is lost. Its positions also change from run to run, so an agent is harder to find again.
- Run the simulation once and cache the result: rejected. Any data change needs a new run, and the result depends on the starting state, so positions are not stable.

## Revisit If

- A real host has so many agents that fixed clusters overlap or leave the screen, shown by a layout test with more than 500 nodes whose cluster clearance falls below the gap.
- The owner asks for dragging nodes or a free arrangement of the map.
- The render budget changes so that continuous rendering is accepted while the map is open.
