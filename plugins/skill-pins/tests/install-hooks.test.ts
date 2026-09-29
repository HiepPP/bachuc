import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtemp, mkdir, readFile, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const script = fileURLToPath(new URL("../scripts/install-hooks.mjs", import.meta.url));
const ids = ["watchtower", "chase-goal-claude", "sequential-thinking"];
const other = { hooks: [{ type: "command", command: "echo other" }] };

async function fixture(skills = ids) {
  const home = await mkdtemp(path.join(tmpdir(), "skill-pins-install-"));
  const env = {
    ...process.env,
    HOME: home,
    PASEO_HOME: path.join(home, ".paseo"),
    CODEX_HOME: path.join(home, ".codex"),
    CLAUDE_CONFIG_DIR: path.join(home, ".claude"),
  };
  const skillsRoot = path.join(home, ".claude/skills");
  for (const id of skills) {
    await mkdir(path.join(skillsRoot, id), { recursive: true });
    await writeFile(path.join(skillsRoot, id, "SKILL.md"), "# skill");
  }
  const claude = path.join(env.CLAUDE_CONFIG_DIR, "settings.json");
  await mkdir(path.dirname(claude), { recursive: true });
  await writeFile(claude, JSON.stringify({ model: "x", hooks: { UserPromptSubmit: [other] } }));
  const install = (arg: string) =>
    execFileSync(process.execPath, [script, arg], { env, stdio: "pipe" }).toString();
  const read = async (file: string) => JSON.parse(await readFile(file, "utf8"));
  return { env, skillsRoot, claude, codex: path.join(env.CODEX_HOME, "hooks.json"), install, read };
}
const ours = (config: { hooks?: { UserPromptSubmit?: { hooks: { command: string }[] }[] } }) =>
  (config.hooks?.UserPromptSubmit ?? [])
    .flatMap((group) => group.hooks)
    .filter((hook) => hook.command.includes("skill-pins-hook.cjs"));

test("installing twice keeps one entry per provider and preserves other settings", async () => {
  const { env, skillsRoot, claude, codex, install, read } = await fixture();
  install(skillsRoot);
  install(skillsRoot);
  const claudeConfig = await read(claude);
  assert.equal(claudeConfig.model, "x");
  assert.deepEqual(claudeConfig.hooks.UserPromptSubmit[0], other);
  assert.equal(ours(claudeConfig).length, 1);
  assert.match(ours(claudeConfig)[0].command, / --claude$/);
  assert.equal(ours(await read(codex)).length, 1);
  assert.match(ours(await read(codex))[0].command, / --codex$/);
  const runtime = await read(path.join(env.PASEO_HOME, "plugin-data/skill-pins/hook-runtime.json"));
  assert.equal(runtime.skillPaths.watchtower, path.join(skillsRoot, "watchtower/SKILL.md"));
  const backup = await read(`${claude}.skill-pins-backup`);
  assert.equal(ours(backup).length, 0);
});

test("--remove deletes only this plugin's entries", async () => {
  const { skillsRoot, claude, codex, install, read } = await fixture();
  install(skillsRoot);
  install("--remove");
  const claudeConfig = await read(claude);
  assert.deepEqual(claudeConfig.hooks.UserPromptSubmit, [other]);
  assert.equal(claudeConfig.model, "x");
  assert.equal((await read(codex)).hooks.UserPromptSubmit, undefined);
});

test("a missing SKILL.md fails the install without touching configs", async () => {
  const { skillsRoot, claude, install, read } = await fixture(ids.slice(0, 2));
  assert.throws(() => install(skillsRoot));
  assert.equal(ours(await read(claude)).length, 0);
});
