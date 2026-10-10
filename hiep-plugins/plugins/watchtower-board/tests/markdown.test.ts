import assert from "node:assert/strict";
import { test } from "node:test";
import { parseMarkdown } from "../client/markdown";

test("headings, bullets, and blank-line paragraphs become blocks", () => {
  const blocks = parseMarkdown(
    [
      "# ADR-0003 Chip-only sources",
      "",
      "Status: proposed",
      "Date: 2026-10-07",
      "",
      "## Context",
      "",
      "- First `code` point",
      "  - Nested point",
      "* Star bullet",
      "",
      "Closing words.",
    ].join("\n"),
  );
  assert.deepEqual(blocks, [
    { kind: "heading", level: 1, text: "ADR-0003 Chip-only sources" },
    { kind: "paragraph", text: "Status: proposed\nDate: 2026-10-07" },
    { kind: "heading", level: 2, text: "Context" },
    { kind: "bullet", depth: 0, text: "First `code` point" },
    { kind: "bullet", depth: 1, text: "Nested point" },
    { kind: "bullet", depth: 0, text: "Star bullet" },
    { kind: "paragraph", text: "Closing words." },
  ]);
});

test("links and HTML stay as text, and a hash without a space is not a heading", () => {
  assert.deepEqual(parseMarkdown("See [ADR-0004](a.md) and <b>bold</b>.\n#tag\nC#"), [
    { kind: "paragraph", text: "See [ADR-0004](a.md) and <b>bold</b>.\n#tag\nC#" },
  ]);
  assert.deepEqual(parseMarkdown("## Heading with C#"), [
    { kind: "heading", level: 2, text: "Heading with C#" },
  ]);
});

test("a fenced block keeps its lines, even an unclosed one", () => {
  assert.deepEqual(parseMarkdown("Before\n```ts\n# not a heading\n- not a bullet\n```\nAfter"), [
    { kind: "paragraph", text: "Before" },
    { kind: "code", text: "# not a heading\n- not a bullet" },
    { kind: "paragraph", text: "After" },
  ]);
  assert.deepEqual(parseMarkdown("```\nopen"), [{ kind: "code", text: "open" }]);
  assert.deepEqual(parseMarkdown(""), []);
});
