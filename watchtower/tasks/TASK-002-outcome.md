# TASK-002 Outcome

## Outcome

Status: DONE

Changed:
- Added [plugins/skill-pins/server/skill-pins-hook.cjs](plugins/skill-pins/server/skill-pins-hook.cjs). It exports `run(data, env, provider)`, `digest`, and `skillIds`, and has a stdin/stdout main.
- Added [plugins/skill-pins/scripts/install-hooks.mjs](plugins/skill-pins/scripts/install-hooks.mjs). It takes the absolute superpowers `skills` folder, checks each `SKILL.md`, and writes `hook-runtime.json` as `{ skillPaths }`.
- Added [plugins/skill-pins/tests/hook.test.ts](plugins/skill-pins/tests/hook.test.ts) and [plugins/skill-pins/tests/install-hooks.test.ts](plugins/skill-pins/tests/install-hooks.test.ts).
- Added `scripts` to the `lint` and `format` scripts in [plugins/skill-pins/package.json](plugins/skill-pins/package.json).
- Updated the Install section in [plugins/skill-pins/README.md](plugins/skill-pins/README.md) for the new installer argument.

Contract:
- The hook returns `{}` without a valid `PASEO_AGENT_ID`, without an agent folder, or with no skills left after dedupe.
- A matching snapshot wins over `skills.json` and is deleted with its `.queue` file. Snapshots older than 24 hours are deleted.
- The hook exits 1 on error and never exits 2.
- `last-hook.json` holds metadata only: `agentId`, `sessionId`, `skills`, `source`, `hash`, `contextBytes`, `at`.
- The hook keeps its own copy of the catalog IDs. A test checks it equals [plugins/skill-pins/shared/catalog.ts](plugins/skill-pins/shared/catalog.ts).

Verified:
- `npx tsc --noEmit` -> exit 0.
- `S=lint; npm run $S` -> 0 warnings, 0 errors on 10 files.
- `npm test` -> 25 pass, 0 fail. The tests cover every case in Expected result.
- `echo '{"prompt":"hi"}' | env -u PASEO_AGENT_ID node server/skill-pins-hook.cjs --claude` -> printed `{}`, exit 0.
- `ls ~/.claude/settings.json.skill-pins-backup ~/.codex/hooks.json.skill-pins-backup` -> no such files. `~/.paseo/plugin-data/skill-pins` also does not exist.

Anti-goal:
- After the hook first worked: p90 28.4 ms over 20 CLI runs; `tests/hook.test.ts`, 2026-09-29 11:51.
- Final: p90 28.4 ms over 20 CLI runs; same test run, 2026-09-29 11:51.
- Result: PASS against the 200 ms limit.
