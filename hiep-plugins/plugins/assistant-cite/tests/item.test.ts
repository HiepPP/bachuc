import assert from "node:assert/strict";
import test from "node:test";
import { PluginAttachmentItemSchema } from "@getpaseo/plugin";
import { MAX_QUOTE_LENGTH } from "../shared/format";
import { buildQuoteItem, parseQuoteItem, type QuoteSource } from "../shared/item";

const source = { serverId: "host-1", agentId: "agent-1", messageId: "msg-1" };

function item(selection: string, from: QuoteSource = source) {
  const built = buildQuoteItem(from, selection);
  assert.ok(built.ok);
  return built.item;
}

test("builds a chip item the host accepts, titled by the first words", () => {
  const quote = item(
    "Keep the queue in the daemon so every client sees the same order of messages",
  );

  assert.equal(PluginAttachmentItemSchema.safeParse(quote).success, true);
  assert.equal(quote.title, "Keep the queue in the daemon so every…");
  assert.equal(quote.resourceType, "quote");
});

test("gives the same passage the same id, so citing it again replaces the chip", () => {
  assert.equal(item("Same words").id, item("  Same words ").id);
  assert.notEqual(item("Same words").id, item("Other words").id);
});

test("jumps back to the source message with the first line as the highlight", () => {
  assert.deepEqual(parseQuoteItem(item("**Use** the queue.\nSecond line")), {
    serverId: "host-1",
    agentId: "agent-1",
    messageId: "msg-1",
    text: "Use the queue.",
  });
});

test("does not jump back without a source message or for another plugin's item", () => {
  const { messageId: _messageId, ...noMessage } = source;
  assert.equal(parseQuoteItem(item("Quote", noMessage)), null);
  assert.equal(parseQuoteItem({ ...item("Quote"), url: "https://example.com/q" }), null);
});

test("adds no chip for an empty or over-long selection", () => {
  assert.deepEqual(buildQuoteItem(source, ""), { ok: false, reason: "empty" });
  assert.deepEqual(buildQuoteItem(source, "x".repeat(MAX_QUOTE_LENGTH + 1)), {
    ok: false,
    reason: "too_long",
  });
});
