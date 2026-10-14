# TASK-101 Outcome

## Status

DONE (live screenshots UNVERIFIED, unattended)

## Changed

- `hiep-plugins/plugins/watchtower-board/client/board-rail.tsx`: `ProgressCard` shows one bar segment per TASK and one mono count line, not four count boxes.
- `hiep-plugins/plugins/watchtower-board/client/needs-you.tsx`: `ActionButton` draws a text link, not a bordered button.
- `hiep-plugins/plugins/watchtower-board/client/plan-tree-view.tsx`: done rows read "committed <sha>", and the spine is 2 px.

## Contract

- Segment colors: done `statusSuccess`, running (status IN PROGRESS) `accent`, waiting (status BLOCKED) `statusWarning`, not started `surface2`. The count line is `N done · N running · N waiting · N not started`; running and waiting are left out at 0. The percent and "N of M done" are unchanged.
- Needs you: every action is accent text with no border. `Reject` and `Confirm reject` are `statusDanger`. `Cancel` is `foregroundMuted`. The actions of one row sit in one wrapping line. All handlers, the Reject confirm step, and the disabled rules are unchanged.
- Done row: second line is `committed <sha>` when `commits` has the TASK, else "Group A, code" as before.
- Spine: `colors.border` at 2 px (`SPINE_WIDTH`), with a 2 px tick to each row. The theme has one border token, so "full strength" means the same token at twice the width.

## Verified

- `npm test` -> exit 0, 92 pass, 0 fail (before the TASK-100 builder's test edits landed).
- `npm run typecheck` -> no error in `client/`. The run at the end shows errors only in `tests/overview.test.ts`, which the TASK-100 builder is editing (`runFromGitLog` is not exported yet). Not mine.
- `rtk proxy npm run lint` -> `Found 0 warnings and 0 errors` for the final tree. One earlier run showed a `gitMessage` unused warning in `server/overview.ts`, the other builder's file.
- `npm run format` -> `git status` shows my five client files plus the other builder's `server/overview.ts` and `tests/overview.test.ts`.
- `rg -n "#[0-9A-Fa-f]{6}" client` -> no match.
- Live screenshot at 1440 px and 500 px: UNVERIFIED (unattended). The lead checks it.

## Anti-goal

Tripwire: `tests/actions.test.ts` and `tests/overview.test.ts` keep their pass counts.

| Checkpoint               | Before  | After                                                                          |
| ------------------------ | ------- | ------------------------------------------------------------------------------ |
| `tests/actions.test.ts`  | 6 pass  | 6 pass                                                                         |
| `tests/overview.test.ts` | 19 pass | 19 pass (read before the other builder's edits landed; the file is theirs now) |

Source: `npx tsx --test tests/<file>.test.ts`, the `ℹ pass` line, 2026-10-14. Not crossed.

## Lessons

- The plugin theme has one `border` token. A stronger spine needs more width, not a darker color.
- `ProgressCard` is also used by `client/board.tsx` (the project panel); it gets the same bar.
- Tests cannot import `client/*.tsx` (they import `react-native`), so the segment and link changes have no unit test. The live check covers them.
