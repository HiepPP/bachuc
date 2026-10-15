# NEXT

## Current Active Plan

- Title: Instant thread open from the board
- Slug: 20261015-instant-thread-open
- Status: ARCHIVED
- Updated: 2026-10-15

## Tracker

One row per TASK. Group ties together items that ship as one transaction. This plan was written after the work shipped, from the design and the agent reports of 2026-10-15.

| Order | TASK                                           | Group | Status | Spec                                                                                                                       | Deps                         | Context                                        | Notes                                                |
| ----- | ---------------------------------------------- | ----- | ------ | -------------------------------------------------------------------------------------------------------------------------- | ---------------------------- | ---------------------------------------------- | ---------------------------------------------------- |
| 1     | TASK-108 Protocol, client, and plugin contract | A     | DONE   | [watchtower/tasks/TASK-108-protocol-client-contract.md](watchtower/tasks/TASK-108-protocol-client-contract.md)             | -                            | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Not committed.                                       |
| 2     | TASK-109 SQLite timeline store                 | B     | DONE   | [watchtower/tasks/TASK-109-sqlite-timeline-store.md](watchtower/tasks/TASK-109-sqlite-timeline-store.md)                   | -                            | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Not committed. ADR-0031.                             |
| 3     | TASK-110 AgentManager durable timeline         | C     | DONE   | [watchtower/tasks/TASK-110-agent-manager-durable-timeline.md](watchtower/tasks/TASK-110-agent-manager-durable-timeline.md) | TASK-109                     | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Not committed. Codex backfill not run live.          |
| 4     | TASK-111 Daemon read path and bootstrap        | C     | DONE   | [watchtower/tasks/TASK-111-daemon-read-path.md](watchtower/tasks/TASK-111-daemon-read-path.md)                             | TASK-108, TASK-110           | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Not committed. Live daemon check passed.             |
| 5     | TASK-112 Client sync concurrency and flag      | D     | DONE   | [watchtower/tasks/TASK-112-client-sync-concurrency.md](watchtower/tasks/TASK-112-client-sync-concurrency.md)               | TASK-108                     | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Not committed.                                       |
| 6     | TASK-113 No-jump latest-prompt anchor          | D     | DONE   | [watchtower/tasks/TASK-113-no-jump-anchor.md](watchtower/tasks/TASK-113-no-jump-anchor.md)                                 | -                            | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Not committed. Tested in jsdom only.                 |
| 7     | TASK-114 Prefetch host API                     | D     | DONE   | [watchtower/tasks/TASK-114-prefetch-host-api.md](watchtower/tasks/TASK-114-prefetch-host-api.md)                           | TASK-108                     | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Not committed.                                       |
| 8     | TASK-115 Board prefetch and follow-ups         | D     | DONE   | [watchtower/tasks/TASK-115-board-prefetch.md](watchtower/tasks/TASK-115-board-prefetch.md)                                 | TASK-112, TASK-113, TASK-114 | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Not committed. Plugin reloaded on live, not clicked. |
| 9     | TASK-116 Docs                                  | E     | DONE   | [watchtower/tasks/TASK-116-docs.md](watchtower/tasks/TASK-116-docs.md)                                                     | TASK-111, TASK-115           | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Not committed.                                       |

TASK Status labels: TODO, IN PROGRESS, BLOCKED, DONE.
Plan-level Status header: ACTIVE while any row is open, DONE when all rows DONE, ARCHIVED after archive.

## Plan Verify

- `npm run typecheck` -> exit 0. Passed 2026-10-15.
- `npm run lint` -> exit 0. Passed 2026-10-15.
- `npm run format:check` -> exit 0. Passed 2026-10-15.
- On live, close an agent, restart the live daemon, fetch its timeline -> `.dev/paseo-home/daemon.log` has no "Agent resumed from persistence" line for that id, and `.dev/paseo-home/timeline-cache.sqlite` exists. Passed 2026-10-15 for agent `847221a2`.
- UNVERIFIED: in the live app, click a board card of a closed thread -> the thread shows at its latest prompt with no spinner and no jump. Needs a human or a `cua_repl` check.
- UNVERIFIED: backfill of a real Codex agent -> no writer-lock error and the timeline shows. TASK-110 decided from code only.
- UNVERIFIED: a release build -> the same behavior on `~/.bachuc`. Only the owner installs a release.

## Handoff

- All changes are in the working tree on `main`, not committed.
- The full design is summarized in [watchtower/CONTEXT.md](watchtower/CONTEXT.md). The scratch design file `/tmp/bachuc-instant-open-design.md` is not kept.
- The first open of each old thread after rollout costs one backfill.

## Archive

- Archived: 2026-10-15 -> watchtower/archive/20261015-instant-thread-open/
