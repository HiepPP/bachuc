# Plan Context

## Shared Context

- Design: mock "D v2 - Watchtower plan tree" in Claude Design project `9aea602d-2a65-49e5-ac6a-2ef2814fdca8` (https://claude.ai/design/p/9aea602d-2a65-49e5-ac6a-2ef2814fdca8?file=D+v2+-+Watchtower+plan+tree.dc.html). The owner picked it on 2026-10-14.
- The plugin is [hiep-plugins/plugins/watchtower-board](hiep-plugins/plugins/watchtower-board). Read [hiep-plugins/AGENTS.md](hiep-plugins/AGENTS.md) and the plugin [README](hiep-plugins/plugins/watchtower-board/README.md) first.
- The right side of the Watchtower page reads `watchtower.overview.read` ([shared/overview.ts](hiep-plugins/plugins/watchtower-board/shared/overview.ts)). Each task has `status`, `depIds`, `group`, and class. The run log has start and end minutes per TASK.
- [client/timeline.ts](hiep-plugins/plugins/watchtower-board/client/timeline.ts) already computes bars, the average, the finish minute, and pace stats. Keep it and reuse it; only the hour-axis view goes.
- Agents come from `paseo.agents.list`, as [client/agent-target.ts](hiep-plugins/plugins/watchtower-board/client/agent-target.ts) does. A snapshot has `status`, `labels`, `parentAgentId`, `requiresAttention`, and `attentionReason`. It has no current tool or file.
- Builders carry the label `watchtower.task`. The implement run on 2026-10-13 set it to the Group tag (`A`, `C`). Board expects a TASK ID (`TASK-069`). Match both.

## Decisions

- Replace the branch timeline, Running now, and Up next with the tree. Keep one pace line with four numbers. (owner, 2026-10-14)
- Group TASKs by wave from `depIds`, not by Group. (owner, 2026-10-14)
- Show live builder agents in the tree. (owner, 2026-10-14)
- Visual rules from D v2: one accent for running work, amber only for work that needs the owner, a state icon only where it carries real state, monospace for IDs, times, and numbers. Use the plugin theme colors, not fixed hex values.
- The plugin cannot answer a permission, so a waiting builder gets Open thread, not Review.

## Open Decisions

- [Q-036](watchtower/QUESTIONS.md): what the lead row shows when no lead agent is found.

## References

- Commands: `cd hiep-plugins/plugins/watchtower-board && npm run typecheck`, `npm run lint`, `npx tsx --test tests/<file>.test.ts`, `npm run format`.
- Live reload, only when live already runs: `npm run cli -- plugin reload watchtower-board` from the repo root. Never reload a plugin on release.
- ADRs that touch this area: [ADR-0015](watchtower/decisions/ADR-0015-questions-on-the-watchtower-page-over-board.md) (questions stay on the Watchtower page), [ADR-0011](watchtower/decisions/ADR-0011-repo-wide-ids-over-per-plan-ids.md) (IDs).
