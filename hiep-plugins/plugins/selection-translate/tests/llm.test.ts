import { test } from "node:test";
import assert from "node:assert/strict";
import { resolveEndpoint } from "../server/credentials";
import { buildMessages, maxTokens, TIMEOUT_MS } from "../server/prompts";
import { createCompleter, type FetchLike } from "../server/llm";

test("openrouter prefers the environment, then the macOS Keychain", async () => {
  const keychain = async (service: string) => (service === "TEXT_MODEL_API_KEY" ? "kc" : null);
  assert.deepEqual(
    await resolveEndpoint("openrouter", "/x", { OPENROUTER_API_KEY: "o" }, keychain),
    { baseUrl: "https://openrouter.ai/api/v1", apiKey: "o" },
  );
  assert.deepEqual(await resolveEndpoint("openrouter", "/x", {}, keychain), {
    baseUrl: "https://openrouter.ai/api/v1",
    apiKey: "kc",
  });
});

test("missing credentials name the provider", async () => {
  await assert.rejects(resolveEndpoint("vercel", "/nonexistent/config.json"), {
    message: "vercel credential unavailable",
  });
  await assert.rejects(
    resolveEndpoint("openrouter", "/x", {}, async () => null),
    { message: "openrouter credential unavailable" },
  );
});

test("messages wrap the text and budget two tokens per character", () => {
  const [system, user] = buildMessages("Open the file.");
  assert.equal(system.role, "system");
  assert.match(system.content, /Vietnamese/);
  assert.equal(user.content, "<message>\nOpen the file.\n</message>");
  assert.equal(maxTokens("x".repeat(100)), 456);
  assert.equal(maxTokens("x".repeat(4000)), 8192);
  assert.equal(TIMEOUT_MS, 15_000);
});

test("completer posts one non-streamed request and unwraps the message", async () => {
  let seen:
    | { url: string; headers: Record<string, string>; body: Record<string, unknown> }
    | undefined;
  const fetchImpl: FetchLike = async (url, init) => {
    seen = { url, headers: init.headers, body: JSON.parse(init.body) };
    return {
      ok: true,
      status: 200,
      json: async () => ({ choices: [{ message: { content: " <message>Xin chào</message> " } }] }),
    };
  };
  const complete = createCompleter(
    async () => ({ baseUrl: "https://g/v1", apiKey: "k" }),
    fetchImpl,
  );
  assert.equal(await complete({ provider: "vercel", model: "m", text: "Hello" }), "Xin chào");
  assert.equal(seen?.url, "https://g/v1/chat/completions");
  assert.equal(seen?.headers.authorization, "Bearer k");
  assert.equal(seen?.body.model, "m");
  assert.equal(seen?.body.temperature, 0);
  assert.equal(seen?.body.stream, false);
});

test("completer errors carry status and detail, never the key", async () => {
  const failing: FetchLike = async () => ({
    ok: false,
    status: 401,
    json: async () => ({ error: { message: "bad key" } }),
  });
  const request = { provider: "vercel", model: "m", text: "x" } as const;
  await assert.rejects(
    createCompleter(async () => ({ baseUrl: "https://g", apiKey: "secret" }), failing)(request),
    (error: Error) =>
      error.message === "Model request failed (401): bad key" && !error.message.includes("secret"),
  );
  const empty: FetchLike = async () => ({
    ok: true,
    status: 200,
    json: async () => ({ choices: [] }),
  });
  await assert.rejects(
    createCompleter(async () => ({ baseUrl: "https://g", apiKey: "k" }), empty)(request),
    { message: "Model returned no text" },
  );
});

test("completer retries one network failure and reports its cause", async () => {
  const dropped = () =>
    new TypeError("fetch failed", {
      cause: Object.assign(new Error("reset"), { code: "ECONNRESET" }),
    });
  let calls = 0;
  const flaky: FetchLike = async () => {
    calls += 1;
    if (calls === 1) throw dropped();
    return {
      ok: true,
      status: 200,
      json: async () => ({ choices: [{ message: { content: "Hi" } }] }),
    };
  };
  const endpoint = async () => ({ baseUrl: "https://g", apiKey: "k" });
  const request = { provider: "vercel", model: "m", text: "x" } as const;
  assert.equal(await createCompleter(endpoint, flaky)(request), "Hi");
  assert.equal(calls, 2);
  calls = 0;
  const down: FetchLike = async () => {
    calls += 1;
    throw dropped();
  };
  await assert.rejects(createCompleter(endpoint, down)(request), {
    message: "fetch failed: ECONNRESET",
  });
  assert.equal(calls, 2);
});
