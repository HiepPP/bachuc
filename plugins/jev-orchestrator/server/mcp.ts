import { readFileSync } from "node:fs";
import { homedir } from "node:os";
import path from "node:path";
import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { z } from "zod";
import { nativePrepareSchema } from "../shared/native";
import { submitSchema } from "../shared/contracts";

function currentBridgeUrl() {
  try {
    const home = process.env.PASEO_HOME || path.join(homedir(), ".paseo");
    const { port } = JSON.parse(
      readFileSync(path.join(home, "plugin-data/jev-orchestrator/bridge.json"), "utf8"),
    );
    if (Number.isInteger(port) && port > 0) return `http://127.0.0.1:${port}/mcp`;
  } catch {}
}

const server = new McpServer({ name: "jev-orchestrator", version: "0.1.0" });
async function call(action: string, input?: unknown, id?: string) {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), 28000);
  try {
    const url = process.env.PASEO_ORCH_URL;
    const token = process.env.PASEO_ORCH_TOKEN;
    if (!url || !token || !/^http:\/\/127\.0\.0\.1:\d+\/mcp$/.test(url))
      throw new Error("Missing launch binding");
    const post = (target: string) =>
      fetch(target, {
        method: "POST",
        headers: { "Content-Type": "application/json", Authorization: `Bearer ${token}` },
        body: JSON.stringify({ action, input, id }),
        signal: controller.signal,
        redirect: "error",
      });
    let response;
    try {
      response = await post(url);
    } catch (error) {
      // A daemon restart can move the bridge; the persisted config still names the old port.
      const current = currentBridgeUrl();
      if (!current || current === url || controller.signal.aborted) throw error;
      response = await post(current);
    }
    const data = (await response.json()) as Record<string, unknown>;
    return {
      content: [{ type: "text" as const, text: JSON.stringify(data) }],
      structuredContent: data,
      ...(response.ok ? {} : { isError: true }),
    };
  } catch {
    return {
      content: [
        {
          type: "text" as const,
          text: "Orchestrator unavailable. After plugin reload, create a fresh agent. Never repeat a submission under a new task ID without checking existing jobs.",
        },
      ],
      isError: true,
    };
  } finally {
    clearTimeout(timer);
  }
}
server.registerTool(
  "delegate_task",
  {
    description:
      "Delegate authorized scoped tasks to real Paseo subagents. Read profiles first and pass allowedProfileIds. Explicit profileId is preserved. Submission runs children, external Jev evaluations and caller-specified check commands in your workspace. Declare exact files and shared resources; no automatic worktrees. Use only sanitized task data approved for Jev. Stable task IDs deduplicate retries. Returns immediately; inspect orchestrator_status. Never use from a managed child.",
    inputSchema: submitSchema,
    annotations: {
      readOnlyHint: false,
      destructiveHint: true,
      idempotentHint: true,
      openWorldHint: true,
    },
  },
  (input) => call("submit", input),
);
server.registerTool(
  "orchestrator_status",
  {
    description:
      "Read your scoped delegation jobs, children, checks, and measured routing history. Child completion without checks is unverified.",
    inputSchema: z.strictObject({}),
    annotations: { readOnlyHint: true, idempotentHint: true },
  },
  () => call("status"),
);
server.registerTool(
  "cancel_delegation",
  {
    description:
      "Cancel your job and archive only its known managed children. Resolve interrupted/permission jobs explicitly before resubmitting.",
    inputSchema: z.strictObject({ id: z.string() }),
    annotations: { readOnlyHint: false, destructiveHint: true, idempotentHint: true },
  },
  ({ id }) => call("cancel", undefined, id),
);
server.registerTool(
  "prepare_native_delegate",
  {
    description:
      "Policy: when already running Astra/low, do small bounded changes with clear acceptance and local checks yourself unless independent parallel work materially helps; do not call preflight merely because a spec exists. REQUIRED before Codex native v2 collaboration.spawn_agent through Paseo. Send the complete authorized task (including acceptance) in plaintext for one Jev model/effort evaluation. Three one-use slots per fresh Paseo session, forkTurns none only. Use a stable requestId; never retry failures with a new ID. If action is self, execute the submitted task yourself; do not spawn. Only for action delegate use returned taskName as native task_name, sourceRole as agent_type, fork_turns none and returned message unchanged. Do not set model/effort. A ticket is not a child: call native spawn exactly once. Do not use this tool for Paseo-managed children or Claude.",
    inputSchema: nativePrepareSchema,
    annotations: {
      readOnlyHint: false,
      destructiveHint: false,
      idempotentHint: true,
      openWorldHint: true,
    },
  },
  (input) => call("native_prepare", input),
);
server.registerTool(
  "native_delegation_status",
  {
    description:
      "Read this parent session's Jev ticket states, choices and evaluator usage. Consumed means spawn permission issued, not child success. Do not respawn a consumed ticket.",
    inputSchema: z.strictObject({}),
    annotations: { readOnlyHint: true, idempotentHint: true },
  },
  () => call("native_status", {}),
);
await server.connect(new StdioServerTransport());
