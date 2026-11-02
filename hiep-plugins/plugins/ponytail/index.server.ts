import type { PluginServerContext } from "@getpaseo/plugin/server";
import { readFileSync } from "node:fs";
import { homedir } from "node:os";
import path from "node:path";
import { modeReadRpc, modeWriteRpc } from "./shared/contracts";
import { DEFAULT_MODE, type PonytailMode } from "./shared/modes";
import { seedDraftPrefs } from "./server/draft";
import { AgentModes } from "./server/modes";
import {
  appendContext,
  parseCommand,
  promptText,
  readRuntime,
  RESEND_AFTER,
  turnContext,
} from "./server/turn";

export default function contribute(server: PluginServerContext) {
  const home = process.env.PASEO_HOME || path.join(homedir(), ".paseo");
  // The plugin bundle has no import.meta, so the install path comes from the daemon config.
  const root = JSON.parse(readFileSync(path.join(home, "config.json"), "utf8")).plugins?.ponytail
    ?.path;
  if (typeof root !== "string" || !path.isAbsolute(root))
    throw new Error("Install under directory ID ponytail.");
  const dataDir = path.join(home, "plugin-data/ponytail");
  const modes = new AgentModes(dataDir);
  // The last mode sent and the reminder turns since the last full ruleset, per agent. A
  // restart clears it, so the next turn sends the full ruleset again.
  const sent = new Map<string, { mode: PonytailMode; turns: number }>();
  let missingWarned = false;
  server.handle(modeReadRpc, ({ agentId }) => modes.get(agentId));
  server.handle(modeWriteRpc, ({ agentId, mode }) => modes.set(agentId, mode));
  const sessionOpen = server.before("agent.session_open", ({ request }) =>
    seedDraftPrefs(request, modes),
  );
  const prompt = server.before("agent.prompt", async ({ request }) => {
    if (request.kind !== "turn") return;
    if (request.provider !== "claude" && request.provider !== "codex") return;
    const runtime = readRuntime(dataDir);
    if (!runtime) {
      // Warn once per process; a missing install would otherwise log on every turn.
      if (!missingWarned) console.warn("[ponytail] no hook-runtime.json; run install-hooks.mjs");
      missingWarned = true;
      return;
    }
    try {
      const command = parseCommand(promptText(request.prompt));
      if (command) await modes.set(request.agentId, command);
      const mode = command ?? (await modes.find(request.agentId)) ?? DEFAULT_MODE;
      const last = sent.get(request.agentId);
      const context = turnContext({
        ponytailRoot: runtime.ponytailRoot,
        mode,
        lastMode: last?.mode,
        turnsSinceRules: last?.turns ?? 0,
      });
      const full = last?.mode !== mode || (last?.turns ?? 0) >= RESEND_AFTER;
      sent.set(request.agentId, { mode, turns: mode === "off" || full ? 0 : last!.turns + 1 });
      if (context === null) return;
      await modes.write(path.join(modes.dir(request.agentId), "last-hook.json"), {
        mode,
        kind: mode === "off" ? "off" : full ? "rules" : "reminder",
      });
      return { ...request, prompt: appendContext(request.prompt, context) };
    } catch (error) {
      // A failed Ponytail step must not fail the user's turn; the turn goes out without context.
      console.warn("[ponytail] context failed", error);
    }
  });
  return () => {
    sessionOpen();
    prompt();
  };
}
