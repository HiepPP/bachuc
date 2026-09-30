# prompt-translate design

Date: 2026-09-24. Status: implemented in source; focused live mode checks passed.

## Goal

Help the user learn English prompting from their own Vietnamese prompts.

1. Every new Vietnamese user prompt shows an English translation inside its user bubble,
   directly under the original text.
2. Cmd/Ctrl+Enter in the composer translates and enhances the draft into a standard LLM prompt,
   replaces the composer text, and sends it with the host's default send action.

Both features can be switched off in settings. Latency is the top priority.

## Non-goals

- Browser, iOS, and Android clients. They receive no contribution.
- Translating assistant messages, or prompts that were sent before the plugin was enabled.
- Streaming output into the UI. Plugin RPC is request/response.
- Automatic retries, and changes to Paseo source.

## Decisions

| Topic | Decision | Reason |
| --- | --- | --- |
| Render | Private DOM adapter on desktop | Keeps native bubble images, attachments, rewind, and copy. `addTimelineTransformer` would replace the whole bubble. |
| Shortcut | Cmd/Ctrl+Enter always runs enhance, including while the agent runs | User choice. This removes the host keyboard Queue action (`packages/app/src/composer/input/input.tsx:417`). Queue remains on the button. |
| Enhance output | Replaces composer text, then sends it with a synthetic plain Enter | User changed this on 2026-09-24: send immediately, no review step. |
| Scope | Only new prompts that contain Vietnamese diacritics | Avoids spending quota on history and English prompts. |
| Provider | OpenAI-compatible OpenRouter (default) or Vercel AI Gateway | User choice after the 2026-09-24 benchmark. The Vercel free tier blocked 5 candidates (403) and rate-limited bursts (429). |
| Models | Separate models for translate and enhance | Translate needs the lowest latency. Enhance needs slightly better quality. |

## Host facts (Paseo 0.9.1 source at `~/Projects/paseo`)

- User bubble: `[data-testid="user-message"]`. Its text node is `[data-message-text="true"]`, inside
  the bubble view (`packages/app/src/components/message.tsx:507-546`). The `UserMessage` React props
  include `message`, `timestamp` (ms), `agentId`, and `messageId` (`message.tsx:118-132`). They are
  readable from the DOM node's fiber, the same way `plugins/next-prompt-actions/client/web.ts` `binding()` does.
- Composer field: `[data-composer-input]` or `[data-testid="message-input-root"] textarea`
  (as used by `plugins/next-prompt-actions/client/web.ts:133`).
- No plugin hook runs before a prompt is sent, and there is no keybinding API. Keyboard
  interception must happen in the DOM.
- Settings: `defineSettings` + `server.registerSettings` + `useSettings` / `settingsRpc`
  (see `plugins/next-prompt-actions/shared/settings.ts`).
- The AI SDK fails inside the bundled plugin subprocess (`plugins/jev-permission-gate/server/jev-worker.ts:1`).
  Use plain `fetch` against `/chat/completions`.

## Architecture

```text
plugins/prompt-translate/
  paseo-plugin.json          # id prompt-translate, requirements.paseo "^0.8.0 || >=0.9.0-beta.2"
  index.server.ts            # registerSettings, RPC handlers
  index.client.tsx           # web + desktop only: install adapters, settings screen
  shared/contracts.ts        # Zod RPC contracts
  shared/settings.ts         # defineSettings
  shared/vietnamese.ts       # hasVietnamese(text)
  server/llm.ts              # chat completion via fetch, timeout, no retry
  server/prompts.ts          # system prompts for translate and enhance
  server/credentials.ts      # read provider key from daemon config / env
  server/store.ts            # cache + in-flight dedupe + enhanced->original pairs
  client/bubble.ts           # user bubble adapter
  client/composer.ts         # Cmd/Ctrl+Enter enhance adapter
  client/settings.tsx        # settings screen
  benchmark/run.ts           # manual latency benchmark
  benchmark/samples.json     # synthetic Vietnamese prompts, no real user prompts
  tests/*.test.ts
```

### RPC contracts

- `translate({ text, cacheOnly }) -> { translation: string | null }`. The server rejects text without
  Vietnamese. With `cacheOnly`, it returns a cached value or `null` and never calls the LLM.
- `enhance({ text }) -> { prompt }`. The server stores the pair `sha256(prompt) -> text`.
- `original({ text }) -> { original: string | null }`. Returns the Vietnamese source when `text`
  is exactly an enhance output.

Inputs are capped at 20,000 characters, and longer inputs are rejected.

### Settings (`scope: "host"`, version 1)

```ts
{
  translate: boolean;          // default true
  enhanceShortcut: boolean;    // default true
  matchReplyLanguage: boolean; // default true; conditional preference for Vietnamese drafts
  cavemanMode: "follow-agent" | "lite" | "full" | "ultra" | "wenyan-lite" | "wenyan-full" | "wenyan-ultra"; // default "follow-agent"
  chineseScript: "skill-default" | "simplified"; // default "skill-default"; selected Wenyan modes only
  provider: "vercel" | "openrouter"; // default "openrouter"
  translateModel: string;      // default chosen from benchmark
  enhanceModel: string;        // default chosen from benchmark
}
```

The client caches settings at startup and keeps them current from the settings screen, following the
`backToBoard` pattern in `plugins/next-prompt-actions/index.client.tsx`.

### Credentials

- `vercel`: `agents.providers["vercel-gateway"].env.OPENAI_API_KEY` and `OPENAI_BASE_URL` from
  `$PASEO_HOME/config.json`. The base URL falls back to `https://ai-gateway.vercel.sh/v1`.
- `openrouter`: `OPENROUTER_API_KEY` from the daemon process environment, else the macOS Keychain
  generic password for the current user with service `OPENROUTER_API_KEY`, then `TEXT_MODEL_API_KEY`
  (the key the user's shell profile already reads). Base URL `https://openrouter.ai/api/v1`.

The config is read on each call, so a changed key applies without a reload. Keys never reach the
client, logs, or errors. A missing key raises `"<provider> credential unavailable"`.

### Store

- Cache key: `sha256(mode + provider + model + text)`. Kept in memory with an LRU of 500 entries,
  and persisted to `$PASEO_HOME/plugin-data/prompt-translate/cache.json` with a write debounce.
- Concurrent identical requests share one in-flight promise.
- Enhance pairs: `sha256(prompt) -> original`, at most 200 entries, stored in the same file.
- A corrupt cache file is ignored and replaced. Cache failures never fail a translation.

### LLM calls

- `temperature: 0`, `stream: false`.
- Translate `max_tokens` scales with input length. Timeouts: translate 15 s, enhance 30 s.
- RPC handlers receive no abort signal (`PluginHandlerContext` has only `paseo`), so only the
  timeout aborts a call.
- The translate prompt asks for faithful, natural English and outputs only the translation. It
  keeps code, paths, identifiers, and fenced blocks unchanged.
- The enhance prompt rewrites into clear English with goal, context, constraints, and acceptance
  criteria when present. It must not invent requirements, and outputs only the prompt text.

## Client behavior

### Bubble adapter

- A MutationObserver on `document.body` finds new `[data-testid="user-message"]` elements. It also
  reprocesses when a bubble's text node changes.
- For each text node, the adapter uses the node's text and reads `timestamp` from the fiber props.
  If `timestamp` is missing, the bubble counts as old, so it is only looked up in the cache.
  1. If translation is off, or the node has already been processed with the same text, skip it.
  2. If the text is exactly an enhance output (`original` RPC hit), show the original with label
     `VI gốc`.
  3. Else, if the text contains Vietnamese, call `translate` and show the result with label `EN`.
     Set `cacheOnly` when `timestamp` is older than `activeSince`, the client time when translation
     became active (adapter install, or the setting being switched on).
  4. Else do nothing.
- Old prompts are never sent to the LLM. Bubbles that virtualized scrolling remounts still show
  their cached translation.
- The annotation is appended as a sibling after the text node, inside the bubble. It has a
  hairline divider, a small label, and selectable muted text. Colors come from `currentColor` and
  `opacity`, so they follow the bubble's own text color. It has no hardcoded theme colors.
- Loading shows a one-line pulse. An error shows `Không dịch được · Thử lại`, and a click retries
  once per click.
- Unknown DOM shapes fail closed with no annotation. Removing the adapter removes all annotations.

### Composer adapter

- A capture-phase `keydown` on `document` targets the composer field. On Enter with
  `metaKey || ctrlKey`, no Shift, not IME composing, shortcut on, and non-empty text, it calls
  `preventDefault` and `stopImmediatePropagation`.
- The draft is sent to `enhance`. Meanwhile the field shows a small `Enhancing… (Esc để hủy)`
  badge anchored to the composer root.
- On success, the result is applied only if the field text still equals the submitted draft. The
  field value is replaced through the native value setter plus a bubbling `input` event, so React
  state updates. The caret moves to the end.
- After 50 ms (so React adopts the text), if the field still holds the prompt, a synthetic plain
  Enter `keydown` runs the host's default send action. That action honors the user's send-or-queue
  preference while the agent runs. If the field still holds the prompt 1.5 s later (for example, a
  compact window does not submit on Enter), the badge shows `Chưa gửi được, nhấn Enter để gửi`.
- Esc cancels: the client ignores the result, and the server may still finish and cache it.
- The badge is a fixed-position toast at the bottom right, so it never shifts the host layout.
- On error, the text is kept and the badge shows the error for 4 s.
- A second Cmd+Enter while a request is pending is ignored.

## Risks

- The DOM adapter depends on private host markup, and may break on Paseo updates. It fails closed.
- Prompts are sent to a third-party gateway. The README must state this.
- The keyboard Queue shortcut is lost while the plugin is enabled with the shortcut on.
- An exact-match mapping means an enhanced prompt edited before sending shows no `VI gốc`. This is
  accepted.

## Benchmark

`npm run benchmark` runs `tsx benchmark/run.ts`. It is manual and consumes quota.

- Lists the provider's `/models` first and skips unavailable candidates.
- Candidates: `google/gemini-3.5-flash-lite`, `google/gemini-3.1-flash-lite`,
  `google/gemini-2.5-flash-lite`, `openai/gpt-4.1-nano-fast`, `openai/gpt-4.1-mini-fast`,
  `openai/gpt-oss-120b`, `meta/llama-3.3-70b`, `anthropic/claude-haiku-4.5`,
  `moonshotai/kimi-k3-fast`, `deepseek/deepseek-v4.1-flash`, and `mistral/ministral-8b`. Each was
  listed by the Vercel Gateway on 2026-09-24.
- Uses the production system prompts from `server/prompts.ts`, and 3 synthetic Vietnamese samples
  (short, medium, long).
- Each call is streamed. It measures TTFT (first non-empty content delta) and total time (stream
  end, a close proxy for the production non-streamed call). Each model × mode × sample runs 5
  times, and the report gives p50/p90.
- Raw outputs go to ignored `artifacts/` for manual quality review.
- Failures are recorded, never rerun to improve scores.
- Provisional defaults until then: translate `google/gemini-2.5-flash-lite`, enhance
  `openai/gpt-4.1-mini`. The chosen defaults, with measured numbers and date, go into
  `shared/settings.ts` and the README.

## Testing

- Unit (`tsx --test`): `hasVietnamese`, prompt building, credential resolution (config missing,
  key missing), store (LRU, dedupe, corrupt file, pair lookup), settings defaults, bubble adapter
  and composer adapter on `linkedom` fixtures (processed/skip/label/retry/IME/stale result/Esc).
- Gates: `npm run format && npm run typecheck && npm run lint && npm test`.
- Live: `paseo plugin install`, `paseo plugin ls prompt-translate --json` shows `running`. Then a
  Vietnamese prompt shows `EN`, an English prompt shows nothing, Cmd+Enter replaces the draft,
  and the sent enhanced prompt shows `VI gốc`.

## Follow-up: preserve response language and mode

Status: implemented in source. Enhancing a Vietnamese draft into an English prompt can make
the agent's final response English even when the user expects Vietnamese. The plugin should carry
the user's response-language intent through enhancement, without changing the agent's own mode.

- Keep explicit language, script, format, and style requests in the draft. Preserve skill commands
  and mode names verbatim, including `caveman` modes such as `ultra` and `wenyan-ultra`.
- For a Vietnamese draft, append a conditional preference to the enhanced prompt: explicit
  language and script requests take priority over active mode defaults; keep mode brevity where
  compatible. If neither sets a language, use Vietnamese. Let the agent resolve higher-priority
  instructions.
- Add a default-on `Match original language` switch under Settings → Plugins → Prompt translate.
  With it off, enhancement sends the rewritten prompt without the conditional preference.
- Do not infer an active mode from the draft's language. The plugin cannot inspect persistent
  agent skills or settings, so it must not claim that Vietnamese output is guaranteed.
- Keep a user-selected Chinese response mode, including an explicit Simplified Chinese request,
  even when the draft is Vietnamese. Do not switch languages to save tokens on the user's behalf.
  The current local `caveman` skill lists `wenyan-*` modes but does not define a Simplified Chinese
  mode; its examples use traditional characters. Preserve the requested script instead of inferring
  it from a mode name.
- This phase changes only the prompt sent to the agent and adds a settings control. It does not
  translate or replace the agent's response in the UI, add another model call, or change the
  default bubble translation.

Check with a plain Vietnamese draft, an explicit English-response request, a draft invoking
`caveman ultra` (still terse Vietnamese), a `wenyan-ultra` request (still Chinese), and an explicit
Simplified Chinese request (still simplified). In every case, preserve code, paths, commands, and
fenced blocks exactly. Also check a mode set in an earlier agent turn.

Verified on 2026-09-24: offline checks passed; live enhancement preserved `/caveman ultra`,
`/caveman wenyan-ultra`, and an explicit Simplified Chinese request. After plugin reload, its RPC
returned the conditional preference and `original` lookup matched the draft. Paseo settings showed
the new switch and accepted off/on changes. A real agent gave a terse Vietnamese reply under
`caveman ultra`. Its first `wenyan-ultra` reply used traditional characters; inspection showed
the cached enhancement had dropped the explicit script request. Strengthening the enhance prompt
and changing its cache key made the same synthetic test preserve every instruction; a fresh agent
replied `检查完毕。` in Simplified Chinese. Another agent kept `wenyan-ultra` across turns and replied
`核查已毕。` to a later Vietnamese draft without repeating the mode. These observations do not
guarantee every agent or prompt.

## Follow-up: choose Caveman mode in settings

The Reply language section also offers Caveman mode: Follow agent (default), Lite, Full, Ultra,
Wenyan Lite, Wenyan Full, and Wenyan Ultra. Each selected mode adds its documented `/caveman`
command to enhanced prompts. An explicit `/caveman` command in the draft wins. For a selected
Wenyan mode, Chinese script offers Skill default or Simplified Chinese; an explicit script request
in the draft wins. These are prompt instructions and require the skill in the agent environment.
The plugin cannot read or guarantee the agent's persistent mode. Mode and script settings apply
to Cmd/Ctrl+Enter enhancement and desktop composer sends, not displayed translations.

Verified on 2026-09-24: the installed plugin showed all mode options, exposed Chinese script for
Wenyan Ultra, and returned a prompt with `/caveman wenyan-ultra` plus the Simplified Chinese
instruction. A fresh Codex agent replied `一加一等于二。`; the test agent was archived and settings
were restored to Follow agent / Skill default. The agent result is one focused smoke test, not a
guarantee for every provider or prompt.

## Follow-up: Caveman menu in the composer toolbar

The composer toolbar uses Paseo's internal `AgentControlTrigger` and `Combobox`. The plugin SDK
only exposes `addComposerPill`, which renders in the separate track above the composer. The plugin
cannot import app-internal UI modules. A native DOM select was tried and rejected for its visual
style, then a host-rendered pill was tried and rejected for its placement. Within plugin scope,
the desktop DOM adapter adds a custom dropdown to the toolbar, styled to match the adjacent
controls. Its seven choices write the existing host-scoped `cavemanMode` setting. The control and
observer are removed when the plugin unloads.

## Follow-up: apply Caveman mode on each composer send

Paseo has no plugin hook before prompt submission; `agent.turn_started` runs after the turn begins.
The desktop adapter adds the currently selected `/caveman` command immediately before Enter or
the primary send/queue button submits the draft. The selected mode is applied in client state as
soon as the dropdown choice is made, so Follow agent → Lite works even before settings persistence
finishes. Explicit `/caveman` text in the draft wins. Follow agent adds no command. Sends outside
the desktop composer, such as API calls and voice auto-send, do not use this adapter.

The Claude Caveman `UserPromptSubmit` hook reads a `/caveman` command at the start of each prompt
and injects mode context for that turn. Codex uses the Caveman skill instead of that Claude hook;
the outgoing prompt carries the same command, but the plugin cannot confirm skill activation or
the response style from source alone.
