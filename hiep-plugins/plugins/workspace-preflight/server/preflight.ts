import { execFile } from "node:child_process";
import { constants } from "node:fs";
import { access, realpath, stat } from "node:fs/promises";
import semver from "semver";
import { discover } from "./discovery";
import { inside, metadata } from "./metadata";
import net from "node:net";
import path from "node:path";
import {
  configSchema,
  type Check,
  type Config,
  type Report,
  type Measurement,
  type Category,
} from "../shared/preflight";

const TIMEOUT = 1500;
const CONFIG_PATH = ".paseo/preflight.json";
function errorCode(error: unknown): string {
  return (error as NodeJS.ErrnoException)?.code ?? "UNKNOWN";
}
async function runtimeCheck(
  root: string,
  item: NonNullable<Config["runtimes"]>[number],
  constraints?: string[],
): Promise<Measurement> {
  const expected = item.range ?? `${item.major}.x`;
  const label = `${item.executable} ${expected}`;
  if (!semver.validRange(expected))
    return {
      label,
      status: "blocker",
      detail: "Invalid explicit version range.",
      repair: item.repair,
    };
  let executable: string | undefined;
  for (const directory of (process.env.PATH ?? "").split(path.delimiter)) {
    if (!path.isAbsolute(directory)) continue;
    try {
      const candidate = await realpath(path.join(directory, item.executable));
      if (inside(root, candidate))
        return {
          label,
          status: "unknown",
          detail: "Runtime resolves inside the workspace; repository executables are not probed.",
          repair: item.repair,
        };
      await access(candidate, constants.X_OK);
      if (!(await stat(candidate)).isFile()) continue;
      executable = candidate;
      break;
    } catch {
      /* Continue searching the trusted daemon PATH. */
    }
  }
  if (!executable)
    return {
      label,
      status: "blocker",
      detail: "Runtime not found on daemon PATH.",
      repair: item.repair,
    };
  return new Promise((resolve) => {
    // Fixed argv only. Repository repair text never reaches a process API.
    execFile(
      executable,
      ["--version"],
      {
        cwd: root,
        timeout: TIMEOUT,
        killSignal: "SIGKILL",
        maxBuffer: 4096,
        env: { ...process.env, NODE_OPTIONS: "", PYTHONSTARTUP: "", ELECTRON_RUN_AS_NODE: "1" },
      },
      (error, stdout) => {
        if (error) {
          resolve({
            label,
            status: errorCode(error) === "ENOENT" ? "blocker" : "unknown",
            detail:
              errorCode(error) === "ENOENT"
                ? "Runtime not found on daemon PATH."
                : "Version probe failed or timed out.",
            repair: item.repair,
          });
          return;
        }
        const match = stdout
          .trim()
          .match(
            /^(?:v|Python |deno )?(\d+)\.(\d+)(?:\.(\d+))?(-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?(?:\s|$)/,
          );
        resolve({
          label,
          status: !match
            ? "unknown"
            : (constraints ?? [expected]).every((range) =>
                  semver.satisfies(
                    `${match[1]}.${match[2]}.${match[3] ?? "0"}${match[4] ?? ""}`,
                    range,
                  ),
                )
              ? "pass"
              : "blocker",
          detail: match
            ? `Detected ${match[0]}; expected ${item.range ? expected : `major ${item.major}`}.`
            : "Unrecognized version output.",
          repair: item.repair,
        });
      },
    );
  });
}
async function dependencyCheck(
  root: string,
  item: NonNullable<Config["dependencies"]>[number],
): Promise<Measurement> {
  try {
    const target = await realpath(path.resolve(root, item.path));
    if (!inside(root, target))
      return {
        ...item,
        status: "unknown",
        detail: "Dependency path resolves outside this workspace.",
      };
    const info = await stat(target);
    return {
      ...item,
      status: info.isFile() || info.isDirectory() ? "pass" : "unknown",
      detail: `${item.path}: ${info.isFile() || info.isDirectory() ? "present" : "not a regular file or directory"}.`,
    };
  } catch (error) {
    const missing = ["ENOENT", "ENOTDIR"].includes(errorCode(error));
    return {
      ...item,
      status: missing ? "blocker" : "unknown",
      detail: missing ? `${item.path}: missing.` : "Could not inspect dependency path.",
    };
  }
}
async function portCheck(item: NonNullable<Config["ports"]>[number]): Promise<Measurement> {
  return new Promise((resolve) => {
    const socket = net.createConnection({ host: "127.0.0.1", port: item.port });
    let settled = false;
    const finish = (observed: "listening" | "closed" | "unknown") => {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      socket.destroy();
      resolve({
        label: item.label,
        repair: item.repair,
        status: observed === "unknown" ? "unknown" : observed === item.expect ? "pass" : "blocker",
        detail: `127.0.0.1:${item.port}: ${observed}; expected ${item.expect}.`,
      });
    };
    const timer = setTimeout(() => finish("unknown"), TIMEOUT);
    socket.once("connect", () => finish("listening"));
    socket.once("error", (error) =>
      finish(errorCode(error) === "ECONNREFUSED" ? "closed" : "unknown"),
    );
  });
}
async function healthCheck(item: NonNullable<Config["health"]>[number]): Promise<Measurement> {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), TIMEOUT);
  try {
    const response = await fetch(item.url, {
      method: "GET",
      redirect: "manual",
      signal: controller.signal,
    });
    await response.body?.cancel();
    return {
      label: item.label,
      repair: item.repair,
      status: response.status === item.status ? "pass" : "blocker",
      detail: `HTTP ${response.status}; expected ${item.status}.`,
    };
  } catch (error) {
    const timeout =
      (error as Error).name === "TimeoutError" || (error as Error).name === "AbortError";
    return {
      label: item.label,
      repair: item.repair,
      status: timeout ? "unknown" : "blocker",
      detail: timeout
        ? "Health request timed out."
        : "Health endpoint unreachable or TLS validation failed.",
    };
  } finally {
    clearTimeout(timer);
  }
}

export async function runPreflight(directory: string): Promise<Report> {
  const checkedAt = new Date().toISOString();
  const coverage =
    "Task requirements must be checked separately; passing measurements do not prove readiness." as const;
  const evidence = (
    measurement: Measurement,
    category: Category,
    source: string,
    index: number,
  ): Check => ({
    ...measurement,
    id: `${category}:${index}`,
    category,
    source,
    observation: measurement.detail,
    checkedAt,
  });
  const failed = (label: string, detail: string): Report => ({
    checkedAt,
    mode: "explicit",
    coverage,
    checks: [
      evidence({ label, detail, status: "blocker", repair: "" }, "configuration", CONFIG_PATH, 0),
    ],
  });
  let root: string;
  try {
    root = await realpath(directory);
    if (!(await stat(root)).isDirectory()) throw new Error();
  } catch {
    return failed("Workspace unavailable", "Creating workspace is missing or unreadable.");
  }
  let config: Config = { version: 1 };
  let configured = false;
  try {
    const raw = await metadata(root, CONFIG_PATH);
    if (raw !== null) {
      config = configSchema.parse(JSON.parse(raw));
      configured = true;
    }
    if (config.runtimes?.some((item) => item.range && !semver.validRange(item.range)))
      throw new Error();
  } catch {
    return failed(
      "Configuration",
      `Cannot read ${CONFIG_PATH}: invalid JSON/schema/range, unsafe path, unreadable file, or size over 64 KiB.`,
    );
  }
  const categories = ["runtimes", "dependencies", "ports", "health"] as const;
  const discovered = await discover(root, config);
  const mode = !configured
    ? "discovery"
    : categories.every((c) => config[c] !== undefined)
      ? "explicit"
      : "mixed";
  const checks: Check[] = [];
  const jobs: (() => Promise<Check>)[] = [];
  const engines = {
    runtimes: (root: string, item: NonNullable<Config["runtimes"]>[number]) =>
      runtimeCheck(
        root,
        item,
        config.runtimes === undefined ? discovered.nodeConstraints : undefined,
      ),
    dependencies: dependencyCheck,
    ports: (_root: string, item: NonNullable<Config["ports"]>[number]) => portCheck(item),
    health: (_root: string, item: NonNullable<Config["health"]>[number]) => healthCheck(item),
  };
  for (const category of categories) {
    const values = config[category] ?? discovered.config[category];
    const source =
      config[category] !== undefined
        ? `${CONFIG_PATH}#${category}`
        : (discovered.sources[category] ?? "metadata discovery");
    if (values?.length === 0)
      checks.push(
        evidence(
          {
            label: category,
            status: "unknown",
            detail: "Explicit empty category: checks disabled; task coverage not established.",
            repair: "",
          },
          category,
          source,
          0,
        ),
      );
    values?.forEach((item, index) =>
      jobs.push(async () =>
        evidence(
          await (
            engines[category] as (
              root: string,
              item: NonNullable<typeof values>[number],
            ) => Promise<Measurement>
          )(root, item),
          category,
          source,
          index,
        ),
      ),
    );
  }
  for (const item of discovered.unknowns)
    checks.push(evidence(item.measurement, item.category, item.source, 0));
  for (let i = 0; i < jobs.length; i += 4)
    checks.push(...(await Promise.all(jobs.slice(i, i + 4).map((job) => job()))));
  return { checkedAt, mode, coverage, checks };
}
