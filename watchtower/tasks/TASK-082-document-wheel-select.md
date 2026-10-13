# TASK-082 Document wheel select

Group: D (shared file: `docs/menus.md`)
Class: docs

## Brief

Goal: record the wheel-select contract in the doc that owns the menu engine, so the next agent does not rediscover it.

Change: no doc mentions the wheel on a pill -> [docs/menus.md](docs/menus.md) states the rule and links the ADR.

Boundaries:

- One fact, one doc. [docs/menus.md](docs/menus.md) owns the menu engine, so the rule goes there. Do not repeat it in [docs/hover.md](docs/hover.md) or [CLAUDE.md](CLAUDE.md).
- Do not document the code. Write the rule, the reason, and the gotcha. Leave the arithmetic in the helper.
- Integrate the text into the existing section about the trigger. Do not append a new section at the end of the file.
- Follow the doc voice in [CLAUDE.md](CLAUDE.md): plain, short, second person.

How:

- Read the final code of TASK-080 and TASK-081 before writing, so the text matches what shipped.
- State the rule: on web, the wheel over a closed pill steps its value by one option per tick, applies at once, and clamps at both ends.
- State the three gotchas: the listener is non-passive so it can call `preventDefault()`, the `sheet` surface is excluded because a sheet scrolls, and the wheel needs no hover state because the event only fires over the node.
- State that the model pill wheel stays inside the current provider.
- Link [ADR-0024](watchtower/decisions/ADR-0024-current-provider-wheel-order-over-every-provider.md) for the decision and the rejected options.

Files:

- [docs/menus.md](docs/menus.md) (the wheel-select rule, inside the existing trigger section)

Expected result:

- A reader of [docs/menus.md](docs/menus.md) learns that the wheel steps a closed pill, and learns why the sheet is excluded.
- The doc links the ADR instead of repeating its options.
- No other doc gains the same paragraph.

Anti-goal: the doc stays short; measured by `git diff --stat docs/menus.md` showing 20 added lines or fewer; drift gauge; read at completion.

## Verify

- `rg -n "wheel" docs/menus.md` -> the rule appears once, in the trigger section.
- `rg -n "wheel" docs/hover.md CLAUDE.md` -> no match, so the fact lives in one doc.
- `git diff --stat docs/menus.md` -> 20 added lines or fewer.
- `npm run format:check:files -- docs/menus.md` -> passes.
