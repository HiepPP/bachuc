# TASK-327 Outcome

## Status

Status: DONE. Release install waits on Hiep (Q-201).

## UI Checks

UI checker run 2026-11-02, check bundle `Bachuc GPUI Check` against live (6768), AX actions only. Limit: macOS Stage Manager held the background window in the side strip, so screenshots are 178x182 thumbnails and the pill label text is not readable. Pill state is proven by the pill menu's `(selected)` row and by daemon files.

- C3 PASS (label text UNVERIFIED by sight):
  1. Open New workspace in project `bachuc` (Isolation: Local). The draft pill menu shows `(selected)` on Full.
  2. Pick Off, reopen the menu: `(selected)` on Off. The message is `Reply with the single word OK.` and the model is `Haiku 5.5` of the Claude provider (the filtered list also holds `Claude Haiku 5.5` entries of another provider; the first try picked one, see below).
  3. Send. New agent `83813d66-4ae5-4005-ad47-46c4ac205d5d` (`claude/claude-haiku-5-5`, status idle). Its tab pill menu shows `(selected)` on Off.
  4. `.dev/paseo-home/plugin-data/ponytail/agents/83813d66-4ae5-4005-ad47-46c4ac205d5d/mode.json` is `{"mode":"off"}`. `last-hook.json` is `{"mode":"off","kind":"off"}` after the first turn.
- First attempt (not the evidence run): the model filter `Haiku 5.5` selected `Claude Haiku 5.5` of provider `opencode/cloudflare-ai-gateway`. The agent `f3930c57-c49a-422b-98d6-ab44252c0412` was created with pill Off and `mode.json` `{"mode":"off"}`, then ended in status `error` and wrote no `last-hook.json` (the hook applies to the Claude provider). It is not archived.
- Not mine: agent directories `5c4e14bc-af83-44ad-8d60-3b241be6e7b6` and `e7ed008b-acf8-4d00-9fda-77d6d156cf69` (both `{"mode":"full"}`) appeared in the same folder; this run did not create them.
- Screenshots (thumbnails): `/tmp/c3-draft-off.png`, `/tmp/c3-thread1-pill-off.png`, `/tmp/c3-draft2-off-haiku.png`, `/tmp/c3-thread2-pill-off.png`.
- Note: an AX click on a model row in the unfiltered model menu did not change the model; typing a filter with `setValue` and then clicking the row did.

## Lead Results

Changed:

- `claude plugin install ponytail@ponytail` (5.3.3) then `claude plugin disable`; `claude plugin list` shows it disabled.
- Copied `ponytail`, `ponytail-review`, `ponytail-audit`, `ponytail-debt`, `ponytail-gain`, and `ponytail-help` to `~/.claude/skills/`; no `band`.
- Ran `scripts/install-hooks.mjs ~/.claude/plugins/cache/ponytail/ponytail/5.3.3` for `~/.bachuc` and `.dev/paseo-home`. `~/.claude/settings.json` holds one bridge entry each for SessionStart, UserPromptSubmit, and SubagentStart. Backup: `/tmp/claude-settings.before-ponytail.json` and `settings.json.ponytail-backup`.
- Installed the plugin on live only (Q-201 default). Release install is left to Hiep.

Verified:

- Bridge SessionStart outside Paseo -> prints `PONYTAIL MODE ACTIVE — level: full` and the ruleset.
- `npm run cli -- plugin ls` -> `ponytail` running, enabled on live.
- Claude agent on live: turn 1 `kind: rules`, turn 2 `kind: reminder` (mode `full`); `/ponytail off` -> `mode.json` `off`, `kind: off`.
- Codex agent on live: turn 1 `{"mode":"full","kind":"rules"}`.
- Missing `hook-runtime.json` -> turn sent; `[ponytail] no hook-runtime.json` warning in `daemon.log` after fix `0630c21e6`.
- New workspace draft with Off -> new thread `mode.json` `off`, `last-hook.json` `{"mode":"off","kind":"off"}` (UI checker C3).
- UNVERIFIED (unattended): release plugin install, pending Q-201.

Anti-goal:

- Before: release daemon PID 2198; `PASEO_HOME=~/.bachuc paseo daemon status`, 2026-11-02 10:28.
- Final: PID 2198; same command, 2026-11-02 11:05.
- Result: PASS, same PID.

Lessons:

- The native `ponytail-activate.js` also writes `~/.claude/.ponytail-active` for plain Claude sessions; Paseo agents skip it through the bridge.
