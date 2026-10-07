# TASK-002 Overview run, questions, and history

Group: A (shared file: `server/overview.ts`, `shared/overview.ts`, and `tests/overview.test.ts`, also written by TASK-001)
Class: code

## Brief

Goal: Extend the overview RPC with the full run log as minute offsets, the owner questions, the ADR index, and the archived plan history.

Change: the overview has plan and task data only -> it also has `run`, `questions`, `decisions`, and `history`.

Boundaries:

- The board RPC keeps its five-row run log. The overview has its own full log.
- Read only through the guarded `readFile`. List `watchtower/archive/` with `readdir` and `withFileTypes`. Skip an entry that is not a real directory, and skip a symlink.
- Read at most 5 archived `NEXT.md` files per call. Keep the 1 MiB total cap.

How:

- `run`: `runner`, `started`, `finished`, and `log` with every row of `RUN.md` `## Log`. Each row has `task`, `result`, `detail`, `start`, `end`, `startMinute`, and `endMinute`.
- Minutes count from the date and time in `Started:`. Log times are `HH:MM` with no date. Walk the rows in order and add 24 hours whenever a time is earlier than the time before it. An `end` of `now` or `-` gives `endMinute: null`.
- `nowMinute`: minutes from `Started:` to the read time, or `null` when there is no run.
- `questions`: every row of `QUESTIONS.md` with Status `OPEN` or `DEFAULTED`. Keep `id`, `tasks` (TASK column), `blocks` (TASK IDs, empty for `-`), `question`, `default`, and `status`.
- `decisions`: every row of the `watchtower/DECISIONS.md` index, with `id`, `date`, `title`, and `status`.
- `history`: the 5 newest folders of `watchtower/archive/`, by name, newest first. Each has `slug`, `date` (from the `YYYYMMDD` slug prefix, or `null`), `title` (from the archived `NEXT.md` `Title:`, or the slug), and `hasLearn` (a `LEARN.md` file exists).
- A missing optional file gives an empty value. An unreadable one adds a warning, like `readRunState` does.

Files:

- [hiep-plugins/plugins/watchtower-board/shared/overview.ts](hiep-plugins/plugins/watchtower-board/shared/overview.ts) (add the new fields)
- [hiep-plugins/plugins/watchtower-board/server/overview.ts](hiep-plugins/plugins/watchtower-board/server/overview.ts) (read the new fields)
- [hiep-plugins/plugins/watchtower-board/tests/overview.test.ts](hiep-plugins/plugins/watchtower-board/tests/overview.test.ts) (new cases)

Expected result:

- A log `22:52-23:06`, `23:09-23:28`, `00:02-00:21` with `Started: 2026-10-06 22:52` gives start minutes 0, 17, and 70.
- A row with end `now` gives `endMinute: null`.
- `questions` keeps `OPEN` and `DEFAULTED` rows and drops `ANSWERED` and `DROPPED` rows.
- A fixture with 8 archive folders gives 5 history entries, newest first.
- A symlinked archive folder is not listed.
- The real archived plan [watchtower/archive/20261006-t3code-orchestration-port/](watchtower/archive/20261006-t3code-orchestration-port/), copied into a temp fixture, parses without warnings.

Anti-goal: the overview stays small: `JSON.stringify(readOverview(fixture))` for the copied t3code plan fixture stays at or below 32 KiB; drift gauge; read in the test at completion.

## Verify

- In the plugin folder: `npx tsx --test tests/overview.test.ts` -> every test passes, including the rollover, history, and symlink cases.
- In the plugin folder: `npx tsx --test tests/board.test.ts` -> 10 pass, 0 fail.
- In the plugin folder: `npm run typecheck` and `npm run lint` -> exit 0.
- The overview size test prints or asserts the JSON length -> 32768 bytes or less.
