# NEXT

## Current Active Plan

- Title: Fleet map with three.js
- Slug: 20261013-fleet-map
- Status: ACTIVE
- Updated: 2026-10-13

## Tracker

One row per TASK. Group ties together items that write the same files.

| Order | TASK                                           | Group | Status      | Spec                                                                                                           | Deps                                   | Context                                        | Notes                                                                 |
| ----- | ---------------------------------------------- | ----- | ----------- | -------------------------------------------------------------------------------------------------------------- | -------------------------------------- | ---------------------------------------------- | --------------------------------------------------------------------- |
| 1     | TASK-086 Fleet map layout function             | A     | DONE        | [watchtower/tasks/TASK-086-fleet-map-layout.md](watchtower/tasks/TASK-086-fleet-map-layout.md)                 | -                                      | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Pure function. No three.js.                                           |
| 2     | TASK-087 Fleet node selector and hook          | B     | DONE        | [watchtower/tasks/TASK-087-fleet-map-nodes-selector.md](watchtower/tasks/TASK-087-fleet-map-nodes-selector.md) | -                                      | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Reads the session store. No new RPC.                                  |
| 3     | TASK-088 Fleet map tab wiring                  | C     | DONE        | [watchtower/tasks/TASK-088-fleet-map-tab-wiring.md](watchtower/tasks/TASK-088-fleet-map-tab-wiring.md)         | -                                      | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | New `fleet_map` kind, launcher entry, locales. Touches saved layouts. |
| 4     | TASK-089 Shared renderer, scene, and scheduler | D     | DONE        | [watchtower/tasks/TASK-089-fleet-map-renderer-scene.md](watchtower/tasks/TASK-089-fleet-map-renderer-scene.md) | -                                      | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Adds the `three` dependency. Owns the render budget.                  |
| 5     | TASK-090 Fleet map web view                    | E     | DONE        | [watchtower/tasks/TASK-090-fleet-map-view.md](watchtower/tasks/TASK-090-fleet-map-view.md)                     | TASK-086, TASK-087, TASK-088, TASK-089 | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Joins all parts. Hover label, click opens agent.                      |
| 6     | TASK-091 Fleet map docs                        | F     | IN PROGRESS | [watchtower/tasks/TASK-091-fleet-map-docs.md](watchtower/tasks/TASK-091-fleet-map-docs.md)                     | TASK-090                               | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | New doc, docs table row, glossary term.                               |

TASK Status labels: TODO, IN PROGRESS, BLOCKED, DONE.
Plan-level Status header: ACTIVE while any row is open, DONE when all rows DONE, ARCHIVED after archive.

## Plan Verify

- `npm run typecheck` -> exit 0.
- `npm run lint -- packages/app/src/fleet-map packages/app/src/panels/fleet-map-panel.tsx` -> exit 0.
- `cd packages/app && npx vitest run src/fleet-map --bail=1` -> every fleet-map test file passes.
- Needs a human or a browser session: on live (`npm run dev` and `npm run dev:desktop`), open New Tab, then Fleet map, with at least three agents and one subagent. Nodes show the sidebar status colors, a running node pulses, and a click on a node opens that agent.
- Needs a human or a browser session: Activity Monitor or the Chrome performance panel shows no steady GPU or frame work while the Fleet map tab is open, the window is blurred, and no agent runs.

## Handoff

- Next action: run `implement`. Wave 1 is TASK-086, TASK-087, TASK-088, and TASK-089, which share no file. TASK-090 and TASK-091 follow.

## Archive

- [watchtower/archive/20261013-turn-step-list-follow-ups/](watchtower/archive/20261013-turn-step-list-follow-ups/)
- [watchtower/archive/20261013-composer-pill-wheel-select/](watchtower/archive/20261013-composer-pill-wheel-select/)
- [watchtower/archive/20261012-turn-step-list/](watchtower/archive/20261012-turn-step-list/)
- [watchtower/archive/20261012-repo-wide-ids-and-board-lanes/](watchtower/archive/20261012-repo-wide-ids-and-board-lanes/)
