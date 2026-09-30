import { spawn } from "node:child_process";
import path from "node:path";
import type { Judge } from "./engine";

export function createJudge(root: string, configFile: string): Judge {
  return async (state, signal) => {
    if (signal.aborted) throw new Error("Cancelled.");
    return new Promise<boolean>((resolve, reject) => {
      const child = spawn(
        process.execPath,
        [path.join(root, "server/jev-worker.mjs"), configFile],
        {
          stdio: ["pipe", "pipe", "pipe"],
          env: { ...process.env, ELECTRON_RUN_AS_NODE: "1" },
        },
      );
      let output = "";
      const kill = () => {
        child.kill("SIGKILL");
      };
      const timer = setTimeout(kill, 45000);
      signal.addEventListener("abort", kill, { once: true });
      child.stdout.on("data", (data) => {
        output += data;
        if (output.length > 32000) kill();
      });
      child.stderr.resume();
      child.stdin.on("error", () => {});
      child.on("error", () => {
        clearTimeout(timer);
        signal.removeEventListener("abort", kill);
        reject(new Error("Jev unavailable."));
      });
      child.on("close", (code) => {
        clearTimeout(timer);
        signal.removeEventListener("abort", kill);
        try {
          if (code !== 0 || signal.aborted) throw new Error();
          const result = JSON.parse(output);
          if (typeof result.allowed !== "boolean") throw new Error();
          console.info(
            "Jev continuation decision",
            JSON.stringify({
              decision: result.decision,
              probability: result.probability,
              allowed: result.allowed,
              usage: result.usage,
            }),
          );
          resolve(result.allowed);
        } catch {
          reject(new Error("Jev unavailable."));
        }
      });
      child.stdin.end(JSON.stringify(state));
    });
  };
}
