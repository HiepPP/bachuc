import { createServer } from "node:http";
import { randomBytes } from "node:crypto";
import { mkdirSync, readFileSync, renameSync, writeFileSync } from "node:fs";
import path from "node:path";

export type RemoveResult =
  | { result: "removed"; count: number }
  | { result: "scheduled" | "not_found" | "forbidden" };

// Local bridge for the board_remove MCP tool. A token identifies the calling agent, so an agent
// can only remove itself and its descendants.
export function createBridge(
  remove: (callerId: string, agentId: string) => Promise<RemoveResult>,
  stateFile?: string,
) {
  // Resumed agents keep the MCP binding persisted in their config, so bound leases and the port
  // must survive daemon restarts. Unbound leases belong to a dead create.
  let saved: { port?: number; leases?: [string, { agentId?: string }][] } = {};
  try {
    if (stateFile) saved = JSON.parse(readFileSync(stateFile, "utf8"));
  } catch {}
  const leases = new Map<string, { agentId?: string }>(
    (saved.leases ?? []).filter(([, lease]) => typeof lease?.agentId === "string"),
  );
  let port = Number.isInteger(saved.port) ? saved.port! : 0;
  const save = () => {
    if (!stateFile) return;
    mkdirSync(path.dirname(stateFile), { recursive: true, mode: 0o700 });
    const temp = `${stateFile}.${process.pid}.tmp`;
    writeFileSync(temp, JSON.stringify({ port, leases: [...leases] }), { mode: 0o600 });
    renameSync(temp, stateFile);
  };
  const server = createServer(async (request, response) => {
    const token = request.headers.authorization?.replace(/^Bearer /, "");
    const callerId = token ? leases.get(token)?.agentId : undefined;
    response.setHeader("Content-Type", "application/json");
    response.setHeader("Cache-Control", "no-store");
    if (
      request.method !== "POST" ||
      request.url !== "/mcp" ||
      request.headers.origin ||
      !callerId
    ) {
      response.writeHead(403).end('{"error":"Invalid session scope."}');
      return;
    }
    try {
      let text = "";
      for await (const chunk of request) {
        text += chunk;
        if (Buffer.byteLength(text) > 16000) throw new Error("Request too large.");
      }
      const input = JSON.parse(text);
      const agentId = input.input?.agentId ?? callerId;
      if (input.action !== "remove" || typeof agentId !== "string")
        throw new Error("Unknown action.");
      response.end(JSON.stringify(await remove(callerId, agentId)));
    } catch {
      response.writeHead(400).end('{"error":"Operation rejected."}');
    }
  });
  server.requestTimeout = 30000;
  server.headersTimeout = 10000;
  const ready = new Promise<string>((resolve, reject) => {
    const listen = (wanted: number) => {
      server.once("error", (error: NodeJS.ErrnoException) => {
        if (wanted && error.code === "EADDRINUSE") listen(0);
        else reject(error);
      });
      server.listen(wanted, "127.0.0.1", () => {
        const address = server.address();
        if (!address || typeof address === "string")
          return reject(new Error("Bridge unavailable."));
        port = address.port;
        save();
        resolve(`http://127.0.0.1:${address.port}/mcp`);
      });
    };
    listen(port);
  });
  return {
    ready,
    issue() {
      if (leases.size > 500) throw new Error("Too many Board sessions; reload the plugin.");
      const token = randomBytes(32).toString("hex");
      leases.set(token, {});
      return token;
    },
    bind(token: string, agentId: string) {
      const lease = leases.get(token);
      if (!lease || (lease.agentId && lease.agentId !== agentId)) return false;
      if (!lease.agentId) {
        lease.agentId = agentId;
        save();
      }
      return true;
    },
    revoke(agentId: string) {
      let changed = false;
      for (const [token, lease] of leases)
        if (lease.agentId === agentId) changed = leases.delete(token);
      if (changed) save();
    },
    close() {
      leases.clear();
      server.closeAllConnections();
      server.close();
    },
  };
}

type CreateRequest = {
  env?: Record<string, string>;
  config: { provider: string; mcpServers?: Record<string, unknown> };
};

/** Adds the stdio board MCP server to a new Claude or Codex agent. */
export function withBoardMcp<T extends CreateRequest>(
  request: T,
  root: string,
  url: string,
  issue: () => string,
): T {
  if (!["codex", "claude"].includes(request.config.provider) || request.config.mcpServers?.board)
    return request;
  const token = issue();
  return {
    ...request,
    env: { ...request.env, PASEO_BOARD_TOKEN: token },
    config: {
      ...request.config,
      mcpServers: {
        ...request.config.mcpServers,
        board: {
          type: "stdio" as const,
          command: process.execPath,
          args: [
            "--import",
            path.join(root, "node_modules/tsx/dist/loader.mjs"),
            path.join(root, "server/mcp.ts"),
          ],
          env: { ELECTRON_RUN_AS_NODE: "1", PASEO_BOARD_TOKEN: token, PASEO_BOARD_URL: url },
          alwaysLoad: true,
        },
      },
    },
  };
}
