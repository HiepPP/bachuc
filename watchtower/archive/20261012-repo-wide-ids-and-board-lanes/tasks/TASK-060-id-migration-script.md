# TASK-060 ID migration script

Group: M (`~/.claude/skills/watchtower/scripts/migrate-ids.mjs` and its test)
Class: risky

## Brief

Goal: One script renumbers a repo's archived TASKs and Q-IDs into one repo-wide sequence. It shows a dry run first and changes files only with `--apply`.

Change: no tool -> `node migrate-ids.mjs --repo <root> [--apply]`.

Boundaries:

- Follows [ADR-0011](watchtower/decisions/ADR-0011-repo-wide-ids-over-per-plan-ids.md).
- Order: archive folder date prefix, then slug, then the old ID. The active plan comes last.
- Scope of each mapping: IDs inside one archive folder map through that folder's table only. Cross-plan references are not guessed.
- ADR and `MEMORY.md` references: map an ID through the plan that was active on the ADR's `Date:` or the MEMORY entry's date. If no plan or more than one plan fits, leave the ID and list it as unresolved in the output.
- Idempotent: on a migrated repo the mapping is the identity, and the script prints "nothing to migrate".
- Never delete a file. Renames use `git mv` when the path is tracked.
- No ID map file is written. The dry run output is the only mapping record.
- Rewrite only whole-word `TASK-\d+` and `Q-\d+` tokens, including those inside file names and links.

How:

- Scan `watchtower/archive/*/` and the active plan. Build one table per plan: old ID -> new ID.
- Dry run: print each plan's table, every file rename, the count of replaced tokens per file, and the unresolved references.
- `--apply`: rename `TASK-NNN-*.md` files, rewrite tokens in each plan folder, then in `watchtower/decisions/*.md`, `watchtower/DECISIONS.md`, and `watchtower/MEMORY.md`.
- Write `migrate-ids.test.mjs` that builds a throwaway repo with two archived plans and one ADR, runs the dry run and `--apply`, and checks the result and a second run.

Files:

- [scripts/migrate-ids.mjs](/Users/hiep/.claude/skills/watchtower/scripts/migrate-ids.mjs) (new)
- [scripts/migrate-ids.test.mjs](/Users/hiep/.claude/skills/watchtower/scripts/migrate-ids.test.mjs) (new)

Expected result:

- Dry run changes no file: `git status` is clean after it.
- `--apply` gives ascending, gap-free TASK and Q-IDs across plans.
- A second run prints "nothing to migrate".
- An ADR reference with no matching plan stays and appears under unresolved.

Anti-goal: files changed outside `watchtower/` stay at 0; tripwire; read from `git status --short` in the test repo after `--apply`, and in this repo before TASK-061 starts.

## Verify

- `node --test ~/.claude/skills/watchtower/scripts/migrate-ids.test.mjs` -> all tests pass.
- `node ~/.claude/skills/watchtower/scripts/migrate-ids.mjs --repo /Users/hiep/Projects/bachuc` -> dry run prints 5 archived plans plus the active plan, TASK-001 to TASK-056, Q-001 to Q-021, and no file changes in `git status`.
- The test's `--apply` case -> `git status --short` shows only paths under `watchtower/`.
