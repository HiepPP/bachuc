import semver from "semver";
import type { Category, Config, Measurement } from "../shared/preflight";
import { metadata, present } from "./metadata";

export type Discovered = {
  config: Config;
  nodeConstraints?: string[];
  sources: Partial<Record<Category, string>>;
  unknowns: { category: Category; source: string; measurement: Measurement }[];
};
const locks: Record<string, string> = {
  "package-lock.json": "npm",
  "npm-shrinkwrap.json": "npm",
  "pnpm-lock.yaml": "pnpm",
  "yarn.lock": "yarn",
  "bun.lock": "bun",
  "bun.lockb": "bun",
};
export async function discover(root: string, explicit: Config): Promise<Discovered> {
  const result: Discovered = { config: { version: 1 }, sources: {}, unknowns: [] };
  const unknown = (category: Category, source: string, detail: string) =>
    result.unknowns.push({
      category,
      source,
      measurement: {
        label: category === "runtimes" ? "Node requirement" : "Dependency layout",
        status: "unknown",
        detail,
        repair: "",
      },
    });
  let pkg: Record<string, unknown> = {};
  let packageValid = true;
  try {
    const raw = await metadata(root, "package.json");
    if (raw !== null) {
      const parsed = JSON.parse(raw);
      if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) throw new Error();
      pkg = parsed;
    }
  } catch {
    packageValid = false;
  }
  if (explicit.runtimes === undefined) {
    const ranges: string[] = [];
    const sources: string[] = [];
    let invalid = !packageValid;
    const add = (value: unknown, source: string) => {
      sources.push(source);
      if (
        typeof value !== "string" ||
        value.length > 200 ||
        !value.trim() ||
        !semver.validRange(value.trim()) ||
        value.split("||").length > 8
      )
        invalid = true;
      else ranges.push(value.trim());
    };
    const engines = pkg.engines;
    if (engines !== undefined) {
      if (!engines || typeof engines !== "object" || Array.isArray(engines)) invalid = true;
      else if ("node" in engines) add(engines.node, "package.json#engines.node");
    }
    for (const file of [".nvmrc", ".node-version"]) {
      try {
        const value = await metadata(root, file, 256);
        if (value !== null) add(value, file);
      } catch {
        sources.push(file);
        invalid = true;
      }
    }
    const source = sources.join(", ") || "package.json#engines.node, .nvmrc, .node-version";
    if (invalid)
      unknown(
        "runtimes",
        source,
        "Unreadable metadata, unsupported alias, or invalid Node range; no version file was executed.",
      );
    else if (!ranges.length) unknown("runtimes", source, "No Node version requirement declared.");
    else {
      let combinations = [""];
      for (const range of ranges)
        combinations = combinations.flatMap((left) =>
          new semver.Range(range).set.map((set) =>
            `${left} ${set.map((c) => c.value).join(" ")}`.trim(),
          ),
        );
      const compatible = combinations.filter((range) => {
        const min = semver.minVersion(range);
        if (!min) return false;
        const candidates = [min.version, `${min.major}.${min.minor}.${min.patch}`];
        return candidates.some(
          (candidate) =>
            semver.satisfies(candidate, range) &&
            ranges.every((r) => semver.satisfies(candidate, r)),
        );
      });
      if (!compatible.length)
        unknown(
          "runtimes",
          source,
          "Declared Node constraints conflict; no common supported version.",
        );
      else if (compatible.join(" || ").length > 200)
        unknown(
          "runtimes",
          source,
          "Combined Node range exceeds supported complexity; configure a simpler explicit range.",
        );
      else {
        result.nodeConstraints = ranges;
        result.config.runtimes = [
          {
            executable: "node",
            range: compatible.join(" || "),
            repair: "Use a Node runtime satisfying the declared constraints.",
          },
        ];
        result.sources.runtimes = source;
      }
    }
  }
  if (explicit.dependencies === undefined) {
    const source = "package.json#packageManager; root lockfiles/layout markers";
    try {
      const found: string[] = [];
      for (const file of Object.keys(locks)) if (await present(root, file)) found.push(file);
      const installConfig = pkg.installConfig;
      const declaredPnp =
        installConfig !== undefined &&
        (!installConfig ||
          typeof installConfig !== "object" ||
          Array.isArray(installConfig) ||
          ("pnp" in installConfig && installConfig.pnp !== false));
      const pnp =
        declaredPnp || (await present(root, ".pnp.cjs")) || (await present(root, ".pnp.js"));
      const monorepo =
        pkg.workspaces !== undefined ||
        (await present(root, "pnpm-workspace.yaml")) ||
        (await present(root, "lerna.json"));
      const declaration = pkg.packageManager;
      const declared =
        typeof declaration === "string"
          ? /^(npm|pnpm|yarn|bun)@(\d+\.\d+\.\d+(?:-[\w.-]+)?)(?:\+sha(?:224|256|384|512)\.[a-fA-F0-9]+)?$/.exec(
              declaration,
            )
          : null;
      const manager = found.length === 1 ? locks[found[0]] : undefined;
      if (!packageValid || monorepo)
        unknown(
          "dependencies",
          source,
          "Unreadable package metadata or unsupported monorepo layout; configure dependencies explicitly.",
        );
      else if (
        found.length !== 1 ||
        (declaration !== undefined && !declared) ||
        (declared && declared[1] !== manager)
      )
        unknown(
          "dependencies",
          source,
          "Missing, multiple, or conflicting package-manager declarations/lockfiles; installation command unknown.",
        );
      else if (
        pnp ||
        (manager === "yarn" && (!declared || Number(declared[2].split(".")[0]) >= 2)) ||
        manager === "bun" ||
        (manager === "pnpm" && (await present(root, ".npmrc")))
      )
        unknown(
          "dependencies",
          source,
          "PnP or unsupported/unclear linker layout; node_modules is not assumed. Configure dependencies explicitly.",
        );
      else {
        result.config.dependencies = [
          {
            label: "Installed dependencies",
            path: "node_modules",
            repair: "Review the declared package manager and install dependencies manually.",
          },
        ];
        result.sources.dependencies = `${found[0]}${declared ? "; package.json#packageManager" : ""}; node_modules`;
      }
    } catch {
      unknown(
        "dependencies",
        source,
        "Unsafe or unreadable lockfile/layout marker; dependency layout unknown.",
      );
    }
  }
  for (const category of ["ports", "health"] as const)
    if (explicit[category] === undefined)
      result.unknowns.push({
        category,
        source: ".paseo/preflight.json (not configured)",
        measurement: {
          label: category === "ports" ? "Service ports" : "Backend health",
          status: "unknown",
          detail:
            "Not configured; service requirements must come from the task. Script text is not inspected.",
          repair: "",
        },
      });
  return result;
}
