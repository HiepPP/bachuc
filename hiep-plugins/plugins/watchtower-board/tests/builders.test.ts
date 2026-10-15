import assert from "node:assert/strict";
import { test } from "node:test";
import { type BuilderCandidate, liveTaskIds, matchBuilders } from "../client/builders";

const NOW = Date.parse("2026-10-14T12:00:00.000Z");
const TASKS = [
  { id: "TASK-089", group: "D" },
  { id: "TASK-090", group: "D" },
  { id: "TASK-091", group: "E" },
  { id: "TASK-092", group: null },
];

function agent(id: string, patch: Partial<BuilderCandidate> = {}): BuilderCandidate {
  return {
    id,
    title: null,
    status: "idle",
    workspaceId: "ws-1",
    archivedAt: null,
    updatedAt: "2026-10-14T11:50:00.000Z",
    labels: {},
    provider: "claude",
    ...patch,
  };
}

function builder(id: string, label: string, patch: Partial<BuilderCandidate> = {}) {
  return agent(id, { labels: { "watchtower.task": label }, ...patch });
}

test("a TASK ID label maps to every TASK of its group", () => {
  const { byTask } = matchBuilders([builder("b1", "TASK-089")], TASKS, NOW);
  assert.deepEqual(Object.keys(byTask).sort(), ["TASK-089", "TASK-090"]);
  assert.equal(byTask["TASK-090"]?.id, "b1");
});

test("a standalone TASK ID label maps to that TASK only", () => {
  const tasks = [
    { id: "TASK-001", group: "standalone" },
    { id: "TASK-002", group: "standalone" },
  ];
  const { byTask } = matchBuilders([builder("b1", "TASK-001")], tasks, NOW);
  assert.deepEqual(Object.keys(byTask), ["TASK-001"]);
  assert.deepEqual(
    Object.keys(matchBuilders([builder("b2", "standalone")], tasks, NOW).byTask),
    [],
  );
});

test("live TASKs are those whose builder runs or waits, sorted", () => {
  const match = matchBuilders(
    [
      builder("b1", "TASK-091", { status: "running" }),
      builder("b2", "TASK-089", { status: "running", pendingPermissions: [{} as never] }),
      builder("b3", "TASK-092", { status: "idle" }),
    ],
    TASKS,
    NOW,
  );
  assert.deepEqual(liveTaskIds(match), ["TASK-089", "TASK-090", "TASK-091"]);
});

test("a TASK ID label of a TASK without a group maps to that TASK only", () => {
  const { byTask } = matchBuilders([builder("b1", "TASK-092")], TASKS, NOW);
  assert.deepEqual(Object.keys(byTask), ["TASK-092"]);
});

test("a Group tag label maps to every TASK of that group", () => {
  const { byTask } = matchBuilders([builder("b1", "E"), builder("b2", "D")], TASKS, NOW);
  assert.equal(byTask["TASK-091"]?.id, "b1");
  assert.equal(byTask["TASK-089"]?.id, "b2");
  assert.equal(byTask["TASK-090"]?.id, "b2");
});

test("unknown labels, missing labels, archived and mock agents never match", () => {
  const agents = [
    builder("unknown-task", "TASK-999"),
    builder("unknown-group", "Z"),
    agent("no-label"),
    builder("archived", "TASK-089", { archivedAt: "2026-10-14T11:00:00.000Z" }),
    builder("mock", "TASK-089", { provider: "mock-provider" }),
    builder("blank", "  "),
  ];
  const { byTask, lead } = matchBuilders(agents, TASKS, NOW);
  assert.deepEqual(byTask, {});
  assert.equal(lead, null);
});

test("agents of another workspace still match: the caller filters the workspace", () => {
  const { byTask } = matchBuilders(
    [builder("b1", "TASK-091", { workspaceId: "ws-2" })],
    TASKS,
    NOW,
  );
  assert.equal(byTask["TASK-091"]?.id, "b1");
});

test("two agents on one TASK keep the newest by updatedAt", () => {
  const older = builder("old", "TASK-089", { updatedAt: "2026-10-14T09:00:00.000Z" });
  const newer = builder("new", "D", { updatedAt: "2026-10-14T11:00:00.000Z" });
  assert.equal(matchBuilders([newer, older], TASKS, NOW).byTask["TASK-089"]?.id, "new");
  assert.equal(matchBuilders([older, newer], TASKS, NOW).byTask["TASK-089"]?.id, "new");
});

test("builder state: running, waiting, failed, else idle", () => {
  const states = (patch: Partial<BuilderCandidate>) =>
    matchBuilders([builder("b", "TASK-091", patch)], TASKS, NOW).byTask["TASK-091"]?.state;
  assert.equal(states({ status: "running" }), "running");
  assert.equal(
    states({ status: "running", requiresAttention: true, attentionReason: "permission" }),
    "waiting",
  );
  assert.equal(
    states({ status: "idle", requiresAttention: true, attentionReason: "permission" }),
    "waiting",
  );
  assert.equal(
    states({
      status: "running",
      pendingPermissions: [{}] as unknown as BuilderCandidate["pendingPermissions"],
    }),
    "waiting",
  );
  assert.equal(
    states({ status: "running", pendingPermissions: [] as BuilderCandidate["pendingPermissions"] }),
    "running",
  );
  assert.equal(states({ status: "error" }), "failed");
  assert.equal(
    states({ status: "idle", requiresAttention: true, attentionReason: "finished" }),
    "idle",
  );
  assert.equal(states({ status: "idle" }), "idle");
  assert.equal(states({ status: "closed" }), "idle");
});

test("activeAgo counts from lastActivityAt, else updatedAt", () => {
  const ago = (patch: Partial<BuilderCandidate>) =>
    matchBuilders([builder("b", "TASK-091", patch)], TASKS, NOW).byTask["TASK-091"]?.activeAgo;
  assert.equal(ago({}), 10 * 60_000);
  assert.equal(ago({ lastActivityAt: "2026-10-14T11:59:00.000Z" }), 60_000);
  assert.equal(ago({ updatedAt: "not a date" }), null);
  assert.equal(ago({ updatedAt: "2026-10-14T12:30:00.000Z" }), 0);
});

test("the title falls back to the agent id", () => {
  const { byTask } = matchBuilders(
    [builder("b1", "TASK-091", { title: " Builder E " }), builder("b2", "TASK-092")],
    TASKS,
    NOW,
  );
  assert.equal(byTask["TASK-091"]?.title, "Builder E");
  assert.equal(byTask["TASK-092"]?.title, "b2");
});

test("the lead is the parent most matched builders name", () => {
  const agents = [
    builder("b1", "TASK-089", {
      labels: { "watchtower.task": "TASK-089", "paseo.parent-agent-id": "lead-a" },
    }),
    builder("b2", "TASK-091", { parentAgentId: "lead-a" }),
    builder("b3", "TASK-092", { parentAgentId: "lead-b" }),
    agent("lead-a", {
      title: "Run lead",
      status: "running",
      updatedAt: "2026-10-14T11:58:00.000Z",
    }),
    agent("lead-b"),
  ];
  const { lead } = matchBuilders(agents, TASKS, NOW);
  assert.deepEqual(lead, {
    id: "lead-a",
    title: "Run lead",
    state: "running",
    activeAgo: 2 * 60_000,
  });
});

test("a lead missing from the list keeps its id with no title", () => {
  const { lead } = matchBuilders(
    [builder("b1", "TASK-091", { parentAgentId: "gone" })],
    TASKS,
    NOW,
  );
  assert.deepEqual(lead, { id: "gone", title: null, state: "idle", activeAgo: null });
});

test("with no parent there is no lead", () => {
  assert.equal(matchBuilders([builder("b1", "TASK-091")], TASKS, NOW).lead, null);
});

test("builders replaced by a newer agent do not vote for the lead", () => {
  const agents = [
    builder("old", "TASK-091", { updatedAt: "2026-10-14T08:00:00.000Z", parentAgentId: "stale" }),
    builder("new", "TASK-091", { updatedAt: "2026-10-14T11:00:00.000Z", parentAgentId: "fresh" }),
  ];
  assert.equal(matchBuilders(agents, TASKS, NOW).lead?.id, "fresh");
});
