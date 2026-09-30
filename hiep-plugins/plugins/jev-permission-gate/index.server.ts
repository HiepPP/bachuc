import type { PluginServerContext } from "@getpaseo/plugin/server";
import type { AgentPermissionRequest } from "@getpaseo/protocol/agent-types";
import { existsSync, readFileSync } from "node:fs";
import { homedir } from "node:os";
import path from "node:path";
import { createJudge } from "./server/judge";
import { createLedger, hashCommand } from "./server/ledger";
import { gate } from "./server/policy";

const HOOK_BUDGET_MS = 25000;

function shellCommand(request: AgentPermissionRequest): string | null {
  if (request.detail?.type === "shell") return request.detail.command;
  const input = request.input;
  if (input && typeof input.command === "string") return input.command;
  if (input && typeof input.cmd === "string") return input.cmd;
  return null;
}

export default function contribute(server: PluginServerContext) {
  const home = process.env.PASEO_HOME || path.join(homedir(), ".paseo");
  const configFile = path.join(home, "config.json");
  const root = JSON.parse(readFileSync(configFile, "utf8")).plugins?.["jev-permission-gate"]?.path;
  if (typeof root !== "string" || !existsSync(path.join(root, "server/jev-worker.ts")))
    throw new Error("Install under directory ID jev-permission-gate.");
  const judge = createJudge(configFile, root);
  const record = createLedger(path.join(home, "plugin-data/jev-permission-gate/decisions.jsonl"));

  const off = server.on("agent.permission_requested", async ({ agent, request }, context) => {
    if (request.kind !== "tool") return;
    const command = shellCommand(request);
    if (command === null) return;
    const started = Date.now();
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), HOOK_BUDGET_MS);
    const detailCwd = request.detail?.type === "shell" ? request.detail.cwd : undefined;
    const cwdRelative = detailCwd ? path.relative(agent.cwd, detailCwd) || "." : ".";
    try {
      const result = await gate(
        { command, tool: request.name, cwdRelative },
        judge,
        controller.signal,
      );
      await record({
        ...result,
        ts: new Date().toISOString(),
        agentId: agent.id,
        requestId: request.id,
        commandHash: hashCommand(command),
        ms: Date.now() - started,
      });
      if (result.decision === "escalate") return;
      await context.paseo.agents.ref(agent.id).respondToPermission({
        requestId: request.id,
        response:
          result.decision === "allow"
            ? { behavior: "allow" }
            : { behavior: "deny", message: `jev-permission-gate: ${result.reason}` },
      });
    } catch (error) {
      console.error("jev-permission-gate failed", error instanceof Error ? error.message : error);
    } finally {
      clearTimeout(timer);
    }
  });
  return () => off();
}
