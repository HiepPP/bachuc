# TASK-116 Outcome

## Status

DONE. Not committed.

## Changed

- `docs/timeline-sync.md`: sections on the cache and reads, prefetch, and the latest-prompt open; the stale-cache risk; the hidden catch-up cap of 2.
- `docs/architecture.md`: rows live in memory and mirror into a rebuildable SQLite cache.
- `docs/data-model.md`: `timeline-cache.sqlite` in the home layout, a cache with no migrations.
- `docs/agent-lifecycle.md`: only prompting resumes a closed agent.
- `docs/plugins.md`: `navigation.prefetchAgent`.
- `packages/server/CLAUDE.md`: removed both `db:query` lines.
- Board `README.md`: one sentence on hover and press-in prefetch.

## Verified

Measured 2026-10-15.

- `npm run format:files` on the 7 files -> pass.
- The lead checked the numbers in the docs against the code: 64 cold timelines, 2 backfills, 2 hidden catch-ups, 30 s TTL, queue 3, cap 200.

## Lessons

- The `db:query` script in the root `package.json` still exists, with no code behind it.
