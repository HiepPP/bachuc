# TASK-125 Docs

Group: E
Class: docs

## Brief

Goal: Make the docs say how timelines are cached, read, and prefetched now.

Change: the docs say timeline rows are memory-only and reads resume -> they describe the SQLite cache, reads without resume, prefetch, the latest-prompt open, and the stale-cache risk.

Boundaries:

- Follow "Writing docs" and "Doc voice" in [CLAUDE.md](CLAUDE.md). Integrate, do not append. One fact, one doc.
- Facts follow the shipped code, not the design.

Files:

- [docs/timeline-sync.md](docs/timeline-sync.md)
- [docs/architecture.md](docs/architecture.md)
- [docs/data-model.md](docs/data-model.md)
- [docs/agent-lifecycle.md](docs/agent-lifecycle.md)
- [docs/plugins.md](docs/plugins.md)
- [packages/server/CLAUDE.md](packages/server/CLAUDE.md)
- [hiep-plugins/plugins/board/README.md](hiep-plugins/plugins/board/README.md)

Expected result:

- A reader learns that `claude --resume` outside Paseo shows the old copy until Reload agent.

## Verify

- `npm run format:files -- <files>` -> exit 0.
- `rg -n "db:query|agent_timeline_rows" packages/server/CLAUDE.md` -> no match.
