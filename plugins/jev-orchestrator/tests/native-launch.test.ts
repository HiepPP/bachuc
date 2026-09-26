import assert from "node:assert/strict";
import { execFile } from "node:child_process";
import { mkdtemp, mkdir, readFile, readdir, rmdir, unlink, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { promisify } from "node:util";
import { parse } from "smol-toml";
import { parse as parseYaml } from "yaml";
import test from "node:test";
import {
  defaultNativeSettings,
  parseFlatFrontmatter,
  prepareNativeLaunch,
} from "../server/native-launch";
import { installNativeHooks } from "../server/native-install";

async function cleanup(directory: string) {
  for (const item of await readdir(directory, { withFileTypes: true })) {
    const target = path.join(directory, item.name);
    if (item.isDirectory()) await cleanup(target);
    else await unlink(target);
  }
  await rmdir(directory);
}
test("Paseo launch prepares immutable pinned variants without modifying source roles", async () => {
  const temp = await mkdtemp(path.join(tmpdir(), "jev-launch-"));
  const userHome = path.join(temp, "user");
  const paseoHome = path.join(temp, "paseo");
  const base = path.join(paseoHome, "plugin-data/jev-orchestrator/native");
  const roles = path.join(userHome, ".codex/agents");
  try {
    await mkdir(roles, { recursive: true });
    await mkdir(base, { recursive: true });
    const source =
      'name="worker"\ndescription="Test role"\nmodel="old-model"\nsandbox_mode="read-only"\ndeveloper_instructions="Keep these exact instructions."\n';
    await writeFile(path.join(roles, "worker.toml"), source);
    assert.deepEqual(await prepareNativeLaunch(paseoHome, userHome, temp, "codex"), {});
    await writeFile(path.join(base, "settings.json"), JSON.stringify(defaultNativeSettings()));
    const env = await prepareNativeLaunch(paseoHome, userHome, temp, "codex");
    assert.equal(env.PASEO_JEV_NATIVE_RUNTIME, "codex");
    assert.ok(env.PASEO_JEV_NATIVE_POLICY);
    const manifest = JSON.parse(await readFile(env.PASEO_JEV_NATIVE_POLICY, "utf8"));
    const worker = manifest.policy.routes.find((item: any) => item.sourceType === "worker");
    assert.equal(worker.candidates.length, 2);
    for (const candidate of worker.candidates) {
      const definition = manifest.definitions.find(
        (item: any) => item.agentType === candidate.agentType,
      );
      const fields = parse(await readFile(definition.path, "utf8"));
      assert.equal(fields.model, candidate.model);
      assert.equal(fields.model_reasoning_effort, candidate.effort);
      assert.equal(fields.sandbox_mode, "read-only");
      assert.equal(fields.developer_instructions, "Keep these exact instructions.");
    }
    assert.equal(await readFile(path.join(roles, "worker.toml"), "utf8"), source);
    assert.deepEqual(await prepareNativeLaunch(paseoHome, userHome, temp, "codex"), env);
    const claudeRoles = path.join(userHome, ".claude/agents");
    await mkdir(claudeRoles, { recursive: true });
    const claudeSource =
      "---\nname: audit\ndescription: Audit role\npermissionMode: plan\ntools: [Read]\nmodel: old-model\n---\nKeep this exact body.\n";
    await writeFile(path.join(claudeRoles, "audit.md"), claudeSource);
    const claudeEnv = await prepareNativeLaunch(paseoHome, userHome, temp, "claude");
    const claudeManifest = JSON.parse(await readFile(claudeEnv.PASEO_JEV_NATIVE_POLICY, "utf8"));
    const audit = claudeManifest.policy.routes.find((item: any) => item.sourceType === "audit");
    for (const candidate of audit.candidates) {
      const definition = claudeManifest.definitions.find(
        (item: any) => item.agentType === candidate.agentType,
      );
      const generated = await readFile(definition.path, "utf8");
      const fields = parseYaml(generated.split("---\n")[1]);
      assert.equal(fields.model, candidate.model);
      assert.equal(fields.effort, candidate.effort);
      assert.equal(fields.permissionMode, "plan");
      assert.deepEqual(fields.tools, ["Read"]);
      assert.ok(generated.endsWith("Keep this exact body.\n"));
    }
    assert.equal(await readFile(path.join(claudeRoles, "audit.md"), "utf8"), claudeSource);
    // Strict YAML rejects ": " inside a plain scalar; Claude Code still loads this role.
    await writeFile(
      path.join(claudeRoles, "debugger.md"),
      "---\nname: debugger\ndescription: Use when: tests fail. Example: user: fix it\ntools: Read, Grep\nmodel: sonnet\n---\nDebug carefully.\n",
    );
    await writeFile(
      path.join(claudeRoles, "nested.md"),
      "---\nname: nested\ndescription: Use when: nested\ntools:\n  read: true\n---\nBody.\n",
    );
    const lenientEnv = await prepareNativeLaunch(paseoHome, userHome, temp, "claude");
    const lenient = JSON.parse(await readFile(lenientEnv.PASEO_JEV_NATIVE_POLICY, "utf8"));
    const debuggerRoute = lenient.policy.routes.find((item: any) => item.sourceType === "debugger");
    assert.ok(debuggerRoute);
    assert.equal(
      lenient.policy.routes.some((item: any) => item.sourceType === "nested"),
      false,
    );
    for (const candidate of debuggerRoute.candidates) {
      const definition = lenient.definitions.find(
        (item: any) => item.agentType === candidate.agentType,
      );
      const generated = await readFile(definition.path, "utf8");
      const fields = parseYaml(generated.split("---\n")[1]);
      assert.equal(fields.tools, "Read, Grep");
      assert.equal(fields.model, candidate.model);
      assert.ok(generated.endsWith("Debug carefully.\n"));
    }
    const settings = defaultNativeSettings();
    settings.enabled = false;
    await writeFile(path.join(base, "settings.json"), JSON.stringify(settings));
    assert.deepEqual(await prepareNativeLaunch(paseoHome, userHome, temp, "codex"), {});
  } finally {
    await cleanup(temp);
  }
});

test("each PASEO_HOME gets its own definition names in the shared agents folder", async () => {
  const temp = await mkdtemp(path.join(tmpdir(), "jev-homes-"));
  try {
    const userHome = path.join(temp, "user");
    await mkdir(path.join(userHome, ".claude/agents"), { recursive: true });
    const names: Set<string>[] = [];
    for (const home of ["paseo-a", "paseo-b"]) {
      const paseoHome = path.join(temp, home);
      const base = path.join(paseoHome, "plugin-data/jev-orchestrator/native");
      await mkdir(base, { recursive: true });
      await writeFile(path.join(base, "settings.json"), JSON.stringify(defaultNativeSettings()));
      const env = await prepareNativeLaunch(paseoHome, userHome, temp, "claude");
      const manifest = JSON.parse(await readFile(env.PASEO_JEV_NATIVE_POLICY, "utf8"));
      names.push(new Set(manifest.definitions.map((item: any) => item.agentType)));
    }
    assert.ok([...names[0]].every((name) => !names[1].has(name)));
  } finally {
    await cleanup(temp);
  }
});

test("flat frontmatter accepts plain scalars and refuses nested values", () => {
  assert.deepEqual(parseFlatFrontmatter("name: a\n# note\ndescription: 'x: y'\n"), {
    name: "a",
    description: "x: y",
  });
  assert.equal(parseFlatFrontmatter("name: a\ntools:\n  - Read"), undefined);
  assert.equal(parseFlatFrontmatter("name: a\ncontinued line"), undefined);
});

test("registration preserves existing hooks and settings and is idempotent", async () => {
  const temp = await mkdtemp(path.join(tmpdir(), "jev-install-"));
  try {
    await mkdir(path.join(temp, ".claude"), { recursive: true });
    const original = {
      model: "existing-model",
      hooks: {
        PreToolUse: [
          { matcher: "Bash", hooks: [{ type: "command", command: "existing-command" }] },
        ],
        Stop: [],
      },
    };
    await writeFile(path.join(temp, ".claude/settings.json"), JSON.stringify(original));
    const result = await installNativeHooks(temp, path.join(temp, "paseo"), "/plugin", "/node");
    assert.equal(result.changed.length, 2);
    const codexFile = path.join(temp, ".codex/hooks.json");
    const codex = JSON.parse(await readFile(codexFile, "utf8"));
    const matcher = codex.hooks.PreToolUse[0].matcher;
    assert.equal(new RegExp(matcher).test("collaborationspawn_agent"), true);
    assert.equal(new RegExp(matcher).test("collaborationsend_message"), false);
    codex.hooks.PreToolUse[0].matcher = "^(spawn_agent|Agent)$";
    codex.hooks.PreToolUse.unshift({
      matcher: "Bash",
      hooks: [{ type: "command", command: "unrelated" }],
    });
    await writeFile(codexFile, JSON.stringify(codex));
    const migrated = await installNativeHooks(temp, path.join(temp, "paseo"), "/plugin", "/node");
    assert.deepEqual(migrated.changed, [codexFile]);
    const restored = JSON.parse(await readFile(codexFile, "utf8"));
    assert.equal(restored.hooks.PreToolUse.length, 2);
    assert.deepEqual(restored.hooks.PreToolUse[0], codex.hooks.PreToolUse[0]);
    assert.equal(restored.hooks.PreToolUse[1].matcher, matcher);

    const after = JSON.parse(await readFile(path.join(temp, ".claude/settings.json"), "utf8"));
    assert.equal(after.model, original.model);
    assert.deepEqual(after.hooks.PreToolUse[0], original.hooks.PreToolUse[0]);
    assert.deepEqual(after.hooks.Stop, []);
    assert.equal(after.hooks.PreToolUse.length, 2);
    assert.equal(
      (await installNativeHooks(temp, path.join(temp, "paseo"), "/plugin", "/node")).changed.length,
      0,
    );
  } finally {
    await cleanup(temp);
  }
});

test("native command outside Paseo is a no-op and partial scope denies", async () => {
  const env = { ...process.env };
  delete env.PASEO_JEV_NATIVE_POLICY;
  delete env.PASEO_JEV_NATIVE_RUNTIME;
  const root = path.resolve(import.meta.dirname, "..");
  const args = [
    "--import",
    path.join(root, "node_modules/tsx/dist/loader.mjs"),
    path.join(root, "server/native-hook-command.ts"),
  ];
  const outside = await promisify(execFile)(process.execPath, args, { env, timeout: 5000 });
  assert.deepEqual(JSON.parse(outside.stdout), {});
  const partial = await promisify(execFile)(process.execPath, args, {
    env: { ...env, PASEO_JEV_NATIVE_RUNTIME: "codex" },
    timeout: 5000,
  });
  assert.equal(JSON.parse(partial.stdout).hookSpecificOutput.permissionDecision, "deny");
});
