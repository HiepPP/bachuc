import { chmod, copyFile, mkdir, readFile, rename, writeFile } from "node:fs/promises";
import { homedir } from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { defaultNativeSettings, nativeSettingsSchema } from "./native-launch";

const quote = (text: string) => "'" + text.replaceAll("'", "'\\''") + "'";
async function readObject(file: string): Promise<Record<string, any>> {
  try {
    return JSON.parse(await readFile(file, "utf8"));
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code === "ENOENT") return {};
    throw error;
  }
}

export async function installNativeHooks(
  userHome: string,
  paseoHome: string,
  root: string,
  node: string,
) {
  const base = path.join(paseoHome, "plugin-data/jev-orchestrator/native");
  await mkdir(base, { recursive: true, mode: 0o700 });
  const settingsPath = path.join(base, "settings.json");
  const existing = await readObject(settingsPath);
  const settings = Object.keys(existing).length
    ? nativeSettingsSchema.parse(existing)
    : defaultNativeSettings();
  const command = [
    node,
    "--import",
    path.join(root, "node_modules/tsx/dist/loader.mjs"),
    path.join(root, "server/native-hook-command.ts"),
  ]
    .map(quote)
    .join(" ");
  const changed: string[] = [];
  for (const [relative, matcher] of [
    [
      ".codex/hooks.json",
      "^(spawn_agent|Agent|collaborationspawn_agent|mcp__jev_orchestrator__prepare_native_delegate)$",
    ],
    [".claude/settings.json", "^(Agent|Task)$"],
  ]) {
    const file = path.join(userHome, relative);
    const config = await readObject(file);
    const hooks = config.hooks ?? {};
    const groups = hooks.PreToolUse ?? [];
    if (!Array.isArray(groups)) throw new Error("Invalid existing PreToolUse hooks.");
    const owned = groups.filter((group: any) =>
      group.hooks?.some((hook: any) => hook.command === command),
    );
    if (owned.length === 1 && owned[0].matcher === matcher) continue;
    const migrateLegacy =
      relative === ".codex/hooks.json" &&
      owned.length === 1 &&
      ["^(spawn_agent|Agent)$", "^(spawn_agent|Agent|collaborationspawn_agent)$"].includes(
        owned[0].matcher,
      ) &&
      owned[0].hooks.length === 1 &&
      owned[0].hooks[0].type === "command" &&
      owned[0].hooks[0].timeout === 30;
    if (owned.length && !migrateLegacy)
      throw new Error("Conflicting Jev hook registration; inspect before changing.");
    const updated = {
      ...config,
      hooks: {
        ...hooks,
        PreToolUse: migrateLegacy
          ? groups.map((group: any) => (group === owned[0] ? { ...group, matcher } : group))
          : [...groups, { matcher, hooks: [{ type: "command", command, timeout: 30 }] }],
      },
    };
    await mkdir(path.dirname(file), { recursive: true });
    try {
      const backup = path.join(base, `${relative.replaceAll("/", "-")}.${Date.now()}.bak`);
      await copyFile(file, backup);
      await chmod(backup, 0o600);
    } catch (error) {
      if ((error as NodeJS.ErrnoException).code !== "ENOENT") throw error;
    }
    const temporary = `${file}.jev-${process.pid}.tmp`;
    await writeFile(temporary, JSON.stringify(updated, null, 2) + "\n", {
      mode: 0o600,
      flag: "wx",
    });
    await rename(temporary, file);
    changed.push(file);
  }
  await writeFile(settingsPath, JSON.stringify(settings, null, 2) + "\n", { mode: 0o600 });
  return { changed, settingsPath, codexTrustRequired: true };
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
  installNativeHooks(
    homedir(),
    process.env.PASEO_HOME || path.join(homedir(), ".paseo"),
    root,
    process.execPath,
  )
    .then((result) => process.stdout.write(JSON.stringify(result) + "\n"))
    .catch(() => {
      process.stderr.write("Native hook installation failed; inspect existing configuration.\n");
      process.exitCode = 1;
    });
}
