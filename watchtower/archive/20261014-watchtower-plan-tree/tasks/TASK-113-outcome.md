# TASK-113 Outcome

## Status

DONE

## Changed

- `hiep-plugins/plugins/watchtower-board/server/overview.ts`: `runFromGitLog`, `readGitLog`, and the fallback in `readOverview`.
- `hiep-plugins/plugins/watchtower-board/tests/overview.test.ts`: 8 new cases.

## Contract

- `runFromGitLog(lines, tasks, slug, planLines = [], now = new Date())` is pure. `lines` are `sha<TAB>iso<TAB>subject`, newest first. `planLines` are `sha<TAB>iso<TAB>subject<TAB>body` records of commits that touch `watchtower/NEXT.md`.
- `readOverview(directory, now, git = readGitLog)` calls it only when `RUN.md` is absent and the Tracker has a TASK. An unreadable `RUN.md` already warns and does not fall back.
- A commit counts when a parenthesised group of its subject names a Tracker TASK, such as `(TASK-108)` or `(TASK-001, TASK-002)`. Newest counted commit is the TASK end. Start is the commit just before its oldest counted commit.
- `Started:` is the oldest `NEXT.md` commit whose subject or body has the slug, else the parent of the oldest counted commit.
- Rows: `result` `DONE`, `detail` `commit <sha7>`. A time on another day than `Started:` carries its date. `runner` is `implement (git)`. `finished` is set when every Tracker row is `DONE`.
- `readGitLog` uses `execFile("git", ...)` with `cwd`, 5 s timeout, `-n 500`, two calls in parallel. Outside a repository, or with no commits, it returns no lines and adds no warning. Any other git error adds one warning, `git log: <reason>`, and `run` stays null.

## Verified

- `npx tsx --test tests/overview.test.ts` -> 27 pass, 0 fail (8 new git-log cases, including a real temp repository).
- `npm run typecheck` -> exit 0.
- `rtk proxy npm run lint` -> 0 warnings, 0 errors.
- `npm run format` -> changed only my two files (the other modified `client/*.tsx` files belong to the other builder).
- On this repo, the timed run showed pace data: 5 rows (TASK-108..099), `Started: 2026-10-14 12:00`, `finished` null (TASK-113 to TASK-116 are open).

## Anti-goal

Tripwire: `readOverview` on this repo under 300 ms. Timer around `readOverview`, script `/tmp/wt100-time.mts`, 2026-10-14 05:46 to 05:49 UTC, source: `performance.now()`.

| Checkpoint | Before (`run: null`) | After (git log) |
| ---------- | -------------------- | --------------- |
| first read | 11.4 ms              | 179.9 ms        |
| reads 2-4  | 2.2 to 2.6 ms        | 91.5 to 99.6 ms |

Tripwire not crossed. The path-limited `git log -- watchtower/NEXT.md` costs about 86 ms on this repo and dominates; the plain log costs about 27 ms.

## Lessons

- A subject such as `add TASK-113..103 to the plan` would count TASK-113 as finished if any bare `TASK-NNN` counted, so only parenthesised IDs count.
- `git log` outside a repository exits 128. Existing fixtures sit in a temp folder with no repository and a Tracker, and assert `warnings: []`, so "not a git repository" stays silent. This differs from "any git error adds one warning".
- Tests build log times in local time (`new Date(y, m, d, h, min).toISOString()`) because the run log shows local clock times.
- `npm run format` for the plugin formats all plugin files, so it can touch the other builder's files. Check `git status` after it.
