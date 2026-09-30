# Prompt translate

Learn English prompting from your own Vietnamese prompts, on Paseo desktop.

- Each new Vietnamese prompt shows its English translation inside the user bubble, labeled `EN`.
- Cmd/Ctrl+Enter in the composer rewrites the draft as a clear English prompt and sends it as if
  you pressed Enter, so it follows your send-or-queue preference while the agent runs. Esc during
  the rewrite cancels it. The sent bubble shows your Vietnamese original, labeled `VI gốc`.
  If the host does not send it (for example in a compact window), the prompt stays in the
  composer with a hint to press Enter.
- For Vietnamese drafts, Match original language asks the agent to answer in Vietnamese unless
  you requested another language or an active response mode specifies one. Your explicit language
  or script choice takes priority over mode defaults; compatible `caveman` brevity stays. This is
  a prompt preference, not a translation of the reply.
- Caveman mode is read immediately before each desktop composer send (Enter or the primary
  send/queue button). Cmd/Ctrl+Enter enhances the text first, then applies the latest selection,
  including changes made while enhancement is running.
- With multiple hosts connected, only the composer's host handles its Enter/Send actions and
  Caveman menu. Unidentified hosts leave normal submission untouched.
- Default resets earlier Caveman modes and restores the agent's normal response style and the
  current request's language. The stored `follow-agent` value is retained for compatibility.
  Explicit `/caveman` or `$caveman` commands in the draft take priority over the dropdown.
- Mode instructions use native `UserPromptSubmit` **hidden context**. The plugin never inserts
  mode commands or a response-mode block into existing agents' prompts.
  New-thread composers keep their selection locally: the first send prepends `/caveman <mode>`
  for a non-Default selection unless the draft already contains an explicit Caveman command.
  The native hook initializes that agent's mode from the command; subsequent sends use hidden context. Enhancement changes only the
  prompt content; reply-language preferences also travel through the hook.
- The hook calls the installed Caveman parser and mode tracker, including its mode-specific
  ruleset and reminders. State is isolated per Paseo agent and native session. Explicit user
  commands override the dropdown for that turn. Model compliance is still probabilistic.
  Caveman shortens wording while preserving required response formats, endings, and next-step prompts.

Choose Caveman mode in each conversation's composer. New-thread drafts start at the host's
**New-thread Caveman mode** setting (`cavemanMode`, default Default) until you pick another mode.
A new agent inherits its first-turn command and keeps its own selection. Settings also hold the
shared reply-language and Chinese-script preferences.

## Install native hooks

Requires an inspected Caveman installation with `src/hooks/caveman-mode-tracker.js`,
`caveman-config.js`, and `caveman-parse.js` (verified with Caveman 2.7.0).
With the user's authorization, run:

```sh
node scripts/install-hooks.mjs /absolute/path/to/installed/caveman
```

This appends only this plugin's hook to `~/.claude/settings.json` and `~/.codex/hooks.json`,
preserves other settings, and makes a private `.prompt-translate-backup` beside each config.
It records the installed Caveman path under the Paseo home, never in Git. Re-run after moving
Node, this plugin, or Caveman. Do not restore the full backup over later unrelated config edits.

For Claude, the hook is registered for `UserPromptSubmit` and `SessionStart` with `--claude`.
Disable the native Claude plugin (`"caveman@caveman": false` in `enabledPlugins`): its hooks
write the shared `~/.claude/.caveman-active`, so one agent's `/caveman` command leaks into every
Claude session. The bridge keeps Paseo agents isolated and runs the installed Caveman
`caveman-activate.js` and `caveman-mode-tracker.js` for Claude sessions outside Paseo.

Codex requires reviewing/trusting the exact new hook through `/hooks`. Trust only the
`prompt-translate/server/caveman-hook.cjs` entry. Existing Paseo Codex agents need
`paseo agent reload <id>` after trust/config changes; reloading the plugin alone cannot reload
provider hook configuration. Claude sessions may also need an agent reload after registration.
No daemon restart or Paseo rebuild is needed.

Remove this registration with `node scripts/install-hooks.mjs --remove` before removing or
moving this plugin. Disabling a Paseo plugin does not unregister native provider hooks.

## Compatibility and limits

Paseo 0.8.x and 0.9.x desktop, through a private DOM/React adapter. Verified natively with Codex
0.156.0 and Claude via Paseo. Browser/mobile clients have no composer contribution. New-agent
composers without an agent ID send normally; mode selection becomes available once the agent
exists. Unsupported providers cannot execute these Claude/Codex hooks.

Each composer send stores its mode and a hash of the prompt, never the raw text, in
`plugin-data/prompt-translate/agents/<id>/pending`. The hook consumes matching snapshots in
order; snapshots expire after 24 hours. Outside-composer sends use the agent's saved mode.
New queue entries bind their snapshot token to the host queue item ID. Editing an entry
waits for its snapshot to be cancelled before restoring the draft; sending again captures
the newly selected mode. Bindings survive plugin reload. Entries created before this
queue-binding update have no binding and can still retain old snapshots until expiry. `last-hook.json` contains only
mode/session/hash/timestamp metadata for diagnosis, not prompt text or model output.

While enhancement is on, Cmd/Ctrl+Enter enhances instead of the host keyboard Queue shortcut;
use the Queue button. Prompts sent before translation was enabled are cache-only.

## Privacy and credentials

Prompt text is sent to the selected third-party provider. Keys stay on the daemon and are read on
every call:

- OpenRouter (default): `OPENROUTER_API_KEY` in the daemon environment, else the macOS Keychain
  generic password for the current user with service `OPENROUTER_API_KEY`, then
  `TEXT_MODEL_API_KEY`.
- Vercel AI Gateway: `agents.providers.vercel-gateway.env.OPENAI_API_KEY` (and optional
  `OPENAI_BASE_URL`) in the daemon's `config.json`. The free tier blocks some models and
  rate-limits bursts.

The cache lives at `$PASEO_HOME/plugin-data/prompt-translate/cache.json`. Never add it to Git.

## Models

Default for both translate and enhance: `google/gemini-2.5-flash-lite`, chosen from a benchmark
on OpenRouter on 2026-09-24 (15 calls per cell, 3 synthetic samples, total time p50/p90):

| Model                          | Translate       | Enhance         |
| ------------------------------ | --------------- | --------------- |
| `google/gemini-2.5-flash-lite` | 935 / 1244 ms   | 858 / 1223 ms   |
| `google/gemini-3.5-flash-lite` | 1057 / 1268 ms  | 1187 / 1458 ms  |
| `google/gemini-3.1-flash-lite` | 1142 / 1553 ms  | 1174 / 1547 ms  |
| `openai/gpt-4.1-nano`          | 1329 / 3175 ms  | 1133 / 2450 ms  |
| `anthropic/claude-haiku-4.5`   | 1618 / 2285 ms  | 1770 / 2440 ms  |
| `openai/gpt-oss-120b`          | 9572 / 19859 ms | 3668 / 15608 ms |

Output quality was reviewed only for the default model. Change models in settings.
`npm run benchmark -- --provider=openrouter --runs=1 --models=<id>` re-measures; every call
consumes quota, and raw outputs are written only when the run finishes.

## Install and verify

```sh
npm install
npm run format && npm run typecheck && npm run lint && npm test
paseo plugin install "$PWD" --id prompt-translate
paseo plugin ls prompt-translate --json
```
