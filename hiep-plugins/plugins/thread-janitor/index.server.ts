import type { PluginServerContext } from "@getpaseo/plugin/server";
import { janitorSettings } from "./shared/settings";
import { SWEEP_INTERVAL_MS, createJanitor } from "./server/janitor";

export default function contribute(server: PluginServerContext) {
  const settings = server.registerSettings(janitorSettings);
  const janitor = createJanitor({
    readSettings: () => settings.read(),
    log: (line) => console.log(`[thread-janitor] ${line}`),
  });
  // Sweeps run detached so agent lifecycle hooks return immediately.
  const removeTurnEnded = server.on("agent.turn_ended", (_, { paseo }) => {
    void janitor.fromHook(paseo, "turn_ended");
  });
  const removeCreated = server.on("agent.created", (_, { paseo }) => {
    void janitor.fromHook(paseo, "created");
  });
  // Reading a closed thread's timeline resumes its runtime. This hook is awaited inside the
  // open, so it only schedules a sweep.
  const removeSessionOpen = server.before("agent.session_open", (_, { paseo }) => {
    janitor.afterResume(paseo);
  });
  const timer = setInterval(() => void janitor.fromTimer(), SWEEP_INTERVAL_MS);
  return () => {
    clearInterval(timer);
    removeTurnEnded();
    removeCreated();
    removeSessionOpen();
    janitor.stop();
  };
}
