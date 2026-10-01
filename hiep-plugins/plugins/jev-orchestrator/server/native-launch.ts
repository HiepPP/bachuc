import { createHash } from "node:crypto";
import { mkdir, readFile, readdir, realpath, writeFile } from "node:fs/promises";
import path from "node:path";
import { parse as parseToml, stringify as stringifyToml } from "smol-toml";
import { parse as parseYaml, stringify as stringifyYaml } from "yaml";
import { z } from "zod";
import { nativePolicySchema } from "./native-hook";
import { nativePreset } from "./native-preset";

type Runtime = "codex" | "claude";
const pair = z.strictObject({
  model: z.string().min(1),
  effort: z.enum(["low", "medium", "high", "xhigh", "max"]),
  description: z.string().min(1),
});
const roleList = z.array(z.string().min(1).max(100)).max(50);
// Claude routes only these source roles unless settings list others. Every routed role adds one
// generated definition per pair to the shared agents folder and to each session's agent catalog.
export const DEFAULT_CLAUDE_ROLES = [
  "general-purpose",
  "Explore",
  "Plan",
  "debugger",
  "code-reviewer",
];
export const nativeSettingsSchema = z.strictObject({
  enabled: z.boolean(),
  pairs: z.strictObject({
    codex: z.array(pair).min(1).max(8),
    claude: z.array(pair).min(1).max(8),
  }),
  // A missing Claude list uses DEFAULT_CLAUDE_ROLES; a missing Codex list routes every Codex role.
  roles: z.strictObject({ codex: roleList.optional(), claude: roleList.optional() }).optional(),
});
export function defaultNativeSettings() {
  const get = (runtime: Runtime) =>
    nativePreset(runtime, ["default"]).routes[0].candidates.map(
      ({ agentType: _type, ...item }) => item,
    );
  return {
    enabled: true,
    pairs: { codex: get("codex"), claude: get("claude") },
    roles: { claude: [...DEFAULT_CLAUDE_ROLES] },
  };
}
const hash = (value: string) => createHash("sha256").update(value).digest("hex");
interface Role {
  fields: Record<string, unknown>;
  body?: string;
}
const warned = new Set<string>();

// Claude Code accepts plain `key: value` frontmatter that strict YAML rejects, such as a
// description containing ": ". Nested values are refused so a dropped `tools` list cannot
// widen the generated role's permissions.
export function parseFlatFrontmatter(text: string): Record<string, string> | undefined {
  const fields: Record<string, string> = {};
  for (const line of text.split(/\r?\n/)) {
    if (!line.trim() || line.trimStart().startsWith("#")) continue;
    const match = /^([A-Za-z_][\w-]*):(?:\s+(.*))?$/.exec(line);
    if (!match || !match[2]?.trim()) return;
    const value = match[2].trim();
    fields[match[1]] = /^(["']).*\1$/.test(value) ? value.slice(1, -1) : value;
  }
  return fields;
}
function parseFrontmatter(text: string, file: string) {
  try {
    return parseYaml(text);
  } catch {
    const fields = parseFlatFrontmatter(text);
    if (fields) return fields;
    if (!warned.has(file))
      console.warn(`Jev native: skipped invalid YAML agent definition: ${file}`);
    warned.add(file);
    return undefined;
  }
}
async function scan(directory: string, runtime: Runtime, roles: Map<string, Role>) {
  let entries;
  try {
    entries = await readdir(directory, { withFileTypes: true });
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code === "ENOENT") return;
    throw error;
  }
  for (const entry of entries.sort((a, b) => a.name.localeCompare(b.name))) {
    if (entry.name.startsWith("jev-native-")) continue;
    const file = path.join(directory, entry.name);
    if (entry.isDirectory()) {
      await scan(file, runtime, roles);
      continue;
    }
    if (!entry.isFile() || !entry.name.endsWith(runtime === "codex" ? ".toml" : ".md")) continue;
    const text = await readFile(file, "utf8");
    if (runtime === "codex") {
      const fields = parseToml(text);
      if (typeof fields.name === "string") roles.set(fields.name, { fields });
    } else {
      const match = /^---\r?\n([\s\S]*?)\r?\n---(?:\r?\n|$)([\s\S]*)$/.exec(text);
      if (!match) continue;
      const fields = parseFrontmatter(match[1], file);
      if (fields && typeof fields.name === "string")
        roles.set(fields.name, { fields, body: match[2] });
    }
  }
}

async function immutable(file: string, text: string) {
  try {
    await writeFile(file, text, { flag: "wx", mode: 0o600 });
  } catch (error) {
    if (
      (error as NodeJS.ErrnoException).code !== "EEXIST" ||
      (await readFile(file, "utf8")) !== text
    )
      throw error;
  }
}

export async function prepareNativeLaunch(
  paseoHome: string,
  userHome: string,
  cwd: string,
  runtime: Runtime,
): Promise<Record<string, string>> {
  const base = path.join(paseoHome, "plugin-data/jev-orchestrator/native");
  let settings;
  try {
    settings = nativeSettingsSchema.parse(
      JSON.parse(await readFile(path.join(base, "settings.json"), "utf8")),
    );
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code === "ENOENT") return {};
    throw error;
  }
  if (!settings.enabled) return {};
  const directory = await realpath(cwd);
  const roles = new Map<string, Role>();
  if (runtime === "codex") {
    roles.set("default", {
      fields: {
        name: "default",
        description: "General delegated work",
        developer_instructions:
          "Complete the delegated task and respect its scope and constraints.",
      },
    });
  } else {
    for (const name of ["general-purpose", "claude"])
      roles.set(name, {
        fields: { name, description: "General delegated work" },
        body: "Complete the delegated task and respect its scope and constraints.\n",
      });
    // Built-in source prompts are not public. These substitutes enforce a narrower read-only tool set.
    for (const name of ["Explore", "Plan"])
      roles.set(name, {
        fields: {
          name,
          description: "Read-only research and planning",
          tools: ["Read", "Grep", "Glob"],
        },
        body: "Research the assigned question read-only. Do not modify files. Return evidence with file references and unresolved questions.\n",
      });
  }
  const agentDirectory = path.join(userHome, `.${runtime}`, "agents");
  await scan(agentDirectory, runtime, roles);
  const ancestors: string[] = [];
  for (let current = directory; ; current = path.dirname(current)) {
    ancestors.unshift(current);
    if (path.dirname(current) === current) break;
  }
  for (const ancestor of ancestors) {
    if (ancestor !== userHome)
      await scan(path.join(ancestor, `.${runtime}`, "agents"), runtime, roles);
  }
  const allowed =
    settings.roles?.[runtime] ?? (runtime === "claude" ? DEFAULT_CLAUDE_ROLES : undefined);
  if (allowed) for (const name of roles.keys()) if (!allowed.includes(name)) roles.delete(name);
  // Unrouted roles reach the provider unchanged, so a list with no known role disables routing.
  if (!roles.size) return {};
  const definitions: { agentType: string; path: string; sha256: string }[] = [];
  const routes = [...roles].map(([sourceType, role]) => ({
    sourceType,
    candidates: settings.pairs[runtime].map((candidate) => {
      // paseoHome keeps each daemon's names apart in the shared agents folders.
      const agentType = `jev-native-${hash(JSON.stringify({ paseoHome, runtime, sourceType, role, candidate })).slice(0, 24)}`;
      const fields = {
        ...role.fields,
        name: agentType,
        description: `Reserved for Paseo Jev routing of ${sourceType}.`,
        model: candidate.model,
      };
      const content =
        runtime === "codex"
          ? stringifyToml({ ...fields, model_reasoning_effort: candidate.effort })
          : `---\n${stringifyYaml({ ...fields, effort: candidate.effort })}---\n${role.body ?? ""}`;
      definitions.push({
        agentType,
        path: path.join(agentDirectory, `${agentType}.${runtime === "codex" ? "toml" : "md"}`),
        sha256: hash(content),
      });
      return Object.assign({}, candidate, { agentType, content });
    }),
  }));
  const policy = nativePolicySchema.parse({
    runtime,
    routes: routes.map((route) => ({
      ...route,
      candidates: route.candidates.map(({ content: _content, ...item }) => item),
    })),
  });
  await mkdir(agentDirectory, { recursive: true });
  await mkdir(base, { recursive: true, mode: 0o700 });
  for (const candidate of routes.flatMap((route) => route.candidates)) {
    const definition = definitions.find((item) => item.agentType === candidate.agentType)!;
    await immutable(definition.path, candidate.content);
  }
  const text = JSON.stringify({ cwd: directory, policy, definitions }, null, 2) + "\n";
  const manifestPath = path.join(base, `${runtime}-${hash(text).slice(0, 24)}.json`);
  await immutable(manifestPath, text);
  return {
    PASEO_JEV_NATIVE_POLICY: manifestPath,
    PASEO_JEV_NATIVE_RUNTIME: runtime,
    PASEO_HOME: paseoHome,
  };
}
