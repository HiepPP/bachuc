# TASK-061 Migrate this repo's archive

Group: R (`watchtower/archive/**`, `watchtower/decisions/**`, `watchtower/DECISIONS.md`, `watchtower/MEMORY.md`)
Class: mechanical

## Brief

Goal: This repo's 5 archived plans use TASK-001 to TASK-056 and Q-001 to Q-021, in archive date order.

Change: each archived plan starts at TASK-001 -> one sequence across the archive.

Boundaries:

- Follows [ADR-0011](watchtower/decisions/ADR-0011-repo-wide-ids-over-per-plan-ids.md).
- The owner reviews the dry run before `--apply`. Do not apply without that review.
- Q-022 default: migrate only `watchtower/` at the git root. Leave `hiep-plugins/watchtower/` alone unless Q-022 says otherwise.
- The active plan already starts at TASK-059 and Q-022, so it stays.
- Commit messages keep their old IDs. Do not rewrite git history.

How:

- Run the dry run and paste its summary into the outcome sidecar.
- Ask the owner to approve the tables and the unresolved list.
- Run with `--apply`, then run the dry run again.
- Run `npm run format:files` on the changed markdown files.

Files:

- [watchtower/archive/](watchtower/archive/) (renames and ID tokens)
- [watchtower/decisions/](watchtower/decisions/) (ID tokens only)
- [watchtower/DECISIONS.md](watchtower/DECISIONS.md) (ID tokens only)
- [watchtower/MEMORY.md](watchtower/MEMORY.md) (ID tokens only)

Expected result:

- `ls watchtower/archive/*/tasks` shows TASK-001 to TASK-056 with no repeats.
- Each archived `NEXT.md` Tracker links resolve to existing files.
- The second dry run prints "nothing to migrate".

Anti-goal: broken markdown links under `watchtower/` do not rise above the baseline of 61 (archived plans still link to `watchtower/tasks/`); tripwire; read with a link check over `watchtower/**/*.md` before and after `--apply`.

## Verify

- `node ~/.claude/skills/watchtower/scripts/migrate-ids.mjs --repo .` -> "nothing to migrate".
- For each archive folder, list its distinct `TASK-NNN` file prefixes; across folders, `sort | uniq -d` -> no output (a spec and its outcome share one ID inside a folder).
- Link check over `watchtower/**/*.md` (each `](watchtower/...)` target exists) -> 61 missing or fewer, the same baseline as before `--apply`.
- `git status --short` -> only paths under `watchtower/`.
