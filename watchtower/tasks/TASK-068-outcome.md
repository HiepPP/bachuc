# TASK-068 Outcome

## Outcome

Status: DONE

Changed:

- `shared/lanes.ts`: `filterTrees` (title, project, or TASK-ID of a card or any subagent) and `columnTrees` (project groups, starred first) replace `filterLanes` and `laneTrees`.
- `client/page.tsx`: both columns show filtered trees in `columnTrees` order; key entries run Needs you, then Running, then Just finished; the selection ring sits on the column's card wrapper. Plan lines follow the filtered columns, so a plan line moves to Just finished when no running card of its project matches.
- `tests/keys.test.ts`: search and reading-order tests for the columns.
- `README.md`: Search and Keys bullets describe the columns.

Contract:

- Every TASK-065 key rule holds, including `isShown` for a hidden Board page.

Verified:

- `npx tsx --test tests/keys.test.ts tests/shortcut.test.ts` -> pass (inside `npm test`, 127 pass, 0 fail).
- Live check, 2026-10-12, Expo web on daemon 6768:
  - `j` x3 moved Q-024 -> "Columns check" (Running) -> "Lanes check B" (Just finished); `k` moved back to "Columns check".
  - `/` then `TASK-066` kept only "Lanes check B", with the one plan line now in Just finished. Screenshot `/tmp/cols-search.png`.
  - Enter opened "Lanes check B"; after returning to Board, `j` x3 and `x` hid only "Lanes check B" (6 finished left; orb size, orb hover 3, stop-orb hover, stop-orb demo, edit-diffs codex demo, edit-diffs demo still shown).

Anti-goal:

- Before: `tests/keys.test.ts` and `tests/shortcut.test.ts` all pass after TASK-065.
- Final: all pass, 0 new failures; 2026-10-12.
- Result: PASS.

Lessons:

- `none`
