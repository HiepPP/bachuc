import { readFileSync } from "node:fs";
import { homedir } from "node:os";
import path from "node:path";
import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { z } from "zod";

function currentBridgeUrl() {
  try {
    const home = process.env.PASEO_HOME || path.join(homedir(), ".paseo");
    const { port } = JSON.parse(
      readFileSync(path.join(home, "plugin-data/board/bridge.json"), "utf8"),
    );
    if (Number.isInteger(port) && port > 0) return `http://127.0.0.1:${port}/mcp`;
  } catch {}
}

const server = new McpServer({ name: "board", version: "0.1.0" });
server.registerTool(
  "board_remove",
  {
    description:
      "Remove your own Paseo thread, or one of your subagent threads, from Board. Omit agentId for your own thread. A finished thread is removed now with its finished subagents; a running thread is removed when its current turn completes, while failed or cancelled turns stay visible. Only hides the card: never archives or deletes the agent, and a new turn shows it again.",
    inputSchema: { agentId: z.string().min(1).max(100).optional() },
    annotations: {
      readOnlyHint: false,
      destructiveHint: false,
      idempotentHint: true,
      openWorldHint: false,
    },
  },
  async ({ agentId }) => {
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), 28000);
    try {
      const url = process.env.PASEO_BOARD_URL;
      const token = process.env.PASEO_BOARD_TOKEN;
      if (!url || !token || !/^http:\/\/127\.0\.0\.1:\d+\/mcp$/.test(url))
        throw new Error("Missing launch binding");
      const post = (target: string) =>
        fetch(target, {
          method: "POST",
          headers: { "Content-Type": "application/json", Authorization: `Bearer ${token}` },
          body: JSON.stringify({ action: "remove", input: { agentId } }),
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
        ...(response.ok && ["removed", "scheduled"].includes(String(data.result))
          ? {}
          : { isError: true }),
      };
    } catch {
      return {
        content: [
          {
            type: "text" as const,
            text: "Board unavailable. After plugin reload, create a fresh agent.",
          },
        ],
        isError: true,
      };
    } finally {
      clearTimeout(timer);
    }
  },
);
await server.connect(new StdioServerTransport());
