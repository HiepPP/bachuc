# TASK-011 Outcome

## Outcome

Status: DONE

Changed:

- Removed the DOM menu, queue snapshots, draft state, the native `UserPromptSubmit` hook, and its installer: `client/dom.ts`, `client/skills-menu.ts`, `client/queue.ts`, `client/state.ts`, `server/skill-pins-hook.cjs`, `scripts/install-hooks.mjs`, and their tests.
- Added [hiep-plugins/plugins/skill-pins/client/popover.tsx](hiep-plugins/plugins/skill-pins/client/popover.tsx) and a shared `Skills` pill in [hiep-plugins/plugins/skill-pins/index.client.tsx](hiep-plugins/plugins/skill-pins/index.client.tsx).
- Added [hiep-plugins/plugins/skill-pins/server/context.ts](hiep-plugins/plugins/skill-pins/server/context.ts). [hiep-plugins/plugins/skill-pins/index.server.ts](hiep-plugins/plugins/skill-pins/index.server.ts) appends it in `before("agent.prompt")` for Claude and Codex turns and seeds defaults on create.
- [hiep-plugins/plugins/skill-pins/server/state.ts](hiep-plugins/plugins/skill-pins/server/state.ts) keeps get, set, seed, and remove, under `plugin-data/skill-pins/v2`.
- Requirements `>=0.10.2-beta.900`; `file:` links to [packages](packages); `tsconfig.json` maps `zod` and `react` types to the repo root so the linked SDK and the plugin share one copy; lint uses `--disable-nested-config`.

Contract:

- Deviation from the spec: the native hook and installer were deleted, not kept. The hook returned nothing without `PASEO_AGENT_ID`, so it never served sessions outside Paseo.
- Codex paths come from `~/.claude/skills` or `SKILL_PINS_SKILLS_ROOT`, not from `hook-runtime.json`.

Verified:

- `npx tsc --noEmit` -> exit 0. `npm run lint` -> 0 errors. `npm test` -> 14 passed.
- DOM check `rg -n "querySelector|MutationObserver|document\." client index.client.tsx` -> no match.
- Real host check, Paseo Dev 2026-10-01T00:38: the `Skills` pill popover saved `sequential-thinking` to `v2/agents/<id>/skills.json`; the next turn invoked that skill and the user bubble held only the typed text.

Anti-goal:

- Before changes: native hook output for 4 fixed cases captured to the test fixture; 2026-10-01T00:12.
- Final: `tests/context.test.ts` asserts equal text for all 4 cases and passes; 2026-10-01T00:16.
- Result: PASS.
