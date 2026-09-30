import type { PaseoApi } from "@getpaseo/client";
import type { PluginServerContext } from "@getpaseo/plugin/server";
import { homedir } from "node:os";
import path from "node:path";
import { Engine } from "./server/engine";
import { createDriver } from "./server/paseo";
import { Store } from "./server/store";
import { runVerify } from "./server/verify";

export default function contribute(server: PluginServerContext) {
  const home = process.env.PASEO_HOME || path.join(homedir(), ".paseo");
  let api: PaseoApi | undefined;
  const engine = new Engine(
    new Store(path.join(home, "plugin-data/loop-verify/state.json")),
    createDriver(() => {
      if (!api) throw new Error("SDK unavailable.");
      return api;
    }),
    runVerify,
    (line) => console.log(`[loop-verify] ${line}`),
  );
  const ended = server.on("agent.turn_ended", ({ agent, outcome, timeline }, { paseo }) => {
    api = paseo;
    const first = timeline.find((item) => item.type === "user_message");
    const last = timeline.at(-1);
    // Verify and the next round run detached so the lifecycle hook returns immediately.
    void engine.ended({
      agentId: agent.id,
      outcome: outcome.kind,
      firstUserMessage: first?.type === "user_message" ? first.text : null,
      lastAssistantMessage: last?.type === "assistant_message" ? last.text : null,
    });
  });
  const archived = server.on("agent.archived", ({ agent }) => engine.stop(agent.id));
  return () => {
    ended();
    archived();
  };
}
