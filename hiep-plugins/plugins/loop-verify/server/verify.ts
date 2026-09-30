import { spawn } from "node:child_process";
import type { VerifyResult } from "./engine";

const TIMEOUT_MS = 15 * 60 * 1000;
const MAX_BUFFER = 256 * 1024;

export function runVerify(command: string, cwd: string): Promise<VerifyResult> {
  return new Promise((resolve) => {
    const child = spawn("/bin/sh", ["-c", command], { cwd, env: process.env });
    let output = "";
    const append = (chunk: Buffer) => {
      output = (output + chunk.toString("utf8")).slice(-MAX_BUFFER);
    };
    child.stdout.on("data", append);
    child.stderr.on("data", append);
    const timer = setTimeout(() => child.kill("SIGKILL"), TIMEOUT_MS);
    child.on("error", (error) => {
      clearTimeout(timer);
      resolve({ exitCode: 127, output: `${output}\n${error.message}` });
    });
    child.on("close", (code) => {
      clearTimeout(timer);
      resolve({ exitCode: code, output });
    });
  });
}
