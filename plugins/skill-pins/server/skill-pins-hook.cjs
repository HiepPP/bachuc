#!/usr/bin/env node
const fs = require("node:fs");
const path = require("node:path");
const os = require("node:os");
const crypto = require("node:crypto");

const uuid = /^[a-f0-9]{8}-(?:[a-f0-9]{4}-){3}[a-f0-9]{12}$/i;
// Must match shared/catalog.ts; tests/hook.test.ts checks both lists stay equal.
const skillIds = ["watchtower", "chase-goal-claude", "sequential-thinking"];
// Entries with `reinvoke: true` in shared/catalog.ts. They run work, so loading them once is not
// enough; the context asks for them again on every turn.
const reinvokeIds = ["chase-goal-claude"];
const DAY = 86400000;
// Must match INITIAL_ENV in shared/settings.ts.
const INITIAL_ENV = "SKILL_PINS_INITIAL";
const digest = (text) =>
  crypto.createHash("sha256").update(text.replace(/\r\n/g, "\n")).digest("hex");
const dataRoot = (env) =>
  path.join(env.PASEO_HOME || path.join(os.homedir(), ".paseo"), "plugin-data/skill-pins");
const known = (skills) => skillIds.filter((id) => Array.isArray(skills) && skills.includes(id));
const escape = (text) => text.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
const requested = (prompt, id) =>
  new RegExp(`(?:^|\\s)(?:/(?:[\\w-]+:)?|\\$)${escape(id)}(?![\\w-])`).test(prompt);
const removeSnapshot = (file) => {
  fs.rmSync(file, { force: true });
  fs.rmSync(file.replace(/\.json$/, ".queue"), { force: true });
};

function readSelection(dir, hash, env) {
  const pending = path.join(dir, "pending");
  const now = Date.now();
  for (const name of fs.existsSync(pending) ? fs.readdirSync(pending).sort() : []) {
    if (!name.endsWith(".json")) continue;
    const file = path.join(pending, name);
    let value;
    try {
      value = JSON.parse(fs.readFileSync(file, "utf8"));
    } catch {
      continue;
    }
    if (now - value.createdAt > DAY) {
      removeSnapshot(file);
      continue;
    }
    if (value.hash === hash) return { skills: known(value.skills), source: "turn", file };
  }
  try {
    const saved = JSON.parse(fs.readFileSync(path.join(dir, "skills.json"), "utf8"));
    return { skills: known(saved.skills), source: "agent" };
  } catch (error) {
    // Until skills.json exists, use the set that agent.create put in the env.
    if (error.code === "ENOENT")
      return { skills: known(String(env[INITIAL_ENV] || "").split(",")), source: "initial" };
    throw error;
  }
}

const HEADERS = {
  codex: {
    once: "The user pinned these skills for this conversation. Before you respond, read each file and follow it unless you already read it in this conversation:",
    every:
      "Read and follow these files again on this turn, even if you already read them earlier in this conversation:",
  },
  claude: {
    once: "The user pinned these skills for this conversation. Before you respond, invoke every one of them with the Skill tool. Skip a skill only if it is already loaded in this conversation:",
    every:
      "Invoke these skills with the Skill tool on this turn, even if you already invoked them earlier in this conversation:",
  },
};

function context(skills, provider, root) {
  const headers = HEADERS[provider === "codex" ? "codex" : "claude"];
  let entry;
  if (provider === "codex") {
    const { skillPaths } = JSON.parse(
      fs.readFileSync(path.join(root, "hook-runtime.json"), "utf8"),
    );
    entry = (id) => {
      if (typeof skillPaths?.[id] !== "string") throw new Error(`No SKILL.md path for ${id}`);
      return `- ${skillPaths[id]}`;
    };
  } else entry = (id) => `- ${id}`;
  const block = (group, header) => (group.length ? [header, ...group.map(entry)] : []);
  return [
    ...block(
      skills.filter((id) => !reinvokeIds.includes(id)),
      headers.once,
    ),
    ...block(
      skills.filter((id) => reinvokeIds.includes(id)),
      headers.every,
    ),
    "Only the skills listed above are pinned now. Stop following any skill that was pinned earlier in this conversation but is missing from this list.",
    "User instructions in the current prompt take priority over pinned skills.",
  ].join("\n");
}

function run(data, env = process.env, provider = "claude") {
  const agentId = env.PASEO_AGENT_ID;
  if (!uuid.test(agentId || "") || typeof data.prompt !== "string") return {};
  const root = dataRoot(env);
  const dir = path.join(root, "agents", agentId);
  if (!fs.existsSync(dir) && !env[INITIAL_ENV]) return {};
  const hash = digest(data.prompt);
  const selection = readSelection(dir, hash, env);
  // An explicit request in the prompt already loads that skill; do not ask twice.
  const skills = selection.skills.filter((id) => !requested(data.prompt, id));
  const additionalContext = skills.length ? context(skills, provider, root) : "";
  fs.mkdirSync(dir, { recursive: true, mode: 0o700 });
  fs.writeFileSync(
    path.join(dir, "last-hook.json"),
    JSON.stringify({
      agentId,
      sessionId: data.session_id,
      skills,
      source: selection.source,
      hash,
      contextBytes: Buffer.byteLength(additionalContext),
      at: new Date().toISOString(),
    }),
    { mode: 0o600 },
  );
  if (selection.file) removeSnapshot(selection.file);
  if (!additionalContext) return {};
  return { hookSpecificOutput: { hookEventName: "UserPromptSubmit", additionalContext } };
}

module.exports = { run, digest, skillIds, reinvokeIds };
if (require.main === module) {
  const provider = process.argv.includes("--codex") ? "codex" : "claude";
  let input = "";
  process.stdin.setEncoding("utf8");
  process.stdin.on("data", (chunk) => {
    input += chunk;
  });
  process.stdin.on("end", () => {
    try {
      process.stdout.write(JSON.stringify(run(JSON.parse(input), process.env, provider)));
    } catch (error) {
      process.stderr.write(`skill-pins hook: ${String(error.message).split("\n")[0]}\n`);
      // Exit 2 would block the user's prompt; a pinned-skill failure must never do that.
      process.exitCode = 1;
    }
  });
}
