# TASK-082 Outcome

## Outcome

Status: DONE

Changed: `docs/menus.md` (12 lines added after the `ContextMenu` paragraph in "Two presentations"; the doc has no section named for the trigger, so this is the nearest trigger text).

Contract: `docs/menus.md` states the wheel rule, three gotchas, the model-pill provider limit, and links ADR-0024.

Verified:

- `rg -n "wheel" docs/menus.md` -> matches only lines 29-35 and 39, one block.
- `rg -n "wheel" docs/hover.md CLAUDE.md` -> no match.
- `git diff --stat docs/menus.md` -> 12 insertions.
- `npm run format:check:files -- docs/menus.md` -> all matched files use the correct format.

Anti-goal: Final: the doc stays short (20 added lines or fewer). Result: pass, 12 added lines.

Lessons:

- `docs/menus.md` has no trigger section; the rule sits in "Two presentations" next to the trigger text.
