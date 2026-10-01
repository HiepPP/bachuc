import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtemp, mkdir, writeFile, readFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { createRequire } from "node:module";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { AgentModes } from "../server/modes";
const require = createRequire(import.meta.url);
const { run, daemonContext } = require("../server/caveman-hook.cjs");
const A = "00000000-0000-4000-8000-000000000001",
  B = "00000000-0000-4000-8000-000000000002";
async function fixture(nativeRules = "Native rules") {
  const home = await mkdtemp(path.join(tmpdir(), "pt-modes-"));
  const root = path.join(home, "plugin-data/prompt-translate");
  const native = path.join(home, "caveman/src/hooks");
  await mkdir(native, { recursive: true });
  await mkdir(root, { recursive: true });
  await writeFile(
    path.join(native, "caveman-config.js"),
    `const fs=require('fs');const path=require('path'); exports.getDefaultMode=()=>process.env.CAVEMAN_DEFAULT_MODE||'full'; exports.resolveActiveMode=d=>{try{return JSON.parse(fs.readFileSync(path.join(d,'mode.json'))).mode;}catch{return null;}}; exports.canonicalModeLabel=m=>m==='wenyan'?'wenyan-full':m; exports.loadFilteredRuleset=m=>'Example "Why re-render?"\\n- '+(m==='wenyan'?'wenyan-full':m)+': "'+m+' sample."';`,
  );
  await writeFile(
    path.join(native, "caveman-parse.js"),
    `exports.parseModeChange=(p,o)=>{const m=/^[/$]caveman(?:[ \\t]+(\\S+))?(?:\\s|$)/.exec(p);const mode=m&&(m[1]||o.getDefaultMode());return m ? {action:mode==='off'?'clear':'set',mode} : null;};`,
  );
  await writeFile(
    path.join(native, "caveman-mode-tracker.js"),
    `const fs=require('fs');const path=require('path');let s='';process.stdin.on('data',c=>s+=c);process.stdin.on('end',()=>{const mode=JSON.parse(s).prompt.split(/\\s/)[1]||process.env.CAVEMAN_DEFAULT_MODE||'full';fs.writeFileSync(path.join(process.env.CLAUDE_CONFIG_DIR,'mode.json'),JSON.stringify({mode:mode==='off'?null:mode}));console.log(JSON.stringify({hookSpecificOutput:{additionalContext:mode==='off'||process.env.CAVEMAN_DEFAULT_MODE==='off'?'':${JSON.stringify(nativeRules)}+' '+mode}}));});`,
  );
  await writeFile(
    path.join(native, "caveman-activate.js"),
    `process.stdout.write('Activated '+require('path').basename(process.env.CLAUDE_PLUGIN_ROOT));`,
  );
  await writeFile(
    path.join(root, "hook-runtime.json"),
    JSON.stringify({ cavemanRoot: path.join(home, "caveman") }),
  );
  return {
    home,
    root,
    modes: new AgentModes(root),
    env: { ...process.env, PASEO_HOME: home, PASEO_AGENT_ID: A },
  };
}
test("Caveman compression preserves required response structure after native reinforcement", async () => {
  const { modes, env } = await fixture("No preamble or recap.");
  await modes.set(A, "ultra");
  const context = run({ prompt: "request" }, env).hookSpecificOutput.additionalContext;
  const finalInstruction = context.split("\n\n").at(-1);
  assert.ok(context.indexOf("No preamble or recap.") < context.lastIndexOf(finalInstruction));
  assert.match(
    finalInstruction,
    /Preserve.*headings.*sections.*lists.*tables.*code blocks.*endings.*next-step prompts/,
  );
  assert.match(finalInstruction, /optional repetition, never required sections/);
  assert.match(finalInstruction, /Answer Endings.*Suggested Prompts.*fenced.*prompt:/);
  assert.match(finalInstruction, /Explicit user format requests take priority/);

  const normal = run({ prompt: "$caveman off" }, env).hookSpecificOutput.additionalContext;
  assert.match(normal, /Normal mode\. Stop caveman/);
  assert.doesNotMatch(normal, /Caveman changes wording only/);
});
test("explicit commands beat selection and Default resets", async () => {
  const { modes, env } = await fixture();
  await modes.set(A, "wenyan-ultra");
  assert.match(
    run({ prompt: "$caveman lite\nrequest" }, env).hookSpecificOutput.additionalContext,
    /Native rules lite/,
  );
  await modes.set(A, "follow-agent");
  assert.match(
    run({ prompt: "request" }, env).hookSpecificOutput.additionalContext,
    /Normal mode. Stop caveman/,
  );
  await assert.rejects(modes.set("../outside", "lite"));
  assert.deepEqual(run({ prompt: "request" }, { ...env, PASEO_AGENT_ID: B }), {});
});

test("hook installer preserves unrelated config, is idempotent, and removes only its hook", async () => {
  const { home } = await fixture();
  const codex = path.join(home, "codex"),
    claude = path.join(home, "claude");
  await mkdir(codex);
  await mkdir(claude);
  const existing = {
    enabledPlugins: { existing: true },
    hooks: { UserPromptSubmit: [{ hooks: [{ type: "command", command: "existing-hook" }] }] },
  };
  const files = [path.join(codex, "hooks.json"), path.join(claude, "settings.json")];
  for (const file of files) await writeFile(file, JSON.stringify(existing));
  const { execFileSync } = await import("node:child_process");
  const script = path.resolve("scripts/install-hooks.mjs");
  const env = { ...process.env, CODEX_HOME: codex, CLAUDE_CONFIG_DIR: claude, PASEO_HOME: home };
  for (let i = 0; i < 2; i++)
    execFileSync(process.execPath, [script, path.join(home, "caveman")], { env });
  for (const file of files) {
    const value = JSON.parse(await readFile(file, "utf8"));
    assert.deepEqual(value.enabledPlugins, existing.enabledPlugins);
    assert.equal(value.hooks.UserPromptSubmit.length, 2);
    assert.equal(value.hooks.UserPromptSubmit[0].hooks[0].command, "existing-hook");
  }
  execFileSync(process.execPath, [script, "--remove"], { env });
  for (const file of files) assert.deepEqual(JSON.parse(await readFile(file, "utf8")), existing);
});

test("first-turn command bootstraps isolated mode for later hidden turns", async () => {
  const { modes, env } = await fixture();
  assert.deepEqual(run({ prompt: "hello" }, env), {});
  const first = run({ prompt: "$caveman wenyan-ultra\n\nhello" }, env);
  assert.match(first.hookSpecificOutput.additionalContext, /wenyan-ultra/);
  assert.equal((await modes.get(A)).mode, "wenyan-ultra");
  assert.equal((await modes.get(B)).mode, "follow-agent");
  assert.match(run({ prompt: "next" }, env).hookSpecificOutput.additionalContext, /wenyan-ultra/);
});

test("bridge restores the user default that Paseo hid from native Caveman", async () => {
  const { env } = await fixture();
  const hook = path.join(
    path.dirname(fileURLToPath(import.meta.url)),
    "../server/caveman-hook.cjs",
  );
  const out = JSON.parse(
    execFileSync(process.execPath, [hook], {
      input: JSON.stringify({ prompt: "$caveman\n\nhello" }),
      env: { ...env, CAVEMAN_DEFAULT_MODE: "off", PROMPT_TRANSLATE_CAVEMAN_DEFAULT_MODE: "ultra" },
      encoding: "utf8",
    }),
  );
  assert.match(out.hookSpecificOutput.additionalContext, /Use Caveman ultra/);
  assert.match(out.hookSpecificOutput.additionalContext, /Native rules ultra/);
});

test("a global default of off keeps the selected mode's native rules", async () => {
  const { env } = await fixture();
  const off = { ...env, CAVEMAN_DEFAULT_MODE: "off" };
  assert.match(
    run({ prompt: "$caveman ultra\n\nhello" }, off).hookSpecificOutput.additionalContext,
    /Native rules ultra/,
  );
  assert.match(
    run({ prompt: "next" }, off).hookSpecificOutput.additionalContext,
    /Native rules ultra/,
  );
});

test("Claude sessions outside Paseo run native Caveman hooks against their own config", async () => {
  const { home, env } = await fixture();
  const hook = path.join(
    path.dirname(fileURLToPath(import.meta.url)),
    "../server/caveman-hook.cjs",
  );
  const claudeDir = path.join(home, "claude");
  await mkdir(claudeDir);
  const call = (args: string[], data: object, extra: object) =>
    execFileSync(process.execPath, [hook, ...args], {
      input: JSON.stringify(data),
      env: { ...env, PASEO_AGENT_ID: "", CLAUDE_CONFIG_DIR: claudeDir, ...extra },
      encoding: "utf8",
    });
  const prompt = { hook_event_name: "UserPromptSubmit", prompt: "/caveman lite" };
  assert.match(call(["--claude"], prompt, {}), /Native rules lite/);
  assert.deepEqual(JSON.parse(await readFile(path.join(claudeDir, "mode.json"), "utf8")), {
    mode: "lite",
  });
  assert.equal(call(["--claude"], { hook_event_name: "SessionStart" }, {}), "Activated caveman");
  // Codex registration and Paseo agents never touch the shared Claude config.
  assert.equal(call([], prompt, {}), "{}");
  await writeFile(path.join(claudeDir, "mode.json"), "{}");
  assert.equal(
    call(["--claude"], { hook_event_name: "SessionStart" }, { PASEO_AGENT_ID: A }),
    "{}",
  );
  assert.match(call(["--claude"], prompt, { PASEO_AGENT_ID: A }), /Use Caveman lite/);
  assert.equal(await readFile(path.join(claudeDir, "mode.json"), "utf8"), "{}");
});

test("every active turn carries one example of the running level", async () => {
  const { modes, env } = await fixture();
  await modes.set(A, "ultra");
  const reminder = run({ prompt: "request" }, env).hookSpecificOutput.additionalContext;
  assert.match(
    reminder,
    /Match this ultra density\. Example "Why re-render\?" - ultra: "ultra sample\."/,
  );
  await modes.set(A, "wenyan-ultra");
  assert.match(
    run({ prompt: "next" }, env).hookSpecificOutput.additionalContext,
    /Match this wenyan-ultra density\. .* - wenyan-ultra: "wenyan-ultra sample\."/,
  );
  await modes.set(A, "follow-agent");
  assert.doesNotMatch(
    run({ prompt: "plain" }, env).hookSpecificOutput.additionalContext,
    /density/,
  );
});

test("the example is not repeated when native rules already include it", async () => {
  const { modes, env } = await fixture('- ultra: "ultra sample."');
  await modes.set(A, "ultra");
  const context = run({ prompt: "request" }, env).hookSpecificOutput.additionalContext;
  assert.equal(context.split('- ultra: "ultra sample."').length, 2);
  assert.doesNotMatch(context, /density/);
});

test("the daemon context uses the given choice and a separate state folder", async () => {
  const { home } = await fixture();
  const dir = path.join(home, "plugin-data/prompt-translate/v2/agents", A);
  const runtime = { cavemanRoot: path.join(home, "caveman") };
  const context = daemonContext({
    prompt: "Giải thích",
    agentId: A,
    cwd: home,
    dir,
    runtime,
    choice: { mode: "ultra", replyVietnamese: true, chineseScript: "skill-default" },
  });
  assert.match(context, /Use Caveman ultra/);
  assert.match(context, /Native rules ultra/);
  assert.match(context, /Reply in Vietnamese/);
  // Only the per-agent native folder is written; no mode.json or snapshots.
  assert.deepEqual(JSON.parse(await readFile(path.join(dir, "native/mode.json"), "utf8")), {
    mode: "ultra",
  });
  const plain = daemonContext({
    prompt: "request",
    agentId: A,
    cwd: home,
    dir,
    runtime,
    choice: { mode: "follow-agent", replyVietnamese: false, chineseScript: "skill-default" },
  });
  assert.match(plain, /Normal mode\. Stop caveman/);
});

test("agent prefs merge mode and rewrite and fall back to the default rewrite", async () => {
  const { root } = await fixture();
  const modes = new AgentModes(root);
  assert.equal(await modes.find(A), null);
  assert.deepEqual(await modes.prefs(A, true), { mode: "follow-agent", rewrite: true });
  await modes.update(A, { rewrite: false });
  await modes.update(A, { mode: "lite" });
  assert.deepEqual(await modes.prefs(A, true), { mode: "lite", rewrite: false });
  assert.equal(await modes.find(A), "lite");
});
