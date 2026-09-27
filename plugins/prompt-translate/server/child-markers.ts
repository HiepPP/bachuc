import { mkdirSync, rmSync, writeFileSync } from "node:fs";
import path from "node:path";
import type { PluginHookAgent } from "@getpaseo/plugin/server";

// Session-open requests carry no parent, and child env matches the parent's, so Claude
// hooks (e.g. ~/.claude/hooks/answer-ending-check.cjs) detect a Paseo child by the
// marker file named after $PASEO_AGENT_ID.
const safeId = /^[\w-]+$/;

export function childMarkerDir(dataDir: string): string {
  return path.join(dataDir, "children");
}

export function markChild(dataDir: string, agent: PluginHookAgent): void {
  if (!agent.parentAgentId || !safeId.test(agent.id)) return;
  const dir = childMarkerDir(dataDir);
  mkdirSync(dir, { recursive: true });
  writeFileSync(path.join(dir, agent.id), agent.parentAgentId);
}

export function unmarkChild(dataDir: string, agent: PluginHookAgent): void {
  if (!safeId.test(agent.id)) return;
  rmSync(path.join(childMarkerDir(dataDir), agent.id), { force: true });
}
