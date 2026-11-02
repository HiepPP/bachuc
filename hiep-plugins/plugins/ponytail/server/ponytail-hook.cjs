#!/usr/bin/env node
const fs = require("node:fs");
const path = require("node:path");
const os = require("node:os");
const { execFileSync } = require("node:child_process");
const uuid = /^[a-f0-9]{8}-(?:[a-f0-9]{4}-){3}[a-f0-9]{12}$/i;
const scripts = {
  SessionStart: "ponytail-activate.js",
  UserPromptSubmit: "ponytail-mode-tracker.js",
  SubagentStart: "ponytail-subagent.js",
};
const dataRoot = (env) =>
  path.join(env.PASEO_HOME || path.join(os.homedir(), ".paseo"), "plugin-data/ponytail");

// The native Ponytail Claude plugin stays disabled: its mode flag is shared by every Claude
// session. Paseo agents get their rules from the daemon hook, so this bridge serves only
// Claude sessions outside Paseo, with the native scripts unchanged.
function native(data, env = process.env) {
  const script = scripts[data.hook_event_name];
  if (!script) return "";
  const { ponytailRoot } = JSON.parse(
    fs.readFileSync(path.join(dataRoot(env), "hook-runtime.json"), "utf8"),
  );
  return execFileSync(process.execPath, [path.join(ponytailRoot, "hooks", script)], {
    input: JSON.stringify(data),
    env: { ...env, CLAUDE_PLUGIN_ROOT: ponytailRoot },
    timeout: 8000,
    maxBuffer: 1024 * 1024,
    encoding: "utf8",
  });
}

function run(data, env = process.env) {
  return uuid.test(env.PASEO_AGENT_ID || "") ? "" : native(data, env);
}
module.exports = { run, native };

if (require.main === module) {
  let input = "";
  process.stdin.setEncoding("utf8");
  process.stdin.on("data", (chunk) => {
    input += chunk;
  });
  process.stdin.on("end", () => {
    try {
      process.stdout.write(run(JSON.parse(input.replace(/^\uFEFF/, ""))));
    } catch (error) {
      process.stderr.write(`ponytail hook: ${String(error.message).split("\n")[0]}\n`);
      // Exit 2 blocks the user's Claude prompt; a Ponytail failure must never do that.
      process.exitCode = 1;
    }
  });
}
