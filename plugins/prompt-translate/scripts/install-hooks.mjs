import fs from "node:fs";
import path from "node:path";
import os from "node:os";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const bridge = path.join(root, "server/caveman-hook.cjs");
const remove = process.argv[2] === "--remove";
const cavemanRoot = process.argv[2];
if (!remove) {
  if (!cavemanRoot || !path.isAbsolute(cavemanRoot)) throw new Error("Pass the absolute installed Caveman directory");
  for (const file of ["caveman-mode-tracker.js", "caveman-config.js", "caveman-parse.js"])
    fs.accessSync(path.join(cavemanRoot, "src/hooks", file));
  const dataDir = path.join(process.env.PASEO_HOME || path.join(os.homedir(), ".paseo"), "plugin-data/prompt-translate");
  fs.mkdirSync(dataDir, { recursive: true });
  fs.writeFileSync(path.join(dataDir, "hook-runtime.json"), JSON.stringify({ cavemanRoot }), { mode: 0o600 });
}
const quote = s => "'" + s.replaceAll("'", "'\\''") + "'";
const command = `${quote(process.execPath)} ${quote(bridge)}`;
const codexHooks = path.join(process.env.CODEX_HOME || path.join(os.homedir(), ".codex"), "hooks.json");
const claudeSettings = path.join(process.env.CLAUDE_CONFIG_DIR || path.join(os.homedir(), ".claude"), "settings.json");
// Claude also gets SessionStart and --claude so the bridge can replace the disabled
// native Caveman plugin hooks for Claude sessions outside Paseo.
const targets = [
  { file: codexHooks, command, events: ["UserPromptSubmit"] },
  { file: claudeSettings, command: `${command} --claude`, events: ["UserPromptSubmit", "SessionStart"] },
];
for (const { file, command, events } of targets) {
  if (remove && !fs.existsSync(file)) continue;
  const raw = fs.existsSync(file) ? fs.readFileSync(file, "utf8") : "{}";
  const config = JSON.parse(raw);
  config.hooks ??= {};
  // Match this script path, including registrations made by an older Node executable.
  const ours = hook =>
    hook.type === "command" && [` ${quote(bridge)}`, ` ${quote(bridge)} --claude`].some(end => hook.command?.endsWith(end));
  for (const event of ["UserPromptSubmit", "SessionStart"]) {
    if (!config.hooks[event]) continue;
    config.hooks[event] = config.hooks[event].map(entry => ({ ...entry, hooks: entry.hooks.filter(hook => !ours(hook)) })).filter(entry => entry.hooks.length);
    if (!config.hooks[event].length) delete config.hooks[event];
  }
  if (!remove)
    for (const event of events) (config.hooks[event] ??= []).push({ hooks: [{ type: "command", command, timeout: 10 }] });
  fs.mkdirSync(path.dirname(file), { recursive: true });
  const backup = `${file}.prompt-translate-backup`;
  if (!fs.existsSync(backup)) fs.writeFileSync(backup, raw, { mode: 0o600 });
  const temporary = `${file}.prompt-translate-tmp`;
  fs.writeFileSync(temporary, JSON.stringify(config, null, 2) + "\n", { mode: 0o600 });
  fs.renameSync(temporary, file);
  console.log(`${remove ? "Removed" : "Registered"} prompt-translate ${events.join(", ")}: ${file}`);
}
if (!remove) console.log("Codex: review/trust this hook in /hooks, then reload existing Paseo agents.");
