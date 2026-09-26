import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtemp, mkdir, writeFile, readFile, readdir } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { createRequire } from "node:module";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { AgentModes } from "../server/modes";
import { translateSettings } from "../shared/settings";
const require = createRequire(import.meta.url);
const { run, digest } = require("../server/caveman-hook.cjs");
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
    `const fs=require('fs');const path=require('path'); exports.getDefaultMode=()=>process.env.CAVEMAN_DEFAULT_MODE||'full'; exports.resolveActiveMode=d=>{try{return JSON.parse(fs.readFileSync(path.join(d,'mode.json'))).mode;}catch{return null;}};`,
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
test("snapshots isolate agents and keep only prompt hashes", async () => {
  const { root, modes, env } = await fixture();
  const settings = translateSettings.schema.parse({});
  await modes.prepare(
    { agentId: A, text: "Giải thích", source: "Giải thích", mode: "lite" },
    settings,
  );
  await modes.set(A, "wenyan-ultra");
  await modes.set(B, "full");
  const pending = path.join(root, "agents", A, "pending");
  const raw = await readFile(path.join(pending, (await readdir(pending))[0]), "utf8");
  assert.ok(!raw.includes("Giải thích"));
  assert.equal(JSON.parse(raw).hash, digest("Giải thích"));
  const out = run({ session_id: "test", prompt: "Giải thích" }, env);
  assert.match(out.hookSpecificOutput.additionalContext, /Native rules lite/);
  assert.match(out.hookSpecificOutput.additionalContext, /Vietnamese/);
  assert.deepEqual(await modes.get(B), { mode: "full" });
  assert.equal((await readdir(pending)).length, 0);
  const next = run({ session_id: "test", prompt: "next" }, env);
  assert.match(next.hookSpecificOutput.additionalContext, /wenyan-ultra/);
});
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
test("explicit commands beat selection; Default resets; cancelled snapshots are removed", async () => {
  const { root, modes, env } = await fixture();
  const settings = translateSettings.schema.parse({});
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
  const { token } = await modes.prepare(
    { agentId: A, text: "request", source: "request", mode: "lite" },
    settings,
  );
  await modes.cancel(A, token);
  assert.deepEqual(await readdir(path.join(root, "agents", A, "pending")), []);
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

test("editing a queue item cancels only its snapshot, including identical prompts", async () => {
  const { modes, env, root } = await fixture();
  const settings = translateSettings.schema.parse({});
  const first = await modes.prepare(
    { agentId: A, text: "same", source: "same", mode: "lite" },
    settings,
  );
  await modes.bindQueue(A, first.token, "queue-one");
  const second = await modes.prepare(
    { agentId: A, text: "same", source: "same", mode: "wenyan-ultra" },
    settings,
  );
  await modes.bindQueue(A, second.token, "queue-two");
  // A fresh instance proves cancellation survives plugin reload.
  await new AgentModes(root).cancelQueue(A, "queue-one");
  const out = run({ prompt: "same" }, env);
  assert.match(out.hookSpecificOutput.additionalContext, /wenyan-ultra/);
  assert.deepEqual(await readdir(path.join(root, "agents", A, "pending")), []);
  await modes.bindQueue(A, second.token, "already-consumed");
  assert.deepEqual(await readdir(path.join(root, "agents", A, "pending")), []);
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
