# TASK-294 selection-translate plugin server

Group: F (writes hiep-plugins/plugins/selection-translate and the hiep-plugins README catalog)
Class: code

## Brief

Goal: Add the `selection-translate` plugin server with one RPC that translates English text into Vietnamese through the same OpenAI-style chat call that prompt-translate uses.

Change: no plugin -> `hiep-plugins/plugins/selection-translate` with the `selection.translate` RPC, its own settings, and its own cache.

Design: [hiep-plugins/plugins/prompt-translate/server/llm.ts](hiep-plugins/plugins/prompt-translate/server/llm.ts), [hiep-plugins/plugins/prompt-translate/server/credentials.ts](hiep-plugins/plugins/prompt-translate/server/credentials.ts), [hiep-plugins/plugins/prompt-translate/server/store.ts](hiep-plugins/plugins/prompt-translate/server/store.ts)

Boundaries:

- Copy `llm.ts` and drop its `mode` parameter: the new `prompts.ts` has one prompt, one `maxTokens(text)`, and one `TIMEOUT_MS`. Copy `store.ts` without the `pairs` map, `original`, and `pair`, and change its log prefix to `selection-translate:`.
- The server trims the text first; the cache key and the model both get the trimmed text.
- Follows [ADR-0062](watchtower/decisions/ADR-0062-selection-translate-plugin-copies-llm-logic-over-sharing-prompt-translate.md): copy `llm.ts`, `credentials.ts`, and `store.ts` into the new plugin. Do not import from the prompt-translate folder and do not add a shared package. Note at the top of each copied file which prompt-translate file it came from.
- Keep the copied request shape: `POST <baseUrl>/chat/completions`, `temperature: 0`, `stream: false`, one retry on a pre-response "fetch failed", the timeout error text, and the `<message>` unwrap.
- The system prompt translates English to natural Vietnamese. It keeps code, file paths, identifiers, commands, URLs, and fenced blocks as written, and never answers the text. Text that is already Vietnamese comes back unchanged.
- Settings id `translate`, scope `host`, version 1: `provider` (`vercel` or `openrouter`, default `openrouter`) and `model` (default `google/gemini-2.5-flash-lite`, the prompt-translate `DEFAULT_TRANSLATE_MODEL`).
- Timeout 15 s. `max_tokens` is `min(8192, ceil(length * 2) + 256)`, because Vietnamese is longer than English. Planner call.
- The RPC input `text` is 1 to 4000 characters after trim (the Cite `MAX_QUOTE_LENGTH`); a longer text fails with `Selection is longer than 4000 characters`.
- Cache by hash of provider, model, and text in `$PASEO_HOME/plugin-data/selection-translate/cache.json`.
- `requirements.paseo` is `>=0.10.2-beta.900`, as the other plugins. The plugin has no `index.client.tsx`; the React client is frozen.
- Never commit an API key or a real cache file.

How:

- Write `paseo-plugin.json`, `package.json` (scripts like prompt-translate: typecheck, test, lint, format), `tsconfig.json`, `index.server.ts`, `server/llm.ts`, `server/credentials.ts`, `server/store.ts`, `server/prompts.ts`, `server/service.ts` (`createService({ store, complete, settings })` with `translate({ text })`, shaped like the prompt-translate service), `shared/settings.ts`, `shared/contracts.ts`, `tests/*.test.ts`, and `README.md`. Trim the `lint` and `format` script paths to the folders that exist (no `index.client.tsx`, `client`, or `benchmark`).
- Add the plugin to the catalog in [hiep-plugins/README.md](hiep-plugins/README.md), as [hiep-plugins/AGENTS.md](hiep-plugins/AGENTS.md) asks.
- `index.server.ts` registers the settings and handles `selection.translate` with the cache and the completer.

Files:

- [hiep-plugins/plugins/selection-translate](hiep-plugins/plugins/selection-translate) (new plugin folder, all files above)
- [hiep-plugins/README.md](hiep-plugins/README.md) (one catalog row)

Interfaces:

- Produces: RPC `selection.translate` with input `{ "text": string }` and answer `{ "translation": string }` on plugin `selection-translate`, for TASK-295
- Produces: settings id `translate` with fields `provider: "vercel" | "openrouter"` and `model: string`, read and written through `settings.translate.read`, `settings.translate.write`, and `settings.translate.reset`, for TASK-295

Expected result:

- WHEN the client calls `selection.translate` with English text THE server SHALL return its Vietnamese translation.
- WHEN the same text is asked again with the same provider and model THE server SHALL answer from the cache without a model request.
- IF the text is empty after trim THEN THE server SHALL fail with `Selection is empty`; IF it is over 4000 characters THEN with `Selection is longer than 4000 characters`.
- IF the model request fails or times out THEN THE server SHALL fail with the copied error text (status and detail, or the timeout).
- IF the model returns empty text THEN THE server SHALL fail with `Model returned no text`.
- IF no credential is found THEN THE server SHALL fail with `<provider> credential unavailable`.

Red tests:

File: [hiep-plugins/plugins/selection-translate/tests/service.test.ts](hiep-plugins/plugins/selection-translate/tests/service.test.ts)

```ts
import assert from "node:assert/strict";
import { test } from "node:test";
import { mkdtemp } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { createService } from "../server/service";
import { Store } from "../server/store";

async function service(answers: string[]) {
  const dir = await mkdtemp(path.join(tmpdir(), "selection-translate-"));
  const calls: string[] = [];
  const svc = createService({
    store: new Store(path.join(dir, "cache.json")),
    complete: async ({ text }) => {
      calls.push(text);
      return answers.shift() ?? "";
    },
    settings: async () => ({ provider: "openrouter", model: "m" }),
  });
  return { svc, calls };
}

test("translates and caches the same text", async () => {
  const { svc, calls } = await service(["Xin chào"]);
  assert.deepEqual(await svc.translate({ text: "Hello" }), { translation: "Xin chào" });
  assert.deepEqual(await svc.translate({ text: "Hello" }), { translation: "Xin chào" });
  assert.equal(calls.length, 1);
});

test("trims before the cache and the model", async () => {
  const { svc, calls } = await service(["Xin chào"]);
  await svc.translate({ text: "  Hello \n" });
  assert.deepEqual(await svc.translate({ text: "Hello" }), { translation: "Xin chào" });
  assert.deepEqual(calls, ["Hello"]);
});

test("rejects empty and over-long text", async () => {
  const { svc } = await service([]);
  await assert.rejects(svc.translate({ text: "   " }), /empty/i);
  await assert.rejects(svc.translate({ text: "a".repeat(4001) }), /4000 characters/);
});
```

Anti-goal: the prompt-translate plugin folder has 0 changed files; tripwire; read from `git status --short -- hiep-plugins/plugins/prompt-translate` before the change and at completion.

## Verify

- `cd hiep-plugins/plugins/selection-translate && npm install && npm run typecheck && npm test` -> no type errors; all tests pass.
- `cd hiep-plugins/plugins/selection-translate && npm run lint` -> no errors.
- `npm run cli -- plugin install "$PWD/hiep-plugins/plugins/selection-translate" --id selection-translate` from the repo root -> the live host lists the plugin as running in `npm run cli -- plugin ls`.
- Live RPC check with a real key: call `selection.translate` with `{"text":"Open the settings file."}` on live -> the answer has Vietnamese text. If the CLI has no RPC call command, record this as covered by the TASK-295 screen check.
- `git status --short -- hiep-plugins/plugins/selection-translate` -> no `cache.json` and no key file staged.
- Anti-goal: `git status --short -- hiep-plugins/plugins/prompt-translate` -> empty.
