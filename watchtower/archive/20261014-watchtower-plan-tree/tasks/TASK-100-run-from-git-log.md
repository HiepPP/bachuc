# TASK-100 Run timing from the git log

Group: F (writes the server overview reader and its test)
Class: code

## Brief

Goal: Give plans that `implement` runs a run log, so the pace line and row times show without `watchtower/RUN.md`.

Change: Without `RUN.md` the overview has `run: null` and the page says "No autorun yet" -> the server builds `run` from the branch commits that name a TASK of the plan.

Boundaries:

- Only when `RUN.md` is absent. A present `RUN.md` keeps its current parse.
- Read `git log --format=%H%x09%cI%x09%s` in the workspace directory with `execFile` (no shell), a 5 s timeout, and at most 500 commits. Any git error leaves `run: null` and adds one warning, except "not a git repository" and "no commits yet", which stay silent ([Q-039](watchtower/QUESTIONS.md), answered 2026-10-14) so plain fixtures stay warning-free.
- A commit counts when a parenthesised group in its subject names a TASK ID of the Tracker, such as `(TASK-095)` or `(TASK-101, TASK-102)`. A bare ID, as in "add TASK-100..103 to the plan", does not count, so a plan commit does not finish its TASKs ([Q-040](watchtower/QUESTIONS.md), answered 2026-10-14). The newest commit per TASK is its end.
- Start of a TASK: the time of the commit just before its first counted commit on the branch. `Started:` of the run: the time of the oldest commit that touches `watchtower/NEXT.md` with the plan slug in its subject or body, or else the oldest counted commit's parent.
- Each log row: `task`, `result` `DONE`, `detail` `commit <short sha>`, start and end clock times, and start and end minutes from `Started:`. TASK-097 already reads a SHA from `detail`.
- `runner` is `implement (git)`. `finished` is the newest counted commit time when every Tracker row is `DONE`, else null.
- Pure parse in a testable function: `runFromGitLog(lines, tasks, slug)`. The git call stays a thin wrapper.

How:

- Write tests in `tests/overview.test.ts` first with fixed log lines.
- Add the function and the fallback call in `server/overview.ts`.

Files:

- [hiep-plugins/plugins/watchtower-board/server/overview.ts](hiep-plugins/plugins/watchtower-board/server/overview.ts) (git log fallback)
- [hiep-plugins/plugins/watchtower-board/tests/overview.test.ts](hiep-plugins/plugins/watchtower-board/tests/overview.test.ts) (new cases)

Expected result:

- With this plan's own commits, the page shows the pace line and a time on each done row, and each done row names its commit.
- No counted commit leaves `run: null` and the page keeps "No autorun yet".
- A git failure adds one warning and does not break the overview.

Anti-goal: The overview read stays fast. Tripwire: the overview read with git takes under 300 ms on this repo; read with a timer around `readOverview` before and after the change.

## Verify

- `cd hiep-plugins/plugins/watchtower-board && npx tsx --test tests/overview.test.ts` -> all cases pass, with new git-log cases.
- `cd hiep-plugins/plugins/watchtower-board && npm run typecheck` -> exit 0.
- `cd hiep-plugins/plugins/watchtower-board && rtk proxy npm run lint` -> 0 warnings, 0 errors.
- Anti-goal: the timed `readOverview` on this repo is under 300 ms; record both reads in the outcome.
