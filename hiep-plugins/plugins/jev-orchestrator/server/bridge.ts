import { createServer } from "node:http";
import { randomBytes } from "node:crypto";
import { mkdirSync, readFileSync, renameSync, writeFileSync } from "node:fs";
import path from "node:path";
import type { Engine } from "./engine";
import { NativePreflightError } from "./native-preflight-error";

export function createBridge(
  engine: Engine,
  scope: (parentId: string) => Promise<{ cwd: string }>,
  native?: (action: string, parentId: string, cwd: string, input: unknown) => Promise<unknown>,
  stateFile?: string,
) {
  type Lease = { cwd: string; parentId?: string };
  // Resumed agents keep the MCP binding persisted in their config, so bound leases and
  // the port must survive daemon restarts. Unbound leases belong to a dead create.
  let saved: { port?: number; leases?: [string, Lease][] } = {};
  try {
    if (stateFile) saved = JSON.parse(readFileSync(stateFile, "utf8"));
  } catch {}
  const leases = new Map<string, Lease>(
    (saved.leases ?? []).filter(([, lease]) => lease?.parentId && lease.cwd),
  );
  let port = Number.isInteger(saved.port) ? saved.port! : 0;
  const save = () => {
    if (!stateFile) return;
    mkdirSync(path.dirname(stateFile), { recursive: true });
    const temp = `${stateFile}.${process.pid}.tmp`;
    writeFileSync(temp, JSON.stringify({ port, leases: [...leases] }), { mode: 0o600 });
    renameSync(temp, stateFile);
  };
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
    const listen = (wanted: number) => {
      server.once("error", (error: NodeJS.ErrnoException) => {
        if (wanted && error.code === "EADDRINUSE") listen(0);
        else reject(error);
      });
      server.listen(wanted, "127.0.0.1", () => {
        const a = server.address();
        if (!a || typeof a === "string") return reject(new Error("Bridge unavailable."));
        port = a.port;
        save();
        resolve(`http://127.0.0.1:${a.port}/mcp`);
      });
    };
    listen(port);
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
      if (!lease.parentId) {
        lease.parentId = parentId;
        save();
      }
      return true;
    },
    leaseFor(parentId: string, cwd: string) {
      for (const [token, lease] of leases)
        if (lease.parentId === parentId && lease.cwd === cwd) return token;
    },
    revoke(parentId: string) {
      let changed = false;
      for (const [token, lease] of leases)
        if (lease.parentId === parentId) changed = leases.delete(token);
      if (changed) save();
    },
    close() {
      leases.clear();
      server.closeAllConnections();
      server.close();
    },
  };
}
