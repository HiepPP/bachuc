# TASK-059 Repo-wide ID rule in the skill

Group: S (the skill's markdown files under `~/.claude/skills/watchtower/`)
Class: docs

## Brief

Goal: The Watchtower skill numbers TASKs and Q-IDs across the whole repo, and runs the migration the first time a repo gets a new plan.

Change: TASK-001 and Q-001 in every plan -> next TASK and Q-ID after the highest one in `watchtower/` and `watchtower/archive/`.

Boundaries:

- Follows [ADR-0011](watchtower/decisions/ADR-0011-repo-wide-ids-over-per-plan-ids.md).
- Keep the `TASK-NNN` and `Q-NNN` shapes. Pad to 3 digits and let the number grow past 999. Parsers in watchtower-board already match `TASK-\d+`.
- Add one explicit exception to R3: the ID migration may replace TASK and Q-ID references in ADR bodies, and nothing else in them.
- Do not change ADR numbering. It is already repo-wide.

How:

- In `references/plan-format.md` and `references/task-template.md`, state how to pick the next TASK and Q-ID: one more than the highest ID in `watchtower/tasks/`, `watchtower/QUESTIONS.md`, and every `watchtower/archive/*/`.
- In `references/mode-flows.md` Author Plan CREATE, add a step before writing files: run `node ~/.claude/skills/watchtower/scripts/migrate-ids.mjs --repo <root>`. If it reports work, show the dry run, ask the owner, then run it with `--apply`.
- In `references/decision-template.md`, add the R3 exception with a link to the migration step.
- In `SKILL.md`, add one Rules line for repo-wide IDs.
- Update the template examples so they no longer suggest that every plan starts at 001.

Files:

- [SKILL.md](/Users/hiep/.claude/skills/watchtower/SKILL.md) (one Rules line)
- [references/plan-format.md](/Users/hiep/.claude/skills/watchtower/references/plan-format.md) (ID rule)
- [references/task-template.md](/Users/hiep/.claude/skills/watchtower/references/task-template.md) (ID rule and examples)
- [references/mode-flows.md](/Users/hiep/.claude/skills/watchtower/references/mode-flows.md) (migration step in Author Plan)
- [references/decision-template.md](/Users/hiep/.claude/skills/watchtower/references/decision-template.md) (R3 exception)

Expected result:

- A reader of the skill picks TASK-067 for the next TASK added to this repo, not TASK-001.
- The Author Plan flow names the migration script, the dry run, and the owner ask before `--apply`.
- R3 names exactly one exception.

Anti-goal: the skill's total line count grows by no more than 40 lines; drift gauge; read with `wc -l` on the five files at completion.

## Verify

- `rg -n "repo-wide" ~/.claude/skills/watchtower/SKILL.md ~/.claude/skills/watchtower/references/plan-format.md` -> both files state the rule.
- `rg -n "migrate-ids.mjs" ~/.claude/skills/watchtower/references/mode-flows.md` -> the Author Plan CREATE step names it.
- `rg -n "exception" ~/.claude/skills/watchtower/references/decision-template.md` -> R3 names the migration exception.
- `wc -l` on the five files before and after -> growth of 40 lines or less.
