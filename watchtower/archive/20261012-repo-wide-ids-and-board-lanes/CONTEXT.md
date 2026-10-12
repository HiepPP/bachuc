# Plan Context

## Shared Context

- This plan has two parts. Part one changes the Watchtower skill, which lives in the `~/.claude` git repo, not in this repo. Part two changes the Board plugin and a thin host API in this repo.
- The skill files are under `/Users/hiep/.claude/skills/watchtower/`. Commit skill changes in the `~/.claude` repo, and commit plugin changes in this repo.
- Today TASK-IDs and Q-IDs restart at 001 in every plan. ADR-IDs are already repo-wide. Source: [task-template.md](/Users/hiep/.claude/skills/watchtower/references/task-template.md).
- This repo's archive holds 5 plans with 56 TASKs and 21 Q-IDs. Sorted by slug (creation date, then name), they become TASK-001 to TASK-056 and Q-001 to Q-021. This plan starts at TASK-059 and Q-022, above them, so TASK-061 keeps it. TASK-057 and TASK-058 stay unused, because the first count wrongly included two `.patch` files.
- `hiep-plugins/watchtower/` holds 10 older plugin plans. The skill resolves the repo root with git, so it never reads that folder. See Q-022.
- Board already reads agent labels in [hiep-plugins/plugins/board/server/snapshot.ts](hiep-plugins/plugins/board/server/snapshot.ts). Autorun gives each subagent it creates the label `watchtower.task=TASK-NNN`. A schedule iteration agent carries only `paseo.schedule-id` and `paseo.schedule-run`, set in [packages/server/src/server/schedule/service.ts](packages/server/src/server/schedule/service.ts).
- A plugin's `callRpc` reaches only its own plugin, so Board cannot call `watchtower.overview.read`. Board's server reads the plan files itself.
- `openSurface` takes `pluginId`, `serverId`, and `presentation`, but no workspace. Source: [packages/plugin/src/client/contracts.ts](packages/plugin/src/client/contracts.ts).
- Plugins never import each other's code. Copy a small parser when Board needs one.

## Decisions

- IDs: TASK and Q-IDs increase across the repo and never restart. A repo migrates its archive the first time a new plan is created there. See [ADR-0011](watchtower/decisions/ADR-0011-repo-wide-ids-over-per-plan-ids.md).
- Migration order: archive date prefix, then slug, then the old ID.
- The migration rewrites ID references in ADR bodies and `watchtower/MEMORY.md`. This is the only exception to R3. No ID map file is kept, so 42 old commit messages keep IDs that no longer match.
- Board stays on Running and Just finished; a card shows its TASK chip, which opens Watchtower. See [ADR-0012](watchtower/decisions/ADR-0012-status-columns-with-task-chips-over-task-lanes.md). The plan line and Board Needs you were dropped.
- Questions and checks live on the Watchtower page, which shows full text and links the answer target (TASK-063). See [ADR-0013](watchtower/decisions/ADR-0013-thread-input-only-in-board-needs-you.md).
- Answers stay in the thread or on the Watchtower page. Board shows no answer form.

## Open Decisions

- None. Q-022 and Q-023 are answered, and Q-024 is defaulted, in [watchtower/QUESTIONS.md](watchtower/QUESTIONS.md).

## References

- Design: Claude Design project "Watchtower unified UI options", file `Board Watchtower Options.dc.html`, option 2a.
- [hiep-plugins/plugins/board/README.md](hiep-plugins/plugins/board/README.md)
- [hiep-plugins/plugins/watchtower-board/README.md](hiep-plugins/plugins/watchtower-board/README.md)
- [mode-flows.md](/Users/hiep/.claude/skills/watchtower/references/mode-flows.md), [plan-format.md](/Users/hiep/.claude/skills/watchtower/references/plan-format.md), [decision-template.md](/Users/hiep/.claude/skills/watchtower/references/decision-template.md)
