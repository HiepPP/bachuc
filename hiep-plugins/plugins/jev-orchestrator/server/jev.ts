import { spawn } from "node:child_process";
import path from "node:path";
import { setTimeout as delay } from "node:timers/promises";
import type { Decision, Judge } from "./types";
import {
  EvaluationError,
  safeEvaluationDiagnostics,
  type EvaluationCode,
} from "./evaluation-error";

export function createJudge(configFile: string, root: string, spacingMs = 26000): Judge {
  let queue = Promise.resolve(),
    next = 0;
  return async (phase, state, profiles, signal) => {
    const previous = queue;
    let release!: () => void;
    queue = new Promise<void>((resolve) => {
      release = resolve;
    });
    try {
      await previous;
      if (signal.aborted) throw new EvaluationError("JEV_CANCELLED");
      try {
        await delay(Math.max(0, next - Date.now()), undefined, { signal });
      } catch {
        throw new EvaluationError("JEV_CANCELLED");
      }
      next = Date.now() + spacingMs;
      return await new Promise<Decision>((resolve, reject) => {
        const child = spawn(
          process.execPath,
          [
            "--import",
            path.join(root, "node_modules/tsx/dist/loader.mjs"),
            path.join(root, "server/jev-worker.ts"),
          ],
          { stdio: ["pipe", "pipe", "pipe"], env: { ...process.env, ELECTRON_RUN_AS_NODE: "1" } },
        );
        let output = "",
          settled = false;
        let termination: EvaluationCode | undefined;
        const kill = () => {
          termination ??= "JEV_CANCELLED";
          child.kill("SIGKILL");
        };
        const timer = setTimeout(() => {
          termination = "JEV_TIMEOUT";
          kill();
        }, 23000);
        signal.addEventListener("abort", kill, { once: true });
        child.stdout.on("data", (data) => {
          output += data.toString();
          if (output.length > 64000) {
            termination = "JEV_WORKER_OUTPUT_LIMIT";
            kill();
          }
        });
        child.stderr.resume();
        const finish = (code: number | null) => {
          if (settled) return;
          settled = true;
          clearTimeout(timer);
          signal.removeEventListener("abort", kill);
          try {
            if (termination) throw new EvaluationError(termination, { processExitCode: code });
            let value;
            try {
              value = JSON.parse(output);
            } catch {
              throw new EvaluationError(
                code !== 0 ? "JEV_WORKER_EXIT_FAILED" : "JEV_WORKER_OUTPUT_INVALID",
                { processExitCode: code },
              );
            }
            if (!value || typeof value !== "object")
              throw new EvaluationError("JEV_WORKER_OUTPUT_INVALID", { processExitCode: code });
            if (code !== 0 || value.error) {
              const details = safeEvaluationDiagnostics(value.diagnostics);
              const error = new EvaluationError(details?.workerCode ?? "JEV_WORKER_EXIT_FAILED", {
                httpStatus: details?.httpStatus,
                processExitCode: code,
              });
              error.usage = value.usage;
              throw error;
            }
            if (!profiles.some((p) => p.id === value.profileId)) {
              const error = new EvaluationError("JEV_RESPONSE_INVALID", { processExitCode: code });
              error.usage = value.usage;
              throw error;
            }
            resolve(value);
          } catch (error) {
            reject(
              error instanceof EvaluationError
                ? error
                : new EvaluationError("JEV_WORKER_OUTPUT_INVALID", { processExitCode: code }),
            );
          }
        };
        child.on("error", () => {
          termination = "JEV_WORKER_SPAWN_FAILED";
          finish(null);
        });
        child.on("close", finish);
        child.stdin.on("error", () => {});
        child.stdin.end(JSON.stringify({ phase, state, profiles, configFile }));
      });
    } finally {
      release();
    }
  };
}
