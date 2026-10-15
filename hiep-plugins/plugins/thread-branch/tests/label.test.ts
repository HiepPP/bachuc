import assert from "node:assert/strict";
import test from "node:test";
import { draftPillLabel, pillLabel, pillTitle } from "../client/label";
import {
  describeBranchPill,
  describeChangesPill,
  describePrPill,
  syncState,
} from "../client/buttons";
import { branchIcons } from "../client/sync-icon";
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

test("New workspace label names the selected project's branch or why there is none", () => {
  assert.equal(draftPillLabel(undefined, undefined), "No project");
  assert.equal(draftPillLabel("/repo", undefined), "…");
  assert.equal(draftPillLabel("/repo", base), "feat/board-avatars");
  assert.equal(draftPillLabel("/tmp", { ...base, repo: false, branch: null }), "Not a git repo");
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

const noop = async () => {};
const branchActions = { copy: noop, openRepo: noop, fetch: noop, refresh: noop };

function menuItem(info: BranchInfo, id: string, actions = branchActions) {
  const pill = describeBranchPill(info, actions);
  if (pill.behavior.kind !== "menu") throw new Error("expected a menu");
  const item = pill.behavior.items.find((entry) => entry.kind === "item" && entry.id === id);
  if (item?.kind !== "item") throw new Error(`missing ${id} item`);
  return item;
}

test("branch pill adds nonzero push and pull counts and tints its icon", () => {
  const tracked = { ...base, upstream: "origin/feat/board-avatars" };
  const pill = (info: BranchInfo) => describeBranchPill(info, branchActions);
  const untracked = pill({ ...base, ahead: 4, behind: 4 });
  assert.equal(untracked.label, "feat/board-avatars");
  assert.equal(untracked.icon, "GitBranch");
  const inSync = pill({ ...tracked, ahead: 0, behind: 0 });
  assert.equal(inSync.label, "feat/board-avatars");
  assert.equal(inSync.icon, branchIcons.synced);
  const ahead = pill({ ...tracked, ahead: 2, behind: 0 });
  assert.equal(ahead.label, "feat/board-avatars ↑2");
  assert.equal(ahead.icon, branchIcons.ahead);
  assert.equal(pill({ ...tracked, ahead: 0, behind: 1 }).label, "feat/board-avatars ↓1");
  const both = pill({ ...tracked, ahead: 2, behind: 3 });
  assert.equal(both.label, "feat/board-avatars ↑2 ↓3");
  assert.equal(both.icon, branchIcons.behind);
});

test("branch menu opens the origin repository, hidden without a web remote", async () => {
  let opened = 0;
  const actions = {
    ...branchActions,
    openRepo: async () => {
      opened += 1;
    },
  };
  assert.equal(menuItem(base, "open-repo").visible, false);
  const github = menuItem({ ...base, remoteUrl: "https://github.com/o/r" }, "open-repo", actions);
  assert.equal(github.visible, true);
  assert.equal(github.title, "Open r on GitHub");
  assert.equal(github.icon, "Github");
  if (github.behavior.kind === "action") await github.behavior.onPress();
  assert.equal(opened, 1);
  const gitlab = menuItem({ ...base, remoteUrl: "https://gitlab.com/o/r" }, "open-repo");
  assert.equal(gitlab.title, "Open r");
  assert.equal(gitlab.icon, "Globe");
});

test("branch menu offers Fetch only when the branch has an upstream", async () => {
  let fetched = 0;
  const actions = {
    ...branchActions,
    fetch: async () => {
      fetched += 1;
    },
  };
  const fetchItem = (info: BranchInfo) => menuItem(info, "fetch", actions);
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
