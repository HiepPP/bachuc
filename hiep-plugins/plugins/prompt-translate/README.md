# Prompt translate

Learn English prompting from your own Vietnamese prompts, on every Paseo client.
Requires the Paseo fork `>=0.10.2-beta.900`; the stable 0.10.1 host refuses the plugin.

- Each Vietnamese prompt shows its English translation under the user bubble, labeled `EN`.
  Prompts sent before translation was turned on use cached translations only.
- The composer's **Prompt** pill has two per-conversation choices: **Rewrite Vietnamese to
  English** and **Caveman mode**. With rewrite on, a Vietnamese draft is rewritten as a clear
  English prompt before it is sent or queued. The bubble then shows your original, labeled
  `VI gốc`. A failed rewrite keeps the draft and shows the error.
- **Match original language** asks the agent to answer in Vietnamese for Vietnamese drafts,
  including rewritten ones, unless you asked for another language or a mode sets one.
- Caveman mode and the reply-language line are added by the daemon `agent.prompt` hook to each
  turn of Claude and Codex agents. The timeline keeps the text you sent. Explicit `/caveman` or
  `$caveman` commands in the draft take priority for that turn.
- New agents start with the host's **New-thread Caveman mode**. Composers without an agent use
  the host's **Rewrite Vietnamese drafts to English** default.
- Per-agent choices live in `$PASEO_HOME/plugin-data/prompt-translate/v2/agents/<id>/mode.json`.
  The `v2` folder keeps an old native hook from another checkout silent for this host's agents.

## Caveman runtime

The daemon hook runs the installed Caveman parser and mode tracker in-process, including its
mode-specific ruleset and reminders. It needs the Caveman path recorded in
`$PASEO_HOME/plugin-data/prompt-translate/hook-runtime.json`. Without it, no Caveman or
reply-language context is added. Record it with the installer (verified with Caveman 2.7.0):

```sh
node scripts/install-hooks.mjs /absolute/path/to/installed/caveman
```

The installer also registers `server/caveman-hook.cjs` in `~/.claude/settings.json` and
`~/.codex/hooks.json` for Claude sessions outside Paseo, which get the native Caveman hooks
through this bridge. Paseo agents on this host no longer need that registration. Remove it with
`node scripts/install-hooks.mjs --remove`. Model compliance is still probabilistic.

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
