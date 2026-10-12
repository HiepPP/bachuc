# TASK-065 Board keyboard and search

Group: B2 (shares the Board client files with TASK-064)
Class: code

## Brief

Goal: You move through Board with the keyboard and filter it by text or TASK-ID.

Change: mouse only -> `j`/`k` move, Enter opens, `x` removes a finished card, `/` focuses search.

Boundaries:

- Keys work only while Board has focus and no text field is focused, except Escape in search.
- Order of movement: Needs you column top to bottom, then each project grid row by row.
- Search matches thread title, project name, and TASK-ID. Rows with no match hide; Needs you keeps its items.
- Web and desktop only. Native keeps touch behavior.

How:

- Add `client/keys.ts` with the move order and key handling.
- Add the search field to the Board toolbar and a filter in `lanes.tsx`.
- Show the focus ring that Board already uses for keyboard focus.

Files:

- [hiep-plugins/plugins/board/client/keys.ts](hiep-plugins/plugins/board/client/keys.ts) (new)
- [hiep-plugins/plugins/board/client/lanes.tsx](hiep-plugins/plugins/board/client/lanes.tsx) (filter and focus)
- [hiep-plugins/plugins/board/client/page.tsx](hiep-plugins/plugins/board/client/page.tsx) (toolbar search)
- [hiep-plugins/plugins/board/README.md](hiep-plugins/plugins/board/README.md) (keys)
- [hiep-plugins/plugins/board/tests/keys.test.ts](hiep-plugins/plugins/board/tests/keys.test.ts) (new)

Expected result:

- `j` from the last Needs you item moves to the first card of the first grid.
- `x` on a running card does nothing.
- Typing `TASK-063` shows only that row and Ad-hoc cards with that chip.

Anti-goal: Cmd+D still opens Board from the composer, 1 of 1 shortcut tests pass; tripwire; read from `tests/shortcut.test.ts` before and at completion.

## Verify

- `cd hiep-plugins/plugins/board && npx tsx --test tests/keys.test.ts` -> pass.
- `cd hiep-plugins/plugins/board && npx tsx --test tests/shortcut.test.ts` -> pass.
- Human check on live: `j`, `k`, Enter, `x`, and `/` work; typing in search does not move focus.
