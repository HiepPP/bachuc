import { createPaseoApi, type PaseoAgentHandle } from "@getpaseo/client";
import { DaemonClient } from "@getpaseo/client/internal/daemon-client";
import { createHash, randomUUID } from "node:crypto";
import { mkdir, readFile, readdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { readAgentUsage } from "../server/agent-usage";
import { runChecks } from "../server/checks";
import { createDriver } from "../server/paseo";
import type { Profile } from "../server/types";
import { missingUsage, summarizeUsage, type TokenUsage } from "../server/usage";

type Arm = "baseline" | "direct";
type DirectResponse = {
  agentId: string;
  selection: {
    id: string;
    name: string;
    provider: string;
    model: string;
    modeId?: string;
    thinkingOptionId?: string;
    featureValues?: Record<string, unknown>;
  };
  routingMs: number;
  usage: TokenUsage;
};
type DirectRecord = {
  requestId: string;
  workspaceId: string;
  status: "routing" | "creating" | "created" | "failed" | "interrupted";
  routingMs?: number;
  selection?: DirectResponse["selection"];
  agentId?: string;
  usage: TokenUsage;
  error?: string;
};

const repo = path.resolve(import.meta.dirname, "../../..");
const pluginRoot = path.join(repo, "plugins/jev-orchestrator");
const runId = `direct-live-${new Date().toISOString().replace(/[:.]/g, "-")}`;
const folder = path.join(pluginRoot, "artifacts", runId);
const repetitionsText = process.env.JEV_DIRECT_REPETITIONS ?? "2";
if (!/^[1-5]$/.test(repetitionsText))
  throw new Error("JEV_DIRECT_REPETITIONS must be an integer from 1 through 5.");
const repetitions = Number(repetitionsText);

await mkdir(folder, { recursive: true });
const dc = new DaemonClient({
  url: process.env.PASEO_TEST_URL ?? "ws://127.0.0.1:6767/ws",
  clientId: `jev-direct-benchmark-${runId}`,
  clientType: "cli",
  logger: { debug() {}, info() {}, warn() {}, error() {} },
});
const api = createPaseoApi(dc);
const results: Record<string, unknown>[] = [];
const cleanup: Record<string, unknown>[] = [];
const ownedAgents = new Set<string>();
const ownedWorkspaces = new Set<string>();

const fingerprint = createHash("sha256");
for (const directory of ["server", "shared", "client"]) {
  for (const filename of (await readdir(path.join(pluginRoot, directory))).sort()) {
    fingerprint.update(`${directory}/${filename}\0`);
    fingerprint.update(await readFile(path.join(pluginRoot, directory, filename)));
  }
}
for (const filename of ["index.server.ts", "index.client.tsx", "package-lock.json"]) {
  fingerprint.update(`${filename}\0`);
  fingerprint.update(await readFile(path.join(pluginRoot, filename)));
}
const sourceHash = fingerprint.digest("hex");

const save = () =>
  writeFile(
    path.join(folder, "results.json"),
    JSON.stringify(
      {
        runId,
        sourceHash,
        accountingVersion: 3,
        repetitions,
        scope:
          "Matched saved-architect versus direct Jev routing. Each arm uses one fresh workspace, one coding agent, the same pair prompt and unchanged external checks. Direct usage includes exactly one Jev routing call.",
        results,
        cleanup,
      },
      null,
      2,
    ),
  );

const profileConfig = (profile: Profile) => ({
  provider: `${profile.provider}/${profile.model}`,
  modeId: profile.modeId,
  thinkingOptionId: profile.thinkingOptionId,
  featureValues: profile.featureValues,
});

const profileSelection = (profile: Profile) => ({
  id: profile.id,
  name: profile.name,
  provider: profile.provider,
  model: profile.model,
  modeId: profile.modeId,
  thinkingOptionId: profile.thinkingOptionId,
  featureValues: profile.featureValues,
});

const errorMessage = (error: unknown) =>
  error instanceof Error ? `${error.name}: ${error.message}` : String(error);

const fileHash = async (file: string) =>
  createHash("sha256")
    .update(await readFile(file))
    .digest("hex");

const observedSettings = (agent: PaseoAgentHandle) => {
  const snapshot = agent.current();
  const featureValues = Object.fromEntries(
    (snapshot?.features ?? []).map((feature) => [feature.id, feature.value]),
  );
  return snapshot
    ? {
        provider: snapshot.provider,
        model: snapshot.model,
        thinkingOptionId: snapshot.thinkingOptionId,
        effectiveThinkingOptionId: snapshot.effectiveThinkingOptionId,
        modeId: snapshot.currentModeId,
        featureValues,
        features: snapshot.features,
        capabilities: snapshot.capabilities,
        runtimeInfo: snapshot.runtimeInfo,
      }
    : null;
};

const runtimeMatch = (agent: PaseoAgentHandle, selection: DirectResponse["selection"]) => {
  const snapshot = agent.current();
  const actualFeatures = Object.fromEntries(
    (snapshot?.features ?? []).map((feature) => [feature.id, feature.value]),
  );
  const checks = {
    provider: snapshot?.provider === selection.provider,
    model: snapshot?.model === selection.model,
    effectiveThinkingOptionId: snapshot?.effectiveThinkingOptionId === selection.thinkingOptionId,
    modeId: snapshot?.currentModeId === (selection.modeId ?? null),
    featureValues: Object.entries(selection.featureValues ?? {}).every(
      ([key, value]) => JSON.stringify(actualFeatures[key]) === JSON.stringify(value),
    ),
  };
  return { ...checks, passed: Object.values(checks).every(Boolean) };
};

const cases = [
  {
    id: "interval",
    brief:
      "Implement mergeIntervals(intervals): return a fresh array of sorted merged closed numeric intervals. Touching endpoints merge. Reject non-array input, malformed pairs, non-finite numbers and reversed endpoints with TypeError. Never mutate input.",
    initial: 'export function mergeIntervals(){throw new Error("Not implemented");}\n',
    name: "mergeIntervals",
    test: `assert.deepEqual(fn([[5,7],[1,3],[3,6],[9,10]]),[[1,7],[9,10]]);assert.deepEqual(fn([]),[]);const a=[[4,5],[1,2]];fn(a);assert.deepEqual(a,[[4,5],[1,2]]);for(const x of [null,[[2,1]],[[NaN,2]],[[1,Infinity]],[[1]],[["1",2]]])assert.throws(()=>fn(x),TypeError);`,
  },
  {
    id: "cache",
    brief:
      "The expiring cache returns stale values at the exact expiration boundary and changing ttl affects old entries. Inspect implementation and fix it. Preserve injected time, reject non-finite/negative TTL with TypeError, TTL zero expires immediately, and keep each entry's expiration fixed at insertion. Export createCache(now) returning set(key,value,ttl), get(key), and has(key).",
    initial:
      "export function createCache(now){const m=new Map();let lifetime=0;return {set(k,v,ttl){lifetime=ttl;m.set(k,{v,start:now()});},get(k){const e=m.get(k);return e&&now()-e.start<=lifetime?e.v:undefined},has(k){return this.get(k)!==undefined}}}\n",
    name: "createCache",
    test: `let t=100;const c=fn(()=>t);c.set("a",0,10);assert.equal(c.has("a"),true);t=110;assert.equal(c.get("a"),undefined);assert.equal(c.has("a"),false);c.set("z",false,0);assert.equal(c.has("z"),false);c.set("a",1,20);c.set("b",2,100);t=131;assert.equal(c.has("a"),false);assert.equal(c.get("b"),2);for(const ttl of [-1,NaN,Infinity])assert.throws(()=>c.set("x",1,ttl),TypeError);`,
  },
];

async function runArm(
  testCase: (typeof cases)[number],
  repetition: number,
  arm: Arm,
  target: string,
  check: string,
  prompt: string,
  expectedCheckHash: string,
  baseline: Profile,
  allowed: Profile[],
) {
  const harnessStartedAt = Date.now();
  const requestId = randomUUID();
  const record: Record<string, any> = {
    case: testCase.id,
    repetition,
    arm,
    requestId,
    harnessStartedAt,
    prompt,
    promptHash: createHash("sha256").update(prompt).digest("hex"),
    target: path.relative(repo, target),
    invariants: {
      parentAgent: false,
      delegationProhibitedByPrompt: true,
      orchestratorMcpInjectionDisabled: true,
      workspaceKind: "directory",
    },
  };
  results.push(record);
  await save();

  let agent: PaseoAgentHandle | undefined;
  let direct: DirectResponse | undefined;
  let directRecord: DirectRecord | undefined;
  try {
    await writeFile(target, testCase.initial);
    record.checkHashBefore = await fileHash(check);
    record.checkInitiallyUnchanged = record.checkHashBefore === expectedCheckHash;
    if (!record.checkInitiallyUnchanged) throw new Error("Independent check file changed.");
    record.failBeforeChecks = await runChecks(
      repo,
      [{ argv: [process.execPath, check], timeoutMs: 5000 }],
      new AbortController().signal,
    );
    record.failBeforeVerified = record.failBeforeChecks[0]?.exitCode !== 0;
    if (!record.failBeforeVerified)
      throw new Error("Fixture unexpectedly passed before the agent ran.");

    const workspace = await api.workspaces.create({
      source: { kind: "directory", path: repo },
      title: `Jev direct benchmark ${testCase.id} ${repetition} ${arm}`,
      requestId: randomUUID(),
    });
    record.workspaceId = workspace.id;
    ownedWorkspaces.add(workspace.id);

    record.executionStartedAt = Date.now();
    const launchStartedAt = Date.now();
    if (arm === "baseline") {
      record.selection = profileSelection(baseline);
      agent = await workspace.agents.create({
        config: profileConfig(baseline),
        env: { PASEO_ORCH_CHILD: "1" },
        labels: {
          "jev-direct-benchmark": runId,
          "jev-direct-arm": arm,
          "jev-direct-case": testCase.id,
        },
        prompt,
        requestId,
        title: `Jev direct baseline ${testCase.id} ${repetition}`,
      });
      record.routingMs = null;
    } else {
      direct = (await dc.invokePluginRpc("jev-orchestrator", "direct.run", {
        workspaceId: workspace.id,
        requestId,
        prompt,
        allowedProfileIds: allowed.map((profile) => profile.id),
        allowedModels: allowed.map((profile) => ({
          provider: profile.provider,
          model: profile.model,
          effortIds: profile.model === "gpt-5.6-luna" ? ["max"] : ["low", "medium", "high"],
        })),
        shareWithJev: true,
      })) as DirectResponse;
      record.directResponse = direct;
      record.selection = direct.selection;
      record.routingMs = direct.routingMs;
      agent = api.agents.ref(direct.agentId);
    }
    record.launchMs = Date.now() - launchStartedAt;
    record.agentId = agent.id;
    ownedAgents.add(agent.id);
    const completion = await agent.waitForFinish(300000);
    record.completion = completion;
    record.completionPassed = completion.status === "idle";
    if (!record.completionPassed)
      record.failure = `Agent finished with status ${completion.status}: ${completion.error ?? "no error"}`;
    await agent.refresh();
    record.observed = observedSettings(agent);
    record.runtimeMatch = runtimeMatch(agent, record.selection);
  } catch (error) {
    record.failure ??= errorMessage(error);
    if (arm === "direct" && record.workspaceId) {
      try {
        const status = (await dc.invokePluginRpc("jev-orchestrator", "direct.status", {
          workspaceId: record.workspaceId,
          requestId,
        })) as { records: DirectRecord[] };
        record.directStatus = status.records;
        directRecord = status.records.find((entry) => entry.requestId === requestId);
        if (directRecord) {
          record.routingMs ??= directRecord.routingMs ?? null;
          record.selection ??= directRecord.selection;
          if (directRecord.agentId && !agent) {
            agent = api.agents.ref(directRecord.agentId);
            ownedAgents.add(agent.id);
            record.agentId = agent.id;
          }
        }
      } catch (statusError) {
        record.directStatusFailure = errorMessage(statusError);
      }
    }
  }

  try {
    record.finalChecks = await runChecks(
      repo,
      [{ argv: [process.execPath, check], timeoutMs: 10000 }],
      new AbortController().signal,
    );
  } catch (error) {
    record.finalCheckFailure = errorMessage(error);
  }
  record.finalCheckPassed = record.finalChecks?.[0]?.exitCode === 0;
  try {
    record.checkHashAfter = await fileHash(check);
    record.checkUnchanged =
      record.checkHashBefore === expectedCheckHash && record.checkHashAfter === expectedCheckHash;
  } catch (error) {
    record.checkHashFailure = errorMessage(error);
    record.checkUnchanged = false;
  }
  if (record.executionStartedAt) record.executionWallMs = Date.now() - record.executionStartedAt;

  let agentUsage: TokenUsage;
  if (!agent) {
    agentUsage = missingUsage(
      "native-session",
      "Agent was not created or returned to the harness.",
    );
  } else {
    try {
      await agent.refresh();
      if (!agent.archivedAt) await agent.archive();
      await agent.refresh();
      record.archivedBeforeAccounting = Boolean(agent.archivedAt);
      agentUsage = await readAgentUsage(agent.current());
    } catch (error) {
      record.archiveOrAccountingFailure = errorMessage(error);
      agentUsage = missingUsage(
        "native-session",
        "Agent archival or post-archive native accounting failed.",
      );
    }
  }

  const components = [{ id: agent?.id ?? `${requestId}:agent`, role: "agent", usage: agentUsage }];
  if (arm === "direct") {
    components.push({
      id: `${requestId}:jev-route`,
      role: "jev-route",
      usage:
        direct?.usage ??
        directRecord?.usage ??
        missingUsage("ai-sdk-evaluate-response", "Direct routing response was unavailable."),
    });
  }
  record.workflowUsage = summarizeUsage(components);
  record.accountingCollectedAfterArchive = Boolean(agent && record.archivedBeforeAccounting);
  try {
    record.source = await readFile(target, "utf8");
  } catch (error) {
    record.sourceReadFailure = errorMessage(error);
  }
  record.integrityPassed = Boolean(
    record.failBeforeVerified &&
    record.finalCheckPassed &&
    record.checkUnchanged &&
    record.completionPassed &&
    record.runtimeMatch?.passed &&
    !record.failure,
  );
  record.harnessWallMs = Date.now() - harnessStartedAt;
  await save();
  console.log(
    JSON.stringify({
      case: testCase.id,
      repetition,
      arm,
      agentId: agent?.id,
      executionWallMs: record.executionWallMs,
      harnessWallMs: record.harnessWallMs,
      routingMs: record.routingMs,
      exitCode: record.finalChecks?.[0]?.exitCode ?? null,
      failure: record.failure,
      workflowUsage: record.workflowUsage,
    }),
  );
}

try {
  await dc.connect();
  const profiles = await createDriver(() => api).profiles(repo);
  const baseline = profiles.find((profile) => profile.id === "agent_profile_chase_goal_architect");
  if (!baseline)
    throw new Error("Configured architect baseline profile unavailable; no substitute.");
  const allowed = profiles.filter((profile) =>
    ["agent_profile_chase_goal_logic", "agent_profile_chase_goal_architect"].includes(profile.id),
  );
  if (allowed.length !== 2)
    throw new Error("Need the valid configured logic and architect routing profiles.");

  console.log(
    JSON.stringify({
      runId,
      folder,
      repetitions,
      baseline: profileSelection(baseline),
      allowed: allowed.map(profileSelection),
    }),
  );

  for (const testCase of cases) {
    for (let repetition = 1; repetition <= repetitions; repetition += 1) {
      const pairFolder = path.join(folder, `${testCase.id}-pair-${repetition}`);
      await mkdir(pairFolder);
      const target = path.join(pairFolder, "solution.mjs");
      const check = path.join(pairFolder, "check.mjs");
      await writeFile(
        check,
        `import assert from 'node:assert/strict';import {${testCase.name} as fn} from ${JSON.stringify(target)};${testCase.test}\nconsole.log(${JSON.stringify(`${testCase.id}: independent checks passed`)});\n`,
      );
      const expectedCheckHash = await fileHash(check);
      const relative = path.relative(repo, target);
      const prompt = `${testCase.brief}\nEdit only ${relative}. This is a synthetic fixture; do not inspect unrelated repositories or files.\nThe independent external validation command must pass. Do not edit any test or other path.\nStay in the current workspace. Do not delegate or create subagents. Do not commit, push, publish, deploy or create a worktree. Implement the fix; do not stop at a plan.`;
      const order: Arm[] = repetition % 2 === 1 ? ["baseline", "direct"] : ["direct", "baseline"];
      for (const arm of order) {
        try {
          await runArm(
            testCase,
            repetition,
            arm,
            target,
            check,
            prompt,
            expectedCheckHash,
            baseline,
            allowed,
          );
        } catch (error) {
          results.push({
            case: testCase.id,
            repetition,
            arm,
            harnessArmFailure: errorMessage(error),
          });
          process.exitCode = 1;
          await save();
        }
      }
    }
  }
} catch (error) {
  results.push({ harnessFailure: errorMessage(error) });
  process.exitCode = 1;
} finally {
  try {
    for (const id of ownedAgents) {
      try {
        const agent = api.agents.ref(id);
        await agent.refresh();
        if (!agent.archivedAt) await agent.archive();
      } catch (error) {
        cleanup.push({ agentId: id, error: errorMessage(error) });
      }
    }

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

    for (const workspaceId of ownedWorkspaces) {
      const members = agents.filter((agent) => agent.workspaceId === workspaceId);
      if (members.every((agent) => ownedAgents.has(agent.id) && Boolean(agent.archivedAt))) {
        const archived = await api.workspaces.ref(workspaceId).archive();
        cleanup.push({ workspaceId, archived });
      } else {
        cleanup.push({
          workspaceId,
          skipped: true,
          memberIds: members.map((agent) => agent.id),
          reason: "Workspace contains an unowned or active agent.",
        });
      }
    }
  } catch (error) {
    cleanup.push({ failure: errorMessage(error) });
  }
  if (
    results.some(
      (record: Record<string, any>) =>
        record.harnessFailure ||
        record.harnessArmFailure ||
        record.failure ||
        record.integrityPassed === false,
    )
  )
    process.exitCode = 1;
  await save();
  await dc.close();
  console.log(JSON.stringify({ artifact: path.join(folder, "results.json") }));
}
