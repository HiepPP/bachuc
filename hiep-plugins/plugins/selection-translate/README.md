# Selection translate

Server for translating selected English text into Vietnamese. It has one RPC, `selection.translate`
(`{ text }` -> `{ translation }`), and no React client. The GPUI client calls it.
Requires the Paseo fork `>=0.10.2-beta.900`.

- The server trims the text first. The text must be 1 to 4000 characters after trim.
- Text that is already Vietnamese comes back unchanged. Code, paths, identifiers, commands, URLs,
  and fenced blocks stay as written.
- Settings (`translate`, host scope): `provider` (`openrouter` or `vercel`, default `openrouter`)
  and `model` (default `google/gemini-2.5-flash-lite`).
- The LLM call, credential lookup, and cache are copies of the prompt-translate server files
  (ADR-0062). A fix to one must be made in both plugins.

## Privacy and credentials

Selected text is sent to the selected third-party provider. Keys stay on the daemon and are read on
every call:

- OpenRouter (default): `OPENROUTER_API_KEY` in the daemon environment, else the macOS Keychain
  generic password for the current user with service `OPENROUTER_API_KEY`, then
  `TEXT_MODEL_API_KEY`.
- Vercel AI Gateway: `agents.providers.vercel-gateway.env.OPENAI_API_KEY` (and optional
  `OPENAI_BASE_URL`) in the daemon's `config.json`.

The cache lives at `$PASEO_HOME/plugin-data/selection-translate/cache.json`. Never add it to Git.

## Install and verify

```sh
npm install
npm run format && npm run typecheck && npm run lint && npm test
paseo plugin install "$PWD" --id selection-translate
paseo plugin ls selection-translate --json
```
