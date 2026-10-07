# TASK-002 Outcome

## Outcome

Status: DONE

Changed:

- [hiep-plugins/plugins/watchtower-board/shared/overview.ts](hiep-plugins/plugins/watchtower-board/shared/overview.ts) gained `run` (with every log row, `startMinute`, `endMinute`, and `nowMinute`), `questions`, `decisions`, `history`, and `HISTORY_LIMIT` (5).
- [hiep-plugins/plugins/watchtower-board/server/overview.ts](hiep-plugins/plugins/watchtower-board/server/overview.ts) gained `parseOverviewRun`, `parseOverviewQuestions`, `parseDecisions`, and an archive reader. `readOverview` takes an optional `now` for tests.
- [hiep-plugins/plugins/watchtower-board/server/board.ts](hiep-plugins/plugins/watchtower-board/server/board.ts) now exports `message`, with no behavior change.
- [hiep-plugins/plugins/watchtower-board/tests/overview.test.ts](hiep-plugins/plugins/watchtower-board/tests/overview.test.ts) gained 5 tests: midnight rollover, question filter and ADR rows, history with a symlink and a file, an unreadable `QUESTIONS.md`, and the real t3code plan.
- Spec change 1: overview tasks no longer carry `brief`. The anti-goal read 46,041 bytes for the t3code plan, and 37,972 bytes of that were brief bodies, which the overview never shows. TASK-001 said "the board task fields"; the overview now has every board task field except `brief`.
- Spec change 2: `nowMinute` stops at `Finished:` once a run ends, so an old finished run does not stretch the timeline axis to today.

Contract:

- `watchtower.read` and `boardSchema` are unchanged.
- Log minutes count from `Started:`. A time earlier than the time before it adds 24 hours. `now`, `-`, or an unreadable time gives `null`.
- `questions` holds `OPEN` and `DEFAULTED` rows. `history` holds at most 5 real folders, newest first; symlinks and files are skipped, and an archive outside `watchtower/` is not read.
- The overview's own file reads stop at 1 MiB with a warning, on top of the `readBoard` cap.

Verified:

- `npx tsx --test tests/overview.test.ts` -> 11 pass, 0 fail, including the rollover, history, and symlink cases.
- `npx tsx --test tests/board.test.ts` -> 10 pass, 0 fail.
- `npm test` in the plugin folder -> 26 pass, 0 fail.
- `npm run typecheck` -> exit 0. `npm run lint` -> exit 0, 0 warnings.

Anti-goal:

- Before the brief change: 46,041 bytes; `t.diagnostic` in the real t3code plan test, 2026-10-07 12:40. Over the 32,768-byte limit, so the brief bodies were removed.
- Final: 7,497 bytes; same test, 2026-10-07 12:41.
- Result: PASS against 32,768 bytes.

Lessons:

- Overview tasks have no `brief`. A later TASK that needs a brief must read it from `watchtower.read` or add a separate call. Source: [hiep-plugins/plugins/watchtower-board/shared/overview.ts](hiep-plugins/plugins/watchtower-board/shared/overview.ts).
- `readOverview(directory, now)` and `parseOverviewRun(markdown, now)` take a fixed `now`, so timeline tests can pin minutes. Source: [hiep-plugins/plugins/watchtower-board/tests/overview.test.ts](hiep-plugins/plugins/watchtower-board/tests/overview.test.ts).
