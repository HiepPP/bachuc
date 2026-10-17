# TASK-115 Select a TASK without resetting the rail

Group: G (shares `board-rail.tsx` with TASK-114)
Class: code

## Brief

Goal: Pressing a tree row selects the TASK in the rail without resetting its collapsed groups, and brings the TASK into view on a narrow screen.

Change: The tree remounts the rail task card with `initialTaskId` ([Q-045](watchtower/QUESTIONS.md) default) -> the rail takes a controlled `selectedTaskId` and keeps its group state.

Boundaries:

- Add an optional controlled `selectedTaskId` with `onSelectTask` to `TaskList` and `TasksCard`. Keep `initialTaskId` working for the opener path (`openSurface` params).
- When the selected TASK sits in a collapsed group, open only that group.
- Below 720 px, scroll the selected row into view with the scroll view's own method. Do not use `scrollIntoView`.
- Remove the keyed remount from `overview.tsx`.

How:

- Write a test for any pure selection helper you add.
- Edit the task list, the rail card, and the overview wiring.

Files:

- [hiep-plugins/plugins/watchtower-board/client/task-list.tsx](hiep-plugins/plugins/watchtower-board/client/task-list.tsx) (controlled selection)
- [hiep-plugins/plugins/watchtower-board/client/board-rail.tsx](hiep-plugins/plugins/watchtower-board/client/board-rail.tsx) (pass through)
- [hiep-plugins/plugins/watchtower-board/client/overview.tsx](hiep-plugins/plugins/watchtower-board/client/overview.tsx) (drop the remount)

Expected result:

- A collapsed Todo group stays collapsed when you pick a Done TASK from the tree.
- The opener path still starts with its TASK selected.
- On a narrow screen the picked TASK scrolls into view.

Anti-goal: The opener path keeps working. Tripwire: `tests/board.test.ts` and `tests/overview.test.ts` keep their pass counts; read before and after.

## Verify

- `cd hiep-plugins/plugins/watchtower-board && npm test` -> every test file passes.
- `cd hiep-plugins/plugins/watchtower-board && npm run typecheck` -> exit 0.
- `rg -n "key=.*initialTaskId|scrollIntoView" hiep-plugins/plugins/watchtower-board/client` -> no match.
- Needs a human or a browser session: on live, collapse Todo, press a Done TASK in the tree; Todo stays collapsed and the TASK is selected.
- Anti-goal: the two test files' pass counts before and after are equal or higher.
