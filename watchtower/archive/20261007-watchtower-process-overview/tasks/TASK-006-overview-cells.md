# TASK-006 Overview cells

Group: E (standalone: `client/overview-cells.tsx`)
Class: code

## Brief

Goal: Build the presentational parts of the overview: the lifecycle stepper, Needs you, the run log, manual checks, and decisions and history.

Change: no overview parts -> five components that take model data and the plugin theme as props.

Design: option 3f in the mock named in [watchtower/CONTEXT.md](watchtower/CONTEXT.md).

Boundaries:

- Presentational only. The cells take data through props and never fetch. TASK-007 owns the query.
- Read-only actions. Q-001 in [watchtower/QUESTIONS.md](watchtower/QUESTIONS.md) is answered: the overview never writes plan files. Each Needs you row has a Copy action that copies its text with `copyText`.
- Use sentence-case labels, `theme.colors` only, and `Icon` from `@getpaseo/plugin/client/react-native` with Lucide names. No emoji, no em dash.
- Every list shows at most 5 rows, then a `"<n> more"` line.

How:

- `LifecycleStepper`: five equal cells from `lifecyclePhases`. The current cell gets an accent tint, a 2 px accent bottom bar, and a small segmented progress bar of done, running, blocked, and todo counts.
- `NeedsYouCell`: rows from `needsYou`, with a tone icon, title, meta, and a Copy action.
- `RunLogCell`: the newest 5 log rows in a mono grid: start, end, task, result, detail. A running task shows `now` and its minutes in the accent color.
- `ManualChecksCell`: each check as an empty square, the text, and its task IDs.
- `HistoryCell`: ADR rows (id, title, status), then history rows (title, archived date, and `LEARN.md` when present).
- Each cell has an empty line, for example `"Nothing waits on you."`.

Files:

- [hiep-plugins/plugins/watchtower-board/client/overview-cells.tsx](hiep-plugins/plugins/watchtower-board/client/overview-cells.tsx) (new)

Expected result:

- The file exports `LifecycleStepper`, `NeedsYouCell`, `RunLogCell`, `ManualChecksCell`, and `HistoryCell`.
- Each cell typechecks against the TASK-002 overview types and the TASK-003 model types.
- Each cell renders its empty line when its list is empty.
- The visible result is checked on live in Plan Verify, after TASK-008.

Anti-goal: the cells stay presentational: data hooks in `client/overview-cells.tsx` stay at 0; tripwire; read with `rg -c "useQuery|useRpc" client/overview-cells.tsx` before the first edit, after the first cell, and at completion.

## Verify

- In the plugin folder: `npm run typecheck` and `npm run lint` -> exit 0.
- `rg -n "useQuery|useRpc" hiep-plugins/plugins/watchtower-board/client/overview-cells.tsx` -> no match.
- `rg -n "—" hiep-plugins/plugins/watchtower-board/client/overview-cells.tsx` -> no match.
- `rg -n "export function (LifecycleStepper|NeedsYouCell|RunLogCell|ManualChecksCell|HistoryCell)" hiep-plugins/plugins/watchtower-board/client/overview-cells.tsx` -> 5 matches.
