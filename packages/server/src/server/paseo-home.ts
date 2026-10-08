import os from "node:os";
import path from "node:path";

function expandHomeDir(input: string): string {
  if (input.startsWith("~/")) {
    return path.join(os.homedir(), input.slice(2));
  }
  if (input === "~") {
    return os.homedir();
  }
  return input;
}

// Bachuc keeps its daemon home in ~/.bachuc. scripts/paseo-release.sh clones ~/.paseo there once.
export const DEFAULT_PASEO_HOME = "~/.bachuc";

export function resolvePaseoHome(env: NodeJS.ProcessEnv = process.env): string {
  const raw = env.PASEO_HOME ?? DEFAULT_PASEO_HOME;
  const resolved = path.resolve(expandHomeDir(raw));
  return resolved;
}
