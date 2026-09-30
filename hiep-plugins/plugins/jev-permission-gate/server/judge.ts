import { spawn } from "node:child_process";
import path from "node:path";
import type { Judge, Verdict } from "./policy";

export const WORKER_TIMEOUT_MS = 8000;

/** Sequential Jev judge running the worker in a plain Node process. */
export function createJudge(configFile: string, root: string): Judge {
  let queue = Promise.resolve();
  return async (state, signal) => {
    const previous = queue;
    let release!: () => void;
    queue = new Promise<void>((resolve) => (release = resolve));
    try {
      await previous;
      if (signal.aborted) throw new Error("Evaluation cancelled.");
      return await new Promise<Verdict>((resolve, reject) => {
        const child = spawn(
          process.execPath,
          [
            "--import",
            path.join(root, "node_modules/tsx/dist/loader.mjs"),
            path.join(root, "server/jev-worker.ts"),
          ],
          { stdio: ["pipe", "pipe", "pipe"], env: { ...process.env, ELECTRON_RUN_AS_NODE: "1" } },
        );
        let output = "";
        let settled = false;
        const kill = () => child.kill("SIGKILL");
        const timer = setTimeout(kill, WORKER_TIMEOUT_MS);
        signal.addEventListener("abort", kill, { once: true });
        child.stdout.on("data", (data) => {
          output += data.toString();
          if (output.length > 64000) kill();
        });
        child.stderr.resume();
        const finish = (code: number | null) => {
          if (settled) return;
          settled = true;
          clearTimeout(timer);
          signal.removeEventListener("abort", kill);
          try {
            const value = JSON.parse(output);
            if (code !== 0 || value.error)
              throw new Error(typeof value.error === "string" ? value.error : "Jev worker failed.");
            resolve(value);
          } catch (error) {
            reject(error instanceof Error ? error : new Error("Jev worker unavailable."));
          }
        };
        child.on("error", () => finish(null));
        child.on("close", finish);
        child.stdin.on("error", () => {});
        child.stdin.end(JSON.stringify({ state, configFile }));
      });
    } finally {
      release();
    }
  };
}
