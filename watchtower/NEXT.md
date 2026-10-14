# NEXT

## Current Active Plan

- Title: Fleet map with three.js
- Slug: 20261013-fleet-map
- Status: DONE
- Updated: 2026-10-13

## Tracker

One row per TASK. Group ties together items that write the same files.

| Order | TASK                                           | Group | Status | Spec                                                                                                           | Deps                                   | Context                                        | Notes                                                                                               |
| ----- | ---------------------------------------------- | ----- | ------ | -------------------------------------------------------------------------------------------------------------- | -------------------------------------- | ---------------------------------------------- | --------------------------------------------------------------------------------------------------- |
| 1     | TASK-086 Fleet map layout function             | A     | DONE   | [watchtower/tasks/TASK-086-fleet-map-layout.md](watchtower/tasks/TASK-086-fleet-map-layout.md)                 | -                                      | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Pure function. No three.js.                                                                         |
| 2     | TASK-087 Fleet node selector and hook          | B     | DONE   | [watchtower/tasks/TASK-087-fleet-map-nodes-selector.md](watchtower/tasks/TASK-087-fleet-map-nodes-selector.md) | -                                      | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Reads the session store. No new RPC.                                                                |
| 3     | TASK-088 Fleet map tab wiring                  | C     | DONE   | [watchtower/tasks/TASK-088-fleet-map-tab-wiring.md](watchtower/tasks/TASK-088-fleet-map-tab-wiring.md)         | -                                      | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | New `fleet_map` kind, launcher entry, locales. Touches saved layouts. Live check passed 2026-10-14. |
| 4     | TASK-089 Shared renderer, scene, and scheduler | D     | DONE   | [watchtower/tasks/TASK-089-fleet-map-renderer-scene.md](watchtower/tasks/TASK-089-fleet-map-renderer-scene.md) | -                                      | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Adds the `three` dependency. Owns the render budget.                                                |
| 5     | TASK-090 Fleet map web view                    | E     | DONE   | [watchtower/tasks/TASK-090-fleet-map-view.md](watchtower/tasks/TASK-090-fleet-map-view.md)                     | TASK-086, TASK-087, TASK-088, TASK-089 | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Joins all parts. Hover label, click opens agent. Live check passed 2026-10-14.                      |
| 6     | TASK-091 Fleet map docs                        | F     | DONE   | [watchtower/tasks/TASK-091-fleet-map-docs.md](watchtower/tasks/TASK-091-fleet-map-docs.md)                     | TASK-090                               | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | New doc, docs table row, glossary term.                                                             |

TASK Status labels: TODO, IN PROGRESS, BLOCKED, DONE.
Plan-level Status header: ACTIVE while any row is open, DONE when all rows DONE, ARCHIVED after archive.

## Plan Verify

- `npm run typecheck` -> exit 0.
- `npm run lint -- packages/app/src/fleet-map packages/app/src/panels/fleet-map-panel.tsx` -> exit 0.
- `cd packages/app && npx vitest run src/fleet-map --bail=1` -> every fleet-map test file passes.
- PASS on 2026-10-14 (live Electron and headless Chromium, see [watchtower/tasks/TASK-090-outcome.md](watchtower/tasks/TASK-090-outcome.md)): on live, open New Tab, then Fleet map, with at least three agents and one subagent. Nodes show the sidebar status colors, a running node pulses, a click on a node opens that agent, and hover shows a label.
- PASS on 2026-10-14, measured by counting blits: no frame work while the window is blurred (synthetic blur event) and no frame work while focused with no running agent. A real window blur, Activity Monitor, and the Explorer host were not checked.

## Handoff

- Next action: `feat/fleet-map` is merged into `main` locally and [ADR-0026](watchtower/decisions/ADR-0026-shared-renderer-blit-over-canvas-per-view.md) is accepted. Install a new build from Terminal.app to see Fleet map in release. Then run `archive`.
- Known gap: `three` loads in the shared `__common` web chunk at page start. It is not lazy at start. See [Q-034](watchtower/QUESTIONS.md) and [watchtower/tasks/TASK-090-outcome.md](watchtower/tasks/TASK-090-outcome.md).

## Archive

- [watchtower/archive/20261013-turn-step-list-follow-ups/](watchtower/archive/20261013-turn-step-list-follow-ups/)
- [watchtower/archive/20261013-composer-pill-wheel-select/](watchtower/archive/20261013-composer-pill-wheel-select/)
- [watchtower/archive/20261012-turn-step-list/](watchtower/archive/20261012-turn-step-list/)
- [watchtower/archive/20261012-repo-wide-ids-and-board-lanes/](watchtower/archive/20261012-repo-wide-ids-and-board-lanes/)
