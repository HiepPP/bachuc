# prompt-translate Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the desktop Paseo plugin `prompt-translate`. It shows the English translation of new
Vietnamese prompts inside their user bubbles, and makes Cmd/Ctrl+Enter enhance the composer draft
into an English LLM prompt.

**Architecture:** The daemon side owns credentials, the LLM calls (plain `fetch` to an
OpenAI-compatible `/chat/completions`), a persisted cache, and host settings. It exposes three
RPCs. The desktop client installs two private DOM adapters, one for bubbles and one for the
composer, plus a settings screen.

**Tech Stack:** TypeScript, `@getpaseo/plugin` 0.9.0-beta.2, Zod 4, React Native (web) for
settings, `node:test` via `tsx`, `linkedom` for DOM tests, `oxlint`, and `oxfmt`.

**Spec:** `docs/superpowers/specs/2026-09-24-prompt-translate-design.md`

## Global Constraints

- Plugin ID and directory: `prompt-translate` at `plugins/prompt-translate/`.
- `paseo-plugin.json` `requirements.paseo`: `"^0.8.0 || >=0.9.0-beta.2"`.
- Runtime layout: entries at the root; modules only in `server/`, `client/`, `shared/`. `shared/`
  has no runtime-specific imports. The client never imports `server/` or Node modules.
- Desktop only: the client contributes nothing unless `Platform.OS === "web"` and the user agent
  contains `Electron/`.
- Do not use the AI SDK. Use plain `fetch`.
- Keys never reach the client, logs, error messages, or Git.
- Only synthetic prompts go in the repository. Real user prompts are never stored in Git.
- Inputs are capped at 20,000 characters.
- Timeouts: translate 15,000 ms, enhance 30,000 ms. No automatic retries.
- Cache: LRU of 500 translations and 200 enhance pairs at
  `$PASEO_HOME/plugin-data/prompt-translate/cache.json`.
- UI copy, verbatim: `EN`, `VI gốc`, `Không dịch được · `, `Thử lại`, `Enhancing… (Esc để hủy)`.
- Do not commit, push, or reload the plugin unless the user asks. Each task ends at a checkpoint.
- Never use `rm -rf`.

## File map

| File | Responsibility |
| --- | --- |
| `paseo-plugin.json`, `package.json`, `tsconfig.json` | Manifest, scripts, and dev dependencies, the same as `next-prompt-actions` |
| `shared/vietnamese.ts` | `hasVietnamese(text)` |
| `shared/contracts.ts` | Zod RPC contracts |
| `shared/settings.ts` | `translateSettings` definition and defaults |
| `server/credentials.ts` | Provider to `{ baseUrl, apiKey }` |
| `server/prompts.ts` | System prompts, message building, token and timeout limits |
| `server/llm.ts` | `createCompleter`: one chat completion |
| `server/store.ts` | Cache, in-flight dedupe, enhance pairs, and persistence |
| `server/service.ts` | RPC behavior on top of the store, completer, and settings |
| `index.server.ts` | Wiring |
| `client/dom.ts` | Minimal DOM types, fiber props reader, and desktop check |
| `client/bubble.ts` | User bubble adapter |
| `client/composer.ts` | Cmd/Ctrl+Enter adapter |
| `client/state.ts` | Client settings cache and `activeSince` |
| `client/settings.tsx` | Settings screen |
| `index.client.tsx` | Wiring |
| `benchmark/run.ts`, `benchmark/samples.json` | Manual latency benchmark |
| `tests/*.test.ts` | Unit tests |
| `README.md`, root `README.md` | Documentation and catalog row |

---

### Task 1: Scaffold and shared contracts

**Files:**
- Create: `plugins/prompt-translate/paseo-plugin.json`, `package.json`, `tsconfig.json`
- Create: `plugins/prompt-translate/shared/vietnamese.ts`, `shared/contracts.ts`, `shared/settings.ts`
- Test: `plugins/prompt-translate/tests/shared.test.ts`

**Interfaces:**
- Produces: `hasVietnamese(text: string): boolean`; `translateRpc`, `enhanceRpc`, `originalRpc`;
  `translateSettings`, `type TranslateSettings`, `type Provider = "vercel" | "openrouter"`,
  `DEFAULT_TRANSLATE_MODEL`, `DEFAULT_ENHANCE_MODEL`, `MAX_TEXT = 20_000`.

- [ ] **Step 1: Create the manifest and package files**

`paseo-plugin.json`:

```json
{
  "id": "prompt-translate",
  "requirements": {
    "paseo": "^0.8.0 || >=0.9.0-beta.2"
  }
}
```

`package.json`:

```json
{
  "name": "prompt-translate",
  "version": "0.0.1",
  "private": true,
  "type": "module",
  "scripts": {
    "typecheck": "tsc --noEmit",
    "test": "tsx --test tests/*.test.ts",
    "lint": "oxlint index.server.ts index.client.tsx client server shared tests benchmark",
    "format": "oxfmt index.server.ts index.client.tsx client server shared tests benchmark README.md package.json tsconfig.json paseo-plugin.json",
    "benchmark": "tsx benchmark/run.ts"
  },
  "devDependencies": {
    "@getpaseo/client": "0.9.0-beta.2",
    "@getpaseo/plugin": "0.9.0-beta.2",
    "@getpaseo/protocol": "0.9.0-beta.2",
    "@tanstack/react-query": "^5.90.11",
    "@types/node": "^22.0.0",
    "@types/react": "~19.2.0",
    "linkedom": "^0.18.13",
    "oxfmt": "0.46.0",
    "oxlint": "1.61.0",
    "react": "19.1.0",
    "react-native": "0.81.5",
    "tsx": "^4.20.0",
    "typescript": "^5.9.3",
    "zod": "^4.4.3"
  }
}
```

Copy `tsconfig.json` from `plugins/next-prompt-actions/tsconfig.json` unchanged.

- [ ] **Step 2: Install dependencies**

Run: `cd plugins/prompt-translate && npm install`
Expected: `package-lock.json` is created, with no errors.

- [ ] **Step 3: Write the failing test**

`tests/shared.test.ts`:

```ts
import { test } from "node:test";
import assert from "node:assert/strict";
import { hasVietnamese } from "../shared/vietnamese";
import { translateRpc } from "../shared/contracts";
import { translateSettings } from "../shared/settings";

test("hasVietnamese detects diacritics, including words shared with other languages", () => {
  assert.equal(hasVietnamese("cái này là gì"), true);
  assert.equal(hasVietnamese("ĐỔI TÊN HÀM"), true);
  assert.equal(hasVietnamese("rename the function"), false);
  assert.equal(hasVietnamese("fix src/app.ts:12 and run npm test"), false);
});

test("translate input rejects text over 20,000 characters and defaults cacheOnly", () => {
  assert.equal(translateRpc.input.safeParse({ text: "a".repeat(20_001) }).success, false);
  assert.deepEqual(translateRpc.input.parse({ text: "xin chào" }), {
    text: "xin chào",
    cacheOnly: false,
  });
});

test("settings parse {} into complete defaults", () => {
  assert.deepEqual(translateSettings.schema.parse({}), {
    translate: true,
    enhanceShortcut: true,
    provider: "vercel",
    translateModel: "google/gemini-2.5-flash-lite",
    enhanceModel: "openai/gpt-4.1-mini",
  });
});
```

- [ ] **Step 4: Run the test to verify it fails**

Run: `npm test`
Expected: FAIL, because the modules cannot be found.

- [ ] **Step 5: Implement the shared modules**

`shared/vietnamese.ts`:

```ts
// Any Vietnamese diacritic counts. Words like "là" share letters with French, but requiring
// Vietnamese-only letters would miss short prompts such as "cái này là gì".
const VIETNAMESE =
  /[àáảãạăằắẳẵặâầấẩẫậđèéẻẽẹêềếểễệìíỉĩịòóỏõọôồốổỗộơờớởỡợùúủũụưừứửữựỳýỷỹỵ]/iu;

export function hasVietnamese(text: string): boolean {
  return VIETNAMESE.test(text);
}
```

`shared/contracts.ts`:

```ts
import { defineRpc } from "@getpaseo/plugin";
import { z } from "zod";

export const MAX_TEXT = 20_000;
const text = z.string().min(1).max(MAX_TEXT);

export const translateRpc = defineRpc({
  name: "translate.translate",
  input: z.object({ text, cacheOnly: z.boolean().default(false) }),
  output: z.object({ translation: z.string().nullable() }),
});
export const enhanceRpc = defineRpc({
  name: "translate.enhance",
  input: z.object({ text }),
  output: z.object({ prompt: z.string() }),
});
export const originalRpc = defineRpc({
  name: "translate.original",
  input: z.object({ text }),
  output: z.object({ original: z.string().nullable() }),
});
```

`shared/settings.ts`:

```ts
import { defineSettings } from "@getpaseo/plugin";
import { z } from "zod";

// Provisional until Task 5 records benchmark results.
export const DEFAULT_TRANSLATE_MODEL = "google/gemini-2.5-flash-lite";
export const DEFAULT_ENHANCE_MODEL = "openai/gpt-4.1-mini";

export const providerSchema = z.enum(["vercel", "openrouter"]);
export type Provider = z.output<typeof providerSchema>;

export const translateSettings = defineSettings({
  id: "translate",
  scope: "host",
  version: 1,
  schema: z.object({
    translate: z.boolean().default(true),
    enhanceShortcut: z.boolean().default(true),
    provider: providerSchema.default("vercel"),
    translateModel: z.string().trim().min(1).default(DEFAULT_TRANSLATE_MODEL),
    enhanceModel: z.string().trim().min(1).default(DEFAULT_ENHANCE_MODEL),
  }),
});
export type TranslateSettings = z.output<typeof translateSettings.schema>;
```

- [ ] **Step 6: Run tests and typecheck**

Run: `npm test && npm run typecheck`
Expected: 3 tests pass, and typecheck exits 0.

- [ ] **Step 7: Checkpoint.** Stop for review. Commit only if the user asks.

---

### Task 2: Credentials, prompts, and one LLM call

**Files:**
- Create: `server/credentials.ts`, `server/prompts.ts`, `server/llm.ts`
- Test: `tests/llm.test.ts`

**Interfaces:**
- Consumes: `Provider` from `shared/settings.ts`.
- Produces:
  - `type Endpoint = { baseUrl: string; apiKey: string }`
  - `resolveEndpoint(provider: Provider, configFile: string, env?: Record<string, string | undefined>): Promise<Endpoint>`
  - `type Mode = "translate" | "enhance"`
  - `buildMessages(mode: Mode, text: string): { role: "system" | "user"; content: string }[]`
  - `maxTokens(mode: Mode, text: string): number`
  - `TIMEOUT_MS: Record<Mode, number>`
  - `type ChatRequest = { mode: Mode; provider: Provider; model: string; text: string }`
  - `type Complete = (request: ChatRequest) => Promise<string>`
  - `createCompleter(endpoint: (provider: Provider) => Promise<Endpoint>, fetchImpl?: FetchLike): Complete`

- [ ] **Step 1: Write the failing tests**

`tests/llm.test.ts`:

```ts
import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtemp, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { resolveEndpoint } from "../server/credentials";
import { buildMessages, maxTokens } from "../server/prompts";
import { createCompleter, type FetchLike } from "../server/llm";

async function config(value: unknown) {
  const file = path.join(await mkdtemp(path.join(tmpdir(), "pt-")), "config.json");
  await writeFile(file, JSON.stringify(value));
  return file;
}

test("vercel endpoint comes from daemon config and trims the base URL", async () => {
  const file = await config({
    agents: {
      providers: {
        "vercel-gateway": { env: { OPENAI_API_KEY: "k", OPENAI_BASE_URL: "https://g/v1/" } },
      },
    },
  });
  assert.deepEqual(await resolveEndpoint("vercel", file), { baseUrl: "https://g/v1", apiKey: "k" });
});

test("missing credentials fail without leaking anything", async () => {
  await assert.rejects(resolveEndpoint("vercel", "/nonexistent/config.json"), {
    message: "vercel credential unavailable",
  });
  await assert.rejects(resolveEndpoint("openrouter", "/x", {}), {
    message: "openrouter credential unavailable",
  });
  assert.deepEqual(await resolveEndpoint("openrouter", "/x", { OPENROUTER_API_KEY: "o" }), {
    baseUrl: "https://openrouter.ai/api/v1",
    apiKey: "o",
  });
});

test("messages wrap the text so the model translates instead of answering", () => {
  const [system, user] = buildMessages("translate", "bạn là ai?");
  assert.equal(system.role, "system");
  assert.equal(user.content, "<message>\nbạn là ai?\n</message>");
  assert.ok(maxTokens("translate", "x".repeat(100)) >= 256);
  assert.equal(maxTokens("enhance", "x".repeat(20_000)), 8192);
});

test("completer posts one non-streamed request and returns trimmed content", async () => {
  let seen: { url: string; headers: Record<string, string>; body: Record<string, unknown> } | undefined;
  const fetchImpl: FetchLike = async (url, init) => {
    seen = { url, headers: init.headers, body: JSON.parse(init.body) };
    return {
      ok: true,
      status: 200,
      json: async () => ({ choices: [{ message: { content: "  <message>Hello</message> " } }] }),
    };
  };
  const complete = createCompleter(async () => ({ baseUrl: "https://g/v1", apiKey: "k" }), fetchImpl);
  const out = await complete({ mode: "translate", provider: "vercel", model: "m", text: "xin chào" });
  assert.equal(out, "Hello");
  assert.equal(seen?.url, "https://g/v1/chat/completions");
  assert.equal(seen?.headers.authorization, "Bearer k");
  assert.equal(seen?.body.model, "m");
  assert.equal(seen?.body.temperature, 0);
  assert.equal(seen?.body.stream, false);
});

test("completer errors carry status and provider message, never the key", async () => {
  const failing: FetchLike = async () => ({
    ok: false,
    status: 401,
    json: async () => ({ error: { message: "bad key" } }),
  });
  const complete = createCompleter(async () => ({ baseUrl: "https://g", apiKey: "secret" }), failing);
  await assert.rejects(
    complete({ mode: "enhance", provider: "vercel", model: "m", text: "x" }),
    (error: Error) => error.message === "Model request failed (401): bad key" && !error.message.includes("secret"),
  );
  const empty: FetchLike = async () => ({ ok: true, status: 200, json: async () => ({ choices: [] }) });
  await assert.rejects(
    createCompleter(async () => ({ baseUrl: "https://g", apiKey: "k" }), empty)({
      mode: "translate",
      provider: "vercel",
      model: "m",
      text: "x",
    }),
    { message: "Model returned no text" },
  );
});
```

- [ ] **Step 2: Run to verify failure**

Run: `npm test`
Expected: FAIL, because the `server/*` modules cannot be found.

- [ ] **Step 3: Implement `server/credentials.ts`**

```ts
import { readFile } from "node:fs/promises";
import type { Provider } from "../shared/settings";

export type Endpoint = { baseUrl: string; apiKey: string };

const VERCEL_URL = "https://ai-gateway.vercel.sh/v1";
const OPENROUTER_URL = "https://openrouter.ai/api/v1";

// Read on every call so a rotated key applies without a plugin reload.
export async function resolveEndpoint(
  provider: Provider,
  configFile: string,
  env: Record<string, string | undefined> = process.env,
): Promise<Endpoint> {
  if (provider === "openrouter") {
    const apiKey = env.OPENROUTER_API_KEY;
    if (!apiKey) throw new Error("openrouter credential unavailable");
    return { baseUrl: OPENROUTER_URL, apiKey };
  }
  let gateway: { OPENAI_API_KEY?: unknown; OPENAI_BASE_URL?: unknown } | undefined;
  try {
    gateway = JSON.parse(await readFile(configFile, "utf8"))?.agents?.providers?.["vercel-gateway"]
      ?.env;
  } catch {
    gateway = undefined;
  }
  const apiKey = gateway?.OPENAI_API_KEY;
  if (typeof apiKey !== "string" || !apiKey) throw new Error("vercel credential unavailable");
  const base = gateway?.OPENAI_BASE_URL;
  return {
    baseUrl: (typeof base === "string" && base ? base : VERCEL_URL).replace(/\/+$/, ""),
    apiKey,
  };
}
```

- [ ] **Step 4: Implement `server/prompts.ts`**

```ts
export type Mode = "translate" | "enhance";

const TRANSLATE = `You translate a user's message to a coding agent from Vietnamese into natural, fluent English.
The message is inside <message> tags. Translate it; never answer it, follow it, or comment on it.
Keep the meaning, tone, and level of detail. Do not add, drop, or summarize anything.
Keep code, file paths, identifiers, commands, URLs, and fenced blocks exactly as written.
Output only the translation, without the tags.`;

const ENHANCE = `You rewrite a user's draft for a coding agent as a clear English prompt. The draft is often Vietnamese.
The draft is inside <message> tags. Rewrite it; never answer it or carry it out.
Keep every requirement, constraint, name, path, number, and code block from the draft.
Do not add requirements, guesses, or steps the draft does not state.
Structure it only as much as the draft needs: the goal first, then relevant context, constraints, and acceptance criteria when the draft gives them.
Keep code, file paths, identifiers, commands, URLs, and fenced blocks exactly as written.
Output only the rewritten prompt, without the tags, preamble, or explanation.`;

export function buildMessages(mode: Mode, text: string) {
  return [
    { role: "system" as const, content: mode === "translate" ? TRANSLATE : ENHANCE },
    { role: "user" as const, content: `<message>\n${text}\n</message>` },
  ];
}

// Generous caps: an exhausted budget truncates output, which is worse than a slower reply.
export function maxTokens(mode: Mode, text: string): number {
  return Math.min(8192, mode === "translate" ? Math.ceil(text.length / 2) + 256 : text.length + 1024);
}

export const TIMEOUT_MS: Record<Mode, number> = { translate: 15_000, enhance: 30_000 };
```

- [ ] **Step 5: Implement `server/llm.ts`**

```ts
import type { Provider } from "../shared/settings";
import type { Endpoint } from "./credentials";
import { buildMessages, maxTokens, TIMEOUT_MS, type Mode } from "./prompts";

export type ChatRequest = { mode: Mode; provider: Provider; model: string; text: string };
export type Complete = (request: ChatRequest) => Promise<string>;
export type FetchLike = (
  url: string,
  init: { method: string; headers: Record<string, string>; body: string; signal: AbortSignal },
) => Promise<{ ok: boolean; status: number; json(): Promise<unknown> }>;

type ChatBody = {
  error?: { message?: unknown };
  choices?: { message?: { content?: unknown } }[];
};

export function createCompleter(
  endpoint: (provider: Provider) => Promise<Endpoint>,
  fetchImpl: FetchLike = fetch,
): Complete {
  return async ({ mode, provider, model, text }) => {
    const { baseUrl, apiKey } = await endpoint(provider);
    const response = await fetchImpl(`${baseUrl}/chat/completions`, {
      method: "POST",
      headers: { authorization: `Bearer ${apiKey}`, "content-type": "application/json" },
      body: JSON.stringify({
        model,
        messages: buildMessages(mode, text),
        temperature: 0,
        max_tokens: maxTokens(mode, text),
        stream: false,
      }),
      signal: AbortSignal.timeout(TIMEOUT_MS[mode]),
    });
    const body = (await response.json().catch(() => null)) as ChatBody | null;
    if (!response.ok) {
      const detail = body?.error?.message;
      throw new Error(
        `Model request failed (${response.status})${detail ? `: ${String(detail).slice(0, 200)}` : ""}`,
      );
    }
    const content = body?.choices?.[0]?.message?.content;
    const output = typeof content === "string" ? unwrap(content) : "";
    if (!output) throw new Error("Model returned no text");
    return output;
  };
}

function unwrap(text: string): string {
  return text
    .trim()
    .replace(/^<message>\s*/, "")
    .replace(/\s*<\/message>$/, "")
    .trim();
}
```

- [ ] **Step 6: Run tests and typecheck**

Run: `npm test && npm run typecheck`
Expected: all tests pass, and typecheck exits 0.

- [ ] **Step 7: Checkpoint.** Stop for review. Commit only if the user asks.

---

### Task 3: Store with cache, dedupe, and enhance pairs

**Files:**
- Create: `server/store.ts`
- Test: `tests/store.test.ts`

**Interfaces:**
- Produces:
  - `hash(...parts: string[]): string` (sha256 hex)
  - `class Store(file: string, limits?: { cache: number; pairs: number }, delay?: number)` with:
    - `get(key: string): Promise<string | undefined>`
    - `run(key: string, task: () => Promise<string>): Promise<string>`
    - `pair(prompt: string, original: string): Promise<void>`
    - `original(prompt: string): Promise<string | null>`
    - `flush(): Promise<void>`
    - `close(): void`

- [ ] **Step 1: Write the failing tests**

`tests/store.test.ts`:

```ts
import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtemp, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { Store } from "../server/store";

async function file() {
  return path.join(await mkdtemp(path.join(tmpdir(), "pt-store-")), "nested", "cache.json");
}

test("concurrent runs for one key share a single task", async () => {
  const store = new Store(await file(), undefined, 5);
  let calls = 0;
  const task = async () => {
    calls++;
    await new Promise((resolve) => setTimeout(resolve, 10));
    return "v";
  };
  assert.deepEqual(await Promise.all([store.run("k", task), store.run("k", task)]), ["v", "v"]);
  assert.equal(await store.run("k", task), "v");
  assert.equal(calls, 1);
  store.close();
});

test("a failed task is not cached and can run again", async () => {
  const store = new Store(await file(), undefined, 5);
  await assert.rejects(store.run("k", async () => Promise.reject(new Error("boom"))), { message: "boom" });
  assert.equal(await store.run("k", async () => "ok"), "ok");
  store.close();
});

test("least recently used entries are evicted", async () => {
  const store = new Store(await file(), { cache: 2, pairs: 2 }, 5);
  await store.run("a", async () => "1");
  await store.run("b", async () => "2");
  await store.get("a");
  await store.run("c", async () => "3");
  assert.equal(await store.get("a"), "1");
  assert.equal(await store.get("b"), undefined);
  store.close();
});

test("values and pairs persist, and a corrupt file starts empty", async () => {
  const target = await file();
  const first = new Store(target, undefined, 5);
  await first.run("k", async () => "v");
  await first.pair("Enhanced prompt", "Bản gốc");
  await first.flush();
  const second = new Store(target, undefined, 5);
  assert.equal(await second.get("k"), "v");
  assert.equal(await second.original("Enhanced prompt"), "Bản gốc");
  assert.equal(await second.original("Other"), null);
  await writeFile(target, "{not json");
  const third = new Store(target, undefined, 5);
  assert.equal(await third.get("k"), undefined);
  first.close();
  second.close();
  third.close();
});
```

- [ ] **Step 2: Run to verify failure**

Run: `npm test`
Expected: FAIL, because `../server/store` cannot be found.

- [ ] **Step 3: Implement `server/store.ts`**

```ts
import { createHash } from "node:crypto";
import { mkdir, readFile, rename, writeFile } from "node:fs/promises";
import path from "node:path";

export function hash(...parts: string[]): string {
  return createHash("sha256").update(parts.join("\u0000")).digest("hex");
}

type Saved = { version: 1; cache: [string, string][]; pairs: [string, string][] };

export class Store {
  private readonly cache = new Map<string, string>();
  private readonly pairs = new Map<string, string>();
  private readonly inflight = new Map<string, Promise<string>>();
  private readonly loaded: Promise<void>;
  private timer: ReturnType<typeof setTimeout> | undefined;

  constructor(
    private readonly file: string,
    private readonly limits = { cache: 500, pairs: 200 },
    private readonly delay = 500,
  ) {
    this.loaded = this.load();
  }

  async get(key: string): Promise<string | undefined> {
    await this.loaded;
    const value = this.cache.get(key);
    if (value !== undefined) {
      this.cache.delete(key);
      this.cache.set(key, value);
    }
    return value;
  }

  // Register the in-flight promise before any await so a concurrent caller cannot miss it.
  run(key: string, task: () => Promise<string>): Promise<string> {
    const pending = this.inflight.get(key);
    if (pending) return pending;
    const promise = (async () => {
      const hit = await this.get(key);
      if (hit !== undefined) return hit;
      const value = await task();
      this.remember(this.cache, key, value, this.limits.cache);
      return value;
    })().finally(() => this.inflight.delete(key));
    this.inflight.set(key, promise);
    return promise;
  }

  async pair(prompt: string, original: string): Promise<void> {
    await this.loaded;
    this.remember(this.pairs, hash(prompt), original, this.limits.pairs);
  }

  async original(prompt: string): Promise<string | null> {
    await this.loaded;
    return this.pairs.get(hash(prompt)) ?? null;
  }

  async flush(): Promise<void> {
    clearTimeout(this.timer);
    this.timer = undefined;
    const saved: Saved = { version: 1, cache: [...this.cache], pairs: [...this.pairs] };
    try {
      await mkdir(path.dirname(this.file), { recursive: true });
      const temporary = `${this.file}.tmp`;
      await writeFile(temporary, JSON.stringify(saved));
      await rename(temporary, this.file);
    } catch (error) {
      // A failed write only costs future cache hits; never fail the user's request for it.
      console.warn(
        `prompt-translate: cache write failed: ${error instanceof Error ? error.message : String(error)}`,
      );
    }
  }

  close(): void {
    if (this.timer) void this.flush();
  }

  private async load(): Promise<void> {
    try {
      const saved = JSON.parse(await readFile(this.file, "utf8")) as Partial<Saved>;
      for (const [key, value] of saved.cache ?? [])
        if (typeof key === "string" && typeof value === "string") this.cache.set(key, value);
      for (const [key, value] of saved.pairs ?? [])
        if (typeof key === "string" && typeof value === "string") this.pairs.set(key, value);
    } catch {
      // Missing or corrupt: start empty; the next flush replaces the file.
    }
  }

  private remember(map: Map<string, string>, key: string, value: string, limit: number): void {
    map.delete(key);
    map.set(key, value);
    while (map.size > limit) map.delete(map.keys().next().value as string);
    this.timer ??= setTimeout(() => void this.flush(), this.delay);
  }
}
```

- [ ] **Step 4: Run tests and typecheck**

Run: `npm test && npm run typecheck`
Expected: all tests pass, and typecheck exits 0.

- [ ] **Step 5: Checkpoint.** Stop for review. Commit only if the user asks.

---

### Task 4: Service and server entry

**Files:**
- Create: `server/service.ts`, `index.server.ts`
- Test: `tests/service.test.ts`

**Interfaces:**
- Consumes: `Store`, `hash` (Task 3); `Complete`, `createCompleter`, `resolveEndpoint` (Task 2);
  `translateSettings`, `TranslateSettings`, `hasVietnamese`, and the RPC contracts (Task 1).
- Produces: `createService(deps: { store: Store; complete: Complete; settings: () => Promise<TranslateSettings> })`,
  which returns `{ translate(input: { text: string; cacheOnly: boolean }): Promise<{ translation: string | null }>; enhance(input: { text: string }): Promise<{ prompt: string }>; original(input: { text: string }): Promise<{ original: string | null }> }`.
  It also produces the registered RPC names `translate.translate`, `translate.enhance`, and `translate.original`.

- [ ] **Step 1: Write the failing tests**

`tests/service.test.ts`:

```ts
import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtemp } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { Store } from "../server/store";
import { createService } from "../server/service";
import type { ChatRequest } from "../server/llm";
import { translateSettings, type TranslateSettings } from "../shared/settings";

async function setup(overrides: Partial<TranslateSettings> = {}) {
  const dir = await mkdtemp(path.join(tmpdir(), "pt-service-"));
  const store = new Store(path.join(dir, "cache.json"), undefined, 5);
  const requests: ChatRequest[] = [];
  const service = createService({
    store,
    complete: async (request) => {
      requests.push(request);
      return request.mode === "translate" ? `EN:${request.text}` : `PROMPT:${request.text}`;
    },
    settings: async () => ({ ...translateSettings.schema.parse({}), ...overrides }),
  });
  return { service, requests, store };
}

test("cacheOnly never calls the model, and a real translate fills the cache", async () => {
  const { service, requests, store } = await setup();
  assert.deepEqual(await service.translate({ text: "xin chào", cacheOnly: true }), { translation: null });
  assert.deepEqual(await service.translate({ text: "xin chào", cacheOnly: false }), {
    translation: "EN:xin chào",
  });
  assert.deepEqual(await service.translate({ text: "xin chào", cacheOnly: true }), {
    translation: "EN:xin chào",
  });
  assert.equal(requests.length, 1);
  assert.equal(requests[0].model, "google/gemini-2.5-flash-lite");
  store.close();
});

test("English text is rejected and translation off returns null without a call", async () => {
  const { service, requests, store } = await setup();
  await assert.rejects(service.translate({ text: "hello", cacheOnly: false }), {
    message: "Text has no Vietnamese to translate",
  });
  const off = await setup({ translate: false });
  assert.deepEqual(await off.service.translate({ text: "xin chào", cacheOnly: false }), {
    translation: null,
  });
  assert.equal(requests.length + off.requests.length, 0);
  store.close();
  off.store.close();
});

test("enhance uses the enhance model and records the original", async () => {
  const { service, requests, store } = await setup();
  assert.deepEqual(await service.enhance({ text: "sửa lỗi" }), { prompt: "PROMPT:sửa lỗi" });
  assert.equal(requests[0].model, "openai/gpt-4.1-mini");
  assert.deepEqual(await service.original({ text: "PROMPT:sửa lỗi" }), { original: "sửa lỗi" });
  assert.deepEqual(await service.original({ text: "sửa lỗi" }), { original: null });
  const off = await setup({ enhanceShortcut: false });
  await assert.rejects(off.service.enhance({ text: "sửa lỗi" }), { message: "Enhance shortcut is off" });
  store.close();
  off.store.close();
});
```

- [ ] **Step 2: Run to verify failure**

Run: `npm test`
Expected: FAIL, because `../server/service` cannot be found.

- [ ] **Step 3: Implement `server/service.ts`**

```ts
import { hasVietnamese } from "../shared/vietnamese";
import type { TranslateSettings } from "../shared/settings";
import type { Complete } from "./llm";
import { hash, type Store } from "./store";

export function createService(deps: {
  store: Store;
  complete: Complete;
  settings: () => Promise<TranslateSettings>;
}) {
  return {
    async translate({ text, cacheOnly }: { text: string; cacheOnly: boolean }) {
      if (!hasVietnamese(text)) throw new Error("Text has no Vietnamese to translate");
      const settings = await deps.settings();
      if (!settings.translate) return { translation: null };
      const { provider, translateModel: model } = settings;
      const key = hash("translate", provider, model, text);
      if (cacheOnly) return { translation: (await deps.store.get(key)) ?? null };
      const translation = await deps.store.run(key, () =>
        deps.complete({ mode: "translate", provider, model, text }),
      );
      return { translation };
    },
    async enhance({ text }: { text: string }) {
      const settings = await deps.settings();
      if (!settings.enhanceShortcut) throw new Error("Enhance shortcut is off");
      const { provider, enhanceModel: model } = settings;
      const prompt = await deps.store.run(hash("enhance", provider, model, text), () =>
        deps.complete({ mode: "enhance", provider, model, text }),
      );
      await deps.store.pair(prompt, text);
      return { prompt };
    },
    async original({ text }: { text: string }) {
      return { original: await deps.store.original(text) };
    },
  };
}
```

- [ ] **Step 4: Implement `index.server.ts`**

```ts
import type { PluginServerContext } from "@getpaseo/plugin/server";
import { homedir } from "node:os";
import path from "node:path";
import { enhanceRpc, originalRpc, translateRpc } from "./shared/contracts";
import { translateSettings } from "./shared/settings";
import { resolveEndpoint } from "./server/credentials";
import { createCompleter } from "./server/llm";
import { createService } from "./server/service";
import { Store } from "./server/store";

export default function contribute(server: PluginServerContext) {
  const home = process.env.PASEO_HOME || path.join(homedir(), ".paseo");
  const configFile = path.join(home, "config.json");
  const settings = server.registerSettings(translateSettings);
  const store = new Store(path.join(home, "plugin-data/prompt-translate/cache.json"));
  const service = createService({
    store,
    complete: createCompleter((provider) => resolveEndpoint(provider, configFile)),
    async settings() {
      const current = await settings.read();
      // Invalid stored settings keep working on defaults until the user resets them.
      return current.status === "ready" ? current.values : translateSettings.schema.parse({});
    },
  });
  server.handle(translateRpc, (input) => service.translate(input));
  server.handle(enhanceRpc, (input) => service.enhance(input));
  server.handle(originalRpc, (input) => service.original(input));
  return () => store.close();
}
```

- [ ] **Step 5: Run tests, typecheck, and lint**

Run: `npm test && npm run typecheck && npm run lint`
Expected: all pass. If typecheck rejects `current.values` as the wrong type, compare it with
`PluginSettingsState` in `node_modules/@getpaseo/plugin/dist/server/contracts.d.ts` and use its
exact narrowed shape.

- [ ] **Step 6: Checkpoint.** Stop for review. Commit only if the user asks.

---

### Task 5: Benchmark and choose default models

This task consumes API quota. Confirm with the user before Step 3. The default run makes about
330 calls: 11 models × 2 modes × 3 samples × 5 runs.

**Files:**
- Create: `benchmark/run.ts`, `benchmark/samples.json`
- Modify: `shared/settings.ts` (the two default model constants), `tests/shared.test.ts` and
  `tests/service.test.ts` (expected default IDs)

**Interfaces:**
- Consumes: `resolveEndpoint` and `buildMessages`, `maxTokens`, `Mode` from Task 2.

- [ ] **Step 1: Create `benchmark/samples.json` (synthetic only)**

```json
[
  { "id": "short", "text": "Sửa lỗi nút Lưu không phản hồi trên trang cài đặt." },
  {
    "id": "medium",
    "text": "Trong plugin board, khi tôi bấm Send thì giao diện bị giật khoảng một giây. Hãy tìm nguyên nhân, đo lại số lần render và đề xuất cách sửa nhỏ nhất. Không đổi API public."
  },
  {
    "id": "long",
    "text": "Tôi muốn thêm tính năng xuất báo cáo CSV cho trang đơn hàng.\n\nBối cảnh: dữ liệu lấy từ `src/orders/query.ts`, bảng hiển thị ở `src/orders/table.tsx`. Hiện tại người dùng phải copy tay từng dòng.\n\nYêu cầu:\n- Nút \"Xuất CSV\" ở góc phải thanh công cụ.\n- Chỉ xuất các dòng đang được lọc, giữ đúng thứ tự cột.\n- Ngày theo định dạng ISO, số tiền không có dấu phân cách hàng nghìn.\n\n```ts\nexport type Order = { id: string; total: number; createdAt: Date };\n```\n\nKhông thêm thư viện mới. Viết test cho hàm chuyển đổi và kiểm tra với 10.000 dòng vẫn dưới 1 giây."
  }
]
```

- [ ] **Step 2: Create `benchmark/run.ts`**

```ts
// Manual benchmark: consumes API quota. Usage: npm run benchmark -- --provider=vercel --runs=5 [--models=a,b]
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { homedir } from "node:os";
import path from "node:path";
import { resolveEndpoint } from "../server/credentials";
import { buildMessages, maxTokens, type Mode } from "../server/prompts";
import type { Provider } from "../shared/settings";

const CANDIDATES = [
  "google/gemini-3.5-flash-lite",
  "google/gemini-3.1-flash-lite",
  "google/gemini-2.5-flash-lite",
  "openai/gpt-4.1-nano-fast",
  "openai/gpt-4.1-mini-fast",
  "openai/gpt-oss-120b",
  "meta/llama-3.3-70b",
  "anthropic/claude-haiku-4.5",
  "moonshotai/kimi-k3-fast",
  "deepseek/deepseek-v4.1-flash",
  "mistral/ministral-8b",
];
const MODES: Mode[] = ["translate", "enhance"];

const args = new Map(
  process.argv.slice(2).map((arg) => arg.replace(/^--/, "").split("=") as [string, string]),
);
const provider = (args.get("provider") ?? "vercel") as Provider;
const runs = Number(args.get("runs") ?? 5);
const home = process.env.PASEO_HOME || path.join(homedir(), ".paseo");
const endpoint = await resolveEndpoint(provider, path.join(home, "config.json"));
const headers = { authorization: `Bearer ${endpoint.apiKey}`, "content-type": "application/json" };

const listing = (await (await fetch(`${endpoint.baseUrl}/models`, { headers })).json()) as {
  data?: { id: string }[];
};
const listed = new Set((listing.data ?? []).map((model) => model.id));
const models = (args.get("models")?.split(",") ?? CANDIDATES).filter((model) => {
  if (listed.has(model)) return true;
  console.log(`skip ${model}: not listed by ${provider}`);
  return false;
});
const samples = JSON.parse(
  await readFile(new URL("./samples.json", import.meta.url), "utf8"),
) as { id: string; text: string }[];

type Result = { ttftMs?: number; totalMs?: number; output?: string; error?: string };
type Row = Result & { model: string; mode: Mode; sample: string; run: number };

async function call(model: string, mode: Mode, text: string): Promise<Result> {
  const start = performance.now();
  let ttftMs: number | undefined;
  let output = "";
  try {
    const response = await fetch(`${endpoint.baseUrl}/chat/completions`, {
      method: "POST",
      headers,
      signal: AbortSignal.timeout(60_000),
      body: JSON.stringify({
        model,
        messages: buildMessages(mode, text),
        temperature: 0,
        max_tokens: maxTokens(mode, text),
        stream: true,
      }),
    });
    if (!response.ok || !response.body) return { error: `HTTP ${response.status}` };
    const reader = response.body.getReader();
    const decoder = new TextDecoder();
    let buffer = "";
    for (;;) {
      const { done, value } = await reader.read();
      if (done) break;
      buffer += decoder.decode(value, { stream: true });
      const lines = buffer.split("\n");
      buffer = lines.pop() ?? "";
      for (const line of lines) {
        const data = line.startsWith("data:") ? line.slice(5).trim() : "";
        if (!data || data === "[DONE]") continue;
        const delta = (JSON.parse(data) as { choices?: { delta?: { content?: string } }[] })
          .choices?.[0]?.delta?.content;
        if (!delta) continue;
        ttftMs ??= performance.now() - start;
        output += delta;
      }
    }
    return output ? { ttftMs, totalMs: performance.now() - start, output } : { error: "empty output" };
  } catch (error) {
    return { error: error instanceof Error ? error.message : String(error) };
  }
}

function percentile(values: number[], p: number): string {
  if (!values.length) return "-";
  const sorted = [...values].sort((a, b) => a - b);
  return String(Math.round(sorted[Math.min(sorted.length - 1, Math.floor(p * sorted.length))]));
}

// Sequential on purpose: parallel calls would share rate limits and skew latency.
const rows: Row[] = [];
for (const model of models)
  for (const mode of MODES)
    for (const sample of samples)
      for (let run = 1; run <= runs; run++) {
        const result = await call(model, mode, sample.text);
        rows.push({ model, mode, sample: sample.id, run, ...result });
        console.log(
          `${model} ${mode} ${sample.id}#${run} ${result.error ?? `${Math.round(result.ttftMs ?? 0)}/${Math.round(result.totalMs ?? 0)}ms`}`,
        );
      }

console.log("\nmodel | mode | ok | TTFT p50/p90 ms | total p50/p90 ms");
for (const model of models)
  for (const mode of MODES) {
    const group = rows.filter((row) => row.model === model && row.mode === mode);
    const ok = group.filter((row) => !row.error);
    const ttft = ok.map((row) => row.ttftMs ?? 0);
    const total = ok.map((row) => row.totalMs ?? 0);
    console.log(
      `${model} | ${mode} | ${ok.length}/${group.length} | ${percentile(ttft, 0.5)}/${percentile(ttft, 0.9)} | ${percentile(total, 0.5)}/${percentile(total, 0.9)}`,
    );
  }

const out = path.join(import.meta.dirname, "..", "artifacts", `benchmark-${new Date().toISOString().replace(/[:.]/g, "-")}.json`);
await mkdir(path.dirname(out), { recursive: true });
await writeFile(out, JSON.stringify({ provider, runs, rows }, null, 2));
console.log(`\nraw outputs: ${out}`);
```

Run: `npm run typecheck && npm run lint`
Expected: both pass. `git check-ignore plugins/prompt-translate/artifacts/x.json` prints the
path, which confirms the output is ignored.

- [ ] **Step 3: Run a smoke benchmark (needs user approval for quota)**

Run: `npm run benchmark -- --runs=1 --models=google/gemini-2.5-flash-lite`
Expected: 6 lines with latencies and a table row reading `ok 3/3`. If the result is `HTTP 401`,
the credential is wrong: stop and report it, and do not print the key.

- [ ] **Step 4: Run the full benchmark (needs user approval for quota)**

Run: `npm run benchmark -- --runs=5`
Expected: a table for every listed candidate, and a raw JSON file under `artifacts/`. Failed
calls stay in the results. Do not rerun them to improve scores.

- [ ] **Step 5: Choose defaults with the user**

Read the raw outputs for quality. The translation must be faithful and must not answer the
prompt. The enhance output must add no invented requirements and keep code and paths verbatim.
Then pick:
- translate: the lowest total p50 among models whose translate outputs are all acceptable;
- enhance: the lowest total p50 among models whose enhance outputs are all acceptable.

Present the table and the choice with the AskUserQuestion tool. Then update
`DEFAULT_TRANSLATE_MODEL` and `DEFAULT_ENHANCE_MODEL` in `shared/settings.ts`. Update the
expected IDs in `tests/shared.test.ts` and `tests/service.test.ts` to match.

Run: `npm test`
Expected: all pass.

- [ ] **Step 6: Checkpoint.** Stop for review. Commit only if the user asks.

---

### Task 6: Bubble adapter

**Files:**
- Create: `client/dom.ts`, `client/bubble.ts`
- Test: `tests/bubble.test.ts`

**Interfaces:**
- Consumes: `hasVietnamese` (Task 1).
- Produces:
  - `client/dom.ts`: `interface El`, `interface Doc`, `type Key`, `type Observer`,
    `reactProps(node: El, match: (props: Record<string, unknown>) => boolean): Record<string, unknown> | null`,
    `desktopSupported(): boolean`.
  - `client/bubble.ts`: `type BubbleApi = { translate(text: string, cacheOnly: boolean): Promise<string | null>; original(text: string): Promise<string | null> }`,
    `type BubbleOptions = { enabled(): boolean; activeSince(): number }`,
    `installBubbles(api: BubbleApi, options: BubbleOptions, doc: Doc, Observer?: Observer): { scan(): void; stop(): void }`.

- [ ] **Step 1: Implement `client/dom.ts` (types only, plus two helpers)**

```ts
// Structural DOM types: the client tsconfig has no DOM lib, and linkedom satisfies these in tests.
export interface El {
  textContent: string | null;
  isConnected: boolean;
  parentElement: El | null;
  nextElementSibling: El | null;
  value?: string;
  style: { cssText: string };
  querySelector(selector: string): El | null;
  querySelectorAll(selector: string): ArrayLike<El>;
  closest(selector: string): El | null;
  getAttribute(name: string): string | null;
  setAttribute(name: string, value: string): void;
  hasAttribute(name: string): boolean;
  append(...nodes: (El | string)[]): void;
  after(...nodes: El[]): void;
  remove(): void;
  addEventListener(name: string, handler: () => void): void;
  focus?(): void;
  setSelectionRange?(start: number, end: number): void;
  dispatchEvent(event: object): boolean;
}
export type Key = {
  key: string;
  metaKey: boolean;
  ctrlKey: boolean;
  shiftKey: boolean;
  isComposing?: boolean;
  keyCode?: number;
  target: unknown;
  preventDefault(): void;
  stopImmediatePropagation(): void;
};
export interface Doc {
  head: El;
  body: El;
  createElement(tag: string): El;
  querySelectorAll(selector: string): ArrayLike<El>;
  defaultView: {
    Event: new (type: string, init?: { bubbles?: boolean }) => object;
    getComputedStyle?(node: El): { color: string };
  } | null;
  addEventListener(name: "keydown", handler: (event: Key) => void, capture: boolean): void;
  removeEventListener(name: "keydown", handler: (event: Key) => void, capture: boolean): void;
}
export type Observer = new (callback: () => void) => {
  observe(node: El, options: object): void;
  disconnect(): void;
};

type Fiber = { memoizedProps?: Record<string, unknown>; return?: Fiber };

// Private host detail: React stores the fiber on the DOM node under a randomized key.
export function reactProps(
  node: El,
  match: (props: Record<string, unknown>) => boolean,
): Record<string, unknown> | null {
  const key = Object.keys(node).find((name) => name.startsWith("__reactFiber$"));
  if (!key) return null;
  let fiber = (node as unknown as Record<string, Fiber | undefined>)[key];
  for (let depth = 0; fiber && depth < 40; depth++, fiber = fiber.return)
    if (fiber.memoizedProps && match(fiber.memoizedProps)) return fiber.memoizedProps;
  return null;
}

export function desktopSupported(): boolean {
  const scope = globalThis as { document?: unknown; navigator?: { userAgent?: string } };
  return scope.document !== undefined && /Electron\//.test(scope.navigator?.userAgent ?? "");
}
```

- [ ] **Step 2: Write the failing tests**

`tests/bubble.test.ts`:

```ts
import { test } from "node:test";
import assert from "node:assert/strict";
import { parseHTML } from "linkedom";
import { installBubbles, type BubbleApi } from "../client/bubble";
import type { Doc, El } from "../client/dom";

type Message = { text: string; timestamp?: number };
function page(messages: Message[]) {
  const { document, window } = parseHTML(
    `<html><head></head><body>${messages
      .map(() => '<div data-testid="user-message"><div><div data-message-text="true"></div></div></div>')
      .join("")}</body></html>`,
  );
  const texts = Array.from(document.querySelectorAll('[data-message-text="true"]'));
  texts.forEach((node, index) => {
    const { text, timestamp } = messages[index];
    node.textContent = text;
    if (timestamp !== undefined)
      (node as unknown as Record<string, unknown>)["__reactFiber$test"] = {
        memoizedProps: { children: text },
        return: { memoizedProps: { message: text, timestamp } },
      };
  });
  return { doc: document as unknown as Doc, window, texts: texts as unknown as El[] };
}
const settle = () => new Promise((resolve) => setTimeout(resolve, 0));
const annotation = (node: El) =>
  node.nextElementSibling?.hasAttribute("data-prompt-translate") ? node.nextElementSibling : null;

function api(overrides: Partial<BubbleApi> = {}) {
  const calls: { text: string; cacheOnly: boolean }[] = [];
  const value: BubbleApi = {
    translate: async (text, cacheOnly) => {
      calls.push({ text, cacheOnly });
      return cacheOnly ? null : `EN ${text}`;
    },
    original: async () => null,
    ...overrides,
  };
  return { value, calls };
}
const on = { enabled: () => true, activeSince: () => 1_000 };

test("a new Vietnamese prompt gets an EN annotation; English gets nothing", async () => {
  const { doc, texts } = page([
    { text: "sửa lỗi", timestamp: 2_000 },
    { text: "fix the bug", timestamp: 2_000 },
  ]);
  const { value, calls } = api();
  const bubbles = installBubbles(value, on, doc);
  await settle();
  assert.deepEqual(calls, [{ text: "sửa lỗi", cacheOnly: false }]);
  assert.equal(annotation(texts[0])?.textContent, "ENEN sửa lỗi");
  assert.equal(annotation(texts[1]), null);
  bubbles.stop();
  assert.equal(annotation(texts[0]), null);
  assert.equal(doc.querySelectorAll("[data-prompt-translate-style]").length, 0);
});

test("old or unidentifiable prompts are cache-only and stay bare on a miss", async () => {
  const { doc, texts } = page([{ text: "cũ", timestamp: 500 }, { text: "không rõ" }]);
  const { value, calls } = api();
  const bubbles = installBubbles(value, on, doc);
  await settle();
  assert.deepEqual(calls, [
    { text: "cũ", cacheOnly: true },
    { text: "không rõ", cacheOnly: true },
  ]);
  assert.equal(annotation(texts[0]), null);
  assert.equal(annotation(texts[1]), null);
  bubbles.stop();
});

test("an enhanced prompt shows its Vietnamese original", async () => {
  const { doc, texts } = page([{ text: "Fix the save button.", timestamp: 2_000 }]);
  const { value, calls } = api({ original: async () => "sửa nút lưu" });
  const bubbles = installBubbles(value, on, doc);
  await settle();
  assert.equal(annotation(texts[0])?.textContent, "VI gốcsửa nút lưu");
  assert.equal(calls.length, 0);
  bubbles.stop();
});

test("a failed new translation offers a manual retry", async () => {
  const { doc, window, texts } = page([{ text: "lỗi mạng", timestamp: 2_000 }]);
  let fail = true;
  const bubbles = installBubbles(
    {
      translate: async (text) => {
        if (fail) throw new Error("offline");
        return `EN ${text}`;
      },
      original: async () => null,
    },
    on,
    doc,
  );
  await settle();
  assert.equal(annotation(texts[0])?.textContent, "Không dịch được · Thử lại");
  fail = false;
  annotation(texts[0])!.querySelector("button")!.dispatchEvent(new window.Event("click"));
  await settle();
  assert.equal(annotation(texts[0])?.textContent, "ENEN lỗi mạng");
  bubbles.stop();
});

test("disabled translation calls nothing, and changed text is processed again", async () => {
  const { doc, texts } = page([{ text: "một", timestamp: 2_000 }]);
  let enabled = false;
  const { value, calls } = api();
  const bubbles = installBubbles(value, { enabled: () => enabled, activeSince: () => 1_000 }, doc);
  await settle();
  assert.equal(calls.length, 0);
  enabled = true;
  bubbles.scan();
  await settle();
  texts[0].textContent = "hai";
  bubbles.scan();
  await settle();
  assert.deepEqual(
    calls.map((call) => call.text),
    ["một", "hai"],
  );
  assert.equal(annotation(texts[0])?.textContent, "ENEN hai");
  bubbles.stop();
});
```

- [ ] **Step 3: Run to verify failure**

Run: `npm test`
Expected: FAIL, because `../client/bubble` cannot be found.

- [ ] **Step 4: Implement `client/bubble.ts`**

```ts
import { hasVietnamese } from "../shared/vietnamese";
import { reactProps, type Doc, type El, type Observer } from "./dom";

export type BubbleApi = {
  translate(text: string, cacheOnly: boolean): Promise<string | null>;
  original(text: string): Promise<string | null>;
};
export type BubbleOptions = { enabled(): boolean; activeSince(): number };

const TEXT = '[data-testid="user-message"] [data-message-text="true"]';
const MARK = "data-prompt-translate";
// Colors come from currentColor so the annotation follows every host theme.
const STYLE = `
[${MARK}] {display:flex;flex-direction:column;gap:2px;margin-top:6px;padding-top:6px;border-top:1px solid color-mix(in srgb,currentColor 18%,transparent);font-size:.92em;line-height:1.45;}
[${MARK}] .pt-label {font-size:10px;font-weight:600;letter-spacing:.06em;opacity:.55;}
[${MARK}] .pt-text {opacity:.78;white-space:pre-wrap;user-select:text;}
[${MARK}] .pt-loading {height:.9em;width:60%;border-radius:4px;background:currentColor;opacity:.12;animation:pt-pulse 1.2s ease-in-out infinite;}
[${MARK}] button {all:unset;cursor:pointer;opacity:.75;text-decoration:underline;}
@keyframes pt-pulse {50% {opacity:.05;}}
`;

export function installBubbles(api: BubbleApi, options: BubbleOptions, doc: Doc, Observer?: Observer) {
  let seen = new WeakMap<El, string>();
  let stopped = false;
  let scheduled = false;
  const style = doc.createElement("style");
  style.setAttribute(`${MARK}-style`, "");
  style.textContent = STYLE;
  doc.head.append(style);

  const element = (tag: string, className: string, text?: string) => {
    const node = doc.createElement(tag);
    node.setAttribute("class", className);
    if (text !== undefined) node.textContent = text;
    return node;
  };
  const existing = (textNode: El) => {
    const next = textNode.nextElementSibling;
    return next?.hasAttribute(MARK) ? next : null;
  };
  const box = (textNode: El, ...children: El[]) => {
    let node = existing(textNode);
    if (!node) {
      node = doc.createElement("div");
      node.setAttribute(MARK, "");
      const color = doc.defaultView?.getComputedStyle?.(textNode).color;
      if (color) node.style.cssText = `color:${color}`;
      textNode.after(node);
    }
    node.textContent = "";
    node.append(...children);
  };
  const show = (textNode: El, label: string, text: string) =>
    box(textNode, element("span", "pt-label", label), element("span", "pt-text", text));
  const drop = (textNode: El) => existing(textNode)?.remove();
  const current = (textNode: El, text: string) =>
    !stopped && textNode.isConnected && seen.get(textNode) === text;

  async function process(textNode: El, text: string): Promise<void> {
    let cacheOnly = true;
    try {
      const original = await api.original(text);
      if (!current(textNode, text)) return;
      if (original) return show(textNode, "VI gốc", original);
      if (!hasVietnamese(text)) return drop(textNode);
      const props = reactProps(
        textNode,
        (candidate) => typeof candidate.message === "string" && typeof candidate.timestamp === "number",
      );
      // Prompts older than activation, or without a readable timestamp, never spend quota.
      cacheOnly = !props || (props.timestamp as number) < options.activeSince();
      if (!cacheOnly) box(textNode, element("span", "pt-label", "EN"), element("div", "pt-loading"));
      const translation = await api.translate(text, cacheOnly);
      if (!current(textNode, text)) return;
      if (translation) show(textNode, "EN", translation);
      else drop(textNode);
    } catch {
      if (!current(textNode, text)) return;
      if (cacheOnly) return drop(textNode);
      const retry = element("button", "pt-retry", "Thử lại");
      retry.addEventListener("click", () => void process(textNode, text));
      box(textNode, element("span", "pt-text", "Không dịch được · "), retry);
    }
  }

  function clearAll() {
    for (const node of Array.from(doc.querySelectorAll(`[${MARK}]`))) node.remove();
    seen = new WeakMap();
  }

  function scan() {
    scheduled = false;
    if (stopped) return;
    if (!options.enabled()) return clearAll();
    for (const textNode of Array.from(doc.querySelectorAll(TEXT))) {
      const text = (textNode.textContent ?? "").trim();
      if (!text || seen.get(textNode) === text) continue;
      seen.set(textNode, text);
      drop(textNode);
      void process(textNode, text);
    }
  }

  const observer = Observer
    ? new Observer(() => {
        if (scheduled || stopped) return;
        scheduled = true;
        setTimeout(scan, 50);
      })
    : undefined;
  observer?.observe(doc.body, { childList: true, subtree: true, characterData: true });
  scan();
  return {
    scan,
    stop() {
      stopped = true;
      observer?.disconnect();
      clearAll();
      style.remove();
    },
  };
}
```

- [ ] **Step 5: Run tests, typecheck, and lint**

Run: `npm test && npm run typecheck && npm run lint`
Expected: all pass. If linkedom rejects the `style` element's `textContent` or `after`, fix the
adapter to use an API that works in both linkedom and Electron. Do not weaken the assertions.

- [ ] **Step 6: Checkpoint.** Stop for review. Commit only if the user asks.

---

### Task 7: Composer adapter

**Files:**
- Create: `client/composer.ts`
- Test: `tests/composer.test.ts`

**Interfaces:**
- Consumes: `Doc`, `El`, `Key` from `client/dom.ts` (Task 6).
- Produces: `type ComposerApi = { enhance(text: string): Promise<string> }`,
  `installComposer(api: ComposerApi, options: { enabled(): boolean }, doc: Doc): { onKeydown(event: Key): void; stop(): void }`,
  `fillField(field: El, text: string, doc: Doc): boolean`.

- [ ] **Step 1: Write the failing tests**

`tests/composer.test.ts`:

```ts
import { test } from "node:test";
import assert from "node:assert/strict";
import { parseHTML } from "linkedom";
import { installComposer } from "../client/composer";
import type { Doc, El, Key } from "../client/dom";

function page(value: string) {
  const { document } = parseHTML(
    '<html><head></head><body><div data-testid="message-input-root"><textarea data-composer-input></textarea></div><p id="other"></p></body></html>',
  );
  const field = document.querySelector("textarea") as unknown as El;
  field.value = value;
  return { doc: document as unknown as Doc, field, other: document.getElementById("other") };
}
function key(target: unknown, init: Partial<Key> = {}) {
  const calls = { prevented: 0, stopped: 0 };
  const event: Key = {
    key: "Enter",
    metaKey: true,
    ctrlKey: false,
    shiftKey: false,
    isComposing: false,
    target,
    preventDefault: () => void calls.prevented++,
    stopImmediatePropagation: () => void calls.stopped++,
    ...init,
  };
  return { event, calls };
}
function deferred() {
  let resolve!: (value: string) => void;
  let reject!: (error: Error) => void;
  const promise = new Promise<string>((ok, fail) => {
    resolve = ok;
    reject = fail;
  });
  return { promise, resolve, reject };
}
const badge = (doc: Doc) =>
  (doc as unknown as { querySelector(s: string): El | null }).querySelector("[data-prompt-translate-badge]");
const settle = () => new Promise((resolve) => setTimeout(resolve, 0));

test("Cmd+Enter replaces the draft with the enhanced prompt and blocks the host send", async () => {
  const { doc, field } = page("sửa lỗi");
  const pending = deferred();
  const requests: string[] = [];
  const composer = installComposer(
    { enhance: (text) => (requests.push(text), pending.promise) },
    { enabled: () => true },
    doc,
  );
  const { event, calls } = key(field);
  composer.onKeydown(event);
  composer.onKeydown(key(field).event);
  assert.deepEqual(calls, { prevented: 1, stopped: 1 });
  assert.deepEqual(requests, ["sửa lỗi"]);
  assert.equal(badge(doc)?.textContent, "Enhancing… (Esc để hủy)");
  pending.resolve("Fix the bug.");
  await settle();
  assert.equal(field.value, "Fix the bug.");
  assert.equal(badge(doc), null);
  composer.stop();
});

test("other keys, other targets, IME, and the disabled setting are untouched", () => {
  const { doc, field, other } = page("x");
  let enabled = true;
  const requests: string[] = [];
  const composer = installComposer(
    { enhance: async (text) => (requests.push(text), text) },
    { enabled: () => enabled },
    doc,
  );
  for (const init of [
    { metaKey: false },
    { shiftKey: true },
    { isComposing: true },
    { keyCode: 229 },
    { key: "a" },
  ]) {
    const { event, calls } = key(field, init);
    composer.onKeydown(event);
    assert.deepEqual(calls, { prevented: 0, stopped: 0 });
  }
  composer.onKeydown(key(other).event);
  enabled = false;
  composer.onKeydown(key(field).event);
  assert.equal(requests.length, 0);
  composer.stop();
});

test("Ctrl+Enter works too, and an edited draft is never overwritten", async () => {
  const { doc, field } = page("bản nháp");
  const pending = deferred();
  const composer = installComposer({ enhance: () => pending.promise }, { enabled: () => true }, doc);
  composer.onKeydown(key(field, { metaKey: false, ctrlKey: true }).event);
  field.value = "bản nháp đã sửa";
  pending.resolve("Draft.");
  await settle();
  assert.equal(field.value, "bản nháp đã sửa");
  composer.stop();
});

test("Escape cancels, and errors keep the draft with a message", async () => {
  const { doc, field } = page("hủy");
  const first = deferred();
  const second = deferred();
  const queue = [first.promise, second.promise];
  const composer = installComposer({ enhance: () => queue.shift()! }, { enabled: () => true }, doc);
  composer.onKeydown(key(field).event);
  const escape = key(field, { key: "Escape", metaKey: false });
  composer.onKeydown(escape.event);
  assert.deepEqual(escape.calls, { prevented: 1, stopped: 1 });
  first.resolve("Cancelled.");
  await settle();
  assert.equal(field.value, "hủy");
  assert.equal(badge(doc), null);
  composer.onKeydown(key(field).event);
  second.reject(new Error("timeout"));
  await settle();
  assert.equal(field.value, "hủy");
  assert.equal(badge(doc)?.textContent, "Enhance lỗi: timeout");
  composer.stop();
  assert.equal(badge(doc), null);
});
```

- [ ] **Step 2: Run to verify failure**

Run: `npm test`
Expected: FAIL, because `../client/composer` cannot be found.

- [ ] **Step 3: Implement `client/composer.ts`**

```ts
import type { Doc, El, Key } from "./dom";

export type ComposerApi = { enhance(text: string): Promise<string> };

// The host marks the field with dataSet={{composerInput:""}}; the root testID covers older hosts.
const FIELD = '[data-composer-input], [data-testid="message-input-root"] textarea';
const BADGE = "data-prompt-translate-badge";

// The field is uncontrolled, but React tracks the last value it saw: write through the prototype
// setter and replay the input event so the host adopts the new text.
export function fillField(field: El, text: string, doc: Doc): boolean {
  const view = doc.defaultView;
  if (!view) return false;
  const setter = Object.getOwnPropertyDescriptor(Object.getPrototypeOf(field), "value")?.set;
  if (setter) setter.call(field, text);
  else field.value = text;
  field.dispatchEvent(new view.Event("input", { bubbles: true }));
  field.focus?.();
  field.setSelectionRange?.(text.length, text.length);
  return field.value === text;
}

export function installComposer(api: ComposerApi, options: { enabled(): boolean }, doc: Doc) {
  let pending: { field: El; draft: string } | null = null;
  let badge: El | null = null;
  let hideTimer: ReturnType<typeof setTimeout> | undefined;

  function say(text: string | null, ms?: number) {
    clearTimeout(hideTimer);
    if (text === null) {
      badge?.remove();
      badge = null;
      return;
    }
    if (!badge) {
      badge = doc.createElement("div");
      badge.setAttribute(BADGE, "");
      // Fixed position keeps the host layout untouched.
      badge.style.cssText =
        "position:fixed;right:16px;bottom:16px;z-index:2147483647;padding:6px 10px;border-radius:8px;font:12px/1.4 system-ui,sans-serif;background:rgba(24,24,27,.9);color:#fafafa;pointer-events:none;";
      doc.body.append(badge);
    }
    badge.textContent = text;
    if (ms) hideTimer = setTimeout(() => say(null), ms);
  }

  function onKeydown(event: Key) {
    if (event.key === "Escape" && pending) {
      pending = null;
      say(null);
      event.preventDefault();
      event.stopImmediatePropagation();
      return;
    }
    if (event.key !== "Enter" || !(event.metaKey || event.ctrlKey) || event.shiftKey) return;
    if (event.isComposing || event.keyCode === 229) return;
    const field = (event.target as El | null)?.closest?.(FIELD) ?? null;
    if (!field || !options.enabled()) return;
    // Owned even when idle or empty, so the host never sends or queues on Cmd+Enter.
    event.preventDefault();
    event.stopImmediatePropagation();
    const draft = field.value ?? "";
    if (pending || !draft.trim()) return;
    const request = { field, draft };
    pending = request;
    say("Enhancing… (Esc để hủy)");
    api.enhance(draft).then(
      (prompt) => {
        if (pending !== request) return;
        pending = null;
        if (field.value !== draft) return say("Bản nháp đã đổi, bỏ qua kết quả enhance", 4000);
        if (fillField(field, prompt, doc)) say(null);
        else say("Composer không nhận text đã enhance", 4000);
      },
      (error: unknown) => {
        if (pending !== request) return;
        pending = null;
        say(`Enhance lỗi: ${error instanceof Error ? error.message : String(error)}`, 4000);
      },
    );
  }

  doc.addEventListener("keydown", onKeydown, true);
  return {
    onKeydown,
    stop() {
      doc.removeEventListener("keydown", onKeydown, true);
      pending = null;
      say(null);
    },
  };
}
```

Note: the "edited draft" test expects the draft to survive, and the badge then shows
`Bản nháp đã đổi, bỏ qua kết quả enhance`.

- [ ] **Step 4: Run tests, typecheck, and lint**

Run: `npm test && npm run typecheck && npm run lint`
Expected: all pass.

- [ ] **Step 5: Checkpoint.** Stop for review. Commit only if the user asks.

---

### Task 8: Client entry, state, and settings screen

**Files:**
- Create: `client/state.ts`, `client/settings.tsx`, `index.client.tsx`
- Test: `tests/state.test.ts`

**Interfaces:**
- Consumes: `installBubbles` (Task 6), `installComposer` (Task 7), `desktopSupported`, `Doc`,
  `Observer` (Task 6), the RPC contracts and `translateSettings` (Task 1).
- Produces: `current: { values: TranslateSettings; activeSince: number; onChange?: () => void; apply(next: TranslateSettings): void }`.

- [ ] **Step 1: Write the failing test**

`tests/state.test.ts`:

```ts
import { test } from "node:test";
import assert from "node:assert/strict";
import { current } from "../client/state";

test("switching translation back on moves activeSince forward and notifies", async () => {
  let changes = 0;
  current.onChange = () => void changes++;
  current.apply({ ...current.values, translate: false });
  const before = current.activeSince;
  await new Promise((resolve) => setTimeout(resolve, 5));
  current.apply({ ...current.values, translate: true });
  assert.ok(current.activeSince > before);
  assert.equal(changes, 2);
  current.onChange = undefined;
});
```

- [ ] **Step 2: Run to verify failure**

Run: `npm test`
Expected: FAIL, because `../client/state` cannot be found.

- [ ] **Step 3: Implement `client/state.ts`**

```ts
import { translateSettings, type TranslateSettings } from "../shared/settings";

// Adapters read synchronously; the settings screen and the startup read keep this current.
export const current = {
  values: translateSettings.schema.parse({}) as TranslateSettings,
  activeSince: Date.now(),
  onChange: undefined as (() => void) | undefined,
  apply(next: TranslateSettings) {
    // Re-enabling must not translate prompts sent while translation was off.
    if (next.translate && !this.values.translate) this.activeSince = Date.now();
    this.values = next;
    this.onChange?.();
  },
};
```

- [ ] **Step 4: Implement `client/settings.tsx`**

```tsx
import { useEffect, useState } from "react";
import { useSettings, type PluginSurfaceProps } from "@getpaseo/plugin/client";
import {
  SettingsAction,
  SettingsCard,
  SettingsInput,
  SettingsSection,
  SettingsSelect,
  SettingsSwitch,
} from "@getpaseo/plugin/client/ui";
import { Text } from "react-native";
import { providerSchema, translateSettings, type TranslateSettings } from "../shared/settings";
import { current } from "./state";

export function TranslateSettingsScreen({ theme }: PluginSurfaceProps) {
  const settings = useSettings(translateSettings);
  const [models, setModels] = useState<{ translateModel?: string; enhanceModel?: string }>({});
  useEffect(() => {
    if (settings.status === "ready") current.apply(settings.values);
  }, [settings]);
  if (settings.status === "loading" || settings.status === "error") {
    return (
      <Text style={{ color: theme.colors.foregroundMuted, fontSize: 13 }}>
        {settings.status === "loading"
          ? "Loading prompt translate settings…"
          : `Prompt translate settings are unavailable: ${settings.error}`}
      </Text>
    );
  }
  if (settings.status === "invalid") {
    return (
      <SettingsSection title="Prompt translate">
        <SettingsCard>
          <SettingsAction
            label="Stored settings are invalid"
            hint={settings.error}
            error={settings.saveError}
            actionLabel="Reset to defaults"
            disabled={settings.saving}
            onPress={() => void settings.reset()}
          />
        </SettingsCard>
      </SettingsSection>
    );
  }
  const { values, revision } = settings;
  const save = (next: TranslateSettings) => settings.save(next, revision);
  const draft = {
    translateModel: models.translateModel?.trim() || values.translateModel,
    enhanceModel: models.enhanceModel?.trim() || values.enhanceModel,
  };
  const dirty =
    draft.translateModel !== values.translateModel || draft.enhanceModel !== values.enhanceModel;
  return (
    <>
      <SettingsSection title="Prompt translate">
        <SettingsCard>
          <SettingsSwitch
            label="Translate Vietnamese prompts"
            hint="Show an English translation under each new Vietnamese prompt. Desktop only."
            error={settings.saveError}
            value={values.translate}
            disabled={settings.saving}
            onValueChange={(translate) => void save({ ...values, translate })}
          />
          <SettingsSwitch
            label="Cmd/Ctrl+Enter enhances the draft"
            hint="Rewrites the composer draft as an English prompt for review. Replaces the keyboard Queue shortcut."
            value={values.enhanceShortcut}
            disabled={settings.saving}
            onValueChange={(enhanceShortcut) => void save({ ...values, enhanceShortcut })}
          />
        </SettingsCard>
      </SettingsSection>
      <SettingsSection title="Models">
        <SettingsCard>
          <SettingsSelect
            label="Provider"
            hint="Prompts are sent to this third-party service."
            value={values.provider}
            options={[
              { label: "Vercel AI Gateway", value: "vercel" },
              { label: "OpenRouter (needs OPENROUTER_API_KEY)", value: "openrouter" },
            ]}
            disabled={settings.saving}
            onValueChange={(provider) =>
              void save({ ...values, provider: providerSchema.parse(provider) })
            }
          />
          <SettingsInput
            label="Translate model"
            initialValue={values.translateModel}
            onChangeText={(translateModel) => setModels((m) => ({ ...m, translateModel }))}
          />
          <SettingsInput
            label="Enhance model"
            initialValue={values.enhanceModel}
            onChangeText={(enhanceModel) => setModels((m) => ({ ...m, enhanceModel }))}
          />
          <SettingsAction
            label="Save models"
            hint="Use model IDs exactly as the provider lists them."
            actionLabel="Save"
            disabled={settings.saving || !dirty}
            onPress={() =>
              void save({ ...values, ...draft }).then((saved) => {
                if (saved) setModels({});
              })
            }
          />
        </SettingsCard>
      </SettingsSection>
    </>
  );
}
```

- [ ] **Step 5: Implement `index.client.tsx`**

```tsx
import type { PluginClientContext } from "@getpaseo/plugin/client";
import { settingsRpc } from "@getpaseo/plugin";
import { Platform } from "react-native";
import { enhanceRpc, originalRpc, translateRpc } from "./shared/contracts";
import { translateSettings } from "./shared/settings";
import { installBubbles } from "./client/bubble";
import { installComposer } from "./client/composer";
import { desktopSupported, type Doc, type Observer } from "./client/dom";
import { TranslateSettingsScreen } from "./client/settings";
import { current } from "./client/state";

export default function contribute(client: PluginClientContext) {
  if (Platform.OS !== "web" || !desktopSupported()) return () => {};
  const scope = globalThis as unknown as { document: Doc; MutationObserver: Observer };
  void client
    .rpc(settingsRpc(translateSettings.id).read, {})
    .then((saved) => {
      if (saved.status === "ready") current.apply(translateSettings.schema.parse(saved.values));
    })
    .catch(() => undefined);
  const bubbles = installBubbles(
    {
      translate: async (text, cacheOnly) =>
        (await client.rpc(translateRpc, { text, cacheOnly })).translation,
      original: async (text) => (await client.rpc(originalRpc, { text })).original,
    },
    { enabled: () => current.values.translate, activeSince: () => current.activeSince },
    scope.document,
    scope.MutationObserver,
  );
  current.onChange = () => bubbles.scan();
  const composer = installComposer(
    { enhance: async (text) => (await client.rpc(enhanceRpc, { text })).prompt },
    { enabled: () => current.values.enhanceShortcut },
    scope.document,
  );
  const settings = client.addSettingsScreen({
    id: "translate",
    title: "Prompt translate",
    icon: "Languages",
    Component: TranslateSettingsScreen,
  });
  return () => {
    current.onChange = undefined;
    bubbles.stop();
    composer.stop();
    settings();
  };
}
```

- [ ] **Step 6: Run all gates**

Run: `npm run format && npm test && npm run typecheck && npm run lint`
Expected: all pass. If the `SettingsSelect` or `SettingsInput` prop names differ from the
reference, read their types in `node_modules/@getpaseo/plugin/dist/client/ui*.d.ts` and match
them exactly.

- [ ] **Step 7: Checkpoint.** Stop for review. Commit only if the user asks.

---

### Task 9: Documentation, install, and live verification

**Files:**
- Create: `plugins/prompt-translate/README.md`
- Modify: root `README.md` (catalog table near line 157)

- [ ] **Step 1: Write `plugins/prompt-translate/README.md`**

```markdown
# Prompt translate

Learn English prompting from your own Vietnamese prompts, on Paseo desktop.

- Each new Vietnamese prompt shows its English translation inside the user bubble, labeled `EN`.
- Cmd/Ctrl+Enter in the composer rewrites the draft as a clear English prompt and puts it back
  in the composer for review. Press Enter to send it. The sent bubble then shows your Vietnamese
  original, labeled `VI gốc`.

Both features switch on or off under Settings → Plugins → Prompt translate.

## Compatibility and limits

Paseo 0.8.x and 0.9.x desktop only, through a private DOM adapter. Paseo updates can break it;
it then shows nothing rather than failing. Browser and mobile clients receive no contribution.
While the shortcut is on, Cmd/Ctrl+Enter no longer queues a message from the keyboard; use the Queue button.
Prompts sent before translation was switched on are shown only from the cache and never sent to a model.

## Privacy and credentials

Prompt text is sent to the selected third-party provider. Keys stay on the daemon:

- Vercel AI Gateway: `agents.providers.vercel-gateway.env.OPENAI_API_KEY` (and optional
  `OPENAI_BASE_URL`) in the daemon's `config.json`.
- OpenRouter: `OPENROUTER_API_KEY` in the daemon environment.

The cache lives at `$PASEO_HOME/plugin-data/prompt-translate/cache.json`. Never add it to Git.

## Models

Defaults (benchmark on <date from Task 5>): translate `<model>`, enhance `<model>`.
Paste the measured p50/p90 rows from Task 5 here. Change models in settings.
`npm run benchmark` re-measures; it consumes quota.

## Install and verify

npm install
npm run format && npm run typecheck && npm run lint && npm test
paseo plugin install "$PWD" --id prompt-translate
paseo plugin ls prompt-translate --json
```

Fill the `Models` section with the real date, model IDs, and numbers from Task 5 before saving.
Wrap the install commands in a `sh` fence.

- [ ] **Step 2: Add the root catalog row**

Insert after the `next-prompt-actions` row in root `README.md`:

```markdown
| [prompt-translate](plugins/prompt-translate/README.md) | Show English translations under Vietnamese prompts, and enhance drafts into English prompts with Cmd/Ctrl+Enter, on desktop. |
```

- [ ] **Step 3: Install (needs user approval: this changes the running daemon's plugins)**

Run from `plugins/prompt-translate`:

```sh
paseo daemon status --json
paseo plugin install "$PWD" --id prompt-translate
paseo plugin ls prompt-translate --json
paseo plugin logs prompt-translate
```

Expected: `running` with no load error. If the plugin fails, read the logs and fix the cause. Do
not restart the daemon.

- [ ] **Step 4: Live verification on desktop (ask the user to perform or confirm each)**

1. Send a Vietnamese prompt, for example `sửa lỗi chính tả trong README`, to a test agent. An
   `EN` block appears under the text inside the bubble, with the pulse first and then the
   translation.
2. Send `fix typo` and confirm no block appears.
3. Scroll to an older Vietnamese prompt sent before install, and confirm no block and no new
   model call. `paseo plugin logs prompt-translate` shows nothing new.
4. Type a Vietnamese draft and press Cmd+Enter. The badge appears, the draft becomes English,
   and nothing is sent. Press Enter, and the sent bubble shows `VI gốc` with the draft.
5. Press Cmd+Enter while the agent is running. It enhances and does not queue.
6. Press Esc during an enhance, and confirm the draft stays unchanged.
7. Turn off both switches in settings. New prompts show no block, and Cmd+Enter behaves as the
   host default.
8. Confirm `~/.paseo/plugin-data/prompt-translate/cache.json` exists and contains no key.

Record which checks passed and which were not run. Do not claim unrun checks.

- [ ] **Step 5: Final gates**

Run: `cd plugins/prompt-translate && npm run format && npm test && npm run typecheck && npm run lint && git -C ../.. status --short`
Expected: all pass. Status lists only the new plugin, the root README, and the spec and plan docs.

- [ ] **Step 6: Checkpoint.** Report the results. Commit and push only if the user asks.
