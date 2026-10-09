import type { PluginServerContext } from "@getpaseo/plugin/server";
import type { PaseoApi } from "@getpaseo/client";
import { readFileSync } from "node:fs";
import { homedir } from "node:os";
import path from "node:path";
import { hostRpc, inspectRpc, scopeRpc, sendRpc, startRpc, toggleRpc } from "./shared/contracts";
import { Engine } from "./server/engine";
import { Store } from "./server/store";
import { createDriver } from "./server/paseo";
import { createJudge } from "./server/jev";
import { readDependencies } from "./server/dependencies";
import { sendSettings } from "./shared/settings";

export default function contribute(server: PluginServerContext) {
  const home = process.env.PASEO_HOME || path.join(homedir(), ".paseo");
  const serverId = readFileSync(path.join(home, "server-id"), "utf8").trim();
  const configFile = path.join(home, "config.json");
  const root = JSON.parse(readFileSync(configFile, "utf8")).plugins?.["next-prompt-actions"]?.path;
  if (typeof root !== "string" || !path.isAbsolute(root))
    throw new Error("Install as next-prompt-actions.");
  let api: PaseoApi | undefined;
  server.registerSettings(sendSettings);
  server.handle(hostRpc, () => ({ serverId, evaluator: readDependencies(configFile).evaluator }));
  const engine = new Engine(
    new Store(path.join(home, "plugin-data/next-prompt-actions/state.json")),
    createDriver(() => {
      if (!api) throw new Error("SDK unavailable.");
      return api;
    }, serverId),
    createJudge(root, configFile),
    () => readDependencies(configFile),
  );
  server.handle(scopeRpc, async ({ agentId }, context) => {
    api = context.paseo;
    const handle = context.paseo.agents.ref(agentId);
    await handle.refresh();
    if (!handle.workspaceId) throw new Error("Conversation unavailable.");
    return { serverId, agentId, workspaceId: handle.workspaceId };
  });
  server.handle(inspectRpc, (input, context) => {
    api = context.paseo;
    return engine.inspect(input);
  });
  server.handle(toggleRpc, (input, context) => {
    api = context.paseo;
    return engine.toggle(input, input.enabled);
  });
  server.handle(sendRpc, async (input, context) => {
    api = context.paseo;
    return { sent: await engine.send(input, input.key) };
  });
  server.handle(startRpc, async (input, context) => {
    api = context.paseo;
    return { started: await engine.start(input, input.key) };
  });
  const started = server.on("agent.turn_started", ({ agent }, context) => {
    api = context.paseo;
    engine.started(agent.id);
  });
  const ended = server.on("agent.turn_ended", ({ agent, outcome }, context) => {
    api = context.paseo;
    if (!agent.workspaceId) return;
    // Do not hold the lifecycle hook open while evaluating or sending a new turn.
    void engine.ended(
      { serverId, agentId: agent.id, workspaceId: agent.workspaceId },
      outcome.kind === "completed",
    );
  });
  const permission = server.on("agent.permission_requested", ({ agent }) =>
    engine.interrupted(agent.id),
  );
  const archived = server.on("agent.archived", ({ agent }) => engine.interrupted(agent.id));
  return () => {
    engine.close();
    started();
    ended();
    permission();
    archived();
  };
}
