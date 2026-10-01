import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtemp, readdir } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { SkillPins } from "../server/state";

const A = "00000000-0000-4000-8000-000000000001";

async function fixture() {
  const root = await mkdtemp(path.join(tmpdir(), "skill-pins-"));
  const pins = new SkillPins(root);
  return { root, pins };
}

test("an agent without a file has no pinned skills", async () => {
  const { pins } = await fixture();
  assert.deepEqual(await pins.get(A), { skills: [] });
});

test("set saves the selection and get reads it back", async () => {
  const { pins } = await fixture();
  await pins.set(A, ["sequential-thinking", "watchtower"]);
  assert.deepEqual(await pins.get(A), {
    skills: ["watchtower", "sequential-thinking"],
  });
});

test("set rejects unknown skills and invalid agent IDs", async () => {
  const { pins } = await fixture();
  await assert.rejects(pins.set(A, ["writing-plans"]));
  await assert.rejects(pins.set("../escape", ["watchtower"]), /Invalid agent ID/);
});

test("skill IDs from an older catalog are dropped on read", async () => {
  const { root, pins } = await fixture();
  const { mkdir, writeFile } = await import("node:fs/promises");
  await mkdir(path.join(root, "agents", A), { recursive: true });
  await writeFile(
    path.join(root, "agents", A, "skills.json"),
    JSON.stringify({ skills: ["brainstorming", "watchtower"] }),
  );
  assert.deepEqual(await pins.get(A), { skills: ["watchtower"] });
});

test("remove deletes only the archived agent's folder", async () => {
  const { root, pins } = await fixture();
  const other = "00000000-0000-4000-8000-000000000002";
  await pins.set(A, ["watchtower"]);
  await pins.set(other, ["sequential-thinking"]);
  await pins.remove(A);
  assert.deepEqual(await readdir(path.join(root, "agents")), [other]);
  assert.deepEqual(await pins.get(A), { skills: [] });
  assert.deepEqual(await pins.get(other), { skills: ["sequential-thinking"] });
  await pins.remove(A);
});

test("remove rejects an invalid agent ID without touching files", async () => {
  const { root, pins } = await fixture();
  await pins.set(A, ["watchtower"]);
  await assert.rejects(pins.remove(".."), /Invalid agent ID/);
  await assert.rejects(pins.remove(""), /Invalid agent ID/);
  assert.deepEqual(await readdir(path.join(root, "agents")), [A]);
});

test("seed writes the initial set only when no selection exists", async () => {
  const pins = new SkillPins(await mkdtemp(path.join(tmpdir(), "skill-pins-seed-")));
  const id = "00000000-0000-4000-8000-00000000000a";
  await pins.seed(id, ["watchtower"]);
  assert.deepEqual(await pins.get(id), { skills: ["watchtower"] });
  await pins.set(id, []);
  await pins.seed(id, ["sequential-thinking"]);
  assert.deepEqual(await pins.get(id), { skills: [] });
});
