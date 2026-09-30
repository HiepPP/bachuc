import fs from "node:fs";
import path from "node:path";
import os from "node:os";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const hook = path.join(root, "server/skill-pins-hook.cjs");
const remove = process.argv[2] === "--remove";
const skillsRoot = process.argv[2];
const skillIds = ["watchtower", "chase-goal-claude", "sequential-thinking"];
if (!remove) {
  if (!skillsRoot || !path.isAbsolute(skillsRoot))
    throw new Error("Pass the absolute skills directory, for example ~/.claude/skills");
  const skillPaths = Object.fromEntries(
    skillIds.map((id) => [id, path.join(skillsRoot, id, "SKILL.md")]),
  );
  // A missing skill fails the install instead of producing a dead path in every Codex turn.
  for (const file of Object.values(skillPaths)) fs.accessSync(file, fs.constants.R_OK);
  const dataDir = path.join(
    process.env.PASEO_HOME || path.join(os.homedir(), ".paseo"),
    "plugin-data/skill-pins",
  );
  fs.mkdirSync(dataDir, { recursive: true });
  fs.writeFileSync(path.join(dataDir, "hook-runtime.json"), JSON.stringify({ skillPaths }), {
    mode: 0o600,
  });
}
const quote = (s) => "'" + s.replaceAll("'", "'\\''") + "'";
const command = `${quote(process.execPath)} ${quote(hook)}`;
const targets = [
  {
    file: path.join(process.env.CODEX_HOME || path.join(os.homedir(), ".codex"), "hooks.json"),
    command: `${command} --codex`,
  },
  {
    file: path.join(
      process.env.CLAUDE_CONFIG_DIR || path.join(os.homedir(), ".claude"),
      "settings.json",
    ),
    command: `${command} --claude`,
  },
];
// Match this script path, including registrations made by an older Node executable.
const ours = (entry) =>
  entry.type === "command" &&
  [" --codex", " --claude"].some((flag) => entry.command?.endsWith(` ${quote(hook)}${flag}`));
for (const { file, command } of targets) {
  if (remove && !fs.existsSync(file)) continue;
  const raw = fs.existsSync(file) ? fs.readFileSync(file, "utf8") : "{}";
  const config = JSON.parse(raw);
  config.hooks ??= {};
  if (config.hooks.UserPromptSubmit) {
    config.hooks.UserPromptSubmit = config.hooks.UserPromptSubmit.map((group) => ({
      ...group,
      hooks: group.hooks.filter((entry) => !ours(entry)),
    })).filter((group) => group.hooks.length);
    if (!config.hooks.UserPromptSubmit.length) delete config.hooks.UserPromptSubmit;
  }
  if (!remove)
    (config.hooks.UserPromptSubmit ??= []).push({
      hooks: [{ type: "command", command, timeout: 10 }],
    });
  fs.mkdirSync(path.dirname(file), { recursive: true });
  const backup = `${file}.skill-pins-backup`;
  if (!fs.existsSync(backup)) fs.writeFileSync(backup, raw, { mode: 0o600 });
  const temporary = `${file}.skill-pins-tmp`;
  fs.writeFileSync(temporary, JSON.stringify(config, null, 2) + "\n", { mode: 0o600 });
  fs.renameSync(temporary, file);
  console.log(`${remove ? "Removed" : "Registered"} skill-pins UserPromptSubmit: ${file}`);
}
if (!remove)
  console.log("Codex: review/trust this hook in /hooks, then reload existing Paseo agents.");
