import fs from "node:fs";
import path from "node:path";
import os from "node:os";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const bridge = path.join(root, "server/ponytail-hook.cjs");
const remove = process.argv[2] === "--remove";
const ponytailRoot = process.argv[2];
if (!remove) {
  if (!ponytailRoot || !path.isAbsolute(ponytailRoot))
    throw new Error("Pass the absolute installed Ponytail directory");
  fs.accessSync(path.join(ponytailRoot, "hooks/ponytail-instructions.js"));
  const dataDir = path.join(
    process.env.PASEO_HOME || path.join(os.homedir(), ".paseo"),
    "plugin-data/ponytail",
  );
  fs.mkdirSync(dataDir, { recursive: true });
  fs.writeFileSync(path.join(dataDir, "hook-runtime.json"), JSON.stringify({ ponytailRoot }), {
    mode: 0o600,
  });
}
const quote = (s) => "'" + s.replaceAll("'", "'\\''") + "'";
const command = `${quote(process.execPath)} ${quote(bridge)} --claude`;
const file = path.join(
  process.env.CLAUDE_CONFIG_DIR || path.join(os.homedir(), ".claude"),
  "settings.json",
);
// The bridge replaces the disabled native Ponytail plugin hooks for Claude outside Paseo.
const events = ["SessionStart", "UserPromptSubmit", "SubagentStart"];
if (!remove || fs.existsSync(file)) {
  const raw = fs.existsSync(file) ? fs.readFileSync(file, "utf8") : "{}";
  const config = JSON.parse(raw);
  config.hooks ??= {};
  // Match this script path, including registrations made by an older Node executable.
  const ours = (hook) =>
    hook.type === "command" && hook.command?.endsWith(` ${quote(bridge)} --claude`);
  for (const event of events) {
    if (!config.hooks[event]) continue;
    config.hooks[event] = config.hooks[event]
      .map((entry) => ({ ...entry, hooks: entry.hooks.filter((hook) => !ours(hook)) }))
      .filter((entry) => entry.hooks.length);
    if (!config.hooks[event].length) delete config.hooks[event];
  }
  if (!remove)
    for (const event of events)
      (config.hooks[event] ??= []).push({ hooks: [{ type: "command", command, timeout: 10 }] });
  fs.mkdirSync(path.dirname(file), { recursive: true });
  const backup = `${file}.ponytail-backup`;
  if (!fs.existsSync(backup)) fs.writeFileSync(backup, raw, { mode: 0o600 });
  const temporary = `${file}.ponytail-tmp`;
  fs.writeFileSync(temporary, JSON.stringify(config, null, 2) + "\n", { mode: 0o600 });
  fs.renameSync(temporary, file);
  console.log(`${remove ? "Removed" : "Registered"} ponytail ${events.join(", ")}: ${file}`);
}
