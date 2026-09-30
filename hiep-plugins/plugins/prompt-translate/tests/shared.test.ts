import { preserveCavemanCommand, stripCavemanMode } from "../shared/caveman";
import { test } from "node:test";
import assert from "node:assert/strict";
import { hasVietnamese } from "../shared/vietnamese";
import { translateRpc } from "../shared/contracts";
import { translateSettings } from "../shared/settings";

test("hasVietnamese detects diacritics, including words shared with other languages", () => {
  assert.equal(hasVietnamese("cái này là gì"), true);
  assert.equal(hasVietnamese("ĐỔI TÊN HÀM"), true);
  assert.equal(hasVietnamese("rename the function"), false);
  assert.equal(hasVietnamese("fix src/app.ts:12 and run npm test"), false);
});

test("translate input rejects text over 20,000 characters and defaults cacheOnly", () => {
  assert.equal(translateRpc.input.safeParse({ text: "a".repeat(20_001) }).success, false);
  assert.deepEqual(translateRpc.input.parse({ text: "xin chào" }), {
    text: "xin chào",
    cacheOnly: false,
  });
});

test("settings parse {} into complete defaults", () => {
  assert.deepEqual(translateSettings.schema.parse({}), {
    translate: true,
    enhanceShortcut: true,
    matchReplyLanguage: true,
    cavemanMode: "follow-agent",
    chineseScript: "skill-default",
    provider: "openrouter",
    translateModel: "google/gemini-2.5-flash-lite",
    enhanceModel: "google/gemini-2.5-flash-lite",
  });
  assert.equal(
    translateSettings.schema.parse({ translate: false, enhanceShortcut: false }).matchReplyLanguage,
    true,
  );
  assert.equal(
    translateSettings.schema.parse({ translate: false, enhanceShortcut: false }).cavemanMode,
    "follow-agent",
  );
});

test("explicit draft command survives a rewrite that omitted it", () => {
  for (const command of ["/caveman lite", "$caveman full", "/caveman off"]) {
    const result = preserveCavemanCommand(
      "Explain git status.",
      `${command}\n\nGiải thích git status.`,
    );
    assert.equal(result, `${command}\n\nExplain git status.`);
    assert.equal(stripCavemanMode(result), result);
  }
});
