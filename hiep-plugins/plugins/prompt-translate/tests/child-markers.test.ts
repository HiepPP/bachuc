import { test } from "node:test";
import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import { mkdtemp } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import type { PluginHookAgent } from "@getpaseo/plugin/server";
import { childMarkerDir, markChild, unmarkChild } from "../server/child-markers";

const agent = (id: string, parentAgentId: string | null): PluginHookAgent => ({
  id,
  workspaceId: null,
  parentAgentId,
  provider: "claude",
  cwd: "/tmp",
  title: null,
});

test("marks only child agents and removes the marker on archive", async () => {
  const dir = await mkdtemp(path.join(tmpdir(), "pt-children-"));
  const marker = (id: string) => path.join(childMarkerDir(dir), id);
  markChild(dir, agent("top", null));
  assert.equal(existsSync(marker("top")), false);
  markChild(dir, agent("child-1", "parent-1"));
  assert.equal(readFileSync(marker("child-1"), "utf8"), "parent-1");
  unmarkChild(dir, agent("child-1", "parent-1"));
  assert.equal(existsSync(marker("child-1")), false);
  unmarkChild(dir, agent("never-marked", null));
});

test("ignores ids that could escape the marker directory", async () => {
  const dir = await mkdtemp(path.join(tmpdir(), "pt-children-"));
  markChild(dir, agent("../escape", "parent-1"));
  assert.equal(existsSync(path.join(dir, "escape")), false);
  assert.equal(existsSync(childMarkerDir(dir)), false);
});
