import assert from "node:assert/strict";
import test from "node:test";
import { parsePrView } from "../server/gh";

test("parses gh pr view output into a snapshot", () => {
  const snapshot = parsePrView(
    JSON.stringify({
      number: 7,
      url: "https://github.com/acme/app/pull/7",
      state: "OPEN",
      mergeable: "CONFLICTING",
      statusCheckRollup: [
        { name: "test", conclusion: "FAILURE" },
        { context: "deploy", state: "SUCCESS" },
        { name: "lint", status: "IN_PROGRESS", conclusion: "" },
      ],
      comments: [{ id: "IC_1", author: { login: "reviewer" }, body: "Looks good" }],
      reviews: [{ id: "PRR_2", author: { login: "lead" }, state: "APPROVED", body: "" }],
    }),
  );
  assert.deepEqual(snapshot, {
    number: 7,
    url: "https://github.com/acme/app/pull/7",
    state: "OPEN",
    mergeable: "CONFLICTING",
    checks: [
      { name: "test", result: "failed" },
      { name: "deploy", result: "passed" },
      { name: "lint", result: "pending" },
    ],
    notes: [
      { id: "comment:IC_1", author: "reviewer", body: "Looks good", kind: "comment", state: "" },
      { id: "review:PRR_2", author: "lead", body: "", kind: "review", state: "APPROVED" },
    ],
  });
});

test("rejects output with no pull request", () => {
  assert.throws(() => parsePrView("{}"), /no pull request/);
});
