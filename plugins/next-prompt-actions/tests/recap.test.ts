import { test } from "node:test";
import assert from "node:assert/strict";
import { parseHTML } from "linkedom";
import { decorateRecap } from "../client/recap";
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
