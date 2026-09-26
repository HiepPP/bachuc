import type { PluginServerContext } from "@getpaseo/plugin/server";
import type { PaseoApi } from "@getpaseo/client";
import { readFileSync, existsSync } from "node:fs";
import { realpath } from "node:fs/promises";
import { homedir } from "node:os";
import path from "node:path";
import { createBridge } from "./server/bridge";
import { createDriver } from "./server/paseo";
import { createJudge } from "./server/jev";
import { Store } from "./server/store";
import { Engine } from "./server/engine";
import { jobsRpc, submitRpc, cancelRpc } from "./shared/contracts";
import { directProfilesRpc, directRunRpc, directStatusRpc } from "./shared/direct";
import { DirectRouter } from "./server/direct";
import { NativeTickets } from "./server/native-tickets";
import { prepareNativeLaunch } from "./server/native-launch";
import { NativeCleanup, listLiveAgents } from "./server/native-cleanup";

export default function contribute(server: PluginServerContext) {
  const home = process.env.PASEO_HOME || path.join(homedir(), ".paseo");
  const configFile = path.join(home, "config.json");
  const config = JSON.parse(readFileSync(configFile, "utf8"));
  const root = config.plugins?.["jev-orchestrator"]?.path;
  if (
    typeof root !== "string" ||
    !path.isAbsolute(root) ||
    !existsSync(path.join(root, "server/mcp.ts"))
  )
    throw new Error("Install under directory ID jev-orchestrator.");
  let api: PaseoApi | undefined;
  const getApi = () => {
    if (!api) throw new Error("Host SDK not ready.");
    return api;
  };
  const judge = createJudge(configFile, root);
  const direct = new DirectRouter(
    getApi,
    judge,
    path.join(home, "plugin-data/jev-orchestrator/direct.json"),
  );
  const engine = new Engine(
    new Store(path.join(home, "plugin-data/jev-orchestrator/state.json")),
    createDriver(getApi),
    judge,
  );
  async function scope(parentId: string) {
    const parent = getApi().agents.ref(parentId);
    await parent.refresh();
    if (!parent.workspaceId || !parent.cwd || parent.archivedAt)
      throw new Error("Parent unavailable.");
    return { cwd: await realpath(parent.cwd) };
  }
  const native = new NativeTickets(homedir(), judge, Date.now, async (parentId) => {
    const parent = getApi().agents.ref(parentId);
    await parent.refresh();
    if (parent.archivedAt) return undefined;
    const runtime = parent.runtimeInfo;
    if (runtime?.provider !== "codex") return undefined;
    return {
      model: runtime.model ?? undefined,
      thinkingOptionId: runtime.thinkingOptionId ?? undefined,
    };
  });
  const cleanup = new NativeCleanup(path.join(home, "plugin-data/jev-orchestrator/native"), [
    path.join(homedir(), ".claude/agents"),
    path.join(homedir(), ".codex/agents"),
  ]);
  async function sweep(archivedId?: string) {
    const live = (await listLiveAgents(getApi())).filter(({ id }) => id !== archivedId);
    const result = await cleanup.prune(live);
    if (result.manifests || result.definitions)
      console.log(
        `Jev native: removed ${result.manifests} manifests and ${result.definitions} agent definitions.`,
      );
  }
  // The host API exists only inside hooks and RPCs, so the startup sweep waits for the first hook.
  let startupSweep: Promise<void> | undefined;
  const sweepOnce = () => {
    startupSweep ??= sweep().catch((error) =>
      console.warn(`Jev native: cleanup failed: ${(error as Error).message}`),
    );
  };
  const bridge = createBridge(engine, scope, (action, parentId, cwd, input) =>
    native.handle(action, parentId, cwd, input),
  );
  server.handle(directProfilesRpc, async ({ workspaceId }, context) => {
    api = context.paseo;
    return { profiles: await direct.profiles(workspaceId) };
  });
  server.handle(directRunRpc, async (input, context) => {
    api = context.paseo;
    return direct.run(input);
  });
  server.handle(directStatusRpc, ({ workspaceId, requestId }, context) => {
    api = context.paseo;
    return { records: direct.list(workspaceId, requestId) };
  });
  server.handle(jobsRpc, ({ parentId }, context) => {
    api = context.paseo;
    return { jobs: engine.list(parentId), history: engine.history(parentId) };
  });
  server.handle(submitRpc, async ({ parentId, tasks }, context) => {
    api = context.paseo;
    return { ids: await engine.submit(parentId, (await scope(parentId)).cwd, { tasks }) };
  });
  server.handle(cancelRpc, async ({ parentId, id }, context) => {
    api = context.paseo;
    return { cancelled: await engine.cancel(parentId, id) };
  });
  const create = server.before("agent.create", async ({ request }, context) => {
    api = context.paseo;
    sweepOnce();
    if (
      !["codex", "claude"].includes(request.config.provider) ||
      request.env?.PASEO_ORCH_CHILD === "1" ||
      request.config.mcpServers?.jev_orchestrator
    )
      return request;
    const url = await bridge.ready;
    const token = bridge.issue(await realpath(request.config.cwd));
    const env = { ...request.env, PASEO_ORCH_TOKEN: token, PASEO_ORCH_URL: url };
    return {
      ...request,
      env,
      config: {
        ...request.config,
        mcpServers: {
          ...request.config.mcpServers,
          jev_orchestrator: {
            type: "stdio" as const,
            command: process.execPath,
            args: [
              "--import",
              path.join(root, "node_modules/tsx/dist/loader.mjs"),
              path.join(root, "server/mcp.ts"),
            ],
            env: { ELECTRON_RUN_AS_NODE: "1", PASEO_ORCH_TOKEN: token, PASEO_ORCH_URL: url },
            alwaysLoad: true,
          },
        },
      },
    };
  });
  const open = server.before("agent.session_open", async ({ request }, context) => {
    api = context.paseo;
    sweepOnce();
    const token = request.env?.PASEO_ORCH_TOKEN;
    if (token) bridge.bind(token, request.agentId, await realpath(request.cwd));
    const env = { ...request.env };
    delete env.PASEO_JEV_NATIVE_POLICY;
    delete env.PASEO_JEV_NATIVE_RUNTIME;
    if (
      request.purpose === "interactive" &&
      (request.provider === "codex" || request.provider === "claude")
    ) {
      const provider = request.provider;
      const nativeEnv = await cleanup.launch(request.agentId, () =>
        prepareNativeLaunch(home, homedir(), request.cwd, provider),
      );
      if (request.provider === "codex" && token && nativeEnv.PASEO_JEV_NATIVE_POLICY)
        await native.bind(request.agentId, request.cwd, nativeEnv.PASEO_JEV_NATIVE_POLICY);
      else native.revoke(request.agentId);
      return { ...request, env: { ...env, ...nativeEnv } };
    }
    await cleanup.launch(request.agentId, async () => ({}));
    return { ...request, env };
  });
  const archived = server.on("agent.archived", async ({ agent }, context) => {
    api = context.paseo;
    bridge.revoke(agent.id);
    await native.archive(agent.id);
    for (const job of engine.list(agent.id))
      void engine.cancel(agent.id, job.task.id).catch(() => {});
    void sweep(agent.id).catch((error) =>
      console.warn(`Jev native: cleanup failed: ${(error as Error).message}`),
    );
  });
  return () => {
    native.close();
    direct.stop();
    engine.stop();
    bridge.close();
    create();
    open();
    archived();
  };
}
