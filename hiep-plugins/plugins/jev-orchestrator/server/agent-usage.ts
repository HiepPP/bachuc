import { createReadStream } from "node:fs";
import { readdir } from "node:fs/promises";
import { createInterface } from "node:readline";
import { homedir } from "node:os";
import path from "node:path";
import { missingUsage, tokenCount, type TokenUsage } from "./usage";

type Snapshot = {
  provider: string;
  cwd: string;
  status?: string;
  runtimeInfo?: { sessionId?: string | null } | null;
  persistence?: { sessionId?: string; nativeHandle?: string } | null;
};
export function parseCodexUsage(rows: Record<string, any>[]): TokenUsage {
  let total: Record<string, unknown> | undefined;
  let previous = -1,
    regressed = false,
    terminal = false;
  const canonical = rows.filter((row) => row.type === "token_usage_record");
  const requests = new Set(canonical.map((row) => row.payload?.response_id).filter(Boolean));
  for (const row of rows) {
    const payload = row.payload;
    if (row.type === "event_msg") {
      if (payload?.type === "task_started") terminal = false;
      if (payload?.type === "task_complete") terminal = true;
    }
    if (canonical.length) {
      if (row.type !== "token_usage_record" || !payload?.thread_token_usage) continue;
      total = payload.thread_token_usage;
    } else {
      if (
        row.type !== "event_msg" ||
        payload?.type !== "token_count" ||
        !payload.info?.total_token_usage
      )
        continue;
      total = payload.info.total_token_usage;
    }
    const count = tokenCount(total!.total_tokens);
    if (count !== null) {
      if (count < previous) regressed = true;
      previous = count;
    }
  }
  if (!total) return missingUsage("codex-native-cumulative", "No cumulative token_count record.");
  const inputTokens = tokenCount(total.input_tokens),
    outputTokens = tokenCount(total.output_tokens);
  const totalTokens = tokenCount(total.total_tokens);
  const complete =
    terminal &&
    !regressed &&
    inputTokens !== null &&
    outputTokens !== null &&
    totalTokens === inputTokens + outputTokens;
  return {
    source: "codex-native-cumulative",
    complete,
    inputTokens,
    outputTokens,
    totalTokens,
    cachedInputTokens: tokenCount(total.cached_input_tokens),
    cacheWriteTokens: tokenCount(total.cache_write_input_tokens),
    reasoningTokens: tokenCount(total.reasoning_output_tokens),
    requests: canonical.length ? requests.size : null,
    notes: [
      "Final native thread total is cumulative; repeated snapshots are not summed.",
      ...(!terminal ? ["No final task_complete record; session may still be running."] : []),
      ...(regressed ? ["Cumulative counter regressed; full coverage cannot be established."] : []),
    ],
  };
}
export function parseClaudeUsage(rows: Record<string, any>[], idle: boolean): TokenUsage {
  const requests = new Map<string, Record<string, any>>();
  const messageKeys = new Map<string, string>();
  let malformed = false;
  let terminal = false;
  for (const row of rows) {
    if (row.type !== "assistant") continue;
    terminal = ["end_turn", "max_tokens", "stop_sequence", "refusal"].includes(
      row.message?.stop_reason,
    );
    const messageId = row.message?.id;
    const key = (messageId && messageKeys.get(messageId)) || row.requestId || messageId;
    if (!key || !row.message?.usage) {
      malformed = true;
      continue;
    }
    if (messageId) messageKeys.set(messageId, key);
    const prior = requests.get(key);
    const usage = row.message.usage;
    // Streaming snapshots of one request can repeat; keep the greatest output count.
    if (
      !prior ||
      (tokenCount(usage.output_tokens) ?? -1) >= (tokenCount(prior.output_tokens) ?? -1)
    )
      requests.set(key, usage);
  }
  if (!requests.size)
    return missingUsage("claude-native-requests", "No keyed assistant usage records.");
  const sums = {
    inputTokens: 0,
    cachedInputTokens: 0,
    cacheWriteTokens: 0,
    outputTokens: 0,
    reasoningTokens: 0,
  };
  let reasoningKnown = true;
  for (const usage of requests.values()) {
    const input = tokenCount(usage.input_tokens),
      cached = tokenCount(usage.cache_read_input_tokens),
      write = tokenCount(usage.cache_creation_input_tokens),
      output = tokenCount(usage.output_tokens);
    if (input === null || cached === null || write === null || output === null) {
      malformed = true;
      continue;
    }
    sums.inputTokens += input + cached + write;
    sums.cachedInputTokens += cached;
    sums.cacheWriteTokens += write;
    sums.outputTokens += output;
    const reasoning = tokenCount(
      usage.output_tokens_details?.thinking_tokens ??
        usage.output_tokens_details?.reasoning_tokens ??
        usage.output_tokens_details?.reasoning,
    );
    if (reasoning === null) reasoningKnown = false;
    else sums.reasoningTokens += reasoning;
  }
  return {
    source: "claude-native-requests",
    complete: idle && terminal && !malformed,
    ...sums,
    ...(malformed
      ? { inputTokens: null, cachedInputTokens: null, cacheWriteTokens: null, outputTokens: null }
      : {}),
    reasoningTokens: reasoningKnown ? sums.reasoningTokens : null,
    totalTokens: malformed ? null : sums.inputTokens + sums.outputTokens,
    requests: requests.size,
    notes: [
      "Deduplicated requestId/message.id; cache prefixes count once per distinct API request.",
      ...(!idle ? ["Agent is not idle/closed; workflow still running."] : []),
      ...(!terminal
        ? ["Final assistant usage is not persisted yet; terminal stop_reason required."]
        : []),
      ...(malformed ? ["Unkeyed or incomplete usage record; totals withheld."] : []),
    ],
  };
}
async function readUsageRows(file: string) {
  const rows: Record<string, any>[] = [];
  const lines = createInterface({ input: createReadStream(file), crlfDelay: Infinity });
  for await (const line of lines) {
    if (!line.trim()) continue;
    const row = JSON.parse(line);
    // Keep only accounting and identity; never retain prompt/content in the ledger.
    if (
      row.type === "event_msg" &&
      ["token_count", "task_started", "task_complete"].includes(row.payload?.type)
    )
      rows.push({
        type: row.type,
        payload:
          row.payload?.type === "token_count"
            ? { type: "token_count", info: row.payload.info }
            : { type: row.payload.type },
      });
    if (row.type === "token_usage_record")
      rows.push({
        type: row.type,
        payload: {
          response_id: row.payload?.response_id,
          thread_token_usage: row.payload?.thread_token_usage,
        },
      });
    if (row.type === "session_meta")
      rows.push({ type: row.type, payload: { id: row.payload?.id, cwd: row.payload?.cwd } });
    if (row.type === "assistant")
      rows.push({
        type: row.type,
        sessionId: row.sessionId,
        uuid: row.uuid,
        requestId: row.requestId,
        message: {
          id: row.message?.id,
          usage: row.message?.usage,
          stop_reason: row.message?.stop_reason,
        },
      });
  }
  return rows;
}
async function findCodexLog(home: string, sessionId: string) {
  for (const folder of ["sessions", "archived_sessions"]) {
    const root = path.join(home, folder);
    let names: string[];
    try {
      names = await readdir(root, { recursive: true });
    } catch (e) {
      if ((e as NodeJS.ErrnoException).code === "ENOENT") continue;
      throw e;
    }
    const hits = names.filter((name) => name.endsWith(`-${sessionId}.jsonl`));
    if (hits.length > 1) throw Error("Ambiguous native session logs.");
    if (hits[0]) return path.join(root, hits[0]);
  }
  throw Error("Native session log unavailable.");
}
export async function readAgentUsage(
  snapshot: Snapshot | null,
  roots: { codexHome?: string; claudeHome?: string } = {},
): Promise<TokenUsage> {
  if (!snapshot) return missingUsage("native-session", "Agent snapshot unavailable.");
  const sessionId =
    snapshot.runtimeInfo?.sessionId ??
    snapshot.persistence?.sessionId ??
    snapshot.persistence?.nativeHandle;
  if (!sessionId || !/^[a-zA-Z0-9-]{10,100}$/.test(sessionId))
    return missingUsage("native-session", "Native session ID unavailable.");
  try {
    if (snapshot.provider === "codex") {
      const file = await findCodexLog(
        roots.codexHome ?? process.env.CODEX_HOME ?? path.join(homedir(), ".codex"),
        sessionId,
      );
      const rows = await readUsageRows(file);
      const meta = rows.find((row) => row.type === "session_meta")?.payload;
      if (meta?.id !== sessionId || path.resolve(meta?.cwd ?? "") !== path.resolve(snapshot.cwd))
        throw Error("Session identity mismatch.");
      return parseCodexUsage(rows);
    }
    if (snapshot.provider === "claude") {
      const home =
        roots.claudeHome ?? process.env.CLAUDE_CONFIG_DIR ?? path.join(homedir(), ".claude");
      const project = snapshot.cwd.replace(/[^a-zA-Z0-9]/g, "-");
      const rows = await readUsageRows(path.join(home, "projects", project, sessionId + ".jsonl"));
      if (rows.some((row) => row.type === "assistant" && row.sessionId !== sessionId))
        throw Error("Session identity mismatch.");
      return parseClaudeUsage(rows, snapshot.status === "idle" || snapshot.status === "closed");
    }
    return missingUsage(
      snapshot.provider + "-native-session",
      "Provider accounting adapter unavailable.",
    );
  } catch {
    return missingUsage(
      snapshot.provider + "-native-session",
      "Native accounting missing, malformed, or mismatched; no lastUsage fallback.",
    );
  }
}
