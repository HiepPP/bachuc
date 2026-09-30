import assert from "node:assert/strict";
import { mkdir, mkdtemp, readFile, stat, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import test from "node:test";
import type { PluginHookAgent } from "@getpaseo/plugin/server";

import {
  RECAP_MAX_ENTRIES,
  RECAP_RAW_LIMIT,
  createRecapStore,
  groupRecaps,
  localDay,
  parseRecap,
  recapEntry,
} from "../server/recaps";
import { recapDayMarkdown, type RecapEntry } from "../shared/recaps";

const agent: PluginHookAgent = {
  id: "agent-1",
  workspaceId: "ws-1",
  parentAgentId: null,
  provider: "claude",
  cwd: "/Users/me/Projects/alpha",
  title: "Fix login",
};

function entry(overrides: Partial<RecapEntry> = {}): RecapEntry {
  const endedAt = overrides.endedAt ?? "2026-09-24T10:00:00.000Z";
  return {
    ...recapEntry(
      agent,
      "turn-1",
      { branch: "main", did: "Fixed it", commitPush: "none", raw: "## Recap" },
      new Date(endedAt),
      null,
    ),
    ...overrides,
  };
}

async function tempFile() {
  const dir = await mkdtemp(path.join(tmpdir(), "board-recaps-"));
  return path.join(dir, "plugin-data/board/recaps.jsonl");
}

test("parses a recap with bullets", () => {
  const recap = parseRecap(
    "Done.\n\n## Recap\n\n- Branch: main\n- Did: Added the log.\n- Commit/push: none\n",
  );
  assert.deepEqual(recap, {
    branch: "main",
    did: "Added the log.",
    commitPush: "none",
    raw: "## Recap\n\n- Branch: main\n- Did: Added the log.\n- Commit/push: none",
  });
});

test("parses a recap without bullets", () => {
  const recap = parseRecap(
    "## Recap\nBranch: feat/x\nDid: Wrote tests.\nCommit/push: committed abc123",
  );
  assert.equal(recap?.branch, "feat/x");
  assert.equal(recap?.did, "Wrote tests.");
  assert.equal(recap?.commitPush, "committed abc123");
});

test("keeps Vietnamese text", () => {
  const recap = parseRecap(
    "## Recap\n- Branch: main\n- Did: Sửa lỗi đăng nhập và thêm kiểm thử.\n",
  );
  assert.equal(recap?.did, "Sửa lỗi đăng nhập và thêm kiểm thử.");
});

test("a missing field is null", () => {
  const recap = parseRecap("## Recap\n- Branch: main\n- Commit/push: none");
  assert.equal(recap?.did, null);
  assert.equal(recap?.branch, "main");
});

test("a reply without a recap gives no entry", () => {
  assert.equal(parseRecap("All done.\n\n## Summary\n- Did: nothing"), null);
  assert.equal(parseRecap("### Recap\n- Did: nested heading"), null);
});

test("reads only the last recap, up to the next section", () => {
  const recap = parseRecap(
    [
      "## Recap",
      "- Did: old",
      "## Recap",
      "- Branch: dev",
      "- Did: new",
      "- Commit/push: pushed dev",
      "## What Next",
      "- Did: not part of the recap",
    ].join("\n"),
  );
  assert.equal(recap?.did, "new");
  assert.equal(recap?.raw, "## Recap\n- Branch: dev\n- Did: new\n- Commit/push: pushed dev");
});

test("cuts the raw block", () => {
  const recap = parseRecap(`## Recap\n- Did: ${"x".repeat(5_000)}`);
  assert.equal(recap?.raw.length, RECAP_RAW_LIMIT);
});

test("names the project from placement, else the working directory", () => {
  const fields = { branch: null, did: null, commitPush: null, raw: "" };
  const at = new Date(2026, 8, 24, 23, 30);
  const bare = recapEntry(agent, "t", fields, at, null);
  assert.equal(bare.project, "alpha");
  assert.equal(bare.projectKey, "cwd:/Users/me/Projects/alpha");
  assert.equal(bare.day, "2026-09-24");
  const placed = recapEntry(agent, "t", fields, at, { projectName: "Alpha", projectKey: "p1" });
  assert.equal(placed.project, "Alpha");
  assert.equal(placed.projectKey, "project:p1");
});

test("stores entries with mode 0600 and skips a duplicate turn", async () => {
  const file = await tempFile();
  const store = createRecapStore(file, () => new Date("2026-09-24T12:00:00.000Z"));
  assert.equal(await store.add(entry()), true);
  assert.equal(await store.add(entry()), false);
  assert.equal(await store.add(entry({ turnId: "turn-2" })), true);
  const lines = (await readFile(file, "utf8")).trim().split("\n");
  assert.equal(lines.length, 2);
  assert.equal((await stat(file)).mode & 0o777, 0o600);
  const reopened = createRecapStore(file, () => new Date("2026-09-24T12:00:00.000Z"));
  assert.equal(await reopened.add(entry({ turnId: "turn-2" })), false);
});

test("keeps a later turn that reuses a turn id after a session reload", async () => {
  const file = await tempFile();
  const store = createRecapStore(file, () => new Date("2026-09-24T12:00:00.000Z"));
  assert.equal(await store.add(entry({ raw: "## Recap\n- Did: turn one" })), true);
  // Paseo restarts turn ids at 1 when it reloads a session.
  assert.equal(await store.add(entry({ raw: "## Recap\n- Did: turn two" })), true);
  assert.equal(await store.add(entry({ raw: "## Recap\n- Did: turn two" })), false);
  assert.equal((await readFile(file, "utf8")).trim().split("\n").length, 2);
});

test("repairs a torn final line before appending", async () => {
  const file = await tempFile();
  await mkdir(path.dirname(file), { recursive: true });
  await writeFile(file, `${JSON.stringify(entry())}\n{"agentId":"to`, { mode: 0o600 });
  const store = createRecapStore(file, () => new Date("2026-09-24T12:00:00.000Z"));
  await store.add(entry({ turnId: "turn-2" }));
  const lines = (await readFile(file, "utf8")).trim().split("\n");
  assert.deepEqual(
    lines.map((line) => JSON.parse(line).turnId),
    ["turn-1", "turn-2"],
  );
});

test("drops entries older than 90 days", async () => {
  const file = await tempFile();
  const store = createRecapStore(file, () => new Date("2026-09-24T12:00:00.000Z"));
  await store.add(entry({ turnId: "old", endedAt: "2026-06-01T00:00:00.000Z" }));
  await store.add(entry({ turnId: "new" }));
  const kept = (await readFile(file, "utf8"))
    .trim()
    .split("\n")
    .map((line) => JSON.parse(line).turnId);
  assert.deepEqual(kept, ["new"]);
});

test("keeps at most 2,000 entries", async () => {
  const file = await tempFile();
  const old = Array.from({ length: RECAP_MAX_ENTRIES }, (_, index) =>
    JSON.stringify(entry({ turnId: `t${index}` })),
  );
  await mkdir(path.dirname(file), { recursive: true });
  await writeFile(file, `${old.join("\n")}\n`, { mode: 0o600 });
  const store = createRecapStore(file, () => new Date("2026-09-24T12:00:00.000Z"));
  await store.add(entry({ turnId: "latest" }));
  const lines = (await readFile(file, "utf8")).trim().split("\n");
  assert.equal(lines.length, RECAP_MAX_ENTRIES);
  assert.equal(JSON.parse(lines[0]).turnId, "t1");
  assert.equal(JSON.parse(lines.at(-1)!).turnId, "latest");
});

test("groups by day then project, newest first", () => {
  const now = new Date(2026, 8, 24, 12);
  const at = (day: number, hour: number) => new Date(2026, 8, day, hour).toISOString();
  const make = (turnId: string, day: number, hour: number, project: string) =>
    entry({
      turnId,
      endedAt: at(day, hour),
      day: localDay(new Date(at(day, hour))),
      project,
      projectKey: `cwd:/${project}`,
    });
  const days = groupRecaps(
    [
      make("a", 23, 9, "alpha"),
      make("b", 24, 8, "alpha"),
      make("c", 24, 10, "beta"),
      make("d", 24, 11, "alpha"),
      make("e", 10, 9, "alpha"),
    ],
    7,
    now,
  );
  assert.deepEqual(
    days.map((day) => [day.day, day.projects.map((p) => [p.name, p.entries.map((e) => e.turnId)])]),
    [
      [
        "2026-09-24",
        [
          ["alpha", ["d", "b"]],
          ["beta", ["c"]],
        ],
      ],
      ["2026-09-23", [["alpha", ["a"]]]],
    ],
  );
  assert.deepEqual(
    groupRecaps([make("a", 23, 9, "alpha")], 1, now),
    [],
    "one day shows only today",
  );
});

test("copies a day as markdown", () => {
  const markdown = recapDayMarkdown({
    day: "2026-09-24",
    projects: [
      { key: "k", name: "alpha", entries: [entry(), entry({ did: null, commitPush: null })] },
    ],
  });
  assert.equal(
    markdown,
    [
      "## 2026-09-24",
      "",
      "### alpha",
      "- **Fix login** — Fixed it · Branch: main · Commit/push: none",
      "- **Fix login** — Branch: main",
      "",
    ].join("\n"),
  );
});
