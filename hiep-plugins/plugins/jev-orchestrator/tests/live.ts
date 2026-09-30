import { DaemonClient } from "@getpaseo/client/internal/daemon-client";
import { createPaseoApi } from "@getpaseo/client";
import { mkdir, writeFile, readFile, readdir } from "node:fs/promises";
import path from "node:path";
import { createHash } from "node:crypto";
import { setTimeout as delay } from "node:timers/promises";
import { createDriver } from "../server/paseo";
import { runChecks } from "../server/checks";
import type { Job, Profile } from "../server/types";
import { readAgentUsage } from "../server/agent-usage";
import { missingUsage, summarizeUsage, type TokenUsage } from "../server/usage";

const repo = path.resolve(import.meta.dirname, "../../..");
const runId = `live-tokens-${new Date().toISOString().replace(/[:.]/g, "-")}`;
const folder = path.join(repo, "plugins/jev-orchestrator/artifacts", runId);
await mkdir(folder, { recursive: true });
const dc = new DaemonClient({
  url: process.env.PASEO_TEST_URL ?? "ws://127.0.0.1:6767/ws",
  clientId: `jev-benchmark-${runId}`,
  clientType: "cli",
  logger: { debug() {}, info() {}, warn() {}, error() {} },
});
const api = createPaseoApi(dc);
const results: Record<string, unknown>[] = [];
const parents: string[] = [];
const testWorkspaces = new Set<string>();
const fingerprint = createHash("sha256");
for (const directory of ["server", "shared", "client"]) {
  for (const filename of (
    await readdir(path.join(repo, "plugins/jev-orchestrator", directory))
  ).sort()) {
    fingerprint.update(directory + "/" + filename);
    fingerprint.update(
      await readFile(path.join(repo, "plugins/jev-orchestrator", directory, filename)),
    );
  }
}
for (const filename of ["index.server.ts", "index.client.tsx", "package-lock.json"])
  fingerprint.update(await readFile(path.join(repo, "plugins/jev-orchestrator", filename)));
const sourceHash = fingerprint.digest("hex");
const save = () =>
  writeFile(
    path.join(folder, "results.json"),
    JSON.stringify(
      {
        runId,
        sourceHash,
        accountingVersion: 2,
        scope:
          "Fresh parent session plus every managed child and Jev evaluation; no task planning inference before the supplied contract.",
        results,
      },
      null,
      2,
    ),
  );
const config = (p: Profile) => ({
  provider: `${p.provider}/${p.model}`,
  modeId: p.modeId,
  thinkingOptionId: p.thinkingOptionId,
  featureValues: p.featureValues,
});
try {
  await dc.connect();
  const profiles = await createDriver(() => api).profiles(repo);
  const baseline = profiles.find((p) => p.id === "agent_profile_chase_goal_architect");
  if (!baseline)
    throw new Error("Configured architect baseline profile unavailable; no substitute.");
  const allowed = profiles.filter((p) =>
    [
      "agent_profile_chase_goal_logic",
      "agent_profile_chase_goal_architect",
      "agent_profile_chase_goal_reviewer",
    ].includes(p.id),
  );
  if (allowed.length < 2) throw new Error("Need two valid configured profiles.");
  console.log(
    JSON.stringify({
      runId,
      folder,
      profiles: allowed.map((p) => ({ id: p.id, model: p.model, effort: p.thinkingOptionId })),
    }),
  );
  const cases = [
    {
      id: "interval",
      brief:
        "Implement mergeIntervals(intervals): return a fresh array of sorted merged closed numeric intervals. Touching endpoints merge. Reject non-array input, malformed pairs, non-finite numbers and reversed endpoints with TypeError. Never mutate input.",
      test: `assert.deepEqual(fn([[5,7],[1,3],[3,6],[9,10]]),[[1,7],[9,10]]);assert.deepEqual(fn([]),[]);const a=[[4,5],[1,2]];fn(a);assert.deepEqual(a,[[4,5],[1,2]]);for(const x of [null,[[2,1]],[[NaN,2]],[[1,Infinity]],[[1]],[["1",2]]])assert.throws(()=>fn(x),TypeError);`,
      name: "mergeIntervals",
      discovery: "never",
      review: "never",
    },
    {
      id: "cache",
      brief:
        "The expiring cache returns stale values at the exact expiration boundary and changing ttl affects old entries. Inspect implementation and fix it. Preserve injected time, reject non-finite/negative TTL with TypeError, TTL zero expires immediately, and keep each entry's expiration fixed at insertion. Export createCache(now) returning set(key,value,ttl), get(key), and has(key).",
      test: `let t=100;const c=fn(()=>t);c.set("a",0,10);assert.equal(c.has("a"),true);t=110;assert.equal(c.get("a"),undefined);assert.equal(c.has("a"),false);c.set("z",false,0);assert.equal(c.has("z"),false);c.set("a",1,20);c.set("b",2,100);t=131;assert.equal(c.has("a"),false);assert.equal(c.get("b"),2);for(const ttl of [-1,NaN,Infinity])assert.throws(()=>c.set("x",1,ttl),TypeError);`,
      name: "createCache",
      discovery: "always",
      review: "always",
    },
  ];
  for (const [index, c] of cases.entries())
    for (const arm of index === 0 ? ["baseline", "routed"] : ["routed", "baseline"]) {
      const dir = path.join(folder, `${c.id}-${arm}`);
      await mkdir(dir);
      const target = path.join(dir, "solution.mjs");
      await writeFile(
        target,
        c.id === "cache"
          ? "export function createCache(now){const m=new Map();let lifetime=0;return {set(k,v,ttl){lifetime=ttl;m.set(k,{v,start:now()});},get(k){const e=m.get(k);return e&&now()-e.start<=lifetime?e.v:undefined},has(k){return this.get(k)!==undefined}}}\n"
          : `export function ${c.name}(){throw new Error("Not implemented");}\n`,
      );
      const check = path.join(folder, `${c.id}-${arm}-check.mjs`);
      await writeFile(
        check,
        `import assert from 'node:assert/strict';import {${c.name} as fn} from ${JSON.stringify(target)};${c.test}\nconsole.log("${c.id}: independent checks passed");\n`,
      );
      const before = await runChecks(
        repo,
        [{ argv: [process.execPath, check], timeoutMs: 5000 }],
        new AbortController().signal,
      );
      if (before[0].exitCode === 0) throw new Error("Fixture must fail before work");
      const relative = path.relative(repo, target);
      const goal = `${c.brief}\nEdit only ${relative}. This is a synthetic fixture; do not inspect unrelated repositories or files.`;
      const acceptance =
        "The independent external validation command must pass. Do not edit any test or other path.";
      const startedAt = Date.now();
      const parent = await api.agents.create({
        cwd: repo,
        config: config(baseline),
        title: `Jev measured ${c.id} ${arm}`,
        env: arm === "baseline" ? { PASEO_ORCH_CHILD: "1" } : undefined,
      });
      parents.push(parent.id);
      if (parent.workspaceId) testWorkspaces.add(parent.workspaceId);
      const record: Record<string, unknown> = {
        case: c.id,
        arm,
        parentId: parent.id,
        startedAt,
        beforeExit: before[0].exitCode,
      };
      results.push(record);
      await save();
      console.log(JSON.stringify({ case: c.id, arm, parentId: parent.id, status: "started" }));
      if (arm === "baseline") {
        const result = await parent.run(
          `${goal}\n${acceptance}\nStay in the current workspace. No delegation, commit, push or worktree. Implement the fix, do not stop at a plan.`,
          { timeoutMs: 300000 },
        );
        record.result = result;
        record.profile = baseline;
        record.latestUsage = parent.lastUsage;
        record.observed = parent.runtimeInfo;
      } else {
        const delegation = {
          tasks: [
            {
              id: c.id,
              goal,
              acceptance,
              kind: "implementation",
              files: [relative],
              allowedProfileIds: allowed.map((p) => p.id),
              checks: [{ argv: [process.execPath, check], timeoutMs: 10000 }],
              shareWithJev: true,
              discovery: c.discovery,
              review: c.review,
              maxAttempts: 2,
              maxDurationMs: 480000,
            },
          ],
        };
        const submission = await parent.run(
          `Call the injected delegate_task MCP tool exactly once with this JSON: ${JSON.stringify(delegation)}. Do not implement or run shell commands. After submission return only the task ID and stop. On a later completion notification, call orchestrator_status once and report its result without doing more work.`,
          { timeoutMs: 120000 },
        );
        record.submissionStatus = submission.status;
        while (Date.now() - startedAt < 520000) {
          await delay(5000);
          const data = (await dc.invokePluginRpc("jev-orchestrator", "jobs.list", {
            parentId: parent.id,
          })) as { jobs: Job[] };
          const job = data.jobs.find((j) => j.task.id === c.id);
          record.job = job;
          await save();
          if (
            job &&
            !["queued", "running"].includes(job.status) &&
            (job.notificationCompleteAt || job.notifyError)
          ) {
            console.log(
              JSON.stringify({
                case: c.id,
                arm,
                status: job.status,
                message: job.message,
                attempts: job.attempts.map((a) => ({
                  phase: a.phase,
                  profile: a.profile.name,
                  status: a.status,
                })),
              }),
            );
            break;
          }
        }
      }
      // The job becomes terminal before notify() has submitted the parent's completion turn.
      // Above we wait for notificationCompleteAt; now include that parent turn before measuring.
      if (arm === "routed") {
        const completion = await parent.waitForFinish(120000);
        record.parentCompletionStatus = completion.status;
      }
      await parent.refresh();
      const components: { id: string; role: string; usage: TokenUsage }[] = [
        { id: parent.id, role: "parent", usage: await readAgentUsage(parent.current()) },
      ];
      const job = record.job as Job | undefined;
      if (arm === "routed" && !job) {
        components.push({
          id: "missing-job",
          role: "orchestrator",
          usage: missingUsage("harness", "Parent did not submit a job."),
        });
      }
      for (const attempt of job?.attempts ?? []) {
        const child = api.agents.ref(attempt.childId);
        await child.refresh();
        components.push({
          id: child.id,
          role: attempt.phase,
          usage: await readAgentUsage(child.current()),
        });
      }
      for (const [i, evaluation] of (job?.evaluations ?? []).entries()) {
        components.push({
          id: `${job!.key}:jev:${i}`,
          role: `jev-${evaluation.phase}`,
          usage: evaluation.usage,
        });
      }
      record.workflowUsage = summarizeUsage(components);
      record.wallMs = Date.now() - startedAt;
      record.checks = await runChecks(
        repo,
        [{ argv: [process.execPath, check], timeoutMs: 10000 }],
        new AbortController().signal,
      );
      record.source = await readFile(target, "utf8");
      await save();
      // Reconcile after closure: provider logs can flush the final request after idle is emitted.
      record.workflowUsageBeforeArchive = structuredClone(record.workflowUsage);
      await parent.archive();
      for (const component of components) {
        if (component.role.startsWith("jev-") || component.id === "missing-job") continue;
        const actor = api.agents.ref(component.id);
        await actor.refresh();
        if (!actor.archivedAt) await actor.archive();
        await actor.refresh();
        component.usage = await readAgentUsage(actor.current());
      }
      record.workflowUsage = summarizeUsage(components);
      record.accountingCollectedAfterArchive = true;
      await save();
      console.log(
        JSON.stringify({
          case: c.id,
          arm,
          wallMs: record.wallMs,
          exitCode: (record.checks as { exitCode: number }[])[0].exitCode,
          workflowUsage: record.workflowUsage,
        }),
      );
    }
} catch (error) {
  results.push({ error: error instanceof Error ? error.message : "Live harness failed" });
  await save();
  process.exitCode = 1;
} finally {
  for (const id of parents) {
    try {
      const p = api.agents.ref(id);
      await p.refresh();
      if (!p.archivedAt) await p.archive();
    } catch {
      results.push({ cleanupFailed: id });
    }
  }
  // Top-level test agents own fresh local workspace records, not worktrees.
  // Preserve a workspace if another agent has since joined it.
  try {
    const agents = [];
    let cursor: string | undefined;
    do {
      const page = await api.agents.list({
        filter: { includeArchived: true },
        page: { limit: 100, ...(cursor ? { cursor } : {}) },
      });
      agents.push(...page.entries.map((entry) => entry.agent));
      cursor = page.pageInfo.hasMore ? (page.pageInfo.nextCursor ?? undefined) : undefined;
    } while (cursor);
    for (const workspaceId of testWorkspaces) {
      const members = agents.filter((agent) => agent.workspaceId === workspaceId);
      if (
        members.length &&
        members.every(
          (agent) =>
            agent.archivedAt &&
            (parents.includes(agent.id) ||
              parents.includes(agent.labels?.["paseo.parent-agent-id"] ?? "")),
        )
      ) {
        await api.workspaces.ref(workspaceId).archive();
      } else results.push({ workspaceCleanupSkipped: workspaceId });
    }
  } catch {
    results.push({ workspaceCleanupFailed: true });
  }
  await save();
  await dc.close();
  console.log(JSON.stringify({ artifact: path.join(folder, "results.json") }));
}
