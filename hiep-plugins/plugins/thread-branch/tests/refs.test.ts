import assert from "node:assert/strict";
import test from "node:test";
import { describeRefsPill } from "../client/buttons";
import { extractGitRefs, MAX_REFS, refUrl } from "../client/refs";
import type { BranchInfo } from "../shared/branch";

const reply = [
  "Understood. I didn't create a revert for vorgange #8, so `7a5c2f9` stays on vorgange `main`.",
  "In thehub, revert PR #670 (branch `revert/pr-650-vorgange-delivery`, `8cb7bb81`) is still open.",
  "thehub `main` still has `c5e0b412` from the PR #650 merge.",
].join("\n\n");

test("extracts PRs, commits, and branches from a reply without duplicates", () => {
  assert.deepEqual(extractGitRefs(reply), [
    { kind: "pr", value: "8" },
    { kind: "pr", value: "670" },
    { kind: "pr", value: "650" },
    { kind: "commit", value: "7a5c2f9" },
    { kind: "commit", value: "8cb7bb81" },
    { kind: "commit", value: "c5e0b412" },
    { kind: "branch", value: "main" },
    { kind: "branch", value: "revert/pr-650-vorgange-delivery" },
  ]);
});

test("ignores file paths, headings, UUIDs, hex words, URLs, and fenced code", () => {
  const text = [
    "## Recap",
    "Edited `client/refs.ts` and `server/git.ts`; id 8cb7bb81-1234-4abc-9def-0123456789ab.",
    "See https://github.com/o/r/commit/abc1234 and the deadbeef value; issue#12 is not a ref.",
    "```\ngit checkout feat/hidden # 99 fedcba9\n```",
    "Branch: `feat/kept` and 1234567.",
  ].join("\n");
  assert.deepEqual(extractGitRefs(text), [{ kind: "branch", value: "feat/kept" }]);
});

test("caps the list", () => {
  const text = Array.from({ length: MAX_REFS + 5 }, (_, index) => `#${index + 1}`).join(" ");
  assert.equal(extractGitRefs(text).length, MAX_REFS);
});

test("builds GitHub and GitLab URLs", () => {
  const github = "https://github.com/o/r";
  assert.equal(refUrl(github, { kind: "pr", value: "670" }), `${github}/pull/670`);
  assert.equal(refUrl(github, { kind: "commit", value: "8cb7bb81" }), `${github}/commit/8cb7bb81`);
  assert.equal(
    refUrl(github, { kind: "branch", value: "revert/pr#1" }),
    `${github}/tree/revert/pr%231`,
  );
  const gitlab = "https://gitlab.com/g/r";
  assert.equal(refUrl(gitlab, { kind: "pr", value: "3" }), `${gitlab}/-/merge_requests/3`);
  assert.equal(refUrl(gitlab, { kind: "commit", value: "abc1234" }), `${gitlab}/-/commit/abc1234`);
});

test("refs pill opens each link and hides without refs or a remote", async () => {
  const info: BranchInfo = {
    repo: true,
    branch: "main",
    detached: false,
    sha: "a1b2c3d",
    dirty: false,
    changes: null,
    upstream: null,
    ahead: null,
    behind: null,
    pr: null,
    remoteUrl: "https://github.com/o/r",
    prLookup: "ok",
  };
  const opened: string[] = [];
  const open = async (url: string) => {
    opened.push(url);
  };
  const refs = extractGitRefs("PR #670 at `8cb7bb81`");
  const pill = describeRefsPill(info, refs, open);
  assert.equal(pill.visible, true);
  assert.equal(pill.label, "2 links");
  assert.equal(pill.behavior.kind, "menu");
  if (pill.behavior.kind !== "menu") return;
  for (const item of pill.behavior.items)
    if (item.kind === "item" && item.behavior.kind === "action") await item.behavior.onPress();
  assert.deepEqual(opened, [
    "https://github.com/o/r/pull/670",
    "https://github.com/o/r/commit/8cb7bb81",
  ]);
  assert.equal(describeRefsPill(info, [], open).visible, false);
  assert.equal(describeRefsPill({ ...info, remoteUrl: null }, refs, open).visible, false);
});
