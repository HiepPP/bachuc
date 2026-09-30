import { spawn } from "node:child_process";
import type { Task } from "../shared/contracts";
import type { Check } from "./types";

export async function runChecks(
  cwd: string,
  checks: Task["checks"],
  signal: AbortSignal,
): Promise<Check[]> {
  const results: Check[] = [];
  for (const check of checks) {
    if (signal.aborted) throw new Error("Cancelled");
    results.push(
      await new Promise<Check>((resolve) => {
        const start = Date.now();
        let output = "",
          timedOut = false,
          done = false;
        const child = spawn(check.argv[0], check.argv.slice(1), {
          cwd,
          shell: false,
          stdio: ["ignore", "pipe", "pipe"],
          detached: process.platform !== "win32",
        });
        const kill = () => {
          try {
            if (process.platform !== "win32" && child.pid) process.kill(-child.pid, "SIGKILL");
            else child.kill("SIGKILL");
          } catch {
            /* Already exited. */
          }
        };
        const append = (data: Buffer) => {
          output = (output + data.toString()).slice(-12000);
        };
        child.stdout.on("data", append);
        child.stderr.on("data", append);
        const timer = setTimeout(() => {
          timedOut = true;
          kill();
        }, check.timeoutMs);
        signal.addEventListener("abort", kill, { once: true });
        function finish(code: number | null) {
          if (done) return;
          done = true;
          clearTimeout(timer);
          signal.removeEventListener("abort", kill);
          resolve({
            argv: check.argv,
            exitCode: code,
            output,
            durationMs: Date.now() - start,
            timedOut,
          });
        }
        child.on("error", () => {
          output = "Check process could not start.";
          finish(null);
        });
        child.on("close", finish);
      }),
    );
  }
  return results;
}
