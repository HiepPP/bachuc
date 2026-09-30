import assert from "node:assert/strict";
import { mkdir, symlink, writeFile, access } from "node:fs/promises";
import path from "node:path";
import { test } from "node:test";
import { runPreflight } from "../server/preflight";
import { reportSchema } from "../shared/preflight";
import { fixture } from "./fixtures";
const current = process.versions.node;
const major = Number(current.split(".")[0]);

test("semver discovery accepts full ranges and rejects wrong runtimes, conflicts and aliases", async (t) => {
  for (const [range, status] of [
    [`^${current}`, "pass"],
    [`>=${major} <${major + 1}`, "pass"],
    [`${major - 1}.x || ${major}.x`, "pass"],
    ["999.x", "blocker"],
    ["lts/*", "unknown"],
    ["", "unknown"],
  ]) {
    const root = await fixture(t, { "package.json": { engines: { node: range } } });
    const report = reportSchema.parse(await runPreflight(root));
    assert.equal(report.checks.find((c) => c.category === "runtimes")?.status, status, range);
  }
  for (const files of [
    { ".nvmrc": "22", ".node-version": "24" },
    { ".nvmrc": "$(touch unsafe)" },
    { ".nvmrc": "x".repeat(257) },
  ]) {
    const report = await runPreflight(await fixture(t, files));
    assert.equal(report.checks.find((c) => c.category === "runtimes")?.status, "unknown");
  }
  const matching = await fixture(t, {
    "package.json": { engines: { node: `>=${major}` } },
    ".nvmrc": current,
    ".node-version": `${major}`,
  });
  assert.equal(
    (await runPreflight(matching)).checks.find((c) => c.category === "runtimes")?.status,
    "pass",
  );
});

test("npm, pnpm and declared Yarn classic use node_modules; ambiguous and PnP layouts stay unknown", async (t) => {
  for (const [manager, lock] of [
    ["npm@10.0.0", "package-lock.json"],
    ["pnpm@9.0.0", "pnpm-lock.yaml"],
    ["yarn@1.22.22", "yarn.lock"],
  ]) {
    const root = await fixture(t, { "package.json": { packageManager: manager }, [lock]: "" });
    assert.equal(
      (await runPreflight(root)).checks.find((c) => c.category === "dependencies")?.status,
      "blocker",
    );
    await mkdir(path.join(root, "node_modules"));
    assert.equal(
      (await runPreflight(root)).checks.find((c) => c.category === "dependencies")?.status,
      "pass",
    );
  }
  for (const files of [
    {
      "yarn.lock": "",
      "package.json": { packageManager: "yarn@4.0.0" },
      ".pnp.cjs": "throw new Error('never execute')",
    },
    { "yarn.lock": "", "package.json": { packageManager: "yarn@4.0.0" } },
    { "yarn.lock": "" },
    { "package-lock.json": "", "pnpm-lock.yaml": "" },
    { "package-lock.json": "", "package.json": { packageManager: "pnpm@9.0.0" } },
    { "package-lock.json": "", "package.json": { packageManager: "npm@latest" } },
    { "package-lock.json": "", "package.json": { workspaces: ["packages/*"] } },
    { "pnpm-lock.yaml": "", "pnpm-workspace.yaml": "packages: [apps/*]" },
    { "bun.lock": "" },
  ]) {
    const root = await fixture(t, files);
    const dep = (await runPreflight(root)).checks.find((c) => c.category === "dependencies");
    assert.equal(dep?.status, "unknown", JSON.stringify(files));
    assert.equal(dep?.repair, "");
  }
});

test("explicit category replaces discovery, empty disables, invalid remains a blocker", async (t) => {
  const root = await fixture(t, {
    "package.json": { engines: { node: "999" } },
    ".paseo/preflight.json": {
      version: 1,
      runtimes: [{ executable: "node", range: `^${current}`, repair: "touch unsafe" }],
      dependencies: [],
      ports: [],
      health: [],
    },
  });
  const report = await runPreflight(root);
  assert.equal(report.mode, "explicit");
  assert.equal(report.checks.filter((c) => c.category === "runtimes").length, 1);
  assert.equal(report.checks.find((c) => c.category === "runtimes")?.status, "pass");
  assert.ok(
    report.checks
      .filter((c) => c.category !== "runtimes")
      .every((c) => c.detail.includes("disabled")),
  );
  await assert.rejects(access(path.join(root, "unsafe")));
  for (const runtimes of [
    [{ executable: "node", range: "lts/*", repair: "x" }],
    [{ executable: "node", major, range: "*", repair: "x" }],
    null,
  ]) {
    await writeFile(
      path.join(root, ".paseo/preflight.json"),
      JSON.stringify({ version: 1, runtimes }),
    );
    assert.equal((await runPreflight(root)).checks[0].category, "configuration");
    assert.equal((await runPreflight(root)).checks[0].status, "blocker");
  }
});

test("metadata stays bounded and confined; scripts, version files and repairs never execute", async (t) => {
  const outside = await fixture(t, { "package.json": { engines: { node: current } } });
  const root = await fixture(t, {
    ".nvmrc": "touch unsafe",
    "package-lock.json": "",
    "package.json": {
      scripts: { dev: "touch unsafe", postinstall: "touch unsafe" },
      description: "UNRELATED_PRIVATE_CONTENT",
    },
  });
  const result = JSON.stringify(await runPreflight(root));
  assert.ok(!result.includes("UNRELATED_PRIVATE_CONTENT"));
  await assert.rejects(access(path.join(root, "unsafe")));
  await writeFile(path.join(root, "package.json"), " ".repeat(65537));
  assert.ok(
    (await runPreflight(root)).checks
      .filter((c) => ["runtimes", "dependencies"].includes(c.category))
      .every((c) => c.status === "unknown"),
  );
  const linked = await fixture(t);
  await symlink(path.join(outside, "package.json"), path.join(linked, "package.json"));
  assert.equal(
    (await runPreflight(linked)).checks.find((c) => c.category === "runtimes")?.status,
    "unknown",
  );
  await symlink(outside, path.join(linked, ".paseo"));
  assert.equal((await runPreflight(linked)).checks[0].status, "blocker");
});

test("prerelease lower bounds with stable ranges retain their common stable versions", async (t) => {
  const root = await fixture(t, {
    "package.json": { engines: { node: `>=${major}.0.0-rc.1` } },
    ".nvmrc": `>=${major - 1}`,
  });
  assert.equal(
    (await runPreflight(root)).checks.find((c) => c.category === "runtimes")?.status,
    "pass",
  );
});

test("fresh Yarn PnP and pnpm custom linker declarations never imply missing node_modules", async (t) => {
  for (const files of [
    {
      "package.json": { packageManager: "yarn@1.22.22", installConfig: { pnp: true } },
      "yarn.lock": "",
    },
    {
      "package.json": { packageManager: "pnpm@9.0.0" },
      "pnpm-lock.yaml": "",
      ".npmrc": "node-linker=pnp",
    },
  ]) {
    const report = await runPreflight(await fixture(t, files));
    assert.equal(report.checks.find((c) => c.category === "dependencies")?.status, "unknown");
  }
});

test("runtime discovery does not execute a workspace-owned PATH binary", async (t) => {
  const root = await fixture(t, {
    "package.json": { engines: { node: ">=18" } },
    node: "#!/bin/sh\ntouch unsafe\necho v24.0.0\n",
  });
  const previous = process.env.PATH;
  try {
    process.env.PATH = `${root}${path.delimiter}${previous}`;
    const report = await runPreflight(root);
    assert.equal(report.checks.find((c) => c.category === "runtimes")?.status, "unknown");
    await assert.rejects(access(path.join(root, "unsafe")));
  } finally {
    process.env.PATH = previous;
  }
});
