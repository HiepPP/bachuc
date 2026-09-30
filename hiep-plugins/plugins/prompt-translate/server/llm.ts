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
    // AbortSignal.timeout is untyped here: react-native's globals shadow the DOM AbortSignal type.
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), TIMEOUT_MS[mode]);
    let body: ChatBody | null;
    try {
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
        signal: controller.signal,
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
        throw new Error(`Model request timed out after ${TIMEOUT_MS[mode] / 1000} s`);
      throw error;
    } finally {
      clearTimeout(timer);
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
