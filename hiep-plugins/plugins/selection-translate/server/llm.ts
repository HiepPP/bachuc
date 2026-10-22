// Copied from hiep-plugins/plugins/prompt-translate/server/llm.ts without its `mode` parameter
// (ADR-0062). A fix to the LLM call must be made in both plugins.
import type { Provider } from "../shared/settings";
import type { Endpoint } from "./credentials";
import { buildMessages, maxTokens, TIMEOUT_MS } from "./prompts";

export type ChatRequest = { provider: Provider; model: string; text: string };
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
  return async ({ provider, model, text }) => {
    const { baseUrl, apiKey } = await endpoint(provider);
    // AbortSignal.timeout is untyped here: react-native's globals shadow the DOM AbortSignal type.
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), TIMEOUT_MS);
    let body: ChatBody | null;
    try {
      const init = {
        method: "POST",
        headers: { authorization: `Bearer ${apiKey}`, "content-type": "application/json" },
        body: JSON.stringify({
          model,
          messages: buildMessages(text),
          temperature: 0,
          max_tokens: maxTokens(text),
          stream: false,
        }),
        signal: controller.signal,
      };
      const url = `${baseUrl}/chat/completions`;
      // A pooled keep-alive socket can be closed by the provider between prompts; undici then
      // rejects with "fetch failed" before any response, so one retry on a fresh socket is safe.
      const response = await fetchImpl(url, init).catch((error: unknown) => {
        if (controller.signal.aborted) throw error;
        return fetchImpl(url, init);
      });
      body = (await response.json().catch(() => null)) as ChatBody | null;
      if (!response.ok) {
        const detail = body?.error?.message;
        throw new Error(
          `Model request failed (${response.status})${detail ? `: ${String(detail).slice(0, 200)}` : ""}`,
        );
      }
    } catch (error) {
      if (controller.signal.aborted)
        throw new Error(`Model request timed out after ${TIMEOUT_MS / 1000} s`);
      throw withCause(error);
    } finally {
      clearTimeout(timer);
    }
    const content = body?.choices?.[0]?.message?.content;
    const output = typeof content === "string" ? unwrap(content) : "";
    if (!output) throw new Error("Model returned no text");
    return output;
  };
}

// The plugin RPC bridge forwards only error.message, so fold undici's cause code into it.
function withCause(error: unknown): unknown {
  if (!(error instanceof Error) || !(error.cause instanceof Error)) return error;
  const cause = error.cause as Error & { code?: unknown };
  return new Error(
    `${error.message}: ${typeof cause.code === "string" ? cause.code : cause.message}`,
  );
}

function unwrap(text: string): string {
  return text
    .trim()
    .replace(/^<message>\s*/, "")
    .replace(/\s*<\/message>$/, "")
    .trim();
}
