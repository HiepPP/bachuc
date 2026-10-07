import assert from "node:assert/strict";
import test from "node:test";
import {
  formatQuote,
  highlightText,
  MAX_QUOTE_LENGTH,
  QUOTE_LEAD,
  quoteFromText,
} from "../shared/format";

function text(selection: string): string {
  const formatted = formatQuote(selection);
  assert.ok(formatted.ok);
  return formatted.text;
}

test("formats one line under the lead line as a blockquote", () => {
  assert.equal(text("  Use the queue.  "), `${QUOTE_LEAD}\n\n> Use the queue.`);
});

test("quotes every line and keeps blank lines inside the blockquote", () => {
  assert.equal(
    text("First line\n\nSecond line"),
    `${QUOTE_LEAD}\n\n> First line\n>\n> Second line`,
  );
});

test("nests a quote that already holds a blockquote", () => {
  assert.equal(text("> earlier\nnow"), `${QUOTE_LEAD}\n\n> > earlier\n> now`);
});

test("rejects an empty selection", () => {
  assert.deepEqual(formatQuote(" \n "), { ok: false, reason: "empty" });
});

test("leaves the comment to the host, which appends it after the blockquote", () => {
  const sent = `${text("Ship it")}\n\nComment: Why now?`;
  assert.equal(sent, `${QUOTE_LEAD}\n\n> Ship it\n\nComment: Why now?`);
  assert.equal(quoteFromText(text("Ship it")), "Ship it");
});

test("caps the quote body at the limit", () => {
  const atLimit = formatQuote("a".repeat(MAX_QUOTE_LENGTH));
  assert.ok(atLimit.ok);
  assert.ok(atLimit.quote.length <= MAX_QUOTE_LENGTH);
  assert.deepEqual(formatQuote("a".repeat(MAX_QUOTE_LENGTH + 1)), {
    ok: false,
    reason: "too_long",
  });
});

test("round-trips a quote through the agent text", () => {
  const quote = "> nested\n\n  indented\nplain";
  assert.equal(quoteFromText(text(quote)), quote.trim());
  assert.equal(quoteFromText("Something else"), null);
});

test("highlights the first line without its Markdown syntax", () => {
  assert.equal(highlightText("\n## **Bold** [link](https://x.dev) `code`\nmore"), "Bold link code");
  assert.equal(highlightText("- item one\n- item two"), "item one");
  assert.equal(highlightText("> quoted line"), "quoted line");
  assert.equal(highlightText("  \n "), undefined);
});
