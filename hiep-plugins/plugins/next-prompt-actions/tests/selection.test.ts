import { test } from "node:test";
import assert from "node:assert/strict";
import { parseHTML } from "linkedom";
import { layout, reachable, renderSelection } from "../client/selection";
import type { Snapshot } from "../shared/contracts";
import type { Document, Node } from "../client/web";

test("compatible suggestions stay optional without inventing an exclusive group", () => {
  const { document, window } = parseHTML("<html><body><div id='root'></div></body></html>");
  const candidates: Snapshot["candidates"] = ["keyboard", "wrapping"].map((id) => ({
    key: id,
    text: `Review ${id}.`,
    source: "fixture",
    timestamp: 1,
    block: "fixture",
    state: "ready",
    selection: {
      id,
      blockKey: "same",
      exclusiveGroups: [],
      allowedCombinations: [["keyboard", "wrapping"]],
    },
  }));
  let edited: string[] = [];
  renderSelection(
    document as unknown as Document,
    document.querySelector("#root") as unknown as Node,
    document.querySelector("#root") as unknown as Node,
    candidates,
    { busy: false, enabled: false, note: "", candidates },
    {
      edit: (picked) => {
        edited = picked.map((c) => c.text);
      },
      send: async () => {},
    },
  );
  assert.equal(document.querySelectorAll('input[type="radio"]').length, 0);
  assert.equal(document.querySelectorAll('input[type="checkbox"]').length, 2);
  const edit = document.querySelector(".npa-selection-edit")!;
  assert.equal(edit.disabled, true);
  for (const input of document.querySelectorAll("input")) {
    assert.equal(input.checked, false);
    input.checked = true;
    input.dispatchEvent(new window.Event("change"));
  }
  assert.equal(edit.disabled, false);
  edit.dispatchEvent(new window.Event("click"));
  assert.deepEqual(edited, ["Review keyboard.", "Review wrapping."]);
  document.querySelector(".npa-selection-clear")!.dispatchEvent(new window.Event("click"));
  assert.equal(edit.disabled, true);
  assert.equal(document.querySelectorAll("input:checked").length, 0);
});

test("groups follow declared combinations and lock suggestions that cannot join the selection", () => {
  const { document, window } = parseHTML("<html><body><div id='root'></div></body></html>");
  const ids = ["measure", "slim", "stop-gate"];
  const candidates: Snapshot["candidates"] = ids.map((id) => ({
    key: id,
    text: `Run ${id}.`,
    source: "fixture",
    timestamp: 1,
    block: "fixture",
    state: "ready",
    selection: {
      id,
      blockKey: "same",
      exclusiveGroups: [],
      allowedCombinations: [["measure", "stop-gate"]],
    },
  }));
  renderSelection(
    document as unknown as Document,
    document.querySelector("#root") as unknown as Node,
    document.querySelector("#root") as unknown as Node,
    candidates,
    { busy: false, enabled: false, note: "", candidates },
    { edit: () => {}, send: async () => {} },
  );
  const groups = [...document.querySelectorAll("fieldset")].map((group) => [
    group.querySelector("legend")!.textContent,
    [...group.querySelectorAll("input")].map((input) => input.value),
  ]);
  assert.deepEqual(groups, [
    ["Send together", ["measure", "stop-gate"]],
    ["Send alone", ["slim"]],
  ]);
  const input = (id: string) => document.querySelector(`input[value="${id}"]`)!;
  const state = (id: string) =>
    input(id).closest(".npa-choice")!.querySelector(".npa-choice-state")!.textContent;
  const toggle = (id: string) => {
    input(id).checked = !input(id).checked;
    input(id).dispatchEvent(new window.Event("change"));
  };
  toggle("measure");
  assert.deepEqual(
    ids.map((id) => input(id).disabled),
    [false, true, false],
  );
  assert.equal(state("slim"), "Send alone");
  toggle("slim");
  assert.equal(input("slim").checked, false, "locked suggestion ignores clicks");
  toggle("stop-gate");
  assert.equal(document.querySelector(".npa-selection-send")!.textContent, "Send selected (2)");
  assert.equal(document.querySelector(".npa-selection-send")!.disabled, false);
  document.querySelector(".npa-selection-clear")!.dispatchEvent(new window.Event("click"));
  toggle("slim");
  assert.deepEqual(
    ids.map((id) => input(id).disabled),
    [true, false, true],
  );
  assert.equal(state("measure"), "Not with selection");
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
