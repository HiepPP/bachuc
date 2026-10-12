# TASK-059 Outcome

## Outcome

Status: DONE

Changed:

- `SKILL.md`: one Rules line for repo-wide TASK and Q-IDs.
- `references/plan-format.md`: an IDs bullet that says how to pick the next TASK and Q-ID.
- `references/task-template.md`: the template IDs are marked as placeholders.
- `references/mode-flows.md`: Author Plan step 3 runs `migrate-ids.mjs` as a dry run, asks the owner, then applies.
- `references/decision-template.md`: R3 names the ID migration as its one exception.

Contract:

- ADR numbering is unchanged.
- `TASK-NNN` and `Q-NNN` shapes are unchanged; parsers that match `TASK-\d+` keep working.

Verified:

- `rg -n "repo-wide" SKILL.md references/plan-format.md` -> both files state the rule.
- `rg -n "migrate-ids.mjs" references/mode-flows.md` -> line 56, Author Plan step 3.
- `rg -n "exception" references/decision-template.md` -> R3 names the migration exception.

Anti-goal:

- Before: 861 lines in the five files; `wc -l`, 2026-10-12.
- Final: 865 lines; `wc -l`, 2026-10-12.
- Result: PASS against a growth limit of 40 lines (grew by 4).

Lessons:

- The skill lives in the `~/.claude` git repo; commit it there, not in this repo.
