# TASK-005 Outcome

## Outcome

Status: DONE

Changed:
- Added [plugins/skill-pins/shared/settings.ts](plugins/skill-pins/shared/settings.ts) with host settings `defaults` and the `SKILL_PINS_INITIAL` env name. The spec listed `shared/contracts.ts`; a separate file follows the `prompt-translate` settings pattern.
- Added the settings screen [plugins/skill-pins/client/settings.tsx](plugins/skill-pins/client/settings.tsx), registered in [plugins/skill-pins/index.client.tsx](plugins/skill-pins/index.client.tsx). It appears under Settings > Plugins > skill-pins actions > Skill pins.
- [plugins/skill-pins/index.server.ts](plugins/skill-pins/index.server.ts): `before("agent.create")` adds `SKILL_PINS_INITIAL` for Claude and Codex. `before("agent.session_open")` with reason `create` seeds `skills.json`. The spec named `agent.created`, but that event has no env; `session_open` carries `agentId` and `env` and runs before the first turn.
- [plugins/skill-pins/server/state.ts](plugins/skill-pins/server/state.ts): `seed` writes only when `skills.json` is missing.
- [plugins/skill-pins/server/skill-pins-hook.cjs](plugins/skill-pins/server/skill-pins-hook.cjs): env fallback with `source: "initial"` while `skills.json` is missing. It creates the agent folder before writing `last-hook.json`.
- README: added the Default pins section and removed Q3.

Contract:
- An existing `skills.json` always wins over the env value, even with `[]`.
- Existing agents are never backfilled.

Verified:
- Spike 2026-09-29: this Paseo Claude agent's process env holds `PASEO_ORCH_TOKEN`, `PASEO_BOARD_TOKEN`, and `PASEO_JEV_NATIVE_POLICY`, all added by other plugins through `before("agent.create")` `request.env`. Native hooks are child processes of the provider, so they inherit it. The live check below confirms the path end to end.
- `npm run format`, `npm run typecheck`, `S=lint; npm run $S` (0 warnings, 0 errors), `npm test` -> 45 pass, 0 fail. New tests: env fallback and `[]` override in `tests/hook.test.ts`, seed-only-when-missing in `tests/state.test.ts`.
- `paseo plugin reload skill-pins` -> `running`, logs show `Plugin ready`.
- Settings screen rendered with three switches. Watchtower was switched on through the desktop UI.
- Live agent `32c4f8bf-1277-4803-9ba9-02af97a8e6bd` (Sonnet 5.5, low): `skills.json` was seeded as `["watchtower"]`. The third turn "What is 2+2?" showed `[Skill] watchtower`. The first two prompts said "Reply with one word", and the model skipped the skill under the priority rule. The hook still reported `contextBytes` 268 on turn 2.
- Live probe `898e323b-0054-4b5d-88dc-5e0ab9da91af`: the first turn "What is 3+3?" showed `[Skill] watchtower` before the answer.
- Both test agents were archived.

Anti-goal:
- Before: 2 existing `skills.json` files hashed into `/tmp/skill-pins-before.txt`; 2026-09-29T08:14:11Z.
- Final: same 2 hashes, `diff` empty; 2026-09-29T08:16:44Z.
- Result: PASS against the limit that existing selections do not change.
