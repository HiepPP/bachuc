import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdtemp, readFile, rmdir, unlink, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import test from "node:test";
import type { PaseoApi, PaseoProviderModelsResult } from "@getpaseo/client";
import { DirectRouter, expandCandidates } from "../server/direct";
import type { Decision, Judge, Profile } from "../server/types";
import { missingUsage, type TokenUsage } from "../server/usage";
import { directInput, type DirectInput } from "../shared/direct";

const workspaceId = "workspace-1";
const requestId = "11111111-2222-4333-8444-555555555555";
const prompt = "  Preserve this prompt exactly.\n";
const baseProfile: Profile = {
  id: "focused",
  name: "Focused",
  provider: "codex",
  model: "model-a",
  modeId: "auto-review",
  thinkingOptionId: "medium",
  featureValues: { webSearch: false },
  notes: "Focused implementation",
};
const model = {
  id: "model-a",
  label: "Model A",
  description: "Model A description",
  isSelectable: true,
  thinkingOptions: [
    { id: "low", label: "Low", description: "Quick reasoning" },
    { id: "medium", label: "Medium", description: "Balanced reasoning" },
    { id: "high", label: "High", description: "Deep reasoning" },
  ],
};
const usage = (source = "jev"): TokenUsage => ({
  source,
  complete: true,
  inputTokens: 10,
  cachedInputTokens: null,
  cacheWriteTokens: null,
  outputTokens: 2,
  reasoningTokens: null,
  totalTokens: 12,
  requests: 1,
  notes: [],
});
const parsedInput = (overrides: Partial<DirectInput> = {}) =>
  directInput.parse({
    workspaceId,
    requestId,
    prompt,
    allowedProfileIds: [baseProfile.id],
    allowedModels: [{ provider: "codex", model: "model-a", effortIds: ["low", "high"] }],
    shareWithJev: true,
    ...overrides,
  });

type AgentCreateInput = {
  config: {
    provider: string;
    modeId?: string;
    thinkingOptionId?: string;
    featureValues?: Record<string, unknown>;
  };
  prompt: string;
  title: string;
  env: Record<string, string>;
  labels: Record<string, string>;
  requestId: string;
  parent?: string;
};

function harness(root: string) {
  const state = {
    workspaceDirectory: root,
    profiles: [structuredClone(baseProfile)],
    models: [structuredClone(model)],
  };
  const creates: AgentCreateInput[] = [];
  const workspaceLookups: unknown[] = [];
  const modelLookups: { provider: string; cwd?: string }[] = [];
  const api = {
    workspaces: {
      list: async (options: unknown) => {
        workspaceLookups.push(structuredClone(options));
        return {
          entries: [{ id: workspaceId, workspaceDirectory: state.workspaceDirectory }],
        };
      },
      ref: (_workspace: unknown) => ({
        agents: {
          create: async (options: AgentCreateInput) => {
            creates.push(structuredClone(options));
            return { id: `agent-${creates.length}` };
          },
        },
      }),
    },
    config: {
      get: async () => ({
        requestId: "config-request",
        config: { agentProfiles: structuredClone(state.profiles) },
      }),
    },
    providers: {
      listAvailable: async () => ({
        providers: [
          { provider: "codex", available: true },
          { provider: "claude", available: true },
        ],
      }),
      listModels: async (provider: string, options?: { cwd?: string }) => {
        modelLookups.push({ provider, cwd: options?.cwd });
        return { models: structuredClone(state.models) };
      },
      listModes: async () => ({ modes: [{ id: "auto-review", label: "Auto review" }] }),
    },
  };
  return {
    getApi: () => api as unknown as PaseoApi,
    state,
    creates,
    workspaceLookups,
    modelLookups,
  };
}

async function temporaryDirectory(prefix = "jev-direct-") {
  const directory = await mkdtemp(path.join(tmpdir(), prefix));
  return {
    directory,
    async cleanup(...files: string[]) {
      for (const file of files) {
        try {
          await unlink(file);
        } catch (error) {
          if ((error as NodeJS.ErrnoException).code !== "ENOENT") throw error;
        }
      }
      await rmdir(directory);
    },
  };
}

test("direct input requires explicit sharing consent and rejects unknown fields", () => {
  assert.throws(() => directInput.parse({ ...parsedInput(), shareWithJev: false }));
  assert.throws(() => directInput.parse({ ...parsedInput(), unexpected: true }));
  assert.throws(() => directInput.parse({ ...parsedInput(), prompt: "          " }));
});

test("candidate expansion uses supported efforts and catalog descriptions", () => {
  const profiles: Profile[] = [{ ...baseProfile, notes: "Coding" }];
  const catalogs = new Map<string, PaseoProviderModelsResult>([
    [
      "codex",
      {
        models: [
          model,
          {
            ...model,
            id: "model-disabled",
            isSelectable: false,
          },
        ],
      } as PaseoProviderModelsResult,
    ],
    ["claude", { error: "catalog unavailable" } as PaseoProviderModelsResult],
  ]);

  const candidates = expandCandidates(profiles, catalogs, [
    { provider: "codex", model: "model-a", effortIds: ["low", "high"] },
  ]);

  assert.deepEqual(
    candidates.map(({ id, profileId, thinkingOptionId, notes }) => ({
      id,
      profileId,
      thinkingOptionId,
      notes,
    })),
    [
      {
        id: "c0",
        profileId: "focused",
        thinkingOptionId: "low",
        notes: "Coding. Model: Model A description. Effort: Quick reasoning.",
      },
      {
        id: "c1",
        profileId: "focused",
        thinkingOptionId: "high",
        notes: "Coding. Model: Model A description. Effort: Deep reasoning.",
      },
    ],
  );
});

test("one Jev choice creates one direct agent with exact settings and unchanged prompt", async () => {
  const fixture = harness(process.cwd());
  let calls = 0;
  const billed = usage();
  const judge: Judge = async (phase, state, candidates) => {
    calls++;
    assert.equal(phase, "direct");
    assert.deepEqual(state, { task: prompt });
    assert.equal(candidates.length, 2);
    return {
      profileId: candidates[1].id,
      discovery: false,
      risk: "low",
      category: "direct",
      usage: billed,
    };
  };
  const router = new DirectRouter(fixture.getApi, judge);

  const result = await router.run(parsedInput());

  assert.equal(calls, 1);
  assert.equal(result.agentId, "agent-1");
  assert.deepEqual(result.usage, billed);
  assert.equal(fixture.creates.length, 1);
  assert.deepEqual(fixture.creates[0].config, {
    provider: "codex/model-a",
    modeId: "auto-review",
    thinkingOptionId: "high",
    featureValues: { webSearch: false },
  });
  assert.equal(fixture.creates[0].prompt, prompt);
  assert.equal(Object.hasOwn(fixture.creates[0], "parent"), false);
  assert.deepEqual(fixture.workspaceLookups, [
    { filter: { idPrefix: workspaceId }, page: { limit: 100 } },
    { filter: { idPrefix: workspaceId }, page: { limit: 100 } },
  ]);
  assert.ok(fixture.modelLookups.every((lookup) => lookup.cwd === process.cwd()));
});

test("concurrent and persisted retries create only once", async () => {
  const temporary = await temporaryDirectory();
  const file = path.join(temporary.directory, "state.json");
  const fixture = harness(process.cwd());
  let calls = 0;
  const judge: Judge = async (_phase, _state, candidates) => {
    calls++;
    return {
      profileId: candidates[0].id,
      discovery: false,
      risk: "low",
      category: "direct",
    };
  };
  try {
    const router = new DirectRouter(fixture.getApi, judge, file);
    const [first, concurrent] = await Promise.all([
      router.run(parsedInput()),
      router.run(parsedInput()),
    ]);
    const restarted = new DirectRouter(fixture.getApi, judge, file);
    const persisted = await restarted.run(parsedInput());

    assert.equal(first.agentId, "agent-1");
    assert.equal(concurrent.agentId, first.agentId);
    assert.equal(persisted.agentId, first.agentId);
    assert.equal(calls, 1);
    assert.equal(fixture.creates.length, 1);
  } finally {
    await temporary.cleanup(file, `${file}.tmp`);
  }
});

test("a reused request ID with changed input is rejected", async () => {
  const fixture = harness(process.cwd());
  const judge: Judge = async (_phase, _state, candidates) => ({
    profileId: candidates[0].id,
    discovery: false,
    risk: "low",
    category: "direct",
  });
  const router = new DirectRouter(fixture.getApi, judge);
  await router.run(parsedInput());

  assert.throws(
    () => router.run(parsedInput({ prompt: "A different prompt with the same request ID." })),
    /Request ID already used for different input/,
  );
  assert.equal(fixture.creates.length, 1);
});

test("Jev failures retain billed usage and never launch a fallback", async () => {
  const fixture = harness(process.cwd());
  const billed = usage("failed-jev-call");
  const router = new DirectRouter(fixture.getApi, async () => {
    throw Object.assign(new Error("Jev rejected the request."), { usage: billed });
  });

  await assert.rejects(router.run(parsedInput()), /Jev rejected the request/);

  assert.equal(fixture.creates.length, 0);
  const record = router.list(workspaceId, requestId)[0];
  assert.equal(record.status, "failed");
  assert.equal(record.error, "Jev rejected the request.");
  assert.deepEqual(record.usage, billed);
});

test("workspace and profile changes during evaluation fail closed", async () => {
  const alternate = await temporaryDirectory("jev-direct-other-");
  try {
    const workspaceFixture = harness(process.cwd());
    const workspaceRouter = new DirectRouter(
      workspaceFixture.getApi,
      async (_phase, _state, candidates) => {
        workspaceFixture.state.workspaceDirectory = alternate.directory;
        return {
          profileId: candidates[0].id,
          discovery: false,
          risk: "low",
          category: "direct",
        };
      },
    );
    await assert.rejects(
      workspaceRouter.run(parsedInput()),
      /Workspace directory changed during routing/,
    );
    assert.equal(workspaceFixture.creates.length, 0);

    const profileFixture = harness(process.cwd());
    const profileRouter = new DirectRouter(
      profileFixture.getApi,
      async (_phase, _state, candidates) => {
        profileFixture.state.profiles[0].featureValues = { webSearch: true };
        return {
          profileId: candidates[0].id,
          discovery: false,
          risk: "low",
          category: "direct",
        };
      },
    );
    await assert.rejects(
      profileRouter.run(parsedInput()),
      /Candidate settings changed during routing/,
    );
    assert.equal(profileFixture.creates.length, 0);
  } finally {
    await alternate.cleanup();
  }
});

test("shutdown during evaluation prevents agent launch", async () => {
  const fixture = harness(process.cwd());
  let release!: (decision: Decision) => void;
  let entered!: () => void;
  const evaluating = new Promise<void>((resolve) => {
    entered = resolve;
  });
  let observedSignal: AbortSignal | undefined;
  const judge: Judge = async (_phase, _state, candidates, signal) => {
    observedSignal = signal;
    entered();
    return await new Promise<Decision>((resolve) => {
      release = resolve;
    });
  };
  const router = new DirectRouter(fixture.getApi, judge);
  const running = router.run(parsedInput());
  await evaluating;

  router.stop();
  release({ profileId: "c0", discovery: false, risk: "low", category: "direct" });

  await assert.rejects(running, /Routing cancelled before agent creation/);
  assert.equal(observedSignal?.aborted, true);
  assert.equal(fixture.creates.length, 0);
});

test("restart marks a creating ledger entry interrupted and does not replay it", async () => {
  const temporary = await temporaryDirectory();
  const file = path.join(temporary.directory, "state.json");
  const fixture = harness(process.cwd());
  const input = parsedInput();
  const fingerprint = createHash("sha256").update(JSON.stringify(input)).digest("hex");
  let calls = 0;
  try {
    await writeFile(
      file,
      JSON.stringify({
        version: 1,
        records: [
          {
            requestId,
            workspaceId,
            fingerprint,
            status: "creating",
            startedAt: Date.now(),
            selection: { ...baseProfile, thinkingOptionId: "low" },
            routingMs: 5,
            usage: missingUsage("jev-evaluation", "Evaluation response omitted usage."),
          },
        ],
      }),
    );
    const router = new DirectRouter(
      fixture.getApi,
      async () => {
        calls++;
        throw new Error("Judge must not run.");
      },
      file,
    );

    assert.throws(() => router.run(input), /Plugin restarted during request/);
    assert.equal(calls, 0);
    assert.equal(fixture.creates.length, 0);
    assert.equal(router.list(workspaceId, requestId)[0].status, "interrupted");
    const stored = JSON.parse(await readFile(file, "utf8"));
    assert.equal(stored.records[0].status, "interrupted");
  } finally {
    await temporary.cleanup(file, `${file}.tmp`);
  }
});

test("per-model allowlist cannot leak efforts across models or use Luna aliases below max", () => {
  const profiles = [{ ...baseProfile }, { ...baseProfile, id: "luna", model: "small-alias" }];
  const catalogs = new Map<string, PaseoProviderModelsResult>([
    [
      "codex",
      {
        models: [
          model,
          {
            ...model,
            id: "gpt-5.6-luna",
            aliases: ["small-alias"],
            thinkingOptions: [...model.thinkingOptions, { id: "max", label: "Max" }],
          },
        ],
      } as PaseoProviderModelsResult,
    ],
  ]);
  const allowed = [
    { provider: "codex", model: "model-a", effortIds: ["low", "high"] as ("low" | "high")[] },
    { provider: "codex", model: "small-alias", effortIds: ["max"] as "max"[] },
  ];
  assert.deepEqual(
    expandCandidates(profiles, catalogs, allowed).map((c) => [c.model, c.thinkingOptionId]),
    [
      ["model-a", "low"],
      ["model-a", "high"],
      ["small-alias", "max"],
    ],
  );
  assert.throws(
    () => expandCandidates(profiles, catalogs, [allowed[0], { ...allowed[1], effortIds: ["low"] }]),
    /Luna only permits max/,
  );
  assert.throws(() => expandCandidates(profiles, catalogs, [allowed[0]]), /missing from allowlist/);
  assert.throws(
    () => expandCandidates(profiles, catalogs, [{ ...allowed[0], effortIds: ["max"] }, allowed[1]]),
    /Unsupported effort/,
  );
  assert.throws(
    () =>
      expandCandidates(profiles, catalogs, [
        ...allowed,
        { provider: "claude", model: "unknown", effortIds: ["high"] },
      ]),
    /no selected profile/,
  );
});

test("invalid allowlists fail before Jev or launch", async () => {
  const fixture = harness(process.cwd());
  let calls = 0;
  const router = new DirectRouter(fixture.getApi, async (_phase, _state, candidates) => {
    calls++;
    return { profileId: candidates[0].id, discovery: false, risk: "low", category: "direct" };
  });
  const invalid = parsedInput({
    allowedModels: [{ provider: "codex", model: "model-a", effortIds: ["max"] }],
  });
  await assert.rejects(router.run(invalid), /Unsupported effort/);
  assert.equal(calls, 0);
  assert.equal(fixture.creates.length, 0);
  assert.throws(() => directInput.parse({ ...parsedInput(), allowedModels: [] }));
  assert.throws(() =>
    directInput.parse({
      ...parsedInput(),
      allowedModels: [...parsedInput().allowedModels, ...parsedInput().allowedModels],
    }),
  );
  assert.throws(() => directInput.parse({ ...parsedInput(), allowedEffortIds: ["low"] }));
});

test("Luna nonmax is rejected before evaluation and profiles only offer max", async () => {
  const fixture = harness(process.cwd());
  fixture.state.profiles[0].model = "gpt-5.6-luna";
  fixture.state.models[0].id = "gpt-5.6-luna";
  fixture.state.models[0].thinkingOptions.push({ id: "max", label: "Max", description: "Max" });
  let calls = 0;
  const router = new DirectRouter(fixture.getApi, async () => {
    calls++;
    throw new Error("Must not evaluate");
  });
  assert.deepEqual((await router.profiles(workspaceId))[0].effortIds, ["max"]);
  await assert.rejects(
    router.run(
      parsedInput({
        allowedModels: [{ provider: "codex", model: "gpt-5.6-luna", effortIds: ["low"] }],
      }),
    ),
    /Luna only permits max/,
  );
  assert.equal(calls, 0);
  assert.equal(fixture.creates.length, 0);
});

test("a Jev choice outside the supplied candidates cannot launch", async () => {
  const fixture = harness(process.cwd());
  const router = new DirectRouter(fixture.getApi, async () => ({
    profileId: "unlisted",
    discovery: false,
    risk: "low",
    category: "direct",
    usage: usage(),
  }));
  await assert.rejects(router.run(parsedInput()), /unknown candidate/);
  assert.equal(fixture.creates.length, 0);
});
