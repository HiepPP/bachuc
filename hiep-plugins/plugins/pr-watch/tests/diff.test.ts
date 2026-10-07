import assert from "node:assert/strict";
import test from "node:test";
import { diffSnapshots, formatWakePrompt, type PrSnapshot } from "../server/diff";

function snapshot(overrides: Partial<PrSnapshot> = {}): PrSnapshot {
  return {
    number: 42,
    url: "https://github.com/acme/app/pull/42",
    state: "OPEN",
    mergeable: "MERGEABLE",
    checks: [
      { name: "test", result: "pending" },
      { name: "lint", result: "pending" },
    ],
    notes: [],
    ...overrides,
  };
}

test("the first read is a baseline and wakes nobody", () => {
  assert.deepEqual(diffSnapshots(null, snapshot({ mergeable: "CONFLICTING" }), "me"), []);
});

test("a check that starts failing wakes the agent once", () => {
  const failing = snapshot({
    checks: [
      { name: "test", result: "failed" },
      { name: "lint", result: "pending" },
    ],
  });
  assert.deepEqual(diffSnapshots(snapshot(), failing, "me"), [
    { kind: "checks_failed", names: ["test"] },
  ]);
  assert.deepEqual(diffSnapshots(failing, failing, "me"), []);
});

test("all checks passing wakes the agent once", () => {
  const passed = snapshot({
    checks: [
      { name: "test", result: "passed" },
      { name: "lint", result: "passed" },
    ],
  });
  assert.deepEqual(diffSnapshots(snapshot(), passed, "me"), [{ kind: "checks_passed", count: 2 }]);
  assert.deepEqual(diffSnapshots(passed, passed, "me"), []);
});

test("only notes from other people wake the agent", () => {
  const next = snapshot({
    notes: [
      {
        id: "comment:1",
        author: "reviewer",
        body: "Please rename this.",
        kind: "comment",
        state: "",
      },
      { id: "comment:2", author: "me", body: "Done.", kind: "comment", state: "" },
      {
        id: "comment:3",
        author: "ci-helper[bot]",
        body: "Coverage 80%",
        kind: "comment",
        state: "",
      },
    ],
  });
  const triggers = diffSnapshots(snapshot(), next, "me");
  assert.equal(triggers.length, 1);
  assert.equal(triggers[0]?.kind, "new_notes");
  assert.deepEqual(
    triggers[0]?.kind === "new_notes" ? triggers[0].notes.map((note) => note.id) : [],
    ["comment:1"],
  );
});

test("a new merge conflict and a closed PR wake the agent", () => {
  assert.deepEqual(diffSnapshots(snapshot(), snapshot({ mergeable: "CONFLICTING" }), "me"), [
    { kind: "conflict" },
  ]);
  assert.deepEqual(diffSnapshots(snapshot(), snapshot({ state: "MERGED" }), "me"), [
    { kind: "closed", state: "MERGED" },
  ]);
});

test("the wake prompt names every change", () => {
  const prompt = formatWakePrompt(snapshot(), [
    { kind: "checks_failed", names: ["test"] },
    {
      kind: "new_notes",
      notes: [
        {
          id: "review:9",
          author: "lead",
          body: "x".repeat(400),
          kind: "review",
          state: "CHANGES_REQUESTED",
        },
      ],
    },
    { kind: "conflict" },
  ]);
  assert.match(prompt, /^<pr-watch>\nPR #42 changed: https:\/\/github.com\/acme\/app\/pull\/42/);
  assert.match(prompt, /Checks failed: test\./);
  assert.match(prompt, /New review \(CHANGES_REQUESTED\) from lead: x{300}…/);
  assert.match(prompt, /merge conflicts/);
});
