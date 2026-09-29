# TASK-002 Native hook and installer

Group: B (writes server/skill-pins-hook.cjs and scripts/install-hooks.mjs; ordered after TASK-001 by dependency only)
Class: risky

## Brief

Goal: Add the `UserPromptSubmit` hook that tells the agent to invoke pinned skills, and an installer that registers it for Claude and Codex.

Change: selection stored on disk -> hidden per-turn context in Claude and Codex agents.

Design: [plugins/skill-pins/README.md](plugins/skill-pins/README.md) (sections Hook, Install)

Boundaries:

- Follow [plugins/prompt-translate/server/caveman-hook.cjs](plugins/prompt-translate/server/caveman-hook.cjs) for snapshot consumption and 24-hour expiry.
- Output `{}` when `PASEO_AGENT_ID` is missing or invalid, or when no skill is pinned.
- Drop a skill from the context when the prompt already requests it with `/<id>`, `/superpowers:<id>`, or `$<id>`.
- Errors write one stderr line and exit 1. Never exit 2.
- The installer edits `~/.claude/settings.json` and `~/.codex/hooks.json`. This TASK writes and tests the installer only. It does not run it against the real home.
- Follow [plugins/prompt-translate/scripts/install-hooks.mjs](plugins/prompt-translate/scripts/install-hooks.mjs): append only this entry, keep other settings, write a `.skill-pins-backup`, and support `--remove`.

How:

- Write `server/skill-pins-hook.cjs` with an exported `run(data, env, provider)` and a stdin/stdout main.
- Build the Claude and Codex context text exactly as in the README.
- Write `last-hook.json` with metadata only.
- Write `scripts/install-hooks.mjs`. It resolves each Codex `SKILL.md` path and writes `hook-runtime.json`. A missing path fails the install.
- Add `tests/hook.test.ts` and `tests/install-hooks.test.ts`. Use a temp `HOME` and temp `PASEO_HOME`.

Files:

- [plugins/skill-pins/server/skill-pins-hook.cjs](plugins/skill-pins/server/skill-pins-hook.cjs) (new)
- [plugins/skill-pins/scripts/install-hooks.mjs](plugins/skill-pins/scripts/install-hooks.mjs) (new)
- [plugins/skill-pins/tests/hook.test.ts](plugins/skill-pins/tests/hook.test.ts) (new)
- [plugins/skill-pins/tests/install-hooks.test.ts](plugins/skill-pins/tests/install-hooks.test.ts) (new)

Expected result:

- No agent ID -> `{}`.
- Agent with no pinned skills -> `{}`.
- Agent with two pinned skills, Claude -> context lists both `superpowers:` names.
- Same agent, Codex -> context lists both absolute `SKILL.md` paths.
- Prompt that starts with `/superpowers:brainstorming` -> brainstorming is dropped for this turn only.
- Matching snapshot -> snapshot skills win over `skills.json`, and the snapshot is deleted.
- Expired snapshot -> deleted and ignored.
- Installer run twice -> one entry per provider. `--remove` -> other hooks stay unchanged.

Anti-goal: Hook wall time stays under 200 ms per call; tripwire; read from a timed test over 20 runs after the hook first works and at completion.

## Verify

- `cd plugins/skill-pins && npm run typecheck && S=lint; npm run $S` -> exit 0.
- `cd plugins/skill-pins && npm test` -> hook and installer tests pass, covering every case in Expected result.
- `echo '{"prompt":"hi"}' | env -u PASEO_AGENT_ID node plugins/skill-pins/server/skill-pins-hook.cjs --claude` -> prints `{}` and exits 0.
- `ls ~/.claude/settings.json.skill-pins-backup ~/.codex/hooks.json.skill-pins-backup` -> no such files, which proves tests did not touch the real home.
- Timed test in `tests/hook.test.ts` -> p90 under 200 ms over 20 runs.
