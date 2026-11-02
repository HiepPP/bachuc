import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtempSync, mkdirSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { parseCommand, turnContext } from "../server/turn";

function fakeRoot(): string {
  const root = mkdtempSync(path.join(tmpdir(), "ponytail-"));
  mkdirSync(path.join(root, "hooks"));
  writeFileSync(
    path.join(root, "hooks/ponytail-instructions.js"),
    "module.exports = { getPonytailInstructions: (mode) => `RULES ${mode}` };",
  );
  return root;
}

test("parseCommand reads explicit levels and ignores other prompts", () => {
  assert.equal(parseCommand("/ponytail lite\nfix it"), "lite");
  assert.equal(parseCommand("$ponytail ultra"), "ultra");
  assert.equal(parseCommand("/ponytail off"), "off");
  assert.equal(parseCommand("/ponytail"), "full");
  assert.equal(parseCommand("/ponytail-review"), null);
  assert.equal(parseCommand("please use /ponytail lite"), null);
  assert.equal(parseCommand("fix the bug"), null);
});

test("turnContext sends the rules on a mode change and a reminder after", () => {
  const ponytailRoot = fakeRoot();
  const first = turnContext({
    ponytailRoot,
    mode: "full",
    lastMode: undefined,
    turnsSinceRules: 0,
  });
  assert.ok(first?.includes("RULES full"));
  const again = turnContext({ ponytailRoot, mode: "full", lastMode: "full", turnsSinceRules: 1 });
  assert.ok(again && !again.includes("RULES") && again.includes("full"));
  assert.ok(Buffer.byteLength(again) <= 300);
  const changed = turnContext({
    ponytailRoot,
    mode: "ultra",
    lastMode: "full",
    turnsSinceRules: 3,
  });
  assert.ok(changed?.includes("RULES ultra"));
});

test("turnContext resends the rules after 9 reminder turns", () => {
  const ponytailRoot = fakeRoot();
  const eighth = turnContext({ ponytailRoot, mode: "lite", lastMode: "lite", turnsSinceRules: 8 });
  assert.ok(eighth && !eighth.includes("RULES"));
  const tenth = turnContext({ ponytailRoot, mode: "lite", lastMode: "lite", turnsSinceRules: 9 });
  assert.ok(tenth?.includes("RULES lite"));
});

test("turnContext turns off once, then stays silent", () => {
  const ponytailRoot = fakeRoot();
  const stop = turnContext({ ponytailRoot, mode: "off", lastMode: "full", turnsSinceRules: 2 });
  assert.ok(stop && !stop.includes("RULES"));
  assert.equal(
    turnContext({ ponytailRoot, mode: "off", lastMode: "off", turnsSinceRules: 9 }),
    null,
  );
  // After a restart the first off turn still tells the agent to stop.
  assert.ok(turnContext({ ponytailRoot, mode: "off", lastMode: undefined, turnsSinceRules: 0 }));
});
