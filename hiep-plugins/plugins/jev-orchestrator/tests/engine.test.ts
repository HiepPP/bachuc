import test from "node:test";
import assert from "node:assert/strict";
import { mkdtemp, symlink, unlink, rmdir } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { setTimeout as delay } from "node:timers/promises";
import { Engine } from "../server/engine";
import { Store } from "../server/store";
import { taskSchema, type Task } from "../shared/contracts";
import {
  chooseMeasured,
  ownedPath,
  overlap,
  profileVersion,
  validatePaths,
} from "../server/policy";
import { runChecks } from "../server/checks";
import { createBridge } from "../server/bridge";
import type { Driver, Judge, Profile, Check, Metric } from "../server/types";

const cwd = process.cwd();
const p: Profile = {
  id: "fast",
  name: "Fast",
  provider: "codex",
  model: "test-model",
  modeId: "auto-review",
  thinkingOptionId: "low",
  featureValues: { search: false },
  notes: "Focused implementation",
};
const q: Profile = {
  ...p,
  id: "reason",
  name: "Reason",
  thinkingOptionId: "high",
  notes: "Hard diagnosis and independent review",
};
const task = (extra: Partial<Task> = {}) =>
  taskSchema.parse({
    id: "one",
    goal: "Implement bounded parser.",
    acceptance: "Pass the specified parser checks.",
    kind: "implementation",
    files: ["a.ts"],
    allowedProfileIds: [p.id, q.id],
    shareWithJev: true,
    checks: [{ argv: ["node", "-e", "process.exit(0)"] }],
    ...extra,
  });
const pass: Check[] = [
  { argv: ["check"], exitCode: 0, output: "ok", durationMs: 1, timedOut: false },
];
const fail: Check[] = [{ ...pass[0], exitCode: 1, output: "assertion failed" }];
function harness(overrides: Partial<Driver> = {}, judge?: Judge, check?: typeof runChecks) {
  const launched: { profile: Profile; phase: string; prompt: string; id: string }[] = [],
    archived: string[] = [],
    notifications: string[] = [];
  const driver: Driver = {
    profiles: async () => [p, q],
    launch: async (_j, profile, phase, prompt) => {
      const id = `child-${launched.length}`;
      launched.push({ profile, phase, prompt, id });
      return id;
    },
    wait: async () => ({ status: "idle", output: "Source evidence and complete result." }),
    archive: async (id) => {
      archived.push(id);
    },
    notify: async (j) => {
      notifications.push(j.task.id);
    },
    ...overrides,
  };
  const normal: Judge = async (_phase, state, profiles) => ({
    profileId: profiles[0].id,
    discovery: false,
    risk: "low",
    category: "implementation",
    reviewPassed: true,
  });
  const engine = new Engine(new Store(), driver, judge ?? normal, 2, check ?? (async () => pass));
  return { engine, launched, archived, notifications };
}
async function settled(e: Engine, id = "one", parent = "parent") {
  for (let n = 0; n < 200; n++) {
    const job = e.list(parent).find((j) => j.task.id === id);
    if (job && !["queued", "running"].includes(job.status)) return job;
    await delay(5);
  }
  throw new Error("Test job did not settle");
}

test("broker copies profile settings, deduplicates and preserves explicit profile", async () => {
  const h = harness();
  const input = { tasks: [task({ profileId: q.id })] };
  await h.engine.submit("parent", cwd, input);
  await h.engine.submit("parent", cwd, input);
  const job = await settled(h.engine);
  assert.equal(job.status, "passed");
  assert.equal(h.launched.length, 1);
  assert.deepEqual(h.launched[0].profile, q);
  await assert.rejects(
    h.engine.submit("parent", cwd, { tasks: [task({ goal: "A different contract" })] }),
    /different contract/,
  );
});
test("missing checks never count self-reported completion as verified", async () => {
  const h = harness({}, undefined, async () => []);
  await h.engine.submit("parent", cwd, { tasks: [task({ checks: [] })] });
  assert.equal((await settled(h.engine)).status, "unverified");
  assert.equal(h.engine.store.metrics.length, 0);
});
test("discovery runs first and evidence reaches implementation", async () => {
  const h = harness({}, async (_phase, state, ps) => ({
    profileId: ps[0].id,
    discovery: !(state.previous as unknown[]).length,
    risk: "low",
    category: "implementation",
  }));
  await h.engine.submit("parent", cwd, { tasks: [task()] });
  await settled(h.engine);
  assert.deepEqual(
    h.launched.map((x) => x.phase),
    ["discovery", "implementation"],
  );
  assert.match(h.launched[1].prompt, /Source evidence/);
  assert.match(h.launched[0].prompt, /Do not edit/);
});
test("clear tasks skip discovery; risky tasks get a different read-only reviewer", async () => {
  const h = harness({}, async (_phase, _state, ps) => ({
    profileId: ps[0].id,
    discovery: false,
    risk: "high",
    category: "implementation",
    reviewPassed: true,
  }));
  await h.engine.submit("parent", cwd, { tasks: [task()] });
  assert.equal((await settled(h.engine)).status, "passed");
  assert.deepEqual(
    h.launched.map((x) => x.phase),
    ["implementation", "review"],
  );
  assert.equal(h.launched[1].profile.id, q.id);
});
test("environment failure never escalates or repeats a child", async () => {
  const h = harness(
    {},
    async (phase) => ({
      profileId: p.id,
      discovery: false,
      risk: "low",
      category: "implementation",
      recovery: phase === "recovery" ? "environment" : undefined,
    }),
    async () => fail,
  );
  await h.engine.submit("parent", cwd, { tasks: [task()] });
  const j = await settled(h.engine);
  assert.equal(j.status, "needs_input");
  assert.match(j.message, /environment/);
  assert.equal(h.launched.length, 1);
});
test("reasoning failure escalates once with prior check evidence", async () => {
  let checks = 0;
  const h = harness(
    {},
    async (phase) => ({
      profileId: phase === "recovery" ? q.id : p.id,
      discovery: false,
      risk: "low",
      category: "implementation",
      recovery: "escalate",
    }),
    async () => (++checks === 1 ? fail : pass),
  );
  await h.engine.submit("parent", cwd, { tasks: [task()] });
  assert.equal((await settled(h.engine)).status, "passed");
  assert.deepEqual(
    h.launched.map((x) => x.profile.id),
    [p.id, q.id],
  );
  assert.match(h.launched[1].prompt, /assertion failed/);
});
test("explicit profile cannot be silently escalated", async () => {
  const h = harness(
    {},
    async () => ({
      profileId: q.id,
      discovery: false,
      risk: "low",
      category: "implementation",
      recovery: "escalate",
    }),
    async () => fail,
  );
  await h.engine.submit("parent", cwd, { tasks: [task({ profileId: p.id })] });
  assert.equal((await settled(h.engine)).status, "needs_input");
  assert.equal(h.launched.length, 1);
});
test("failed checks exhaust budget and cannot be passed by Jev", async () => {
  const h = harness({}, undefined, async () => fail);
  await h.engine.submit("parent", cwd, { tasks: [task({ maxAttempts: 1 })] });
  assert.equal((await settled(h.engine)).status, "failed");
  assert.equal(h.engine.store.metrics[0].verified, false);
});
test("review disagreement prevents passing despite green tests", async () => {
  const h = harness({}, async (_phase, _state, ps) => ({
    profileId: ps[0].id,
    discovery: false,
    risk: "high",
    category: "implementation",
    reviewPassed: false,
  }));
  await h.engine.submit("parent", cwd, { tasks: [task()] });
  assert.equal((await settled(h.engine)).status, "needs_input");
});
test("dependencies, cycles, unknown IDs and failed predecessors", async () => {
  const h = harness({}, undefined, async () => fail);
  await assert.rejects(
    h.engine.submit("parent", cwd, { tasks: [task({ dependsOn: ["missing"] })] }),
    /Unknown dependency/,
  );
  await assert.rejects(
    h.engine.submit("parent", cwd, {
      tasks: [task({ dependsOn: ["two"] }), task({ id: "two", dependsOn: ["one"] })],
    }),
    /cycle/,
  );
  await h.engine.submit("parent", cwd, {
    tasks: [task({ maxAttempts: 1 }), task({ id: "two", files: ["b.ts"], dependsOn: ["one"] })],
  });
  assert.equal((await settled(h.engine, "two")).status, "failed");
  assert.equal(h.launched.length, 1);
});
test("concurrency runs independent tasks while nested ownership stays locked", async () => {
  let release!: () => void;
  const gate = new Promise<void>((r) => (release = r));
  const h = harness({
    wait: async () => {
      await gate;
      return { status: "idle", output: "ok" };
    },
  });
  await h.engine.submit("parent", cwd, {
    tasks: [
      task({ files: ["src"] }),
      task({ id: "two", files: ["src/a.ts"] }),
      task({ id: "three", files: ["other.ts"] }),
    ],
  });
  await delay(30);
  assert.equal(h.launched.length, 2);
  assert.equal(h.engine.list("parent").find((j) => j.task.id === "two")?.status, "queued");
  release();
  await settled(h.engine, "two");
  assert.equal(h.launched.length, 3);
});
test("resource ownership serializes disjoint files", async () => {
  let release!: () => void;
  const gate = new Promise<void>((r) => (release = r));
  const h = harness({
    wait: async () => {
      await gate;
      return { status: "idle", output: "ok" };
    },
  });
  await h.engine.submit("parent", cwd, {
    tasks: [task({ resources: ["db"] }), task({ id: "two", files: ["b.ts"], resources: ["db"] })],
  });
  await delay(30);
  assert.equal(h.launched.length, 1);
  release();
  await settled(h.engine, "two");
});
test("permission holds ownership; cancellation archives known children", async () => {
  const h = harness({ wait: async () => ({ status: "permission", output: "approval needed" }) });
  await h.engine.submit("parent", cwd, { tasks: [task(), task({ id: "two" })] });
  assert.equal((await settled(h.engine)).status, "needs_input");
  assert.equal(h.launched.length, 1);
  assert.equal(await h.engine.cancel("different", "one"), false);
  await h.engine.cancel("parent", "one");
  assert.deepEqual(h.archived, ["child-0"]);
  await settled(h.engine, "two");
});
test("managed children cannot recursively submit jobs", async () => {
  const h = harness();
  await h.engine.submit("parent", cwd, { tasks: [task()] });
  await settled(h.engine);
  await assert.rejects(h.engine.submit("child-0", cwd, { tasks: [task()] }), /recursively/);
});
test("invalid profile and evaluator failures do not launch substitute agents", async () => {
  const h = harness({}, async () => {
    throw new Error("HTTP 429");
  });
  await h.engine.submit("parent", cwd, { tasks: [task()] });
  assert.equal((await settled(h.engine)).status, "needs_input");
  assert.equal(h.launched.length, 0);
  const absent = harness({ profiles: async () => [p] });
  await absent.engine.submit("parent", cwd, { tasks: [task({ profileId: q.id })] });
  assert.match((await settled(absent.engine)).message, /Explicit profile unavailable/);
});
test("path ownership rejects escapes and handles nesting", async () => {
  for (const s of ["../a", "/tmp/a", "src/*", "a\\b"]) assert.throws(() => ownedPath(s));
  assert.ok(overlap("src", "src/a.ts"));
  assert.ok(!overlap("src", "src2"));
  const dir = await mkdtemp(path.join(tmpdir(), "jev-path-"));
  try {
    await symlink(tmpdir(), path.join(dir, "escape"));
    await assert.rejects(validatePaths(dir, ["escape/file"]), /escapes/);
  } finally {
    await unlink(path.join(dir, "escape"));
    await rmdir(dir);
  }
});
test("persisted running work becomes interrupted without replay", async () => {
  const dir = await mkdtemp(path.join(tmpdir(), "jev-state-"));
  const file = path.join(dir, "state.json");
  try {
    const s = new Store(file);
    s.jobs.push({
      key: "p:one",
      parentId: "p",
      cwd,
      task: task(),
      status: "running",
      createdAt: 0,
      attempts: [],
      message: "",
    });
    s.save();
    assert.equal(new Store(file).jobs[0].status, "interrupted");
  } finally {
    await unlink(file);
    await rmdir(dir);
  }
});
test("history requires comparable measured samples and invalidates changed settings", () => {
  const metric = (profile: Profile, ms: number): Metric => ({
    profileId: profile.id,
    profileVersion: profileVersion(profile),
    category: "implementation",
    phase: "implementation",
    verified: true,
    durationMs: ms,
    at: 0,
  });
  const data = Array.from({ length: 3 }, () => metric(q, 10));
  assert.equal(
    chooseMeasured(p, [p, q], { fast: 0.52, reason: 0.48 }, data, "implementation").id,
    q.id,
  );
  assert.equal(
    chooseMeasured(
      p,
      [p, { ...q, thinkingOptionId: "max" }],
      { fast: 0.52, reason: 0.48 },
      data,
      "implementation",
    ).id,
    p.id,
  );
  assert.equal(
    chooseMeasured(p, [p, q], { fast: 0.9, reason: 0.1 }, data, "implementation").id,
    p.id,
  );
});
test("actual check runner preserves nonzero, timeout and shell-free argv", async () => {
  const signal = new AbortController().signal;
  const results = await runChecks(
    cwd,
    [
      {
        argv: [process.execPath, "-e", "console.log('literal $(echo unsafe)');process.exit(7)"],
        timeoutMs: 1000,
      },
      { argv: [process.execPath, "-e", "setTimeout(()=>{},10000)"], timeoutMs: 100 },
    ],
    signal,
  );
  assert.equal(results[0].exitCode, 7);
  assert.match(results[0].output, /\$\(echo unsafe\)/);
  assert.equal(results[1].timedOut, true);
});
test("bridge rejects absent, unbound, wrong-scope and browser-origin requests", async () => {
  const h = harness();
  const bridge = createBridge(h.engine, async () => ({ cwd }));
  const url = await bridge.ready;
  try {
    const token = bridge.issue(cwd);
    assert.equal((await fetch(url, { method: "POST", body: "{}" })).status, 403);
    const headers = { Authorization: `Bearer ${token}` };
    assert.equal(
      (await fetch(url, { method: "POST", headers, body: '{"action":"status"}' })).status,
      403,
    );
    assert.ok(bridge.bind(token, "parent", cwd));
    assert.equal(bridge.bind(token, "other", cwd), false);
    assert.equal(
      (
        await fetch(url, {
          method: "POST",
          headers: { ...headers, Origin: "https://evil.example" },
          body: "{}",
        })
      ).status,
      403,
    );
    const response = await fetch(url, { method: "POST", headers, body: '{"action":"status"}' });
    assert.equal(response.status, 200);
    bridge.revoke("parent");
    assert.equal((await fetch(url, { method: "POST", headers, body: "{}" })).status, 403);
  } finally {
    bridge.close();
  }
});

test("incomplete discovery stops before any write stage", async () => {
  const h = harness({}, async (_phase, _state, ps) => ({
    profileId: ps[0].id,
    discovery: true,
    risk: "low",
    category: "implementation",
  }));
  await h.engine.submit("parent", cwd, { tasks: [task()] });
  assert.equal((await settled(h.engine)).status, "needs_input");
  assert.deepEqual(
    h.launched.map((a) => a.phase),
    ["discovery"],
  );
});

test("a review finding does not train routing as a success", async () => {
  const h = harness({}, async (_phase, _state, ps) => ({
    profileId: ps[0].id,
    discovery: false,
    risk: "high",
    category: "implementation",
    reviewPassed: false,
  }));
  await h.engine.submit("parent", cwd, { tasks: [task()] });
  await settled(h.engine);
  assert.equal(h.engine.store.metrics[0].verified, false);
});

test("stop preserves unfinished ownership and prevents followup launches", async () => {
  let release!: () => void;
  const gate = new Promise<void>((r) => {
    release = r;
  });
  const h = harness({
    wait: async () => {
      await gate;
      return { status: "idle", output: "ok" };
    },
  });
  await h.engine.submit("parent", cwd, { tasks: [task(), task({ id: "two" })] });
  await delay(20);
  h.engine.stop();
  release();
  await delay(20);
  assert.equal(h.launched.length, 1);
  assert.deepEqual(
    h.engine.list("parent").map((j) => j.status),
    ["interrupted", "interrupted"],
  );
});
