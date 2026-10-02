import type { PluginServerContext } from "@getpaseo/plugin/server";
import { homedir } from "node:os";
import path from "node:path";
import { skillsReadRpc, skillsWriteRpc } from "./shared/contracts";
import { pinSettings } from "./shared/settings";
import { appendContext, promptText, requested, skillContext } from "./server/context";
import { seedDraftSkills } from "./server/draft";
import { SkillPins } from "./server/state";

export default function contribute(server: PluginServerContext) {
  const home = process.env.PASEO_HOME || path.join(homedir(), ".paseo");
  // `v2` keeps the old native UserPromptSubmit hook, which reads `agents/` one level up, silent.
  const pins = new SkillPins(path.join(home, "plugin-data/skill-pins/v2"));
  const skillsRoot = process.env.SKILL_PINS_SKILLS_ROOT || path.join(homedir(), ".claude/skills");
  server.handle(skillsReadRpc, ({ agentId }) => pins.get(agentId));
  server.handle(skillsWriteRpc, ({ agentId, skills }) => pins.set(agentId, skills));
  const settings = server.registerSettings(pinSettings);
  const defaults = async () => {
    const value = await settings.read();
    return value.status === "ready" ? pinSettings.schema.parse(value.values).defaults : [];
  };
  // New agents start with the host defaults. Existing selections are never replaced.
  const open = server.before("agent.session_open", async ({ request }) => {
    return seedDraftSkills(request, pins, defaults);
  });
  const prompt = server.before("agent.prompt", async ({ request }) => {
    if (request.kind !== "turn") return;
    if (request.provider !== "claude" && request.provider !== "codex") return;
    const text = promptText(request.prompt);
    // A broken pin file must not fail the user's turn; the turn goes out without pins.
    const { skills } = await pins.get(request.agentId).catch((error: unknown) => {
      console.warn("[skill-pins] Reading pins failed", error);
      return { skills: [] };
    });
    const active = skills.filter((id) => !requested(text, id));
    if (!active.length) return;
    return {
      ...request,
      prompt: appendContext(request.prompt, skillContext(active, request.provider, skillsRoot)),
    };
  });
  const archived = server.on("agent.archived", ({ agent }) =>
    pins.remove(agent.id).catch((error) => console.warn("[skill-pins] Cleanup failed", error)),
  );
  return () => {
    open();
    prompt();
    archived();
  };
}
