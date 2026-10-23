import assert from "node:assert/strict";
import { test } from "node:test";
import { formatDecision, selectDecisions } from "../server/decisions";
import type { LedgerEntry } from "../server/ledger";

const entry = (decision: LedgerEntry["decision"], source: LedgerEntry["source"], n: number) =>
  ({
    decision,
    source,
    reason: "",
    ts: `2026-10-23T10:00:0${n}.000Z`,
    agentId: "a",
    requestId: `r${n}`,
    commandHash: `h${n}`,
    ms: 1,
  }) satisfies LedgerEntry;

const ledger = [
  entry("allow", "regex", 1),
  entry("allow", "jev", 2),
  entry("escalate", "jev", 3),
  entry("allow", "jev", 4),
];

test("jevAllowedOnly keeps only commands Jev allowed", () => {
  const picked = selectDecisions(ledger, { jevAllowedOnly: true, limit: 30 });
  assert.deepEqual(
    picked.map((e) => e.requestId),
    ["r2", "r4"],
  );
});

test("limit keeps the newest entries", () => {
  const picked = selectDecisions(ledger, { jevAllowedOnly: false, limit: 2 });
  assert.deepEqual(
    picked.map((e) => e.requestId),
    ["r3", "r4"],
  );
});

test("formatDecision shows a placeholder when the command is gone", () => {
  assert.match(formatDecision(ledger[1], undefined), /allow +jev +\(command no longer/);
  assert.match(formatDecision(ledger[1], "rg -n x\nsrc"), /rg -n x src$/);
});
