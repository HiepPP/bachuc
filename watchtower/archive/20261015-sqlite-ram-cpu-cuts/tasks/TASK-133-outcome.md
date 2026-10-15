# TASK-133 Outcome

## Status

DONE. TASK-126 to TASK-132 were all DONE, so all are documented.

## Changed

- `docs/timeline-sync.md`: bounded memory tail, cache windows, FTS5 search (Q-047, Q-048), transcript fingerprint replaces "Stale after external edits", 8-thread stream cap (Q-044).
- `docs/data-model.md`: SQLite cache line, coalesced record writes (Q-043), `agent-requests/` 7-day expiry, replica budget reads per-host totals.
- `docs/architecture.md`: line 388 says memory keeps a bounded tail.

## Contract

Each TASK has its behavior in one doc, `timeline-sync.md` for TASK-127 to 130, `data-model.md` for 126, 131, 132. Others link to it.

## Verified

- `rg -n "Memory stays the serving layer|scan opaque row sizes" docs` -> no match.
- `npm run format:files` and `npm run format:check:files` on the 3 docs -> "Expected at least one target file": oxfmt ignores `docs/`, so nothing to format.
- `npm run format:check` -> flags only watchtower files outside this TASK's scope (e.g. `TASK-129-outcome.md`).

## Anti-goal

Before: 319 + 649 + 483 = 1451 lines. After: 333 + 651 + 483 = 1467 lines. Growth +16, within the limit of 40.

## Lessons

`docs/` is excluded by the formatter config, so the spec's format check is vacuous for these files.
