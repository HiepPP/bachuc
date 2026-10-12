# TASK-060 Outcome

## Outcome

Status: DONE

Changed:

- New `~/.claude/skills/watchtower/scripts/migrate-ids.mjs`: dry run by default, `--apply` writes. Orders plans by slug, keeps a plan whose IDs already sit above the previous highest, renames `TASK-NNN-*` files in two passes with `git mv` when tracked, and rewrites tokens in plan folders, ADRs, `DECISIONS.md`, and `MEMORY.md`.
- New `migrate-ids.test.mjs`: four cases on a throwaway git repo.

Contract:

- A global reference resolves through, in order: a slug on the line, a slug in the section heading, the one slug the file names, the plan of an ADR the line links, then the one plan whose `[slug date, Updated]` span holds the date. Otherwise it stays and is listed as unresolved.
- Plan date spans use the slug date as start, because the archive prefix is the creation date, not the archive date.

Verified:

- `node --test migrate-ids.test.mjs` -> 4 pass, 0 fail.
- Dry run on this repo -> 5 archived plans plus the active plan, TASK-001 to TASK-056, Q-001 to Q-021, 100 renames, 714 tokens in 121 files, 5 unresolved; `git status` unchanged.

Anti-goal:

- Test repo after `--apply`: 0 paths outside `watchtower/`; `git status --short`, 2026-10-12.
- This repo after the dry run: 0 changed paths from the script; `git status --short -- watchtower`, 2026-10-12.
- Result: PASS against a limit of 0.

Lessons:

- This repo's archive holds 56 TASKs, not 58: `plugin-host-apis/tasks/` has two `TASK-014-*.patch` files. The script renames them too.
- ADR-0003 names no plan slug, so its TASK-010 and TASK-007 stay unresolved; they belong to `20261006-t3code-orchestration-port`.
