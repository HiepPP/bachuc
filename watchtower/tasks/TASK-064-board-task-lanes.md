# TASK-064 Board task lanes UI

Group: B2 (`hiep-plugins/plugins/board/client/**`, `hiep-plugins/plugins/board/README.md`, `hiep-plugins/plugins/watchtower-board/index.client.tsx`, `hiep-plugins/plugins/watchtower-board/client/page.tsx`)
Class: code

## Brief

Goal: Board shows each project as one grid of rows and state columns, with Needs you in a left column, as in design 2a.

Change: Running and Just finished columns per host -> per project: TASK rows of the active plan, then an Ad-hoc row, across Todo, Running, Finished, Verified.

Design: Claude Design project "Watchtower unified UI options", file `Board Watchtower Options.dc.html`, option 2a.

Boundaries:

- Follows [ADR-0010](watchtower/decisions/ADR-0010-task-lanes-over-status-columns.md).
- Column rules: a TASK row places its runs by run state, and the row shows Todo, BLOCKED with its Q-ID, or `After TASK-NNN` when nothing ran yet. Verified holds runs of a TASK whose Tracker status is DONE.
- Ad-hoc row: Running and Finished only. Todo and Verified cells are hatched. Remove hides a card as today.
- A run whose `taskId` matches no row of the active plan goes to Ad-hoc and keeps its TASK chip.
- Needs you column: needs-input runs, blocking questions, and manual checks of every host. Answers happen in the thread or on the Watchtower page; Board has no answer form.
- Keep host lanes, stars, unread fading, project colors, subagent clusters, Remove, and the size control.
- Compact layouts stack the Needs you column above the grid and turn each row into a list.
- Hover follows [docs/hover.md](docs/hover.md); no `useUnistyles()` per [docs/unistyles.md](docs/unistyles.md).

How:

- Split the grid into a new `client/lanes.tsx`, so [client/page.tsx](hiep-plugins/plugins/board/client/page.tsx) does not grow.
- Add `client/needs-you.tsx` for the left column.
- Cards in a cell are one line; selecting one shows the full card with model, effort, star, and Remove.
- Update the README sections that describe columns.

Files:

- [hiep-plugins/plugins/board/client/lanes.tsx](hiep-plugins/plugins/board/client/lanes.tsx) (new)
- [hiep-plugins/plugins/board/client/needs-you.tsx](hiep-plugins/plugins/board/client/needs-you.tsx) (new)
- [hiep-plugins/plugins/board/client/page.tsx](hiep-plugins/plugins/board/client/page.tsx) (mount lanes)
- [hiep-plugins/plugins/board/README.md](hiep-plugins/plugins/board/README.md) (layout sections)
- [hiep-plugins/plugins/board/tests/lanes.test.ts](hiep-plugins/plugins/board/tests/lanes.test.ts) (new, placement rules)

Expected result:

- bachuc shows rows TASK-059 to TASK-066 plus Ad-hoc; a project with no plan shows only Ad-hoc.
- A run of a DONE TASK sits in Verified; a needs-input run sits in Running with an amber edge and also in Needs you.
- Cases: TASK with no runs and open deps -> `After TASK-NNN` in Todo; BLOCKED TASK -> Q-ID in Todo; run with an unknown TASK -> Ad-hoc with chip.

Anti-goal: [client/page.tsx](hiep-plugins/plugins/board/client/page.tsx) stays at or under 1764 lines; tripwire; read with `wc -l` before changes, after mounting lanes, and at completion.

## Verify

- `cd hiep-plugins/plugins/board && npx tsx --test tests/lanes.test.ts` -> placement cases pass.
- `cd hiep-plugins/plugins/board && npm run typecheck && npm run lint` -> pass.
- `wc -l hiep-plugins/plugins/board/client/page.tsx` -> 1764 or less.
- Human check on live after `npm run cli -- plugin reload board`: the grid matches design 2a for this repo, at desktop and compact widths.
