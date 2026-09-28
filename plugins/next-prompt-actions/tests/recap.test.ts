import { test } from "node:test";
import assert from "node:assert/strict";
import { parseHTML } from "linkedom";
import { decorateRecap, foldPanel } from "../client/recap";
import type { Node } from "../client/web";

const item = (value: string) =>
  `<div data-paseo-markdown-tag="li"><span data-paseo-markdown-ignore="true" data-paseo-markdown-list-marker="true">•</span><div><span>${value}</span></div></div>`;
const recap = (extra = "") =>
  `<div data-paseo-markdown-tag="h2"><span>Recap</span></div><div data-paseo-markdown-tag="ul">${item('Branch: <span data-paseo-markdown-tag="code">main</span>')}${item('Did: Read the <a href="/report">report</a>.')}${item("Commit/push: none")}${extra}</div><div data-paseo-markdown-tag="h2">What Next</div><div data-paseo-markdown-tag="p">Keep this paragraph.</div>`;

test("compact Recap preserves native content, links, code, and restores exact markup", () => {
  const { document } = parseHTML(`<div id="message">${recap()}</div>`);
  const message = document.querySelector("#message")!;
  const before = message.innerHTML;
  const link = message.querySelector("a");
  const cleanup = decorateRecap(message as unknown as Node);
  assert.ok(cleanup);
  assert.equal(message.querySelectorAll("[data-npa-recap-field]").length, 3);
  assert.equal(message.querySelector("a"), link);
  assert.equal(link!.getAttribute("href"), "/report");
  assert.equal(message.querySelector('[data-paseo-markdown-tag="code"]')!.textContent, "main");
  assert.equal(message.querySelectorAll("[data-paseo-markdown-list-marker]").length, 3);
  cleanup();
  assert.equal(message.innerHTML, before);
});

test("extra or quoted recap content retains the original rendering", () => {
  for (const content of [
    recap(item("Tests: pending")),
    `<div data-paseo-markdown-tag="blockquote">${recap()}</div>`,
    recap().replace("Did:", "Summary:"),
  ]) {
    const { document } = parseHTML(`<div id="message">${content}</div>`);
    const message = document.querySelector("#message")!;
    const before = message.innerHTML;
    assert.equal(decorateRecap(message as unknown as Node), null);
    assert.equal(message.innerHTML, before);
  }
});

test("Did may hold a nested list; other fields may not", () => {
  const nested = `<div data-paseo-markdown-tag="ul">${item("Added config.")}${item("Fixed bridge.")}</div>`;
  const didList = recap().replace('Did: Read the <a href="/report">report</a>.', `Did:${nested}`);
  const { document } = parseHTML(`<div id="message">${didList}</div>`);
  const message = document.querySelector("#message")!;
  const before = message.innerHTML;
  const cleanup = decorateRecap(message as unknown as Node);
  assert.ok(cleanup);
  assert.deepEqual(
    Array.from(message.querySelectorAll("[data-npa-recap-field]")).map((field) =>
      (field as Node).getAttribute("data-npa-recap-field"),
    ),
    ["branch", "did", "commit/push"],
  );
  cleanup();
  assert.equal(message.innerHTML, before);

  const branchList = recap().replace("Branch: ", `Branch: ${nested}`);
  const other = parseHTML(`<div id="message">${branchList}</div>`).document.querySelector(
    "#message",
  )!;
  assert.equal(decorateRecap(other as unknown as Node), null);
});

for (const split of [false, true]) {
  test(`plain Recap folds with ${split ? "separate paragraphs" : "inline hard breaks"} and restores exactly`, () => {
    const fields = [
      'Branch: <span data-paseo-markdown-tag="code">main</span>',
      'Did: Read the <a href="/report">report</a>.',
      "Commit/push: none.",
    ];
    const paragraph = (text: string) =>
      `<div data-paseo-markdown-tag="p"><span>${text}</span></div>`;
    const content = split
      ? fields.map(paragraph).join("")
      : paragraph(fields.join("<span>\n</span>"));
    const { document } = parseHTML(
      `<div id="message"><div data-paseo-markdown-tag="h2">Recap</div>${content}<div data-paseo-markdown-tag="h2">What Next</div><div data-paseo-markdown-tag="pre">prompt: Verify.</div></div><div id="panel"><div id="section"></div></div>`,
    );
    const message = document.querySelector("#message")!;
    const before = message.innerHTML;
    const panel = document.querySelector("#panel")!;
    const folded = foldPanel(
      document as unknown as Parameters<typeof foldPanel>[0],
      message as unknown as Node,
      message.querySelector('[data-paseo-markdown-tag="pre"]') as unknown as Node,
      panel as unknown as Node,
      document.querySelector("#section") as unknown as Node,
    );
    assert.equal(panel.querySelector(".npa-branch")?.textContent?.trim(), "main");
    assert.equal(panel.querySelector(".npa-did")?.textContent?.trim(), "Read the report.");
    assert.equal(
      panel.querySelector(".npa-commit")?.getAttribute("aria-label"),
      "Commit/push: none.",
    );
    assert.equal(panel.querySelector("a")?.getAttribute("href"), "/report");
    assert.equal(panel.querySelector("[data-npa-code]")?.textContent, "main");
    assert.equal(panel.querySelectorAll("[data-paseo-markdown-tag]").length, 0);
    assert.ok(folded.intact());
    folded.undo();
    assert.equal(message.innerHTML, before);
    assert.equal(panel.querySelector(".npa-recap"), null);
  });
}

test("plain Recap rejects extra lines, empty, reordered and unknown fields", () => {
  for (const text of [
    "Branch: main\nDid: done\nCommit/push: none\nTests: pending",
    "Branch: main\nDid: \nCommit/push: none",
    "Did: done\nBranch: main\nCommit/push: none",
    "Branch: main\nSummary: done\nCommit/push: none",
  ]) {
    const { document } = parseHTML(
      `<div id="message"><div data-paseo-markdown-tag="h2">Recap</div><div data-paseo-markdown-tag="p">${text}</div><div data-paseo-markdown-tag="h2">What Next</div><div data-paseo-markdown-tag="pre">prompt: Verify.</div></div><div id="panel"><div id="section"></div></div>`,
    );
    const message = document.querySelector("#message")!;
    const panel = document.querySelector("#panel")!;
    const before = message.innerHTML;
    const folded = foldPanel(
      document as unknown as Parameters<typeof foldPanel>[0],
      message as unknown as Node,
      message.querySelector('[data-paseo-markdown-tag="pre"]') as unknown as Node,
      panel as unknown as Node,
      document.querySelector("#section") as unknown as Node,
    );
    assert.equal(panel.querySelector(".npa-recap"), null);
    folded.undo();
    assert.equal(message.innerHTML, before);
  }
});

test("global Recap fields fold across separate rows without decoration", () => {
  const row = (id: string, content: string) =>
    `<div data-message-id="same"><div id="${id}" data-testid="assistant-message">${content}</div></div>`;
  const { document } = parseHTML(
    `<div id="history">${row("heading", '<div data-paseo-markdown-tag="h2">Recap</div>')}${row("fields", `<div data-paseo-markdown-tag="ul">${item("Branch: main")}${item("Did: Verified rows.")}${item("Commit/push: none.")}</div>`)}${row("next", '<div data-paseo-markdown-tag="h2">Next Steps</div>')}${row("prompt", '<div data-paseo-markdown-tag="pre">prompt: Verify.</div>')}</div><div id="panel"><div id="section"></div></div>`,
  );
  const history = document.querySelector("#history")!;
  const before = history.innerHTML;
  const folded = foldPanel(
    document as unknown as Parameters<typeof foldPanel>[0],
    document.querySelector("#prompt") as unknown as Node,
    document.querySelector('[data-paseo-markdown-tag="pre"]') as unknown as Node,
    document.querySelector("#panel") as unknown as Node,
    document.querySelector("#section") as unknown as Node,
  );
  assert.equal(document.querySelector(".npa-did")?.textContent?.trim(), "Verified rows.");
  assert.equal(document.querySelector(".npa-commit")?.textContent, "No commit");
  assert.equal(document.querySelector(".npa-next-title")?.textContent, "Next Steps");
  assert.ok(folded.intact());
  folded.undo();
  assert.equal(history.innerHTML, before);
});

test("late Recap blocks and changed text invalidate a panel in the same mounted message", () => {
  const { document } = parseHTML(
    '<div id="message"><div data-paseo-markdown-tag="h2">What Next</div><div data-paseo-markdown-tag="pre">prompt: Verify.</div></div><div id="panel"><div id="section"></div></div>',
  );
  const message = document.querySelector("#message")!;
  const mount = () =>
    foldPanel(
      document as unknown as Parameters<typeof foldPanel>[0],
      message as unknown as Node,
      message.querySelector('[data-paseo-markdown-tag="pre"]') as unknown as Node,
      document.querySelector("#panel") as unknown as Node,
      document.querySelector("#section") as unknown as Node,
    );
  const first = mount();
  assert.ok(first.intact());
  message.insertAdjacentHTML(
    "afterbegin",
    `<div data-paseo-markdown-tag="h2">Recap</div><div data-paseo-markdown-tag="ul">${item("Branch: main")}${item("Did: Done.")}${item("Commit/push: none")}</div>`,
  );
  assert.equal(first.intact(), false);
  first.undo();
  const second = mount();
  assert.equal(document.querySelector(".npa-did")?.textContent?.trim(), "Done.");
  assert.ok(second.intact());
  message.querySelectorAll('[data-paseo-markdown-tag="li"]')[1].textContent = "Did: Updated.";
  assert.equal(second.intact(), false);
  second.undo();
  const third = mount();
  assert.equal(document.querySelector(".npa-did")?.textContent?.trim(), "Updated.");
  third.undo();
});
