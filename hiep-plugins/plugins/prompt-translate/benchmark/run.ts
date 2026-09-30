// Manual benchmark: consumes API quota.
// Usage: npm run benchmark -- --provider=vercel --runs=5 [--delay=ms] [--models=a,b]
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
// Free-tier gateways rate-limit bursts; a pause keeps 429s from masking latency.
const delayMs = Number(args.get("delay") ?? 0);
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
const samples = JSON.parse(await readFile(new URL("./samples.json", import.meta.url), "utf8")) as {
  id: string;
  text: string;
}[];

type Result = { ttftMs?: number; totalMs?: number; output?: string; error?: string };
type Row = Result & { model: string; mode: Mode; sample: string; run: number };

async function call(model: string, mode: Mode, text: string): Promise<Result> {
  const start = performance.now();
  let ttftMs: number | undefined;
  let output = "";
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), 60_000);
  try {
    const response = await fetch(`${endpoint.baseUrl}/chat/completions`, {
      method: "POST",
      headers,
      signal: controller.signal,
      body: JSON.stringify({
        model,
        messages: buildMessages(mode, text),
        temperature: 0,
        max_tokens: maxTokens(mode, text),
        stream: true,
      }),
    });
    if (!response.ok || !response.body) {
      const body = (await response.json().catch(() => null)) as {
        error?: { message?: unknown };
      } | null;
      const detail = body?.error?.message ? `: ${String(body.error.message).slice(0, 120)}` : "";
      return { error: `HTTP ${response.status}${detail}` };
    }
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
    return output
      ? { ttftMs, totalMs: performance.now() - start, output }
      : { error: "empty output" };
  } catch (error) {
    return { error: error instanceof Error ? error.message : String(error) };
  } finally {
    clearTimeout(timer);
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
        if (delayMs && rows.length) await new Promise((resolve) => setTimeout(resolve, delayMs));
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

const out = path.join(
  import.meta.dirname,
  "..",
  "artifacts",
  `benchmark-${new Date().toISOString().replace(/[:.]/g, "-")}.json`,
);
await mkdir(path.dirname(out), { recursive: true });
await writeFile(out, JSON.stringify({ provider, runs, delayMs, rows }, null, 2));
console.log(`\nraw outputs: ${out}`);
