# TASK-327 Global install and live check

Group: C (writes `~/.claude/**`, `~/.bachuc/**`, and `.dev/paseo-home/**`; no repo source)
Class: mechanical

## Brief

Goal: Ponytail is installed for global Claude as [ADR-0080](watchtower/decisions/ADR-0080-disabled-native-ponytail-with-bridge-hook.md) says, and the `ponytail` plugin runs on release and live.

Change: no Ponytail on the machine -> disabled native plugin, global skills, bridge hook, and the plugin installed on both hosts.

Boundaries:

- Follows [ADR-0080](watchtower/decisions/ADR-0080-disabled-native-ponytail-with-bridge-hook.md): keep `ponytail@ponytail` disabled in `enabledPlugins`.
- Install only from the official source `DietrichGebert/ponytail`, as its README warns.
- Never restart or stop the release daemon on port 6767. Q-201 default: the run installs the plugin on live only; Hiep installs it on release. The installer still writes `~/.bachuc/plugin-data/ponytail/hook-runtime.json`, which is only a file write.
- Back up `~/.claude/settings.json` before the first change. The installer also keeps `settings.json.ponytail-backup`.
- Copy the skills `ponytail`, `ponytail-review`, `ponytail-audit`, `ponytail-debt`, `ponytail-gain`, and `ponytail-help`. Skip `band`; it needs the enabled native plugin.
- Planner call: the Ponytail root is the folder that `claude plugin install` writes, expected under `~/.claude/plugins/cache/ponytail/ponytail/<version>`. Confirm the real path; do not assume it.
- The GPUI pill check lives in TASK-326. This TASK checks the daemon side and plain Claude.

How:

- Run `claude plugin marketplace add DietrichGebert/ponytail`, `claude plugin install ponytail@ponytail`, then `claude plugin disable ponytail@ponytail`.
- Copy the six skill folders from the installed root to `~/.claude/skills/`.
- Run `node scripts/install-hooks.mjs <ponytailRoot>` once with `PASEO_HOME=~/.bachuc` and once with `PASEO_HOME=$PWD/.dev/paseo-home`.
- Install the plugin on release (`PASEO_HOME=~/.bachuc paseo plugin install ... --id ponytail`) and live (`npm run cli -- plugin install ... --id ponytail`).

Files:

- `~/.claude/settings.json` (marketplace entry, `ponytail@ponytail: false`, three bridge hook entries)
- `~/.claude/skills/ponytail*/` (six copied skill folders)
- `~/.bachuc/plugin-data/ponytail/hook-runtime.json` and `.dev/paseo-home/plugin-data/ponytail/hook-runtime.json` (written by the installer)
- `~/.bachuc/config.json` and `.dev/paseo-home/config.json` (plugin source entries, written by `plugin install`)

Interfaces:

- Consumes: file `hiep-plugins/plugins/ponytail/scripts/install-hooks.mjs`, run as `node scripts/install-hooks.mjs <ponytailRoot>` or `node scripts/install-hooks.mjs --remove`, from TASK-325
- Consumes: file `$PASEO_HOME/plugin-data/ponytail/agents/<agentId>/last-hook.json` with fields `mode` and `kind` (`rules`, `reminder`, or `off`) from TASK-325

Expected result:

- WHEN the install finishes THE `claude plugin list` output SHALL show `ponytail@ponytail` installed and disabled.
- WHEN a plain Claude session outside Paseo starts THE bridge SHALL add the native Ponytail SessionStart context.
- WHEN a new Claude agent on live sends its first turn THE daemon SHALL add the full ruleset, and `last-hook.json` SHALL show `mode: full` and `kind: rules`.
- WHEN the same agent sends a second turn THE `last-hook.json` SHALL show `kind: reminder`.
- WHEN a new Codex agent on live sends its first turn THE `last-hook.json` SHALL show `kind: rules`.
- WHEN the live `hook-runtime.json` is missing THE turn SHALL still be sent, with one `[ponytail]` warning in the daemon log.

Anti-goal: the release daemon keeps running with the same PID through the whole TASK; tripwire; read from `PASEO_HOME=~/.bachuc paseo daemon status` before the first step and at completion.

## Verify

- `claude plugin list` -> `ponytail@ponytail` listed and disabled.
- `ls ~/.claude/skills | rg '^ponytail'` -> the six skill folders; no `band` folder.
- `jq '.hooks | to_entries[] | select(.value[].hooks[].command | test("ponytail-hook.cjs")) | .key' ~/.claude/settings.json` -> `SessionStart`, `UserPromptSubmit`, and `SubagentStart`, once each.
- `printf '{"hook_event_name":"SessionStart","source":"startup"}' | env -u PASEO_AGENT_ID node hiep-plugins/plugins/ponytail/server/ponytail-hook.cjs --claude` -> output holds the Ponytail ruleset.
- `PASEO_HOME=~/.bachuc paseo plugin ls ponytail --json` and `npm run cli -- plugin ls ponytail --json` -> enabled and running on both hosts.
- Live agent check: create a Claude agent on live, send two turns -> `last-hook.json` shows `kind: rules`, then `kind: reminder`, both with `mode: full`.
- Live agent check: create a Codex agent on live, send one turn -> `last-hook.json` shows `kind: rules`.
- Live agent check: create a thread from a New workspace draft whose Ponytail pill is Off, send one turn -> `last-hook.json` shows `mode: off`.
- Live log check: move `.dev/paseo-home/plugin-data/ponytail/hook-runtime.json` aside, send one turn, then move it back -> the turn is sent, and `.dev/paseo-home/daemon.log` shows one `[ponytail]` warning.
- Live agent check: send `/ponytail off` to the Claude agent -> `mode.json` shows `off`, and the next turn's `last-hook.json` shows `kind: off`.
- Anti-goal: `PASEO_HOME=~/.bachuc paseo daemon status` -> the same PID before and after.
