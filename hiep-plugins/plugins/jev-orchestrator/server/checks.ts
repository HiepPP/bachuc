import { spawn } from "node:child_process";
import type { Task } from "../shared/contracts";
import type { Check } from "./types";

async function runCheck(
  cwd: string,
  check: Task["checks"][number],
  signal: AbortSignal,
): Promise<Check> {
  const start = Date.now();
  let output = "",
    timedOut = false;
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
  // The first of error/close settles the check; a later event cannot change it.
  const exit = await new Promise<{ code: number | null; started: boolean }>((resolve) => {
    child.once("error", () => resolve({ code: null, started: false }));
    child.once("close", (code) => resolve({ code, started: true }));
  });
  clearTimeout(timer);
  signal.removeEventListener("abort", kill);
  return {
    argv: check.argv,
    exitCode: exit.code,
    output: exit.started ? output : "Check process could not start.",
    durationMs: Date.now() - start,
    timedOut,
  };
}

export async function runChecks(
  cwd: string,
  checks: Task["checks"],
  signal: AbortSignal,
): Promise<Check[]> {
  const results: Check[] = [];
  for (const check of checks) {
    if (signal.aborted) throw new Error("Cancelled");
    results.push(await runCheck(cwd, check, signal));
  }
  return results;
}
