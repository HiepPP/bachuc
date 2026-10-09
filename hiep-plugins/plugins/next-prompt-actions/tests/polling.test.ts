import { test } from "node:test";
import assert from "node:assert/strict";
import {
  ACTIVE_POLL_MS,
  JEV_REVIEWING,
  agentBusy,
  inspectionActivity,
  inspectionInterval,
} from "../shared/polling";

test("only a running Jev review polls", () => {
  assert.equal(inspectionInterval({ note: "" }), false);
  assert.equal(inspectionInterval(undefined), false);
  assert.equal(inspectionInterval({ note: JEV_REVIEWING }), ACTIVE_POLL_MS);
});

test("an agent is busy while it is not at rest or waits for a permission", () => {
  assert.equal(agentBusy({ status: "idle", attentionReason: null }), false);
  assert.equal(agentBusy({ status: "idle", attentionReason: "finished" }), false);
  assert.equal(agentBusy({ status: "idle", attentionReason: "permission" }), true);
  assert.equal(agentBusy({ status: "running", attentionReason: null }), true);
  assert.equal(agentBusy({ status: "initializing", attentionReason: null }), true);
  // Send resumes a closed thread, so a closed runtime is at rest.
  assert.equal(agentBusy({ status: "closed", attentionReason: null }), false);
  assert.equal(agentBusy(null), false);
});

test("activity tracks status, and the last activity only while the agent is not running", () => {
  const at = "2026-10-09T06:00:00.000Z";
  assert.deepEqual(inspectionActivity({ status: "idle", lastActivityAt: at }), {
    status: "idle",
    lastActivityAt: at,
  });
  // Streaming bumps lastActivityAt constantly; the turn end refreshes the snapshot.
  assert.deepEqual(inspectionActivity({ status: "running", lastActivityAt: at }), {
    status: "running",
    lastActivityAt: null,
  });
});
