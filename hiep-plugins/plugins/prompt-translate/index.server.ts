import type { PluginServerContext } from "@getpaseo/plugin/server";
import { readFileSync } from "node:fs";
import { homedir } from "node:os";
import path from "node:path";
import {
  enhanceRpc,
  originalRpc,
  translateRpc,
  modeReadRpc,
  modeWriteRpc,
} from "./shared/contracts";
import { translateSettings } from "./shared/settings";
import { resolveEndpoint } from "./server/credentials";
import { createCompleter } from "./server/llm";
import { createService } from "./server/service";
import { seedDraftPrefs } from "./server/draft";
import { AgentModes } from "./server/modes";
import { Store } from "./server/store";
import { withBridgeCavemanEnv } from "./server/session-env";
import { markChild, unmarkChild } from "./server/child-markers";
import { appendContext, promptText, readRuntime, turnContext } from "./server/turn";

export default function contribute(server: PluginServerContext) {
  const home = process.env.PASEO_HOME || path.join(homedir(), ".paseo");
  const configFile = path.join(home, "config.json");
  const root = JSON.parse(readFileSync(configFile, "utf8")).plugins?.["prompt-translate"]?.path;
  if (typeof root !== "string" || !path.isAbsolute(root))
    throw new Error("Install under directory ID prompt-translate.");
  const hookFile = path.join(root, "server/caveman-hook.cjs");
  const settings = server.registerSettings(translateSettings);
  const store = new Store(path.join(home, "plugin-data/prompt-translate/cache.json"));
  const dataDir = path.join(home, "plugin-data/prompt-translate");
  // `v2` keeps the old native hook, which reads `agents/` one level up, silent for this host.
  const modesDir = path.join(dataDir, "v2");
  const modes = new AgentModes(modesDir);
  const readSettings = async () => {
    const value = await settings.read();
    return value.status === "ready" ? value.values : translateSettings.schema.parse({});
  };
  server.handle(modeReadRpc, async ({ agentId }) =>
    modes.prefs(agentId, (await readSettings()).enhanceShortcut),
  );
  server.handle(modeWriteRpc, async ({ agentId, mode, rewrite }) => {
    await modes.update(agentId, { mode, rewrite });
    return modes.prefs(agentId, (await readSettings()).enhanceShortcut);
  });
  const service = createService({
    store,
    complete: createCompleter((provider) => resolveEndpoint(provider, configFile)),
    settings: readSettings,
  });
  server.handle(translateRpc, (input) => service.translate(input));
  server.handle(enhanceRpc, (input) => service.enhance(input));
  server.handle(originalRpc, (input) => service.original(input));
  const sessionOpen = server.before("agent.session_open", async ({ request }) => {
    const seeded = await seedDraftPrefs(request, modes, readSettings);
    return withBridgeCavemanEnv(seeded, dataDir) ?? seeded;
  });
  const prompt = server.before("agent.prompt", async ({ request }) => {
    if (request.kind !== "turn") return;
    if (request.provider !== "claude" && request.provider !== "codex") return;
    const runtime = readRuntime(dataDir);
    if (!runtime) return;
    const text = promptText(request.prompt);
    const source = (await service.original({ text })).original ?? text;
    let context: string;
    try {
      context = turnContext({
        hookFile,
        prompt: text,
        source,
        agentId: request.agentId,
        cwd: request.cwd,
        dir: path.join(modesDir, "agents", request.agentId),
        runtime,
        mode: (await modes.find(request.agentId)) ?? (await readSettings()).cavemanMode,
        settings: await readSettings(),
      });
    } catch (error) {
      // A failed Caveman run must not fail the user's turn; the turn goes out without context.
      console.warn("[prompt-translate] Caveman context failed", error);
      return;
    }
    return { ...request, prompt: appendContext(request.prompt, context) };
  });
  const created = server.on("agent.created", ({ agent }) => markChild(dataDir, agent));
  const archived = server.on("agent.archived", ({ agent }) => unmarkChild(dataDir, agent));
  return () => {
    sessionOpen();
    prompt();
    created();
    archived();
    store.close();
  };
}
