import assert from "node:assert/strict";
import { test } from "node:test";
import {
  type AgentCandidate,
  acceptDecisionPrompt,
  answerPrompt,
  chooseAgent,
  defaultAnswerPrompt,
  manualChecksPrompt,
  rejectDecisionPrompt,
} from "../client/actions";

test("prompts name the watchtower skill and the file to change", () => {
  assert.equal(
    answerPrompt("Q-008", "Network access?", "  No network "),
    'Watchtower: the owner answers Q-008 ("Network access?"): No network. Use the watchtower skill: write the answer into its watchtower/QUESTIONS.md row, set ANSWERED, add it to the Boundaries of the specs it changes, and unblock the tasks it blocked.',
  );
  assert.equal(
    defaultAnswerPrompt("Q-010", "Native scroll?", "Open the agent"),
    'Watchtower: the owner answers Q-010 ("Native scroll?"): use the default: Open the agent. Use the watchtower skill: write the answer into its watchtower/QUESTIONS.md row, set ANSWERED, add it to the Boundaries of the specs it changes, and unblock the tasks it blocked.',
  );
  assert.equal(
    acceptDecisionPrompt("ADR-0003", "Chip-only sources"),
    "Watchtower: the owner accepts the proposed ADR-0003 (Chip-only sources). Use the watchtower skill decision rules: set Status: accepted in the ADR file and its watchtower/DECISIONS.md row, and add its ## Avoid item to watchtower/MEMORY.md (R4).",
  );
  assert.equal(
    rejectDecisionPrompt("ADR-0003", "Chip-only sources"),
    "Watchtower: the owner rejects the proposed ADR-0003 (Chip-only sources). Use the watchtower skill decision rules: delete the draft ADR file and its watchtower/DECISIONS.md index row.",
  );
  assert.equal(
    manualChecksPrompt(["Stop a parent (TASK-001).", "Queued message (TASK-003)."]),
    "Watchtower: the owner ran the pending manual checks: Stop a parent (TASK-001); Queued message (TASK-003). Use the watchtower skill: record them as done in the Handoff of watchtower/NEXT.md.",
  );
  assert.match(
    defaultAnswerPrompt("Q-001", "Read-only?", "Read-only. Copy actions."),
    /: use the default: Read-only\. Copy actions\. Use the watchtower skill/,
  );
});

function agent(id: string, patch: Partial<AgentCandidate> = {}): AgentCandidate {
  return {
    id,
    title: null,
    status: "idle",
    workspaceId: "ws-1",
    archivedAt: null,
    updatedAt: "2026-10-09T10:00:00.000Z",
    labels: {},
    ...patch,
  };
}

test("the target is the newest idle agent of the workspace", () => {
  const target = chooseAgent(
    [
      agent("old", { updatedAt: "2026-10-09T08:00:00.000Z", title: "Old" }),
      agent("new", { updatedAt: "2026-10-09T11:00:00.000Z", title: " Newest " }),
      agent("other", { workspaceId: "ws-2", updatedAt: "2026-10-09T12:00:00.000Z" }),
      agent("gone", {
        archivedAt: "2026-10-09T09:00:00.000Z",
        updatedAt: "2026-10-09T13:00:00.000Z",
      }),
    ],
    "ws-1",
  );
  assert.deepEqual(target, { state: "ready", id: "new", name: "Newest" });
});

test("a running agent is skipped for an older idle one, and the name falls back to the id", () => {
  const target = chooseAgent(
    [
      agent("busy", { status: "running", updatedAt: "2026-10-09T12:00:00.000Z" }),
      agent("starting", { status: "initializing", updatedAt: "2026-10-09T11:00:00.000Z" }),
      agent("idle", { status: "error", updatedAt: "2026-10-09T09:00:00.000Z" }),
    ],
    "ws-1",
  );
  assert.deepEqual(target, { state: "ready", id: "idle", name: "idle" });
});

test("only working agents give busy, and no agent gives none", () => {
  assert.deepEqual(chooseAgent([agent("busy", { status: "running" })], "ws-1"), {
    state: "busy",
  });
  assert.deepEqual(chooseAgent([], "ws-1"), { state: "none" });
  assert.deepEqual(chooseAgent([agent("a", { workspaceId: "ws-2" })], "ws-1"), { state: "none" });
  assert.deepEqual(chooseAgent([agent("a", { workspaceId: undefined })], "ws-1"), {
    state: "none",
  });
});

test("a subagent is never the target", () => {
  const child = agent("child", { labels: { "paseo.parent-agent-id": "parent" } });
  assert.deepEqual(chooseAgent([child], "ws-1"), { state: "none" });
  assert.deepEqual(
    chooseAgent([child, agent("parent", { updatedAt: "2026-10-09T01:00:00.000Z" })], "ws-1"),
    { state: "ready", id: "parent", name: "parent" },
  );
});
