# TASK-133 Docs for the SQLite cuts

Group: G (writes `docs/timeline-sync.md`, `docs/data-model.md`, and `docs/architecture.md`; runs after all other groups)
Class: docs

## Brief

Goal: The docs describe the shipped behavior of TASK-126 to TASK-132, and drop text they made wrong.

Change: docs say memory is the serving layer, the app budget scans row sizes, and outside edits need Reload agent -> docs match the new code.

Boundaries:

- Follow "Writing docs" and "Doc voice" in the root `CLAUDE.md`: integrate, do not append; one fact, one doc; do not restate code.
- Read each TASK outcome sidecar first. Document only what shipped. Skip a TASK that is not `DONE` and say so in the outcome.

How:

- [docs/timeline-sync.md](docs/timeline-sync.md): rewrite "Timeline cache and reads" for the bounded tail, SQL pages, FTS5 search, and the transcript fingerprint. Remove the "Stale after external edits" item if TASK-130 shipped.
- [docs/data-model.md](docs/data-model.md): update the `timeline-cache.sqlite` line and the "Replica row store" budget text. Add the `agent-requests/` 7-day expiry and the coalesced record writes where those folders are described.
- [docs/architecture.md](docs/architecture.md): update the line on timeline rows in memory (line 388).
- Add the 8-thread stream cap where the app stream model is described, or link it from the doc that owns it.

Files:

- [docs/timeline-sync.md](docs/timeline-sync.md) (cache and reads)
- [docs/data-model.md](docs/data-model.md) (cache files, replica budget, receipts, record writes)
- [docs/architecture.md](docs/architecture.md) (memory line)

Expected result:

- No doc says a live timeline is held fully in memory.
- No doc says the replica budget scans row sizes.
- Each shipped TASK has its behavior in exactly one doc, with links from the others.

Anti-goal: Total line count of the three docs grows by at most 40 lines; drift gauge; read with `wc -l` at completion against the count before the edit.

## Verify

- `rg -n "Memory stays the serving layer|scan opaque row sizes" docs` -> no match.
- `npm run format:check` -> exit 0 for the changed docs.
- Anti-goal: `wc -l docs/timeline-sync.md docs/data-model.md docs/architecture.md` -> total grows by 40 lines or fewer.
