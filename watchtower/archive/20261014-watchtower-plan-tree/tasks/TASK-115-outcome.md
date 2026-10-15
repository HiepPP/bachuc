# TASK-115 Outcome

## Status

DONE (live check UNVERIFIED, unattended)

## Changed

- `hiep-plugins/plugins/watchtower-board/client/task-list.tsx`: controlled `selectedTaskId` and `onSelectTask`, a group reveal effect, and an `onRevealSelected` hook.
- `hiep-plugins/plugins/watchtower-board/client/board-rail.tsx`: `TasksCard` passes the three props through.
- `hiep-plugins/plugins/watchtower-board/client/overview.tsx`: the keyed remount and the `picked` counter are gone. The page holds `selectedTaskId` (starts from `initialTaskId`) and scrolls the stacked page.

## Contract

- `TaskList` and `TasksCard` take optional `selectedTaskId?: string | null`, `onSelectTask?(id: string | null)`, and `onRevealSelected?(row: View)`. With `selectedTaskId` undefined the list keeps its own selection and `initialTaskId` works as before (`client/board.tsx` and the opener path).
- Pressing a list row calls `onSelectTask(id)`; pressing the selected row calls it with `null`.
- When the selected TASK sits in a collapsed group, an effect opens only that group. It runs when the picked TASK or its group changes, so a later manual collapse stays collapsed.
- `onRevealSelected` gets the selected row once it is mounted and its group is open. The page passes it only below 720 px. It calls `row.measureLayout(scroll.getInnerViewNode(), ...)`, then `scroll.scrollTo({ y: y - 12, animated: true })` on the stacked page `ScrollView`.
- Picking the same TASK in the tree again keeps it selected and does not scroll again.

## Verified

- `npm test` -> exit 0 before the other builder's test edits (92 pass). The last run showed 1 fail in `tests/overview.test.ts`, the TASK-113 builder's file (`runFromGitLog` is not exported yet).
- `npm run typecheck` -> no error in `client/`; errors only in `tests/overview.test.ts` (not mine).
- `rtk proxy npm run lint` -> `Found 0 warnings and 0 errors` for the final tree.
- `npm run format` -> changed only my files plus the other builder's two files.
- `rg -n "key=.*initialTaskId|scrollIntoView" client` -> no match.
- `tests/board.test.ts`, `tests/plan-tree.test.ts`, `tests/actions.test.ts`, `tests/builders.test.ts`: 46 pass, 0 fail.
- Live (collapse Todo, press a Done TASK in the tree; narrow scroll): UNVERIFIED (unattended).

## Anti-goal

Tripwire: `tests/board.test.ts` and `tests/overview.test.ts` keep their pass counts.

| Checkpoint               | Before  | After                                              |
| ------------------------ | ------- | -------------------------------------------------- |
| `tests/board.test.ts`    | 10 pass | 10 pass                                            |
| `tests/overview.test.ts` | 19 pass | not comparable: the TASK-113 builder is editing it |

Source: `npx tsx --test tests/<file>.test.ts`, 2026-10-14. The opener path is unchanged in code: `initialTaskId` seeds `selectedTaskId` and the lazy collapsed state. Not crossed, but the `overview.test.ts` count must be read again by the lead after TASK-113.

## Lessons

- `measureLayout` against `getInnerViewNode()` works on RN-web 0.21 and on native for the plugin `ScrollView` (it forwards the ref to the RN `ScrollView`). Not run live.
- The reveal effect reads a ref to the selected row, which exists only after the group opens, so it depends on the group being open.
- Gap: picking an already selected TASK from the tree on a narrow page does not scroll again. A reveal counter would fix it; not added to stay in scope.
