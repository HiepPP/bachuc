import { test } from "node:test";
import assert from "node:assert/strict";
import type { PaseoApi } from "@getpaseo/client";
import { createDriver } from "../server/paseo";

test("driver rechecks cancellation after refresh and rejects another host or workspace", async () => {
  let allowed = true,
    sent = 0;
  const handle = {
    workspaceId: "w",
    archivedAt: null,
    status: "idle",
    activeTurn: null,
    pendingPermissions: [],
    current: () => ({}),
    refresh: async () => {
      allowed = false;
    },
    send: async () => {
      sent++;
    },
  };
  const driver = createDriver(
    () => ({ agents: { ref: () => handle } }) as unknown as PaseoApi,
    "h",
  );
  const scope = { agentId: "a", workspaceId: "w", serverId: "h" };
  await assert.rejects(
    driver.send(scope, "Test", "id", () => allowed),
    /cancelled/,
  );
  await assert.rejects(
    driver.send({ ...scope, serverId: "other" }, "Test", "id", () => true),
    /Wrong host/,
  );
  await assert.rejects(
    driver.send({ ...scope, workspaceId: "other" }, "Test", "id", () => true),
    /unavailable/,
  );
  assert.equal(sent, 0);
});
