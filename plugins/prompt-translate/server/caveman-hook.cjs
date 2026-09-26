#!/usr/bin/env node
const fs = require("node:fs");
const path = require("node:path");
const os = require("node:os");
const crypto = require("node:crypto");
const { execFileSync } = require("node:child_process");
const uuid = /^[a-f0-9]{8}-(?:[a-f0-9]{4}-){3}[a-f0-9]{12}$/i;
const modes = new Set([
  "follow-agent",
  "lite",
  "full",
  "ultra",
  "wenyan-lite",
  "wenyan-full",
  "wenyan-ultra",
]);
const digest = (text) =>
  crypto.createHash("sha256").update(text.replace(/\r\n/g, "\n")).digest("hex");
const dataRoot = (env) =>
  path.join(env.PASEO_HOME || path.join(os.homedir(), ".paseo"), "plugin-data/prompt-translate");
// The native Claude Caveman plugin is disabled because its hooks write the shared
// ~/.claude flag, leaking one agent's mode into every Claude session. Claude sessions
// outside Paseo get the same Caveman hooks through this bridge instead.
function native(data, env = process.env) {
  const script =
    data.hook_event_name === "SessionStart" ? "caveman-activate.js" : "caveman-mode-tracker.js";
  const { cavemanRoot } = JSON.parse(
    fs.readFileSync(path.join(dataRoot(env), "hook-runtime.json"), "utf8"),
  );
  return execFileSync(process.execPath, [path.join(cavemanRoot, "src/hooks", script)], {
    input: JSON.stringify(data),
    env: { ...env, CLAUDE_PLUGIN_ROOT: cavemanRoot },
    timeout: 4000,
    maxBuffer: 1024 * 1024,
    encoding: "utf8",
  });
}
function example(config, mode, hookDir) {
  if (typeof config.loadFilteredRuleset !== "function") return null;
  const label = config.canonicalModeLabel?.(mode) ?? mode;
  const lines = (config.loadFilteredRuleset(mode, hookDir) || "").split("\n");
  const at = lines.findIndex((line) => line.startsWith(`- ${label}: `));
  const question = lines.slice(0, at).findLast((line) => line.startsWith('Example "'));
  return at < 0 || !question ? null : { label, question, answer: lines[at] };
}
function run(data, env = process.env) {
  const agentId = env.PASEO_AGENT_ID;
  if (!uuid.test(agentId || "") || typeof data.prompt !== "string") return {};
  const root = dataRoot(env);
  const dir = path.join(root, "agents", agentId);
  const modeFile = path.join(dir, "mode.json");
  const initial = !fs.existsSync(modeFile);
  // A new composer has no agent UUID yet. Its explicit first-turn command bootstraps
  // isolated state; later turns use the usual hidden context and mode RPCs.
  if (initial && !/^[/$]caveman(?::caveman)?(?:\s|$)/i.test(data.prompt)) return {};
  const runtime = JSON.parse(fs.readFileSync(path.join(root, "hook-runtime.json"), "utf8"));
  const selected = initial
    ? { mode: "follow-agent" }
    : JSON.parse(fs.readFileSync(modeFile, "utf8"));
  const hash = digest(data.prompt);
  let choice = selected;
  let consumed;
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
    if (now - value.createdAt > 86400000) {
      fs.unlinkSync(file);
      fs.rmSync(file.replace(/\.json$/, ".queue"), { force: true });
      continue;
    }
    if (value.hash === hash) {
      choice = value;
      consumed = file;
      break;
    }
  }
  if (!modes.has(choice.mode)) throw new Error("Invalid Caveman mode");
  const hookDir = path.join(runtime.cavemanRoot, "src/hooks");
  const { parseModeChange } = require(path.join(hookDir, "caveman-parse.js"));
  const config = require(path.join(hookDir, "caveman-config.js"));
  // Parsing and mode rules belong to Caveman, including explicit off/natural-language commands.
  const userPrompt = data.prompt.replace(/^\$caveman\b/, "/caveman");
  const change = parseModeChange(userPrompt, {
    getDefaultMode: () => config.getDefaultMode(data.cwd),
  });
  if (initial) {
    const firstMode =
      change?.action === "set" && modes.has(change.mode) ? change.mode : "follow-agent";
    selected.mode = firstMode;
    fs.mkdirSync(dir, { recursive: true });
    fs.writeFileSync(modeFile, JSON.stringify(selected), { mode: 0o600 });
  }
  const requested = choice.mode === "follow-agent" ? "off" : choice.mode;
  const hookPrompt = change ? userPrompt : `/caveman ${requested}`;
  const nativeDir = path.join(dir, "native");
  fs.mkdirSync(nativeDir, { recursive: true });
  const trackerEnv = { ...env, CLAUDE_CONFIG_DIR: nativeDir };
  // A global defaultMode "off" silences native Claude Caveman, but the tracker also drops
  // the ruleset for any active mode under "off". The mode chosen here must keep its rules.
  const trackerMode = change ? (change.action === "set" ? change.mode : null) : requested;
  if (trackerMode && trackerMode !== "off") trackerEnv.CAVEMAN_DEFAULT_MODE = trackerMode;
  const result = execFileSync(process.execPath, [path.join(hookDir, "caveman-mode-tracker.js")], {
    input: JSON.stringify({ ...data, session_id: data.session_id || agentId, prompt: hookPrompt }),
    env: trackerEnv,
    timeout: 4000,
    maxBuffer: 1024 * 1024,
    encoding: "utf8",
  });
  const output = result.trim() ? JSON.parse(result) : {};
  const active = config.resolveActiveMode(nativeDir, data.session_id || agentId);
  const context = [
    "For this turn, replace earlier Caveman mode and language overrides with the mode below. Explicit language requests in the current user prompt take priority.",
  ];
  if (active) context.push(`Use Caveman ${active} for this turn.`);
  if (String(active).startsWith("wenyan"))
    context.push(
      "Respond in literary Chinese for this turn unless the user explicitly requests another language.",
    );
  const nativeContext = output.hookSpecificOutput?.additionalContext || "";
  if (nativeContext) context.push(nativeContext);
  // Later turns get only Caveman's one-line reminder, and replies drift back to prose.
  // One level example from Caveman's own SKILL.md anchors the expected density.
  const sample = active && example(config, active, hookDir);
  if (sample && !nativeContext.includes(sample.answer))
    context.push(`Match this ${sample.label} density. ${sample.question} ${sample.answer}`);
  if (!active)
    context.push(
      "Normal mode. Stop caveman. Reset earlier Caveman style and Wenyan language instructions. Follow the agent's normal response style and the current request's language unless explicitly requested otherwise.",
    );
  if (!change && !String(active).startsWith("wenyan")) {
    if (choice.replyVietnamese)
      context.push(
        "Reply in Vietnamese unless the current request explicitly asks for another language. This replaces earlier Wenyan language choices.",
      );
    else
      context.push(
        "Use the current request's language unless explicitly requested otherwise; replace any earlier Wenyan language choice.",
      );
  }
  if (!change && String(active).startsWith("wenyan") && choice.chineseScript === "simplified")
    context.push(
      "Use Simplified Chinese characters (简体字) unless the current request explicitly asks for another script.",
    );
  if (active)
    context.push(
      "Caveman changes wording only, not required response format or content. Preserve headings, sections, lists, tables, code blocks, endings, and next-step prompts required by the task or applicable instructions. Any 'No preamble or recap' instruction above removes optional repetition, never required sections. Before sending, verify required Answer Endings and Suggested Prompts remain, including each required fenced prompt: suggestion. Explicit user format requests take priority.",
    );
  const additionalContext = context.join("\n\n");
  fs.writeFileSync(
    path.join(dir, "last-hook.json"),
    JSON.stringify({
      agentId,
      sessionId: data.session_id,
      mode: active || "off",
      selectedMode: choice.mode,
      source: consumed ? "turn" : "agent",
      hash,
      contextBytes: Buffer.byteLength(additionalContext),
      at: new Date().toISOString(),
    }),
  );
  if (consumed) {
    fs.unlinkSync(consumed);
    fs.rmSync(consumed.replace(/\.json$/, ".queue"), { force: true });
  }
  return {
    ...output,
    hookSpecificOutput: { hookEventName: "UserPromptSubmit", additionalContext },
  };
}
module.exports = { run, native, digest };
if (require.main === module) {
  // Only the Claude registration passes --claude; Codex keeps its own Caveman plugin.
  const outside = process.argv.includes("--claude") && !uuid.test(process.env.PASEO_AGENT_ID || "");
  // Paseo launches Claude with CAVEMAN_DEFAULT_MODE=off only to silence the native
  // Caveman plugin; bare commands handled here still use the user's own default.
  const saved = process.env.PROMPT_TRANSLATE_CAVEMAN_DEFAULT_MODE;
  if (saved) process.env.CAVEMAN_DEFAULT_MODE = saved;
  else if (saved === "") delete process.env.CAVEMAN_DEFAULT_MODE;
  let input = "";
  process.stdin.setEncoding("utf8");
  process.stdin.on("data", (chunk) => {
    input += chunk;
  });
  process.stdin.on("end", () => {
    try {
      const data = JSON.parse(input);
      process.stdout.write(outside ? native(data) : JSON.stringify(run(data)));
    } catch (error) {
      process.stderr.write(`prompt-translate hook: ${error.message.split("\n")[0]}\n`);
      // Exit 2 blocks the prompt; a native Caveman failure must not block plain Claude.
      process.exitCode = outside ? 1 : 2;
    }
  });
}
