# TASK-008 Docs for the Bachuc names

Group: G (`CLAUDE.md`, `docs/development.md`, `watchtower/MEMORY.md`)
Class: docs

## Brief

Goal: The repo docs name the release app `Bachuc.app`, its build output, its log folder, and the internal names that stay `paseo`.

Change: docs say `Paseo Fork.app` and `Paseo Dev` -> docs say `Bachuc.app` and `Bachuc Dev`, and link [ADR-0004](watchtower/decisions/ADR-0004-display-brand-over-full-rename.md) for the kept names.

Boundaries:

- Change only facts that TASK-003 and TASK-006 changed. Do not restate the ADR in the docs; link it.
- Read the real paths from code before you write them: the Electron log folder after the rename, and the userData folder of a build opened in place.
- Keep the instance names release and live. They do not change.
- `AGENTS.md` links to `CLAUDE.md`. Edit `CLAUDE.md` only.

How:

- Replace `Paseo Fork.app` with `Bachuc.app` in [CLAUDE.md](CLAUDE.md) and [docs/development.md](docs/development.md).
- Update the build output name, the userData folder of an opened build, and the log folder.
- Add one line to [CLAUDE.md](CLAUDE.md): the product brand is Bachuc, and internal names stay `paseo` per ADR-0004.
- Update `## Core Intent` in [watchtower/MEMORY.md](watchtower/MEMORY.md) with the brand and the new install path.

Files:

- [CLAUDE.md](CLAUDE.md) (release table, install path, brand line)
- [docs/development.md](docs/development.md) (installing a release)
- [watchtower/MEMORY.md](watchtower/MEMORY.md) (Core Intent)

Expected result:

- No doc names `Paseo Fork.app` as the current release app.
- The release table shows the right log folder.

Anti-goal: changed doc lines stay at or below 30; drift gauge; read with `git diff --numstat main -- CLAUDE.md docs/development.md` at completion.

## Verify

- `rg -n "Paseo Fork" CLAUDE.md docs/development.md` -> no match.
- `npm run format:check:files -- CLAUDE.md docs/development.md watchtower/MEMORY.md` -> exit 0.
- Anti-goal: `git diff --numstat main -- CLAUDE.md docs/development.md` -> the sum of added and removed lines is 30 or less.
