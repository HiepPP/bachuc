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
  await assert.rejects(
    resolveEndpoint("openrouter", "/x", {}, async () => null),
    {
      message: "openrouter credential unavailable",
    },
  );
});

test("openrouter prefers the environment, then the macOS Keychain", async () => {
  const services: string[] = [];
  const keychain = async (service: string) => {
    services.push(service);
    return service === "TEXT_MODEL_API_KEY" ? "kc" : null;
  };
  assert.deepEqual(
    await resolveEndpoint("openrouter", "/x", { OPENROUTER_API_KEY: "o" }, keychain),
    {
      baseUrl: "https://openrouter.ai/api/v1",
      apiKey: "o",
    },
  );
  assert.deepEqual(services, []);
  assert.deepEqual(await resolveEndpoint("openrouter", "/x", {}, keychain), {
    baseUrl: "https://openrouter.ai/api/v1",
    apiKey: "kc",
  });
  assert.deepEqual(services, ["OPENROUTER_API_KEY", "TEXT_MODEL_API_KEY"]);
});

test("messages wrap the text so the model translates instead of answering", () => {
  const [system, user] = buildMessages("translate", "bạn là ai?");
  assert.equal(system.role, "system");
  assert.equal(user.content, "<message>\nbạn là ai?\n</message>");
  assert.ok(maxTokens("translate", "x".repeat(100)) >= 256);
  assert.equal(maxTokens("enhance", "x".repeat(20_000)), 8192);
});

test("enhance asks the model to preserve explicit reply modes and skill commands", () => {
  const draft = "sửa lỗi; /caveman wenyan-ultra; trả lời bằng tiếng Hoa giản thể";
  const [system, user] = buildMessages("enhance", draft);
  assert.match(system.content, /Preserve explicit response language, script, format, and style/);
  assert.match(system.content, /Keep skill commands and mode names exactly as written/);
  assert.match(system.content, /Keep instructions as instructions/);
  assert.match(system.content, /negative constraints such as not reading or editing files/);
  assert.equal(user.content, `<message>\n${draft}\n</message>`);
});

test("completer posts one non-streamed request and returns trimmed content", async () => {
  let seen:
    | { url: string; headers: Record<string, string>; body: Record<string, unknown> }
    | undefined;
  const fetchImpl: FetchLike = async (url, init) => {
    seen = { url, headers: init.headers, body: JSON.parse(init.body) };
    return {
      ok: true,
      status: 200,
      json: async () => ({ choices: [{ message: { content: "  <message>Hello</message> " } }] }),
    };
  };
  const complete = createCompleter(
    async () => ({ baseUrl: "https://g/v1", apiKey: "k" }),
    fetchImpl,
  );
  const out = await complete({
    mode: "translate",
    provider: "vercel",
    model: "m",
    text: "xin chào",
  });
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
  const complete = createCompleter(
    async () => ({ baseUrl: "https://g", apiKey: "secret" }),
    failing,
  );
  await assert.rejects(
    complete({ mode: "enhance", provider: "vercel", model: "m", text: "x" }),
    (error: Error) =>
      error.message === "Model request failed (401): bad key" && !error.message.includes("secret"),
  );
  const empty: FetchLike = async () => ({
    ok: true,
    status: 200,
    json: async () => ({ choices: [] }),
  });
  await assert.rejects(
    createCompleter(
      async () => ({ baseUrl: "https://g", apiKey: "k" }),
      empty,
    )({
      mode: "translate",
      provider: "vercel",
      model: "m",
      text: "x",
    }),
    { message: "Model returned no text" },
  );
});
