import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtemp, mkdir, readFile, readdir, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { createRequire } from "node:module";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { performance } from "node:perf_hooks";
import { SkillPins } from "../server/state";
import { catalog } from "../shared/catalog";

const require = createRequire(import.meta.url);
const { run, skillIds, reinvokeIds } = require("../server/skill-pins-hook.cjs");
const hookFile = fileURLToPath(new URL("../server/skill-pins-hook.cjs", import.meta.url));
const A = "00000000-0000-4000-8000-000000000001";

async function fixture() {
  const home = await mkdtemp(path.join(tmpdir(), "skill-pins-hook-"));
  const root = path.join(home, "plugin-data/skill-pins");
  await mkdir(root, { recursive: true });
  const skillPaths = Object.fromEntries(
    skillIds.map((id: string) => [id, `/skills/${id}/SKILL.md`]),
  );
  await writeFile(path.join(root, "hook-runtime.json"), JSON.stringify({ skillPaths }));
  const env = { PASEO_HOME: home, PASEO_AGENT_ID: A };
  const pending = path.join(root, "agents", A, "pending");
  return { home, root, env, pins: new SkillPins(root), pending };
}
const contextOf = (output: { hookSpecificOutput?: { additionalContext: string } }) =>
  output.hookSpecificOutput?.additionalContext ?? "";

test("hook skill IDs match the shared catalog", () => {
  assert.deepEqual(
    skillIds,
    catalog.map((skill) => skill.id),
  );
  assert.deepEqual(
    reinvokeIds,
    catalog.filter((skill) => "reinvoke" in skill && skill.reinvoke).map((skill) => skill.id),
  );
});

test("a reinvoke skill gets its own block that overrides already loaded", async () => {
  const { env, pins } = await fixture();
  await pins.set(A, ["watchtower", "chase-goal-claude"]);
  const text = contextOf(run({ prompt: "hi" }, env, "claude"));
  const once = text.indexOf("Skip a skill only if it is already loaded");
  const every = text.indexOf("even if you already invoked them earlier");
  assert.ok(once >= 0 && every > once, "both blocks appear, the reinvoke block last");
  // Each skill sits under its own header.
  assert.match(text.slice(once, every), /^- watchtower$/m);
  assert.doesNotMatch(text.slice(once, every), /chase-goal-claude/);
  assert.match(text.slice(every), /^- chase-goal-claude$/m);
});

test("only reinvoke skills drops the already loaded block", async () => {
  const { env, pins } = await fixture();
  await pins.set(A, ["chase-goal-claude"]);
  const text = contextOf(run({ prompt: "hi" }, env, "claude"));
  assert.doesNotMatch(text, /Skip a skill only if it is already loaded/);
  assert.match(text, /even if you already invoked them earlier/);
});

test("only normal skills drops the reinvoke block", async () => {
  const { env, pins } = await fixture();
  await pins.set(A, ["sequential-thinking"]);
  const text = contextOf(run({ prompt: "hi" }, env, "claude"));
  assert.match(text, /Skip a skill only if it is already loaded/);
  assert.doesNotMatch(text, /even if you already invoked/);
});

test("Codex splits the same two blocks by absolute path", async () => {
  const { env, pins } = await fixture();
  await pins.set(A, ["watchtower", "chase-goal-claude"]);
  const text = contextOf(run({ prompt: "hi" }, env, "codex"));
  const every = text.indexOf("Read and follow these files again on this turn");
  assert.ok(every > 0);
  assert.match(text.slice(0, every), /^- \/skills\/watchtower\/SKILL\.md$/m);
  assert.match(text.slice(every), /^- \/skills\/chase-goal-claude\/SKILL\.md$/m);
  assert.doesNotMatch(text, /Skill tool/);
});

test("no Paseo agent ID returns an empty result", async () => {
  const { env } = await fixture();
  assert.deepEqual(run({ prompt: "hi" }, { PASEO_HOME: env.PASEO_HOME }), {});
  assert.deepEqual(run({ prompt: "hi" }, { ...env, PASEO_AGENT_ID: "bad" }), {});
});

test("an agent with no pinned skills returns an empty result", async () => {
  const { env, pins } = await fixture();
  assert.deepEqual(run({ prompt: "hi" }, env), {});
  await pins.set(A, []);
  assert.deepEqual(run({ prompt: "hi" }, env), {});
});

test("Claude context names each pinned skill for the Skill tool", async () => {
  const { env, pins } = await fixture();
  await pins.set(A, ["watchtower", "sequential-thinking"]);
  const output = run({ prompt: "hi" }, env, "claude");
  assert.equal(output.hookSpecificOutput.hookEventName, "UserPromptSubmit");
  const text = contextOf(output);
  assert.match(text, /invoke every one of them with the Skill tool/);
  assert.match(text, /Skip a skill only if it is already loaded in this conversation:/);
  assert.doesNotMatch(text, /still applies/);
  assert.match(text, /^- watchtower$/m);
  assert.match(text, /^- sequential-thinking$/m);
  assert.doesNotMatch(text, /chase-goal-claude/);
  assert.match(text, /Stop following any skill that was pinned earlier/);
});

test("Codex context lists the absolute SKILL.md paths", async () => {
  const { env, pins } = await fixture();
  await pins.set(A, ["watchtower", "chase-goal-claude"]);
  const text = contextOf(run({ prompt: "hi" }, env, "codex"));
  assert.match(text, /^- \/skills\/watchtower\/SKILL\.md$/m);
  assert.match(text, /^- \/skills\/chase-goal-claude\/SKILL\.md$/m);
  assert.doesNotMatch(text, /Skill tool/);
  assert.match(text, /Stop following any skill that was pinned earlier/);
});

test("unpinning a skill drops it from the context and keeps the stop line", async () => {
  const { env, pins } = await fixture();
  await pins.set(A, ["watchtower"]);
  assert.match(contextOf(run({ prompt: "hi" }, env, "claude")), /^- watchtower$/m);
  await pins.set(A, ["chase-goal-claude"]);
  const text = contextOf(run({ prompt: "hi" }, env, "claude"));
  assert.doesNotMatch(text, /watchtower/);
  assert.match(text, /^- chase-goal-claude$/m);
  assert.match(text, /Only the skills listed above are pinned now\./);
});

test("a skill requested in the prompt is dropped for this turn only", async () => {
  const { env, pins } = await fixture();
  await pins.set(A, ["watchtower", "chase-goal-claude"]);
  const first = contextOf(run({ prompt: "/superpowers:watchtower plan X" }, env));
  assert.doesNotMatch(first, /watchtower/);
  assert.match(first, /chase-goal-claude/);
  assert.doesNotMatch(
    contextOf(run({ prompt: "use $chase-goal-claude now" }, env)),
    /chase-goal-claude/,
  );
  assert.doesNotMatch(contextOf(run({ prompt: "/watchtower new plan" }, env)), /watchtower/);
  assert.match(contextOf(run({ prompt: "/watchtower-extra" }, env)), /watchtower/);
  assert.match(contextOf(run({ prompt: "next turn" }, env)), /watchtower/);
});

test("a matching snapshot wins over the saved selection and is consumed", async () => {
  const { env, pins, pending, root } = await fixture();
  const { token } = await pins.prepare({
    agentId: A,
    text: "queued\r\nprompt",
    skills: ["watchtower"],
  });
  await pins.bindQueue(A, token, "queue-1");
  await pins.set(A, ["chase-goal-claude"]);
  const text = contextOf(run({ prompt: "queued\nprompt", session_id: "s1" }, env));
  assert.match(text, /watchtower/);
  assert.doesNotMatch(text, /chase-goal-claude/);
  assert.deepEqual(await readdir(pending), []);
  const last = JSON.parse(await readFile(path.join(root, "agents", A, "last-hook.json"), "utf8"));
  assert.equal(last.source, "turn");
  assert.deepEqual(last.skills, ["watchtower"]);
  assert.equal(JSON.stringify(last).includes("queued"), false);
});

test("an expired snapshot is deleted and ignored", async () => {
  const { env, pins, pending } = await fixture();
  await pins.set(A, ["chase-goal-claude"]);
  await mkdir(pending, { recursive: true });
  const { digest } = require("../server/skill-pins-hook.cjs");
  await writeFile(
    path.join(pending, "1-aaaa.json"),
    JSON.stringify({
      hash: digest("old"),
      skills: ["watchtower"],
      createdAt: Date.now() - 86400001,
    }),
  );
  const text = contextOf(run({ prompt: "old" }, env));
  assert.match(text, /chase-goal-claude/);
  assert.doesNotMatch(text, /watchtower/);
  assert.deepEqual(await readdir(pending), []);
});

test("the CLI prints {} outside Paseo and exits 1 on bad input", () => {
  const env = { ...process.env };
  delete env.PASEO_AGENT_ID;
  const out = execFileSync(process.execPath, [hookFile, "--claude"], {
    input: '{"prompt":"hi"}',
    env,
  });
  assert.equal(out.toString(), "{}");
  assert.throws(
    () => execFileSync(process.execPath, [hookFile], { input: "not json", env, stdio: "pipe" }),
    (error: { status: number }) => error.status === 1,
  );
});

test("hook CLI p90 stays under 200 ms over 20 runs", async () => {
  const { env, pins } = await fixture();
  await pins.set(A, ["watchtower", "chase-goal-claude"]);
  const times: number[] = [];
  for (let i = 0; i < 20; i++) {
    const start = performance.now();
    execFileSync(process.execPath, [hookFile, "--codex"], {
      input: JSON.stringify({ prompt: `turn ${i}` }),
      env: { ...process.env, ...env },
    });
    times.push(performance.now() - start);
  }
  const p90 = times.sort((a, b) => a - b)[17];
  console.log(`hook p90 ${p90.toFixed(1)} ms`);
  assert.ok(p90 < 200, `p90 ${p90} ms`);
});

test("the initial env set applies until the agent has skills.json", async () => {
  const { env, pins, root } = await fixture();
  const initial = { ...env, SKILL_PINS_INITIAL: "watchtower,unknown" };
  const text = contextOf(run({ prompt: "hi" }, initial));
  assert.match(text, /^- watchtower$/m);
  assert.doesNotMatch(text, /unknown/);
  const last = JSON.parse(await readFile(path.join(root, "agents", A, "last-hook.json"), "utf8"));
  assert.equal(last.source, "initial");
  await pins.set(A, []);
  assert.deepEqual(run({ prompt: "hi" }, initial), {});
});
