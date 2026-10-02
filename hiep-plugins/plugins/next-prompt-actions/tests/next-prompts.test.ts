import { test } from "node:test";
import assert from "node:assert/strict";
import { parsePrompts } from "../shared/prompts";
import { candidateSchema } from "../shared/contracts";
import { Engine, type Current } from "../server/engine";
import { Store } from "../server/store";

const scope = { serverId: "host", workspaceId: "workspace", agentId: "agent" };
const declaration = () => ({
  version: 1,
  prompts: [
    {
      id: "implement",
      prompt: "Implement the layout.\nPreserve other files.",
      why: "Apply the design.",
    },
    { id: "review", prompt: "Review only. Do not change code." },
    { id: "risks", prompt: "List remaining risks." },
  ],
  exclusiveGroups: [["implement", "review"]],
  allowedCombinations: [
    ["implement", "risks"],
    ["review", "risks"],
  ],
});
const markdown = (value: unknown) =>
  `## What's Next\n\`\`\`next-prompts\n${JSON.stringify(value)}\n\`\`\``;
function fixture(text = markdown(declaration())) {
  const current: Current = {
    epoch: "one",
    busy: false,
    complete: true,
    rows: [
      { type: "user_message", text: "Review possible next steps.", id: "0", timestamp: 1 },
      { type: "assistant_message", text, id: "1", timestamp: 2 },
    ],
  };
  const sent: string[] = [];
  const engine = new Engine(
    new Store(),
    {
      read: async () => structuredClone(current),
      start: async () => {},
      send: async (_scope, text) => {
        sent.push(text);
      },
    },
    async () => true,
  );
  return { current, sent, engine };
}

test("v1 parses exact prompts and reasons only in completed next-step fences", () => {
  const value = declaration();
  const [parsed] = parsePrompts(markdown(value));
  assert.ok(parsed);
  assert.deepEqual(
    parsed.prompts,
    value.prompts.map((p) => p.prompt),
  );
  assert.deepEqual(parsed.whys, ["Apply the design.", "", ""]);
  for (const text of [
    markdown(value).slice(0, -3),
    markdown(value).replace("next-prompts", "json"),
    markdown(value).replace("## What's Next", "## Example"),
    markdown(value)
      .split("\n")
      .map((l) => `> ${l}`)
      .join("\n"),
  ])
    assert.deepEqual(parsePrompts(text), []);
});

test("v1 ignores unknown fields instead of rejecting the block", () => {
  const value = {
    ...declaration(),
    extra: true,
    prompts: [{ id: "commit", prompt: "Commit the fix.", parallel: false }],
    exclusiveGroups: [],
    allowedCombinations: [],
  };
  const [parsed] = parsePrompts(markdown(value));
  assert.ok(parsed);
  assert.deepEqual(parsed.prompts, ["Commit the fix."]);
});

test("v1 rejects invalid versions, references, contradictory declarations and oversized data", () => {
  const invalid: unknown[] = [
    { ...declaration(), version: 2 },
    { ...declaration(), prompts: [] },
    { ...declaration(), prompts: [{ id: "x", prompt: " " }] },
    {
      ...declaration(),
      prompts: [
        { id: "same", prompt: "Test." },
        { id: "same", prompt: "Other." },
      ],
    },
    { ...declaration(), exclusiveGroups: [["implement", "missing"]] },
    {
      ...declaration(),
      exclusiveGroups: [
        ["implement", "review"],
        ["review", "risks"],
      ],
    },
    { ...declaration(), allowedCombinations: [["implement", "review"]] },
    { ...declaration(), allowedCombinations: [["implement", "implement"]] },
    { ...declaration(), allowedCombinations: [["implement", "missing"]] },
    {
      ...declaration(),
      allowedCombinations: [
        ["implement", "risks"],
        ["risks", "implement"],
      ],
    },
    { ...declaration(), prompts: [{ id: "x", prompt: "x".repeat(16001) }] },
  ];
  for (const value of invalid)
    assert.deepEqual(parsePrompts(markdown(value)), [], JSON.stringify(value).slice(0, 150));
  assert.deepEqual(parsePrompts("## What Next\n```next-prompts\n{broken}\n```"), []);
});

test("v1 goal and thread fields parse, and new-thread prompts never join relationships", () => {
  const value = {
    ...declaration(),
    goal: "done",
    prompts: [...declaration().prompts, { id: "audit", prompt: "Audit logs.", thread: "new" }],
  };
  const [parsed] = parsePrompts(markdown(value));
  assert.equal(parsed.declaration?.goal, "done");
  assert.deepEqual(parsed.threads, [false, false, false, true]);
  for (const invalid of [
    { ...value, goal: "open" },
    { ...value, prompts: [{ id: "x", prompt: "Test.", thread: "old" }] },
    { ...value, exclusiveGroups: [["implement", "audit"]] },
    { ...value, allowedCombinations: [["risks", "audit"]] },
  ])
    assert.deepEqual(parsePrompts(markdown(invalid)), [], JSON.stringify(invalid).slice(0, 150));
});

test("v1 backend enforces exclusions, exact combinations, authored order and deduplication", async () => {
  const f = fixture();
  try {
    const c = (await f.engine.inspect(scope)).candidates;
    assert.equal(c.length, 3);
    await assert.rejects(f.engine.send(scope, [c[0].key, c[1].key]), /combination/i);
    await assert.rejects(
      f.engine.send(
        scope,
        c.map((p) => p.key),
      ),
      /combination/i,
    );
    assert.equal(f.sent.length, 0);
    const results = await Promise.allSettled([
      f.engine.send(scope, [c[2].key, c[0].key]),
      f.engine.send(scope, [c[0].key, c[2].key]),
    ]);
    assert.equal(results.filter((r) => r.status === "fulfilled").length, 1);
    assert.deepEqual(f.sent, [
      "1. Implement the layout.\n   Preserve other files.\n2. List remaining risks.",
    ]);
    await assert.rejects(f.engine.send(scope, c[0].key));
  } finally {
    f.engine.close();
  }
});

test("legacy bulk sends are rejected but each prompt remains usable", async () => {
  const f = fixture(
    "## What Next\n```text\nprompt: Implement the layout.\nprompt: Do not implement the layout.\n```",
  );
  try {
    const c = (await f.engine.inspect(scope)).candidates;
    await assert.rejects(
      f.engine.send(
        scope,
        c.map((p) => p.key),
      ),
      /combination/i,
    );
    assert.equal(f.sent.length, 0);
    await f.engine.send(scope, c[1].key);
    assert.deepEqual(f.sent, ["Do not implement the layout."]);
  } finally {
    f.engine.close();
  }
});

test("v1 does not infer subsets, cross-block combinations or compatibility after metadata changes", async () => {
  const value = {
    ...declaration(),
    exclusiveGroups: [],
    allowedCombinations: [["implement", "review", "risks"]],
  };
  const f = fixture(markdown(value) + "\n" + markdown(value));
  try {
    const c = (await f.engine.inspect(scope)).candidates;
    assert.equal(c.length, 6);
    await assert.rejects(f.engine.send(scope, [c[0].key, c[2].key]), /combination/i);
    await assert.rejects(f.engine.send(scope, [c[0].key, c[4].key, c[5].key]), /combination/i);
    f.current.rows[1].text = markdown({ ...value, allowedCombinations: [] });
    await assert.rejects(f.engine.send(scope, c[0].key), /stale/i);
    assert.deepEqual(f.sent, []);
  } finally {
    f.engine.close();
  }
});

test("v1 metadata never bypasses individual Git actions", async () => {
  const value = declaration();
  value.prompts[0].prompt = "Commit the layout. Do not push.";
  const f = fixture(markdown(value));
  try {
    const c = (await f.engine.inspect(scope)).candidates;
    assert.equal(c.length, 3);
    await assert.rejects(f.engine.send(scope, [c[0].key, c[2].key]), /individual manual send/);
    await assert.rejects(
      f.engine.send(scope, c[0].key, { automatic: true }),
      /individual manual send/,
    );
    await f.engine.send(scope, c[0].key);
    assert.match(f.sent[0], /^\/commit --no-push\nCommit the layout\. Do not push\./);
    assert.doesNotMatch(f.sent[0], /exclusiveGroups|allowedCombinations|Apply the design/);
  } finally {
    f.engine.close();
  }
});

test("v1 suggestion flag marks prompts for display and never rejects a block", () => {
  const value = {
    ...declaration(),
    prompts: [
      { id: "implement", prompt: "Implement the layout.", suggestion: true },
      { id: "review", prompt: "Review only.", suggestion: false },
      { id: "risks", prompt: "List remaining risks." },
      { id: "audit", prompt: "Audit logs.", thread: "new", suggestion: true },
      { id: "text", prompt: "Not a boolean.", suggestion: "yes" },
      { id: "number", prompt: "Also not a boolean.", suggestion: 1 },
      { id: "nil", prompt: "Null.", suggestion: null },
    ],
    exclusiveGroups: [["implement", "review"]],
    allowedCombinations: [["implement", "risks"]],
  };
  const [parsed] = parsePrompts(markdown(value));
  assert.ok(parsed, "a stray suggestion value must not hide the block");
  assert.deepEqual(parsed.suggestions, [true, false, false, true, false, false, false]);
  assert.deepEqual(
    parsed.prompts,
    value.prompts.map((p) => p.prompt),
  );
  assert.deepEqual(parsed.declaration?.exclusiveGroups, [["implement", "review"]]);
  assert.deepEqual(parsed.declaration?.allowedCombinations, [["implement", "risks"]]);
});

test("v1 suggestion flag reaches candidates but never changes sending or selection rules", async () => {
  const plain = declaration();
  const value = {
    ...plain,
    prompts: plain.prompts.map((p, index) => ({ ...p, suggestion: index !== 1 })),
  };
  const f = fixture(markdown(value));
  try {
    const c = (await f.engine.inspect(scope)).candidates;
    assert.deepEqual(
      c.map((candidate) => candidate.suggestion),
      [true, undefined, true],
    );
    assert.equal(candidateSchema.parse(c[0]).suggestion, true);
    assert.equal(candidateSchema.parse(c[1]).suggestion, undefined);
    assert.deepEqual(
      c.map((candidate) => candidate.text),
      plain.prompts.map((p) => p.prompt),
    );
    await assert.rejects(f.engine.send(scope, [c[0].key, c[1].key]), /combination/i);
    await f.engine.send(scope, [c[2].key, c[0].key]);
    assert.deepEqual(f.sent, [
      "1. Implement the layout.\n   Preserve other files.\n2. List remaining risks.",
    ]);
  } finally {
    f.engine.close();
  }
});

test("v1 suggestion flag never bypasses individual Git actions", async () => {
  const value = declaration();
  value.prompts[0].prompt = "Commit the layout. Do not push.";
  const flagged = {
    ...value,
    prompts: value.prompts.map((p) => ({ ...p, suggestion: true })),
  };
  const f = fixture(markdown(flagged));
  try {
    const c = (await f.engine.inspect(scope)).candidates;
    assert.equal(c.length, 3);
    await assert.rejects(f.engine.send(scope, [c[0].key, c[2].key]), /individual manual send/);
    await assert.rejects(
      f.engine.send(scope, c[0].key, { automatic: true }),
      /individual manual send/,
    );
    await f.engine.send(scope, c[0].key);
    assert.match(f.sent[0], /^\/commit --no-push\nCommit the layout\. Do not push\./);
    assert.doesNotMatch(f.sent[0], /suggestion/);
  } finally {
    f.engine.close();
  }
});
