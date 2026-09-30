# TASK-005 Default skill set for new agents

Group: E (shares [plugins/skill-pins/index.server.ts](plugins/skill-pins/index.server.ts) and [plugins/skill-pins/server/skill-pins-hook.cjs](plugins/skill-pins/server/skill-pins-hook.cjs) with TASK-006)
Class: code

## Brief

Goal: Let the user choose a default skill set in plugin settings. Every new Claude or Codex agent created through Paseo starts with that set pinned.

Change: new agents start with no pins -> new agents start with the default set.

Design: [plugins/skill-pins/README.md](plugins/skill-pins/README.md) (sections State, Hook, Open questions Q3)

Boundaries:

- Store the default set with host-scoped plugin settings (`registerSettings`). Follow the settings pattern in [plugins/prompt-translate/index.server.ts](plugins/prompt-translate/index.server.ts) and [plugins/prompt-translate/index.client.tsx](plugins/prompt-translate/index.client.tsx).
- In `server.before("agent.create")`, add `SKILL_PINS_INITIAL=<comma-separated IDs>` to `request.env` for providers `claude` and `codex` only. Keep every other request field unchanged.
- The hook uses `SKILL_PINS_INITIAL` only when `skills.json` is missing. An existing `skills.json` always wins, even with an empty list.
- On `agent.created`, write `skills.json` from `agent.env` or the same default set, only when the file is missing. The pill then shows the pins.
- Drop unknown IDs from the env value, as reads already do.
- Existing agents keep their current pins. Do not backfill.
- First confirm that `request.env` reaches the native hook process. If it does not, stop and mark the TASK `BLOCKED` with the evidence.

How:

- Spike: create one Claude agent with a test env value. Check that the hook sees it.
- Add a settings definition and a settings screen with one checkbox per catalog skill.
- Add the `before("agent.create")` and `agent.created` handlers.
- Add the env fallback to `readSelection` in the hook.
- Add tests. Update the README.

Files:

- [plugins/skill-pins/index.server.ts](plugins/skill-pins/index.server.ts) (settings, create hooks)
- [plugins/skill-pins/index.client.tsx](plugins/skill-pins/index.client.tsx) (settings screen)
- [plugins/skill-pins/shared/contracts.ts](plugins/skill-pins/shared/contracts.ts) (settings definition)
- [plugins/skill-pins/server/skill-pins-hook.cjs](plugins/skill-pins/server/skill-pins-hook.cjs) (env fallback)
- [plugins/skill-pins/server/state.ts](plugins/skill-pins/server/state.ts) (write only when missing)
- plugins/skill-pins/tests/ (new cases)
- [plugins/skill-pins/README.md](plugins/skill-pins/README.md) (document the default set and close Q3)

Expected result:

- With defaults `watchtower`, a new Claude agent calls the `watchtower` skill on its first turn.
- Its pill shows `Watch` without any click.
- Clearing its pins stops the skill on the next turn, even though the env value remains.

Anti-goal: Agents that already have a `skills.json` keep the same selection; tripwire; read by hashing every `agents/*/skills.json` under `$PASEO_HOME/plugin-data/skill-pins/` before the change and after the live check.

## Verify

- `cd plugins/skill-pins && npm run typecheck && S=lint; npm run $S && npm test` -> exit 0.
- Hook test: no `skills.json` and `SKILL_PINS_INITIAL=watchtower` -> context lists `watchtower`.
- Hook test: `skills.json` with `[]` and `SKILL_PINS_INITIAL=watchtower` -> output `{}`.
- `paseo plugin reload skill-pins && paseo plugin ls skill-pins --json` -> `running`, no load error.
- Live Claude agent created after setting defaults -> the timeline shows `[Skill] watchtower` on the first turn.
- Anti-goal hash comparison -> no existing `skills.json` changed.
