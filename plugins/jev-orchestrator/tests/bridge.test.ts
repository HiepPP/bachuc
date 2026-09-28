import test from "node:test";
import assert from "node:assert/strict";
import { mkdtemp, readFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { createBridge } from "../server/bridge";

const scope = async () => ({ cwd: "/workspace" });

test("bound leases and port survive a bridge restart", async (t) => {
  const file = path.join(await mkdtemp(path.join(tmpdir(), "jev-bridge-")), "bridge.json");
  const first = createBridge({} as never, scope, undefined, file);
  const url = await first.ready;
  const bound = first.issue("/workspace");
  const unbound = first.issue("/workspace");
  assert.equal(first.bind(bound, "agent", "/workspace"), true);
  first.close();

  const second = createBridge({} as never, scope, undefined, file);
  t.after(() => second.close());
  assert.equal(await second.ready, url);
  assert.equal(second.leaseFor("agent", "/workspace"), bound);
  assert.equal(second.bind(bound, "agent", "/workspace"), true);
  assert.equal(second.bind(bound, "other", "/workspace"), false);
  assert.equal(second.bind(unbound, "agent", "/workspace"), false);

  second.revoke("agent");
  assert.equal(second.leaseFor("agent", "/workspace"), undefined);
  const saved = JSON.parse(await readFile(file, "utf8"));
  assert.deepEqual(saved.leases, []);
});

test("a taken port falls back to a fresh port", async (t) => {
  const file = path.join(await mkdtemp(path.join(tmpdir(), "jev-bridge-")), "bridge.json");
  const first = createBridge({} as never, scope, undefined, file);
  t.after(() => first.close());
  const url = await first.ready;
  const second = createBridge({} as never, scope, undefined, file);
  t.after(() => second.close());
  const moved = await second.ready;
  assert.notEqual(moved, url);
  const saved = JSON.parse(await readFile(file, "utf8"));
  assert.equal(`http://127.0.0.1:${saved.port}/mcp`, moved);
});
