import type { PluginServerContext } from "@getpaseo/plugin/server";
import { closerSettings } from "./shared/settings";
import { SWEEP_INTERVAL_MS, createCloser } from "./server/closer";

export default function contribute(server: PluginServerContext) {
  const settings = server.registerSettings(closerSettings);
  const closer = createCloser({
    readSettings: () => settings.read(),
    log: (line) => console.log(`[idle-runtime-closer] ${line}`),
  });
  // Sweeps run detached so the agent lifecycle hook returns immediately.
  const removeTurnEnded = server.on("agent.turn_ended", (_, { paseo }) => {
    void closer.fromHook(paseo, "turn_ended");
  });
  // Opening a thread resumes its runtime with no turn, so this hook is what arms the timer
  // for a thread that was only read. It is awaited inside the open, so it only stores the API.
  const removeSessionOpen = server.before("agent.session_open", ({ request }, { paseo }) => {
    closer.remember(paseo, request.agentId);
  });
  const timer = setInterval(() => void closer.fromTimer(), SWEEP_INTERVAL_MS);
  return () => {
    clearInterval(timer);
    removeTurnEnded();
    removeSessionOpen();
    closer.stop();
  };
}
