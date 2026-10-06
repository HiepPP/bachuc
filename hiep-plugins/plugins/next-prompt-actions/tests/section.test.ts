import { test } from "node:test";
import assert from "node:assert/strict";
import {
  boldRuns,
  commitChip,
  recapSections,
  sectionIntro,
  splitNextSection,
  splitRecap,
  type Recap,
} from "../shared/section";
import { layout, reachable } from "../shared/selection";

const reply = [
  "Done.",
  "",
  "## Recap",
  "- Branch: `main`",
  "",
  "## What Next",
  "",
  "Run the check first.",
  "",
  "```text",
  "prompt: Run the tests.",
  "why: Pick this to **verify**, because it is fast.",
  "```",
].join("\n");

test("the reply splits at the What Next heading and keeps the text before it", () => {
  const split = splitNextSection(reply);
  assert.ok(split);
  assert.equal(split.title, "What Next");
  assert.equal(split.before, "Done.\n\n## Recap\n- Branch: `main`");
  assert.equal(sectionIntro(split.section), "Run the check first.");
});

test("a reply without an eligible block, or with the heading inside a fence, is left alone", () => {
  assert.equal(splitNextSection("## What Next\n\nNothing to send."), null);
  assert.equal(splitNextSection("```md\n## What Next\n```\n\n```text\nprompt: x\n```"), null);
});

test("layout numbers separate sets and keeps exclusive choices first", () => {
  assert.deepEqual(
    layout(
      ["a", "b", "c", "d", "e"],
      [],
      [
        ["a", "c"],
        ["b", "d"],
      ],
    ),
    [
      { kind: "together", set: 1, ids: ["a", "c"] },
      { kind: "together", set: 2, ids: ["b", "d"] },
      { kind: "alone", ids: ["e"] },
    ],
  );
  assert.deepEqual(layout(["x", "risks", "y"], [["x", "y"]], [["x", "risks"]]), [
    { kind: "choice", index: 0, ids: ["x", "y"] },
    { kind: "follow-up", ids: ["risks"] },
  ]);
  assert.equal(reachable(["a"], "c", [], [["a", "b", "c"]]), true);
  assert.equal(reachable(["x"], "y", [["x", "y"]], []), true, "radio swap replaces the choice");
});

test("a complete recap folds without losing Markdown in Did", () => {
  assert.deepEqual(
    splitRecap("Result.\n\n## Recap\nBranch: `main`\nDid: Kept **formatting**.\nCommit/push: none"),
    {
      before: "Result.",
      recap: { branch: "`main`", did: "Kept **formatting**.", commit: "none" },
    },
  );
  assert.deepEqual(
    splitRecap(
      "## Recap\n- **Branch:** `main`\n- **Did:** Checked it.\n  - Nested evidence.\n- **Commit/push:** none",
    ).recap,
    {
      branch: "`main`",
      did: "Checked it.\n  - Nested evidence.",
      commit: "none",
    },
  );
});

test("unknown or incomplete recap content stays untouched", () => {
  for (const before of [
    "## Recap\nBranch: main\nDid: done",
    "## Recap\nBranch: main\nDid: done\nCommit/push: none\nExtra prose must remain.",
    "## Recap\nBranch: main\nDid: done\nCommit/push: none\n## Evidence\nKeep this.",
    "````md\n## Recap\nBranch: main\nDid: done\n```\nCommit/push: none\n````",
  ])
    assert.deepEqual(splitRecap(before), { before });
});

const five = (lines: string[]) => splitRecap(["Result.", "", "## Recap", ...lines].join("\n"));
const list = [
  "- Branch: `main`",
  "- Commit/push: no",
  "- Did: Rewrote the [section](/report). +12/-8.",
  "- Not yet: nothing",
  "- Need from you: Check the `chip`.",
];

test("a five-field bullet list folds, with nested lists in free-text fields", () => {
  assert.deepEqual(five(list), {
    before: "Result.",
    recap: {
      branch: "`main`",
      commit: "no",
      did: "Rewrote the [section](/report). +12/-8.",
      notYet: "nothing",
      need: "Check the `chip`.",
    },
  });
  assert.deepEqual(
    five([
      "- **Branch:** main",
      "- **Commit/push:** yes, committed abc1234",
      "- **Did:** Ported it.",
      "    - Parser.",
      "        - Deeper.",
      "    - Panel.",
      "",
      "- **Not yet:**",
      "  - Runtime check.",
      "- **Need from you:** nothing",
    ]).recap,
    {
      branch: "main",
      commit: "yes, committed abc1234",
      did: "Ported it.\n- Parser.\n    - Deeper.\n- Panel.",
      notYet: "- Runtime check.",
      need: "nothing",
    },
  );
});

test("five labeled lines fold, each free-text line owning the list that follows it", () => {
  assert.deepEqual(
    five([
      "Branch: main",
      "Commit/push: no",
      "Did: Rewrote the section.",
      "Not yet:",
      "",
      "- Item one.",
      "  - Detail.",
      "- Item two.",
      "",
      "Need from you: Two steps.",
      "1. Open a session.",
      "2. Check the chip.",
    ]).recap,
    {
      branch: "main",
      commit: "no",
      did: "Rewrote the section.",
      notYet: "- Item one.\n  - Detail.\n- Item two.",
      need: "Two steps.\n1. Open a session.\n2. Check the chip.",
    },
  );
  assert.deepEqual(
    five([
      "Branch: main",
      "",
      "Commit/push: no",
      "",
      "Did: x",
      "Not yet: nothing",
      "Need from you: y",
    ]).recap,
    { branch: "main", commit: "no", did: "x", notYet: "nothing", need: "y" },
  );
});

test("any other five-field shape keeps native rendering", () => {
  const swap = (from: number, to: string) =>
    list.map((line, index) => (index === from ? to : line));
  const cases: Record<string, string[]> = {
    "legacy order plus two fields": [list[0], list[2], list[1], list[3], list[4]],
    "missing field": list.slice(0, 4),
    "extra field": [...list, "- Tests: pending"],
    "unknown label": swap(2, "- Summary: Rewrote it."),
    "missing label": swap(3, "- nothing"),
    "duplicate label": [...list.slice(0, 3), list[2], list[4]],
    "unlabeled block after": [...list, "", "Extra prose must remain."],
    "unlabeled block between": [...list.slice(0, 3), "", "Extra prose.", "", ...list.slice(3)],
    "wrapped line at the margin": [...list.slice(0, 3), "continued here.", ...list.slice(3)],
    "empty value without a list": swap(3, "- Not yet:"),
    "list nested in Branch": [list[0], "  - nested", ...list.slice(1)],
    "list nested in Commit/push": [...list.slice(0, 2), "  - nested", ...list.slice(2)],
    "list after a Branch line": [
      "Branch: main",
      "- nested",
      "Commit/push: no",
      "Did: x",
      "Not yet: y",
      "Need from you: z",
    ],
    "list before the first line": [
      "- stray",
      "Branch: main",
      "Commit/push: no",
      "Did: x",
      "Not yet: y",
      "Need from you: z",
    ],
    "lines then bullets": ["Branch: main", "Commit/push: no", ...list.slice(2)],
    "numbered fields": list.map((line, index) => line.replace("-", `${index + 1}.`)),
    "fence in the Recap": [...list, "```", "code", "```"],
  };
  for (const [name, lines] of Object.entries(cases)) {
    const before = ["Result.", "", "## Recap", ...lines].join("\n");
    assert.deepEqual(splitRecap(before), { before }, name);
  }
  for (const before of [
    ["> ## Recap", ...list.map((line) => `> ${line}`)].join("\n"),
    ["````md", "## Recap", ...list, "````"].join("\n"),
    ["## Recap", ...list, "## Evidence", "Keep this."].join("\n"),
  ])
    assert.deepEqual(splitRecap(before), { before });
});

test("the Commit chip maps each value and keeps the full value in its label", () => {
  const rows: [string, string, boolean][] = [
    ["no", "No commit", false],
    ["No.", "No commit", false],
    ["none", "No commit", false],
    ["None.", "No commit", false],
    ["yes", "Committed", true],
    ["Yes!", "Committed", true],
    ["yes, committed abc1234", "committed abc1234", true],
    ["yes - pushed main", "pushed main", true],
    ["Yes \u2014 committed `abc1234`", "committed abc1234", true],
    ["yes committed abc1234", "committed abc1234", true],
    ["committed abc1234", "committed abc1234", true],
    ["Pushed main", "Pushed main", true],
    ["pending review", "pending review", false],
    ["not committed", "Not committed", false],
    ["no changes", "No changes", false],
    ["no changes (files created and edited in working directory)", "No changes", false],
    ["chưa commit", "Chưa commit", false],
    ["không có thay đổi", "Không có thay đổi", false],
    ["committed abc1234, pushed main", "committed abc1234, pushed main", true],
    ["no changesets", "no changesets", false],
    ["yesterday", "yesterday", false],
  ];
  for (const [value, text, done] of rows)
    assert.deepEqual(commitChip(value), {
      text,
      done,
      label: `Commit/push: ${value.replace(/`/g, "")}`,
    });
});

test("five-field sections omit Not yet and Need from you that say nothing", () => {
  const recap = (notYet: string, need: string): Recap => ({
    branch: "main",
    commit: "no",
    did: "nothing",
    notYet,
    need,
  });
  const labels = (value: Recap) => recapSections(value)?.map((section) => section.label);
  for (const nothing of ["nothing", "Nothing.", "NOTHING"]) {
    assert.deepEqual(labels(recap(nothing, nothing)), ["Did"], "Did always shows");
    assert.deepEqual(labels(recap(nothing, "Check it.")), ["Did", "Need from you"]);
    assert.deepEqual(labels(recap("One test.", nothing)), ["Did", "Not yet"]);
  }
  assert.deepEqual(recapSections(recap("- nothing", "Nothing so far.")), [
    { label: "Did", text: "nothing" },
    { label: "Not yet", text: "- nothing" },
    { label: "Need from you", text: "Nothing so far." },
  ]);
  const parsed = five(swapNothing(list)).recap!;
  assert.deepEqual(labels(parsed), ["Did", "Need from you"]);
});

const compact = (lines: string[]) => splitRecap(["Result.", "", "## Recap", ...lines].join("\n"));

test("a compact Recap folds with the fields it wrote", () => {
  const recap = compact([
    "- `main` · `no changes` (files created and edited in working directory)",
    "- Did: Created notes.ts with 5 constants.",
  ]).recap!;
  assert.deepEqual(recap, {
    branch: "`main`",
    commit: "`no changes` (files created and edited in working directory)",
    did: "Created notes.ts with 5 constants.",
    sections: [{ label: "Did", text: "Created notes.ts with 5 constants." }],
  });
  assert.deepEqual(recapSections(recap), recap.sections);
  assert.deepEqual(
    compact([
      "- `main` · not committed",
      "- **Did:** Added the toggle.",
      "- Open: The trace is not measured.",
      "- Need from you: Install the release.",
    ]).recap!.sections,
    [
      { label: "Did", text: "Added the toggle." },
      { label: "Open", text: "The trace is not measured." },
      { label: "Need from you", text: "Install the release." },
    ],
  );
});

test("a Vietnamese compact Recap keeps its labels and nested lists", () => {
  assert.deepEqual(
    compact([
      "- `main` · chưa commit",
      "- Đã làm: Thêm nút trên Board.",
      "- Còn lại:",
      "  - Chưa đo trace.",
      "  - Release chưa cập nhật.",
      "- Cần bạn: Cài bản release.",
    ]).recap,
    {
      branch: "`main`",
      commit: "chưa commit",
      did: "Thêm nút trên Board.",
      sections: [
        { label: "Đã làm", text: "Thêm nút trên Board." },
        { label: "Còn lại", text: "- Chưa đo trace.\n- Release chưa cập nhật." },
        { label: "Cần bạn", text: "Cài bản release." },
      ],
    },
  );
  assert.deepEqual(
    compact([
      "- main · không có thay đổi",
      "- Đã làm: Kiểm tra parser.",
      "- Cần bạn: Trả lời.",
    ]).recap!.sections!.map((section) => section.label),
    ["Đã làm", "Cần bạn"],
  );
});

test("any other compact shape keeps native rendering", () => {
  const head = "- `main` · not committed";
  const cases: Record<string, string[]> = {
    "no Did": [head],
    "Open before Did": [head, "- Open: x", "- Did: y"],
    "Need before Open": [head, "- Did: x", "- Need from you: y", "- Open: z"],
    "duplicate Open": [head, "- Did: x", "- Open: y", "- Open: z"],
    "unknown label": [head, "- Did: x", "- Tests: y"],
    "no middle dot": ["- `main`, not committed", "- Did: x"],
    "lines without bullets": ["`main` · not committed", "Did: x"],
    "list nested in the head": [head, "  - nested", "- Did: x"],
    "empty Did without a list": [head, "- Did:"],
    "prose after": [head, "- Did: x", "", "Extra prose must remain."],
  };
  for (const [name, lines] of Object.entries(cases)) {
    const before = ["Result.", "", "## Recap", ...lines].join("\n");
    assert.deepEqual(splitRecap(before), { before }, name);
  }
});

test("the legacy Recap has no sections and no five-field keys", () => {
  const legacy = splitRecap("## Recap\nBranch: `main`\nDid: Kept it.\nCommit/push: none").recap!;
  assert.deepEqual(legacy, { branch: "`main`", did: "Kept it.", commit: "none" });
  assert.equal(recapSections(legacy), null);
});

function swapNothing(lines: string[]) {
  return lines.map((line) => line.replace("Not yet: nothing", "Not yet: Nothing."));
}

test("boldRuns puts bold text at odd indexes", () => {
  assert.deepEqual(boldRuns("Pick this to **verify**, because it is fast."), [
    "Pick this to ",
    "verify",
    ", because it is fast.",
  ]);
  assert.deepEqual(boldRuns("No bold here."), ["No bold here."]);
});
