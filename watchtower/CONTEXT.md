# Plan Context

## Shared Context

- The full design lives in [plugins/skill-pins/README.md](plugins/skill-pins/README.md). TASK briefs carry only the parts each TASK needs.
- The user pins skills in the composer. Every turn, a native `UserPromptSubmit` hook tells the agent to invoke the pinned skills (invoke mode). The hook never pastes skill bodies.
- The Paseo SDK has no per-turn prompt hook. `server.before` accepts only `agent.create` and `agent.session_open`.
- The fixed catalog is `watchtower`, `chase-goal-claude`, and `sequential-thinking`. It replaced the first catalog of three superpowers skills on 2026-09-29 at the user's request.
- All three live in `~/.claude/skills`. The Codex context points to absolute `SKILL.md` paths that the installer resolves.
- The reference implementation is [plugins/prompt-translate/](plugins/prompt-translate/): [server/modes.ts](plugins/prompt-translate/server/modes.ts), [server/caveman-hook.cjs](plugins/prompt-translate/server/caveman-hook.cjs), [client/mode-menu.ts](plugins/prompt-translate/client/mode-menu.ts), [client/composer.ts](plugins/prompt-translate/client/composer.ts), and [scripts/install-hooks.mjs](plugins/prompt-translate/scripts/install-hooks.mjs). Copy patterns; do not import across plugins.
- RPC wire names must be lowercase with dots and hyphens, for example `skill-pins.bind-queue`. The daemon rejects camelCase names.
- Run lint as `S=lint; npm run $S`. A local hook rewrites a literal `npm run lint`.
- `server.before("agent.create")` may return `request.env`. [plugins/board/index.server.ts](plugins/board/index.server.ts) reads `request.env` there. Whether that env reaches the native hook process is not verified yet.
- Keep `plugin-data` bounded: pending snapshots expire after 24 hours, and no prompt text is stored.

## Decisions

- Invoke mode only (C1). Inline skill bodies are out of scope.
- The hook exits 1 on error, never 2, so a failure never blocks the user's prompt.
- v1 disables the pill in new-thread composers that have no agent ID.
- Q0 resolved on 2026-09-29: keep `skill-pins` as a separate plugin. The user chose this. Skill pinning is a different feature from translation and Caveman style, so each plugin can be installed, disabled, and hook-registered on its own. The MEMORY rule to extend an owning plugin does not apply, because `skill-pins` owns new data (pinned skills), not data that `prompt-translate` already holds.

- Q1 and Q2 resolved on 2026-09-29: the user wants both. Support pins in new-thread composers, and add a default skill set for new agents.
- Q1 does not prepend text to the first prompt. The plugin keeps its rule that the bubble stays unchanged. Instead, `server.before("agent.create")` puts the initial skill list into the agent env as `SKILL_PINS_INITIAL`. The hook reads it when `skills.json` does not exist yet.
- `skills.json` always wins once it exists, even when it holds an empty list. The env value only covers the first turns before the file is written.

## Open Decisions

- None.

## References

- [plugins/skill-pins/README.md](plugins/skill-pins/README.md)
- [plugins/prompt-translate/README.md](plugins/prompt-translate/README.md)
