# TASK-068 Board search and keys on the two columns

Group: B3 (shares the Board client files with TASK-067)
Class: code

## Brief

Goal: Search and keyboard navigation from TASK-065 work on the restored Running and Just finished columns.

Change: `filterLanes` and `laneTrees` over grids -> a filter and a reading order over the two columns.

Boundaries:

- Follows [ADR-0012](watchtower/decisions/ADR-0012-status-columns-with-task-chips-over-task-lanes.md).
- Keep every key rule of TASK-065: j/k and arrows, Enter, x on finished cards only, `/` and Escape, no action in text fields, Enter left to a focused button, and only a shown Board page acts (`isShown`).
- Reading order: Needs you, then Running top to bottom, then Just finished, host by host.
- Search matches thread title, project, and TASK-ID of the card or any subagent; a plan line stays when any card of its project stays. Needs you is never filtered.

How:

- Replace `filterLanes` and `laneTrees` in `shared/lanes.ts` with `filterTrees(trees, query)` and use it for both columns.
- Build the key entries from Needs you, Running, then Just finished in `client/page.tsx`.
- Move the selection ring from the lane cell wrapper to the column's card wrapper.
- Update `tests/keys.test.ts` for the new filter and order.

Files:

- [hiep-plugins/plugins/board/shared/lanes.ts](hiep-plugins/plugins/board/shared/lanes.ts) (`filterTrees`)
- [hiep-plugins/plugins/board/client/page.tsx](hiep-plugins/plugins/board/client/page.tsx) (entries, ring)
- [hiep-plugins/plugins/board/tests/keys.test.ts](hiep-plugins/plugins/board/tests/keys.test.ts)
- [hiep-plugins/plugins/board/README.md](hiep-plugins/plugins/board/README.md) (search and keys wording)

Expected result:

- Typing `TASK-066` keeps only cards with that chip, in either column, plus their plan line.
- `j` from the last Needs you item selects the first Running card, then continues into Just finished.
- After opening a thread and returning, one `x` hides exactly one finished card.

Anti-goal: the TASK-065 key cases keep passing, 0 new failures in `tests/keys.test.ts` and `tests/shortcut.test.ts`; tripwire; read before the change and at completion.

## Verify

- `cd hiep-plugins/plugins/board && npx tsx --test tests/keys.test.ts tests/shortcut.test.ts` -> all pass.
- `cd hiep-plugins/plugins/board && npm test` -> all pass.
- Human check on live (Expo web on daemon 6768): j/k order, Enter, x after returning from a thread, `/`, and Escape behave as above.
