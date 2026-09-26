import type { PluginServerContext } from "@getpaseo/plugin/server";
import { readFileSync } from "node:fs";
import { homedir } from "node:os";
import path from "node:path";
import {
  hostRpc,
  enhanceRpc,
  originalRpc,
  translateRpc,
  modeReadRpc,
  modeWriteRpc,
  prepareModeRpc,
  cancelModeRpc,
  bindQueueModeRpc,
  cancelQueueModeRpc,
} from "./shared/contracts";
import { translateSettings } from "./shared/settings";
import { resolveEndpoint } from "./server/credentials";
import { createCompleter } from "./server/llm";
import { createService } from "./server/service";
import { AgentModes } from "./server/modes";
import { Store } from "./server/store";
import { withBridgeCavemanEnv } from "./server/session-env";

export default function contribute(server: PluginServerContext) {
  const home = process.env.PASEO_HOME || path.join(homedir(), ".paseo");
  const configFile = path.join(home, "config.json");
  const serverId =
    process.env.PASEO_SERVER_ID?.trim() ||
    readFileSync(path.join(home, "server-id"), "utf8").trim();
  const settings = server.registerSettings(translateSettings);
  const store = new Store(path.join(home, "plugin-data/prompt-translate/cache.json"));
  const dataDir = path.join(home, "plugin-data/prompt-translate");
  const modes = new AgentModes(dataDir);
  const readSettings = async () => {
    const value = await settings.read();
    return value.status === "ready" ? value.values : translateSettings.schema.parse({});
  };
  server.handle(hostRpc, () => ({ serverId }));
  server.handle(modeReadRpc, ({ agentId }) => modes.get(agentId));
  server.handle(modeWriteRpc, ({ agentId, mode }) => modes.set(agentId, mode));
  server.handle(prepareModeRpc, async (input) => modes.prepare(input, await readSettings()));
  server.handle(cancelModeRpc, ({ agentId, token }) => modes.cancel(agentId, token));
  server.handle(bindQueueModeRpc, ({ agentId, token, queueId }) =>
    modes.bindQueue(agentId, token, queueId),
  );
  server.handle(cancelQueueModeRpc, ({ agentId, queueId }) => modes.cancelQueue(agentId, queueId));
  const service = createService({
    store,
    complete: createCompleter((provider) => resolveEndpoint(provider, configFile)),
    settings: readSettings,
  });
  server.handle(translateRpc, (input) => service.translate(input));
  server.handle(enhanceRpc, (input) => service.enhance(input));
  server.handle(originalRpc, (input) => service.original(input));
  const sessionOpen = server.before("agent.session_open", ({ request }) =>
    withBridgeCavemanEnv(request, dataDir),
  );
  return () => {
    sessionOpen();
    store.close();
  };
}
