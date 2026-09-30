import { execFile } from "node:child_process";
import { readFile } from "node:fs/promises";
import { userInfo } from "node:os";
import type { Provider } from "../shared/settings";

export type Endpoint = { baseUrl: string; apiKey: string };

const VERCEL_URL = "https://ai-gateway.vercel.sh/v1";
const OPENROUTER_URL = "https://openrouter.ai/api/v1";

// The daemon does not inherit interactive shell exports, so fall back to the Keychain items that
// shell profiles read. TEXT_MODEL_API_KEY is the existing OpenRouter key on this host.
const KEYCHAIN_SERVICES = ["OPENROUTER_API_KEY", "TEXT_MODEL_API_KEY"];

export type Keychain = (service: string) => Promise<string | null>;

export const readKeychain: Keychain = (service) =>
  new Promise((resolve) => {
    if (process.platform !== "darwin") return resolve(null);
    execFile(
      "security",
      ["find-generic-password", "-a", userInfo().username, "-s", service, "-w"],
      { timeout: 5_000 },
      (error, stdout) => resolve(error ? null : stdout.trim() || null),
    );
  });

// Read on every call so a rotated key applies without a plugin reload.
export async function resolveEndpoint(
  provider: Provider,
  configFile: string,
  env: Record<string, string | undefined> = process.env,
  keychain: Keychain = readKeychain,
): Promise<Endpoint> {
  if (provider === "openrouter") {
    let apiKey = env.OPENROUTER_API_KEY || null;
    for (const service of KEYCHAIN_SERVICES) apiKey ??= await keychain(service);
    if (!apiKey) throw new Error("openrouter credential unavailable");
    return { baseUrl: OPENROUTER_URL, apiKey };
  }
  let gateway: { OPENAI_API_KEY?: unknown; OPENAI_BASE_URL?: unknown } | undefined;
  try {
    gateway = JSON.parse(await readFile(configFile, "utf8"))?.agents?.providers?.["vercel-gateway"]
      ?.env;
  } catch {
    gateway = undefined;
  }
  const apiKey = gateway?.OPENAI_API_KEY;
  if (typeof apiKey !== "string" || !apiKey) throw new Error("vercel credential unavailable");
  const base = gateway?.OPENAI_BASE_URL;
  return {
    baseUrl: (typeof base === "string" && base ? base : VERCEL_URL).replace(/\/+$/, ""),
    apiKey,
  };
}
