# TASK-325 Outcome

## Status

DONE. Live-daemon and installed-plugin checks are DEFERRED (lead, TASK-327).

## Changed

New folder `hiep-plugins/plugins/ponytail/`: `paseo-plugin.json`, `package.json`, `package-lock.json`, `tsconfig.json`, `index.server.ts`, `shared/{modes,contracts}.ts`, `server/{modes,turn,draft}.ts`, `server/ponytail-hook.cjs`, `scripts/install-hooks.mjs`, `tests/{turn,modes,draft,bridge}.test.ts`, `README.md`.

## Contract

- RPC `ponytail.mode.read` and `ponytail.mode.write`, input and output as the spec says. No saved mode reads `full`.
- Draft env key `PASEO_PLUGIN_COMPOSER_ponytail` seeds `{mode}` on a `create` and `interactive` session open with no saved mode. The key is always removed.
- `last-hook.json` at `plugin-data/ponytail/agents/<agentId>/` holds `{mode, kind}` with `kind` of `rules`, `reminder`, or `off`. It is written when a turn gets context.
- `node scripts/install-hooks.mjs <ponytailRoot>` and `--remove`. Registers `SessionStart`, `UserPromptSubmit`, `SubagentStart` in `$CLAUDE_CONFIG_DIR/settings.json` and writes `hook-runtime.json` (`{ponytailRoot}`). Backup: `settings.json.ponytail-backup`.
- `turnContext({ponytailRoot, mode, lastMode, turnsSinceRules}): string | null`.

## Verified

- Red run (before `server/turn.ts` existed): `npm test` failed on module load. Green after: 4 red tests pass.
- `npm run typecheck`, `npm run lint`, `npm run format` in the plugin: exit 0, no diff left. `npm test`: 10 of 10 pass (4 turn, 3 draft, 1 modes, 2 bridge).
- Bridge with a UUID `PASEO_AGENT_ID`: empty stdout, exit 0.
- Bridge `UserPromptSubmit` `/ponytail lite` with temp `PASEO_HOME` and `CLAUDE_CONFIG_DIR`, root `/tmp/ponytail-src/repo`: stdout `PONYTAIL MODE CHANGED — level: lite`, exit 0.
- Bridge with no `hook-runtime.json` and no agent ID: one stderr line, exit 1.
- Installer with temp dirs: two runs gave one bridge entry per event; `--remove` restored `settings.json` byte-equal in content to the original (other hooks kept).
- Root `npm run lint` ignores `hiep-plugins` ("No files found"), so the plugin lint script is the check. Root `npm run typecheck` not run: no host file changed.
- DEFERRED (lead, TASK-327): plugin install and reload on live, a real turn through the daemon, the Claude `settings.json` write, daemon-restart behavior.

## Anti-goal

Reminder size at the start, after the `turnContext` change, and at completion: 70 bytes (lite, full), 71 bytes (ultra); limit 300. The test asserts `<= 300`. Full ruleset is about 2.9 KB.

## Lessons

- Root `npm run lint -- <path>` skips `hiep-plugins`; use the plugin's own `npm run lint`.
- zsh uses `pipestatus`, not `PIPESTATUS`.
- Paseo child agents (with `parentAgentId`) go through `agent.prompt` like any agent and get their own mode (default `full`). Provider-internal subagents get nothing (the bridge is silent in Paseo and the daemon hook does not see them).

## Lead Re-check

- 2026-11-02 lead: `npm run typecheck`, `npm test` (10 pass), `npm run lint` -> exit 0. Bridge with a UUID `PASEO_AGENT_ID` -> empty output, exit 0.
- Live daemon checks (run in TASK-327): Claude agent turn 1 `kind: rules`, turn 2 `kind: reminder`, `/ponytail off` saved `off` with `kind: off`; Codex turn 1 `kind: rules`.
- Found: a turn with no `hook-runtime.json` went out unchanged but logged no warning, against the spec. Lead fix in commit `0630c21e6` warns once per process. Re-check on live: the turn went out, and `daemon.log` shows `[ponytail] no hook-runtime.json; run install-hooks.mjs` on stderr.
- Builder note, kept as is: Paseo child agents with their own agent ID go through `agent.prompt`, so they get their own mode (default `full`), like every other agent.
