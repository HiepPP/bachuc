import assert from "node:assert/strict";
import test from "node:test";
import { createBoardHostClient, type BoardHostClient } from "../client/hosts";
import { boardRpc } from "../shared/board";

test("one active bundle targets each host without mixing RPCs or project navigation", async () => {
  const calls: unknown[] = [];
  const navigation: unknown[] = [];
  const client: BoardHostClient = {
    rpc: (async (contract, input, options) => {
      calls.push([contract.name, input, options]);
      return { runs: [], observingSince: "now" };
    }) as BoardHostClient["rpc"],
    openNewWorkspace: (input) => navigation.push(input),
  };
  for (const serverId of ["air", "mini"]) {
    const host = createBoardHostClient(client, serverId);
    await host.rpc(boardRpc, {});
    host.openNewWorkspace({ cwd: "/same/project", serverId: "wrong" });
  }
  assert.deepEqual(calls, [
    [boardRpc.name, {}, { serverId: "air" }],
    [boardRpc.name, {}, { serverId: "mini" }],
  ]);
  assert.deepEqual(navigation, [
    { cwd: "/same/project", serverId: "air" },
    { cwd: "/same/project", serverId: "mini" },
  ]);
});
