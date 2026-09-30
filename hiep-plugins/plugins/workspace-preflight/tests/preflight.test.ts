import assert from "node:assert/strict";
import {
  mkdtemp,
  mkdir,
  writeFile,
  readFile,
  readdir,
  unlink,
  rmdir,
  symlink,
} from "node:fs/promises";
import http from "node:http";
import os from "node:os";
import path from "node:path";
import { test } from "node:test";
import { runPreflight } from "../server/preflight";

async function fixture(t: { after: (fn: () => Promise<void>) => void }, config?: unknown) {
  const root = await mkdtemp(path.join(os.tmpdir(), "preflight-test-"));
  await mkdir(path.join(root, ".paseo"));
  if (config !== undefined)
    await writeFile(path.join(root, ".paseo/preflight.json"), JSON.stringify(config));
  t.after(async () => {
    for (const name of await readdir(path.join(root, ".paseo")))
      await unlink(path.join(root, ".paseo", name));
    await rmdir(path.join(root, ".paseo"));
    for (const name of await readdir(root)) await unlink(path.join(root, name));
    await rmdir(root);
  });
  return root;
}
async function server(
  t: { after: (fn: () => Promise<void>) => void },
  handler: http.RequestListener,
) {
  const instance = http.createServer(handler);
  await new Promise<void>((resolve) => instance.listen(0, "127.0.0.1", resolve));
  t.after(async () => {
    instance.closeAllConnections();
    await new Promise<void>((resolve, reject) =>
      instance.close((error) => (error ? reject(error) : resolve())),
    );
  });
  return (instance.address() as { port: number }).port;
}
const repair = "echo repair-manually";

test("ready fixture passes all configured checks without changing files", async (t) => {
  const port = await server(t, (_req, res) => {
    res.writeHead(200);
    res.end("ok");
  });
  const config = {
    version: 1,
    runtimes: [{ executable: "node", major: Number(process.versions.node.split(".")[0]), repair }],
    dependencies: [{ label: "Installed", path: "installed", repair: "touch must-not-exist" }],
    ports: [{ label: "Listener", port, expect: "listening", repair }],
    health: [{ label: "Health", url: `http://127.0.0.1:${port}/health`, repair }],
  };
  const root = await fixture(t, config);
  await writeFile(path.join(root, "installed"), "unchanged");
  const before = await readFile(path.join(root, ".paseo/preflight.json"), "utf8");
  const report = await runPreflight(root);
  assert.equal(report.checks.length, 4);
  assert.ok(
    report.checks.every((check) => check.status === "pass"),
    JSON.stringify(report),
  );
  assert.equal(await readFile(path.join(root, "installed"), "utf8"), "unchanged");
  assert.equal(await readFile(path.join(root, ".paseo/preflight.json"), "utf8"), before);
  assert.deepEqual((await readdir(root)).sort(), [".paseo", "installed"]);
});

test("wrong runtime, missing dependency, occupied port, and unhealthy endpoint are blockers", async (t) => {
  const port = await server(t, (_req, res) => {
    res.writeHead(503);
    res.end();
  });
  const root = await fixture(t, {
    version: 1,
    runtimes: [{ executable: "node", major: 999, repair }],
    dependencies: [{ label: "Missing", path: "missing", repair: "touch must-not-exist" }],
    ports: [{ label: "Must be closed", port, expect: "closed", repair }],
    health: [{ label: "Unhealthy", url: `http://127.0.0.1:${port}`, repair }],
  });
  const { checks } = await runPreflight(root);
  assert.deepEqual(
    checks.map((check) => check.status),
    ["blocker", "blocker", "blocker", "blocker"],
  );
  assert.match(checks[0].detail, /expected major 999/);
  assert.match(checks[1].detail, /missing/);
  assert.match(checks[3].detail, /HTTP 503/);
  assert.deepEqual(await readdir(root), [".paseo"]);
});

test("health timeout is bounded and redirects are not followed", async (t) => {
  let redirected = false;
  const port = await server(t, (req, res) => {
    if (req.url === "/redirect") {
      res.writeHead(302, { Location: "/target" });
      res.end();
    }
    if (req.url === "/target") {
      redirected = true;
      res.end();
    }
  });
  const root = await fixture(t, {
    version: 1,
    health: [
      { label: "Timeout", url: `http://127.0.0.1:${port}/hang`, repair },
      { label: "Redirect", url: `http://127.0.0.1:${port}/redirect`, repair },
    ],
  });
  const start = Date.now();
  const checks = (await runPreflight(root)).checks.filter((check) => check.category === "health");
  assert.ok(Date.now() - start < 5000);
  assert.equal(checks[0].status, "unknown");
  assert.match(checks[0].detail, /timed out/);
  assert.equal(checks[1].status, "blocker");
  assert.equal(redirected, false);
});

test("missing and empty config remain unknown; invalid and oversized configs block", async (t) => {
  const root = await fixture(t);
  assert.equal((await runPreflight(root)).checks[0].status, "unknown");
  const filename = path.join(root, ".paseo/preflight.json");
  await writeFile(filename, '{"version":1}');
  assert.equal((await runPreflight(root)).checks[0].status, "unknown");
  for (const invalid of [
    "{",
    " ".repeat(65537),
    JSON.stringify({ version: 1, command: "touch unsafe" }),
  ]) {
    await writeFile(filename, invalid);
    assert.equal((await runPreflight(root)).checks[0].status, "blocker");
  }
});

test("reject path escapes, arbitrary executables, and nonlocal or credential-bearing health URLs", async (t) => {
  for (const fields of [
    { dependencies: [{ label: "Escape", path: "../outside", repair }] },
    { runtimes: [{ executable: "sh", major: 1, repair }] },
    ...[
      "https://example.com",
      "http://user:secret@localhost/",
      "http://localhost/?token=secret",
    ].map((url) => ({ health: [{ label: "Unsafe", url, repair }] })),
  ]) {
    const root = await fixture(t, { version: 1, ...fields });
    const { checks } = await runPreflight(root);
    assert.equal(checks[0].status, "blocker");
    assert.ok(!JSON.stringify(checks).includes("secret"));
  }
});

test("external dependency and config symlinks cannot read outside workspace", async (t) => {
  const other = await fixture(t, { version: 1 });
  const root = await fixture(t, {
    version: 1,
    dependencies: [{ label: "Link", path: "external", repair }],
  });
  await symlink(other, path.join(root, "external"));
  assert.equal((await runPreflight(root)).checks[0].status, "unknown");
  const filename = path.join(root, ".paseo/preflight.json");
  await unlink(filename);
  await symlink(path.join(other, ".paseo/preflight.json"), filename);
  assert.equal((await runPreflight(root)).checks[0].status, "blocker");
});
