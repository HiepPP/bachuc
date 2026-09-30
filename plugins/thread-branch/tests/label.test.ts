import assert from "node:assert/strict";
import test from "node:test";
import { pillLabel, pillTitle } from "../client/label";
import {
  describeBranchPill,
  describeChangesPill,
  describePrPill,
  describeRefsPill,
  describeRepoPill,
  describeSyncPill,
  syncState,
} from "../client/buttons";
import { syncIcons } from "../client/sync-icon";
import type { BranchInfo } from "../shared/branch";

const base: BranchInfo = {
  repo: true,
  branch: "feat/board-avatars",
  detached: false,
  sha: "a1b2c3d",
  dirty: false,
  changes: null,
  upstream: null,
  ahead: null,
  behind: null,
  pr: null,
  remoteUrl: null,
  prLookup: "ok",
};

test("label shows branch without PR suffix, detached sha, and truncation", () => {
  assert.equal(pillLabel(base), "feat/board-avatars");
  assert.equal(
    pillLabel({
      ...base,
      pr: { number: 42, url: "https://github.com/o/r/pull/42", state: "OPEN" },
    }),
    "feat/board-avatars",
  );
  assert.equal(pillLabel({ ...base, branch: null, detached: true }), "@a1b2c3d");
  assert.equal(
    pillLabel({ ...base, branch: "feature/very-long-branch-name-that-overflows" }),
    "feature/very-long-branc…",
  );
});

test("title carries upstream, sync counts, dirty state, and PR state", () => {
  assert.equal(
    pillTitle({
      ...base,
      upstream: "origin/feat/board-avatars",
      ahead: 2,
      behind: 0,
      dirty: true,
      pr: { number: 42, url: "https://github.com/o/r/pull/42", state: "OPEN" },
    }),
    "Branch feat/board-avatars · tracks origin/feat/board-avatars · ahead 2 / behind 0 · uncommitted changes · PR #42 open",
  );
});

test("PR pill is a one-click action, hidden without a pull request", () => {
  const open = async () => {};
  assert.equal(describePrPill(base, open).visible, false);
  const withPr = describePrPill(
    { ...base, pr: { number: 7, url: "https://github.com/o/r/pull/7", state: "OPEN" } },
    open,
  );
  assert.equal(withPr.visible, true);
  assert.equal(withPr.label, "#7");
  assert.equal(withPr.behavior.kind, "action");
});

test("repo pill shows the repository name and hides without a web remote", () => {
  const open = async () => {};
  assert.equal(describeRepoPill(base, open).visible, false);
  const pill = describeRepoPill({ ...base, remoteUrl: "https://github.com/o/r" }, open);
  assert.equal(pill.visible, true);
  assert.equal(pill.label, "r");
  assert.equal(pill.icon, "Github");
  assert.equal(pill.title, "Open https://github.com/o/r");
});

test("sync pill shows commits to push and pull, hidden only without an upstream", async () => {
  let fetched = 0;
  const refresh = async () => {
    fetched += 1;
  };
  const tracked = { ...base, upstream: "origin/feat/board-avatars" };
  assert.equal(describeSyncPill(base, refresh).visible, false);
  const inSync = describeSyncPill({ ...tracked, ahead: 0, behind: 0 }, refresh);
  assert.equal(inSync.visible, true);
  assert.equal(inSync.label, "Synced");
  assert.equal(inSync.icon, syncIcons.synced);
  assert.equal(inSync.title, "In sync vs origin/feat/board-avatars · click to fetch");
  const ahead = describeSyncPill({ ...tracked, ahead: 2, behind: 0 }, refresh);
  assert.equal(ahead.visible, true);
  assert.equal(ahead.label, "↑2 | ↓0");
  assert.equal(ahead.icon, syncIcons.ahead);
  assert.equal(ahead.title, "2 to push vs origin/feat/board-avatars · click to fetch");
  const both = describeSyncPill({ ...tracked, ahead: 2, behind: 3 }, refresh);
  assert.equal(both.label, "↑2 | ↓3");
  assert.equal(both.icon, syncIcons.behind);
  assert.equal(both.title, "2 to push, 3 to pull vs origin/feat/board-avatars · click to fetch");
  assert.equal(describeSyncPill({ ...tracked, ahead: 0, behind: 1 }, refresh).label, "↑0 | ↓1");
  assert.equal(both.behavior.kind, "action");
  if (both.behavior.kind === "action") await both.behavior.onPress();
  assert.equal(fetched, 1);
});

test("branch menu offers Fetch only when the branch has an upstream", async () => {
  let fetched = 0;
  const actions = {
    copy: async () => {},
    fetch: async () => {
      fetched += 1;
    },
    refresh: async () => {},
  };
  const fetchItem = (info: BranchInfo) => {
    const pill = describeBranchPill(info, actions);
    if (pill.behavior.kind !== "menu") throw new Error("expected a menu");
    const item = pill.behavior.items.find((entry) => entry.kind === "item" && entry.id === "fetch");
    if (item?.kind !== "item") throw new Error("missing Fetch item");
    return item;
  };
  assert.equal(fetchItem(base).visible, false);
  const tracked = fetchItem({ ...base, upstream: "origin/feat/board-avatars" });
  assert.equal(tracked.visible, true);
  assert.equal(tracked.title, "Fetch");
  if (tracked.behavior.kind === "action") await tracked.behavior.onPress();
  assert.equal(fetched, 1);
});

test("sync state puts pulling first", () => {
  assert.equal(syncState(0, 0), "synced");
  assert.equal(syncState(3, 0), "ahead");
  assert.equal(syncState(0, 1), "behind");
  assert.equal(syncState(3, 1), "behind");
});

test("refs menu item ids satisfy the host button id rule", () => {
  const refs = [
    { kind: "pr" as const, value: "42" },
    { kind: "commit" as const, value: "fad9d70" },
    { kind: "branch" as const, value: "feat/x" },
  ];
  const pill = describeRefsPill(
    { ...base, remoteUrl: "https://github.com/o/r" },
    refs,
    async () => {},
  );
  assert.equal(pill.behavior.kind, "menu");
  if (pill.behavior.kind !== "menu") return;
  const ids = pill.behavior.items.map((item) => item.id);
  for (const id of ids) assert.match(id, /^[a-z][a-z0-9-]*$/);
  assert.equal(new Set(ids).size, refs.length);
});

test("changes pill shows file and line counts only when the work tree is dirty", () => {
  const refresh = async () => {};
  assert.equal(describeChangesPill(base, refresh).visible, false);
  const pill = describeChangesPill(
    { ...base, dirty: true, changes: { files: 3, added: 14, deleted: 3 } },
    refresh,
  );
  assert.equal(pill.visible, true);
  assert.equal(pill.label, "3 files +14 -3");
  assert.equal(pill.title, "3 files uncommitted, +14 -3 lines vs HEAD · click to refresh");
  assert.equal(pill.behavior.kind, "action");
  const single = describeChangesPill(
    { ...base, dirty: true, changes: { files: 1, added: null, deleted: null } },
    refresh,
  );
  assert.equal(single.label, "1 file");
  assert.equal(single.title, "1 file uncommitted · click to refresh");
});
