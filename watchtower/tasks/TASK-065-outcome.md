# TASK-065 Outcome

## Outcome

Status: DONE

Changed:

- New `client/keys.ts`: `boardKeyAction`, `stepKey`, `bindBoardKeys`, and the `BoardEntry` type.
- New `client/board-keys.ts`: `useBoardKeys`, the page-level search text, selection, and per-host entry registry; it binds keys on web only.
- `shared/lanes.ts`: `filterLanes` and `laneTrees`.
- `client/page.tsx`: a search field in the toolbar; each host lane filters its grids and registers its entries in reading order.
- `client/lanes.tsx` and `client/needs-you.tsx`: the selected card or item draws Board's focus ring; an empty search shows "Nothing matches".
- `README.md`: Search and Keys bullets in Task lanes.
- New `tests/keys.test.ts`.

Contract:

- Keys are ignored with Cmd, Ctrl, or Alt, while composing, and while a text field has focus (except Escape). Enter on a focused button or link stays with that control.
- Search never filters Needs you.

Verified:

- `npx tsx --test tests/keys.test.ts` -> 5 pass.
- `npm test` -> 129 pass, 0 fail.
- `npm run typecheck` -> exit 0; `npm run-script lint` -> 0 warnings, 0 errors.
- Human check on live, 2026-10-12, Expo web on daemon 6768: PASS after one fix.
  - `j` x4 moved the focus ring through the 3 Needs you items to the first card (TASK-064 row); `k` moved back one. Screenshot `/tmp/keys-1.png`.
  - `/` focused search; typing `TASK-064` kept only that row; `j` typed into the field and did not move the selection; Escape cleared the field and released focus.
  - Enter on a card opened its thread; Enter on a manual check item did nothing (it has no thread).
  - `x` on a running card did nothing; on a finished card it hid that card.
- Defect found and fixed: after opening a thread and returning, the earlier Board page stays mounted but hidden in the navigation stack, and its key listener still acted. One `x` hid two cards, the selected "Lanes check A" and the live demo card "orb hover 2". Fix: keys act only while the page's root view is shown (`isShown` in `client/keys.ts`, used by `useBoardKeys`), with a test. Retest of the same steps hid only "Lanes check adhoc". "orb hover 2" stays hidden on live until it starts a new turn.

Anti-goal:

- Before: `tests/shortcut.test.ts` 1 of 1 pass; 2026-10-12.
- Final: 1 of 1 pass ("Cmd+D opens Board once, suppresses dictation, and unregisters on cleanup"); 2026-10-12.
- Result: PASS.

Lessons:

- The rtk shell hook rewrites `npm run lint` output; `npm run-script lint > file` gives the plain oxlint result.
