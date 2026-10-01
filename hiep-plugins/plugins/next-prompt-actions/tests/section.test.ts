import { test } from "node:test";
import assert from "node:assert/strict";
import { sectionIntro, splitNextSection } from "../shared/section";
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
