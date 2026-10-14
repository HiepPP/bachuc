# ADR-0028 No Fleet map tab over a three.js Fleet map in the app

Status: accepted
Date: 2026-10-14
Supersedes: ADR-0026, ADR-0027
Scope: `packages/app/src/fleet-map/**`, `packages/app/package.json`, `packages/app/src/stores/workspace-layout-storage.ts`

## Context

- The Fleet map tab shipped on 2026-10-14 (plan 20261013-fleet-map). It drew the agents of one host as 3D dots with three.js.
- The owner tried it on live and found it not useful: the dots had no labels, positions meant nothing, and the sidebar already shows agent status with names.
- Metro put `three` in the shared `__common` web chunk that `index.html` loads at start, about 814 KB raw, even when nobody opens the tab (TASK-090 outcome).
- Three mockups were compared on 2026-10-14 in Claude Design project `9aea602d`. The owner picked the orchestration tree, and chose to build it on the Watchtower page of the `watchtower-board` plugin. That tree is plain 2D UI.

## Decision

- Remove the Fleet map tab, the `fleet_map` tab kind, its locale strings, `docs/fleet-map.md`, and the `three` dependency from `packages/app`.
- Keep a tagged COMPAT shim in `packages/app/src/stores/workspace-layout-storage.ts` that accepts a saved `fleet_map` tab and drops it, so an old saved layout still loads.
- The owner chose this on 2026-10-14.

## Consequences

- The app no longer loads three.js at start.
- ADR-0026 (shared renderer) and ADR-0027 (fixed layout) no longer apply, because their code is gone.
- Agent hierarchy views live on the Watchtower page as 2D UI.

## Options Considered

- Keep the tab and rework it as a 3D timeline: rejected. The mockup read harder than the 2D tree, and it keeps the start-up cost.
- Hide the tab but keep the code: rejected. `three` would still load at start, and the code would rot unused.

## Revisit If

- A view needs real 3D depth that 2D cannot show, such as a timeline the owner reads daily.
- Metro can load `three` only when a view opens, so an unused 3D view costs nothing at start.
