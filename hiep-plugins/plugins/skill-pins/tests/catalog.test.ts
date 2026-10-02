import { test } from "node:test";
import assert from "node:assert/strict";
import { catalog, pillLabel, skillsSchema } from "../shared/catalog";
import { skillsReadRpc, skillsWriteRpc } from "../shared/contracts";

test("catalog holds the three pinned skills in menu order", () => {
  assert.deepEqual(
    catalog.map((skill) => skill.id),
    ["watchtower", "chase-goal-claude", "sequential-thinking"],
  );
  for (const skill of catalog) assert.equal(skill.claude, skill.id);
});

test("skills are returned in catalog order without duplicates", () => {
  assert.deepEqual(skillsSchema.parse(["sequential-thinking", "watchtower", "watchtower"]), [
    "watchtower",
    "sequential-thinking",
  ]);
});

test("the pill names pinned skills by short name in catalog order", () => {
  assert.equal(pillLabel([]), "Skills");
  assert.equal(pillLabel(["sequential-thinking"]), "Seq");
  assert.equal(pillLabel(["sequential-thinking", "watchtower"]), "Watch · Seq");
});

test("unknown skill IDs are rejected", () => {
  assert.equal(skillsSchema.safeParse(["writing-plans"]).success, false);
});

test("RPC wire names are lowercase with dots and hyphens", () => {
  for (const rpc of [skillsReadRpc, skillsWriteRpc])
    assert.match(rpc.name, /^skill-pins\.[a-z-]+$/);
});
