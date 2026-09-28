import { createServer } from "node:http";
import { randomBytes } from "node:crypto";
import type { Engine } from "./engine";
import { NativePreflightError } from "./native-preflight-error";

export function createBridge(
  engine: Engine,
  scope: (parentId: string) => Promise<{ cwd: string }>,
  native?: (action: string, parentId: string, cwd: string, input: unknown) => Promise<unknown>,
) {
  const leases = new Map<string, { cwd: string; parentId?: string }>();
  const server = createServer(async (request, response) => {
    const token = request.headers.authorization?.replace(/^Bearer /, "");
    const lease = token ? leases.get(token) : undefined;
    response.setHeader("Content-Type", "application/json");
    response.setHeader("Cache-Control", "no-store");
    if (
      request.method !== "POST" ||
      request.url !== "/mcp" ||
      request.headers.origin ||
      !lease?.parentId
    ) {
      response.writeHead(403).end('{"error":"Invalid session scope."}');
      return;
    }
    try {
      let text = "";
      for await (const chunk of request) {
        text += chunk;
        if (Buffer.byteLength(text) > 128000) throw new Error("Request too large.");
      }
      const input = JSON.parse(text);
      const current = await scope(lease.parentId);
      if (current.cwd !== lease.cwd) throw new Error("Workspace scope changed.");
      let result: unknown;
      if (typeof input.action === "string" && input.action.startsWith("native_") && native)
        result = await native(input.action, lease.parentId, lease.cwd, input.input);
      else if (input.action === "submit")
        result = { ids: await engine.submit(lease.parentId, lease.cwd, input.input) };
      else if (input.action === "status")
        result = { jobs: engine.list(lease.parentId), history: engine.history(lease.parentId) };
      else if (input.action === "cancel" && typeof input.id === "string")
        result = { cancelled: await engine.cancel(lease.parentId, input.id) };
      else throw new Error("Unknown action.");
      response.end(JSON.stringify(result));
    } catch (error) {
      if (error instanceof NativePreflightError) {
        response.writeHead(400).end(
          JSON.stringify({
            error: error.message,
            failureStage: error.failureStage,
            failureCode: error.code,
            evaluationError: error.evaluation,
          }),
        );
        return;
      }
      // Expose only a known transport failure, never arbitrary SDK errors or task data.
      if (
        error instanceof Error &&
        /^Transport not connected(?: \(status: [a-z_]+\))?$/.test(error.message)
      ) {
        response.writeHead(503).end(
          JSON.stringify({
            code: "daemon_disconnected",
            error:
              "Paseo daemon transport disconnected. Run paseo plugin reload jev-orchestrator, then create a fresh Paseo agent. Do not restart the daemon.",
          }),
        );
        return;
      }
      response
        .writeHead(400)
        .end(
          '{"error":"Operation rejected. Check the task contract, scope, dependency graph, and job status."}',
        );
    }
  });
  server.requestTimeout = 30000;
  server.headersTimeout = 10000;
  const ready = new Promise<string>((resolve, reject) => {
    server.once("error", reject);
    server.listen(0, "127.0.0.1", () => {
      const a = server.address();
      if (!a || typeof a === "string") return reject(new Error("Bridge unavailable."));
      resolve(`http://127.0.0.1:${a.port}/mcp`);
    });
  });
  return {
    ready,
    issue(cwd: string) {
      if (leases.size > 500) throw new Error("Too many orchestrator sessions; reload the plugin.");
      const token = randomBytes(32).toString("hex");
      leases.set(token, { cwd });
      return token;
    },
    bind(token: string, parentId: string, cwd: string) {
      const lease = leases.get(token);
      if (!lease || lease.cwd !== cwd || (lease.parentId && lease.parentId !== parentId))
        return false;
      lease.parentId = parentId;
      return true;
    },
    revoke(parentId: string) {
      for (const [token, lease] of leases) if (lease.parentId === parentId) leases.delete(token);
    },
    close() {
      leases.clear();
      server.closeAllConnections();
      server.close();
    },
  };
}
