import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtemp, readFile, readdir, stat, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { DRAFT_TTL, SkillPins, digest } from "../server/state";

const A = "00000000-0000-4000-8000-000000000001";

async function fixture() {
  const root = await mkdtemp(path.join(tmpdir(), "skill-pins-"));
  const pins = new SkillPins(root);
  const pending = path.join(root, "agents", A, "pending");
  const list = () => readdir(pending).catch(() => [] as string[]);
  return { root, pins, pending, list };
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

test("prepare saves the selection and a hash-only snapshot", async () => {
  const { pins, pending } = await fixture();
  const text = "Secret prompt text\r\nline two";
  const { token } = await pins.prepare({ agentId: A, text, skills: ["chase-goal-claude"] });
  const file = path.join(pending, `${token}.json`);
  const raw = await readFile(file, "utf8");
  const value = JSON.parse(raw);
  assert.deepEqual(Object.keys(value).sort(), ["createdAt", "hash", "skills"]);
  assert.equal(value.hash, digest("Secret prompt text\nline two"));
  assert.deepEqual(value.skills, ["chase-goal-claude"]);
  assert.deepEqual(await pins.get(A), { skills: ["chase-goal-claude"] });
  assert.equal(raw.includes("Secret"), false);
});

test("a snapshot with every skill stays at or under 512 bytes", async () => {
  const { pins, pending } = await fixture();
  const skills = ["watchtower", "chase-goal-claude", "sequential-thinking"];
  const { token } = await pins.prepare({ agentId: A, text: "x".repeat(100_000), skills });
  const size = (await stat(path.join(pending, `${token}.json`))).size;
  assert.ok(size <= 512, `snapshot is ${size} bytes`);
});

test("cancelQueue removes the snapshot and its queue binding", async () => {
  const { pins, list } = await fixture();
  const { token } = await pins.prepare({ agentId: A, text: "queued", skills: ["watchtower"] });
  await pins.bindQueue(A, token, "queue-1");
  assert.equal((await list()).length, 2);
  await pins.cancelQueue(A, "queue-2");
  assert.equal((await list()).length, 2);
  await pins.cancelQueue(A, "queue-1");
  assert.deepEqual(await list(), []);
});

test("bindQueue after the snapshot was consumed removes the orphan binding", async () => {
  const { pins, list } = await fixture();
  const { token } = await pins.prepare({ agentId: A, text: "sent", skills: ["watchtower"] });
  await pins.cancel(A, token);
  await pins.bindQueue(A, token, "queue-1");
  assert.deepEqual(await list(), []);
});

test("cancel rejects malformed tokens", async () => {
  const { pins } = await fixture();
  await assert.rejects(pins.cancel(A, "../../skills"), /Invalid turn token/);
});

test("a second snapshot for an already bound queue row is dropped", async () => {
  const { pins, pending, list } = await fixture();
  const first = await pins.prepare({ agentId: A, text: "same", skills: ["watchtower"] });
  await pins.bindQueue(A, first.token, "queue-1");
  const second = await pins.prepare({ agentId: A, text: "same", skills: ["chase-goal-claude"] });
  await pins.bindQueue(A, second.token, "queue-1");
  assert.deepEqual((await list()).sort(), [`${first.token}.json`, `${first.token}.queue`]);
  const value = JSON.parse(await readFile(path.join(pending, `${first.token}.json`), "utf8"));
  assert.deepEqual(value.skills, ["watchtower"]);
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
  await pins.prepare({ agentId: A, text: "queued", skills: ["watchtower"] });
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

test("a draft is read until it expires and taken once", async () => {
  const root = await mkdtemp(path.join(tmpdir(), "skill-pins-draft-"));
  const pins = new SkillPins(root);
  assert.equal(await pins.readDraft(), null);
  await pins.writeDraft(["sequential-thinking"]);
  assert.deepEqual(await pins.readDraft(), ["sequential-thinking"]);
  assert.deepEqual(await pins.takeDraft(), ["sequential-thinking"]);
  assert.equal(await pins.takeDraft(), null);
  await pins.writeDraft([]);
  assert.deepEqual(await pins.takeDraft(), [], "an empty draft still overrides the defaults");
  await writeFile(
    path.join(root, "draft.json"),
    JSON.stringify({ skills: ["watchtower"], createdAt: Date.now() - DRAFT_TTL - 1 }),
  );
  assert.equal(await pins.readDraft(), null);
  assert.deepEqual(await readdir(root), []);
});
