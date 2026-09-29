import type { PluginServerContext } from "@getpaseo/plugin/server";
import { readFileSync } from "node:fs";
import { homedir } from "node:os";
import path from "node:path";
import {
  bindQueueSkillsRpc,
  cancelQueueSkillsRpc,
  cancelSkillsRpc,
  draftReadRpc,
  draftWriteRpc,
  hostRpc,
  prepareSkillsRpc,
  skillsReadRpc,
  skillsWriteRpc,
} from "./shared/contracts";
import { savedSkillsSchema } from "./shared/catalog";
import { INITIAL_ENV, pinSettings } from "./shared/settings";
import { SkillPins } from "./server/state";

export default function contribute(server: PluginServerContext) {
  const home = process.env.PASEO_HOME || path.join(homedir(), ".paseo");
  const serverId =
    process.env.PASEO_SERVER_ID?.trim() ||
    readFileSync(path.join(home, "server-id"), "utf8").trim();
  const pins = new SkillPins(path.join(home, "plugin-data/skill-pins"));
  server.handle(hostRpc, () => ({ serverId }));
  server.handle(skillsReadRpc, ({ agentId }) => pins.get(agentId));
  server.handle(skillsWriteRpc, ({ agentId, skills }) => pins.set(agentId, skills));
  server.handle(prepareSkillsRpc, (input) => pins.prepare(input));
  server.handle(cancelSkillsRpc, ({ agentId, token }) => pins.cancel(agentId, token));
  server.handle(bindQueueSkillsRpc, ({ agentId, token, queueId }) =>
    pins.bindQueue(agentId, token, queueId),
  );
  server.handle(cancelQueueSkillsRpc, ({ agentId, queueId }) => pins.cancelQueue(agentId, queueId));
  const settings = server.registerSettings(pinSettings);
  const defaults = async () => {
    const value = await settings.read();
    return value.status === "ready" ? pinSettings.schema.parse(value.values).defaults : [];
  };
  server.handle(draftReadRpc, async () => ({
    skills: (await pins.readDraft()) ?? (await defaults()),
  }));
  server.handle(draftWriteRpc, ({ skills }) => pins.writeDraft(skills));
  const create = server.before("agent.create", async ({ request }) => {
    if (request.config.provider !== "claude" && request.config.provider !== "codex") return request;
    if (request.config.internal) return request;
    // A create request does not say which composer sent it, so the next created agent takes the
    // draft. Another agent created in the draft window would take it instead.
    const skills = (await pins.takeDraft()) ?? (await defaults());
    if (!skills.length) return request;
    return { ...request, env: { ...request.env, [INITIAL_ENV]: skills.join(",") } };
  });
  // Saving the initial set lets the pill show it. Existing selections are never replaced.
  const open = server.before("agent.session_open", async ({ request }) => {
    const initial = request.env[INITIAL_ENV];
    if (request.reason === "create" && initial)
      await pins
        .seed(request.agentId, savedSkillsSchema.parse(initial.split(",")))
        .catch((error) => console.warn("[skill-pins] Initial pins failed", error));
    return request;
  });
  const archived = server.on("agent.archived", ({ agent }) =>
    pins.remove(agent.id).catch((error) => console.warn("[skill-pins] Cleanup failed", error)),
  );
  return () => {
    create();
    open();
    archived();
  };
}
