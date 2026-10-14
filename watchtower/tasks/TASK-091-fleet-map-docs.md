# TASK-091 Fleet map docs

Group: F (writes docs only)
Class: docs

## Brief

Goal: Record what the code cannot tell a reader: why the map is shaped this way, and the render budget.

Change: No Fleet map doc exists -> `docs/fleet-map.md` owns the subject, the docs table links it, and the glossary has the term.

Boundaries:

- Follow the doc rules in [CLAUDE.md](CLAUDE.md): integrate, do not append. Do not restate code. One fact lives in one doc.
- Write the doc voice rules: plain, short, second person, no filler.
- Cover only: the render budget and why, the shared renderer and blit, the lazy import, the deterministic layout and why not physics, the data scope and why one `serverId`, and the gotchas found while building (Metro chunking result from Q-034, React Compiler, Unistyles).
- Link the ADR from TASK-089 instead of copying it.

How:

- Write `docs/fleet-map.md`.
- Add one row to the docs table in `CLAUDE.md`.
- Add the term "Fleet map" to `docs/glossary.md` in the format of its neighbors.
- Run the formatter on the changed files.

Files:

- [docs/fleet-map.md](docs/fleet-map.md) (new)
- [CLAUDE.md](CLAUDE.md) (one docs table row)
- [docs/glossary.md](docs/glossary.md) (one term)

Expected result:

- `docs/fleet-map.md` states the render budget numbers from [watchtower/CONTEXT.md](watchtower/CONTEXT.md) as they shipped.
- The docs table in `CLAUDE.md` links the new doc.
- The glossary entry matches the UI label "Fleet map".

Anti-goal: The doc stays short. Tripwire: `docs/fleet-map.md` has at most 80 lines; read at the end of the TASK with `wc -l`.

## Verify

- `wc -l docs/fleet-map.md` -> 80 lines or fewer.
- `rg -n "fleet-map.md" CLAUDE.md` -> one table row.
- `rg -n "Fleet map" docs/glossary.md` -> one entry.
- `npm run format:files -- docs/fleet-map.md CLAUDE.md docs/glossary.md` -> exit 0, and `git diff --check` -> no output.
- Anti-goal: `wc -l` result is 80 or fewer.
