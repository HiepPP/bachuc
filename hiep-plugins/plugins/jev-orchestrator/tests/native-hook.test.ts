import assert from "node:assert/strict";
import test from "node:test";
import { nativePolicySchema, routeNativeHook, type NativePolicy } from "../server/native-hook";
import type { Judge } from "../server/types";

const codexPolicy: NativePolicy = {
  runtime: "codex",
  routes: [
    {
      sourceType: "worker",
      candidates: [
        {
          agentType: "jev-worker-low",
          model: "gpt-6-astra",
          effort: "low",
          description: "Fast bounded implementation",
        },
        {
          agentType: "jev-worker-high",
          model: "gpt-6-astra",
          effort: "high",
          description: "Complex implementation",
        },
      ],
    },
  ],
};

const choose =
  (id: string): Judge =>
  async () => ({
    profileId: id,
    discovery: false,
    risk: "low",
    category: "implementation",
  });

test("Codex routing preserves unrelated input and lets the pinned agent type own model and effort", async () => {
  const event = {
    hook_event_name: "PreToolUse",
    tool_name: "spawn_agent",
    cwd: "/workspace",
    tool_input: {
      message: "Implement the bounded adapter.",
      agent_type: "worker",
      model: "outside-policy",
      reasoning_effort: "max",
      model_reasoning_effort: "medium",
      effort: "xhigh",
      task_name: "adapter",
      fork_turns: "none",
      metadata: { ticket: 7 },
    },
  };
  let calls = 0;
  const judge: Judge = async (phase, state, profiles, signal) => {
    calls++;
    assert.equal(phase, "direct");
    assert.equal(signal.aborted, false);
    assert.deepEqual(state, {
      task: "Implement the bounded adapter.",
      originalRole: "worker",
      allowedPairs: [
        {
          id: "c0",
          agentType: "jev-worker-low",
          model: "gpt-6-astra",
          effort: "low",
        },
        {
          id: "c1",
          agentType: "jev-worker-high",
          model: "gpt-6-astra",
          effort: "high",
        },
      ],
    });
    assert.deepEqual(
      profiles.map(({ id, name, provider, model, thinkingOptionId, notes }) => ({
        id,
        name,
        provider,
        model,
        thinkingOptionId,
        notes,
      })),
      [
        {
          id: "c0",
          name: "jev-worker-low",
          provider: "codex",
          model: "gpt-6-astra",
          thinkingOptionId: "low",
          notes: "Fast bounded implementation",
        },
        {
          id: "c1",
          name: "jev-worker-high",
          provider: "codex",
          model: "gpt-6-astra",
          thinkingOptionId: "high",
          notes: "Complex implementation",
        },
      ],
    );
    return {
      profileId: "c1",
      discovery: false,
      risk: "low",
      category: "implementation",
    };
  };

  const output = await routeNativeHook(event, codexPolicy, judge);

  assert.equal(calls, 1);
  assert.deepEqual(output, {
    hookSpecificOutput: {
      hookEventName: "PreToolUse",
      permissionDecision: "allow",
      updatedInput: {
        message: "Implement the bounded adapter.",
        agent_type: "jev-worker-high",
        task_name: "adapter",
        fork_turns: "none",
        metadata: { ticket: 7 },
      },
    },
  });
  assert.equal(event.tool_input.model, "outside-policy");
});

test("Claude routing removes explicit model but leaves normal permissions and other input unchanged", async () => {
  const policy: NativePolicy = {
    runtime: "claude",
    routes: [
      {
        sourceType: "worker",
        candidates: [
          {
            agentType: "jev-claude-worker",
            model: "claude-opus-4-1",
            effort: "high",
            description: "Deep implementation",
          },
        ],
      },
    ],
  };
  const output = await routeNativeHook(
    {
      hook_event_name: "PreToolUse",
      tool_name: "Task",
      tool_input: {
        prompt: "Implement the feature.",
        subagent_type: "worker",
        model: "haiku",
        run_in_background: true,
      },
    },
    policy,
    choose("c0"),
  );

  assert.deepEqual(output, {
    hookSpecificOutput: {
      hookEventName: "PreToolUse",
      updatedInput: {
        prompt: "Implement the feature.",
        subagent_type: "jev-claude-worker",
        run_in_background: true,
      },
    },
  });
});

test("policy rejects Luna below max, duplicate pairs, generated types and recursive routes", () => {
  assert.equal(
    nativePolicySchema.safeParse({
      runtime: "codex",
      routes: [
        {
          sourceType: "worker",
          candidates: [
            {
              agentType: "jev-low",
              model: "gpt-5.6-luna",
              effort: "low",
              description: "Invalid Luna effort",
            },
          ],
        },
      ],
    }).success,
    false,
  );
  assert.equal(
    nativePolicySchema.safeParse({
      runtime: "codex",
      routes: [
        {
          sourceType: "worker",
          candidates: [
            {
              agentType: "generated-a",
              model: "model-a",
              effort: "low",
              description: "First",
            },
            {
              agentType: "generated-b",
              model: "model-a",
              effort: "low",
              description: "Duplicate pair",
            },
          ],
        },
        {
          sourceType: "generated-a",
          candidates: [
            {
              agentType: "generated-b",
              model: "model-b",
              effort: "high",
              description: "Duplicate generated type and recursive source",
            },
          ],
        },
      ],
    }).success,
    false,
  );
});

test("unrouted roles pass through and unsupported events or tools deny without calling Jev", async () => {
  let calls = 0;
  const judge: Judge = async () => {
    calls++;
    throw new Error("must not run");
  };
  assert.deepEqual(
    await routeNativeHook(
      {
        hook_event_name: "PreToolUse",
        tool_name: "spawn_agent",
        tool_input: { message: "Do the task.", agent_type: "reviewer" },
      },
      codexPolicy,
      judge,
    ),
    {},
  );
  const inputs: [Record<string, unknown>, RegExp][] = [
    [
      {
        hook_event_name: "PostToolUse",
        tool_name: "spawn_agent",
        tool_input: { message: "Do the task.", agent_type: "worker" },
      },
      /unsupported hook event/,
    ],
    [
      {
        hook_event_name: "PreToolUse",
        tool_name: "followup_task",
        tool_input: { message: "Do the task.", agent_type: "worker" },
      },
      /unsupported tool/,
    ],
    [
      {
        hook_event_name: "PreToolUse",
        tool_name: "spawn_agent",
        tool_input: { message: "Do the task.", agent_type: "jev-worker-low" },
      },
      /jev-worker-low is reserved for Jev routing/,
    ],
    [
      {
        hook_event_name: "PreToolUse",
        tool_name: "spawn_agent",
        tool_input: {
          message: "Do the task.",
          agent_type: "jev-native-0123456789abcdef01234567",
        },
      },
      /reserved for Jev routing/,
    ],
  ];

  for (const [event, reason] of inputs) {
    const output = await routeNativeHook(event, codexPolicy, judge);
    assert.equal(output.hookSpecificOutput?.permissionDecision, "deny");
    assert.match(output.hookSpecificOutput?.permissionDecisionReason ?? "", reason);
  }
  assert.equal(calls, 0);
});

test("Claude routes an omitted subagent_type as general-purpose", async () => {
  const policy: NativePolicy = {
    runtime: "claude",
    routes: [
      {
        sourceType: "general-purpose",
        candidates: [
          {
            agentType: "jev-general",
            model: "claude-sonnet-5",
            effort: "medium",
            description: "General work",
          },
        ],
      },
    ],
  };
  const output = await routeNativeHook(
    {
      hook_event_name: "PreToolUse",
      tool_name: "Agent",
      tool_input: { prompt: "Find X." },
    },
    policy,
    choose("c0"),
  );
  assert.deepEqual(output.hookSpecificOutput?.updatedInput, {
    prompt: "Find X.",
    subagent_type: "jev-general",
  });
});

test("invalid Jev choices and failures deny with specific reasons without exposing errors", async () => {
  const event = {
    hook_event_name: "PreToolUse",
    tool_name: "spawn_agent",
    tool_input: { message: "Secret task details.", agent_type: "worker" },
  };
  const invalid = await routeNativeHook(event, codexPolicy, choose("outside-policy"));
  const failed = await routeNativeHook(event, codexPolicy, async () => {
    throw new Error("vendor response containing secret task details");
  });

  for (const [output, reason] of [
    [invalid, /outside this route/],
    [failed, /Jev evaluation failed/],
  ] as const) {
    assert.equal(output.hookSpecificOutput?.permissionDecision, "deny");
    assert.match(output.hookSpecificOutput?.permissionDecisionReason ?? "", reason);
    assert.doesNotMatch(output.hookSpecificOutput?.permissionDecisionReason ?? "", /secret/i);
  }
});

test("resumption calls and missing task text deny before Jev", async () => {
  let calls = 0;
  const judge: Judge = async () => {
    calls++;
    return {
      profileId: "c0",
      discovery: false,
      risk: "low",
      category: "implementation",
    };
  };
  const events = [
    {
      hook_event_name: "PreToolUse",
      tool_name: "spawn_agent",
      tool_input: {
        message: "Continue the task.",
        agent_type: "worker",
        resume: "existing-child",
      },
    },
    {
      hook_event_name: "PreToolUse",
      tool_name: "spawn_agent",
      tool_input: { message: "   ", agent_type: "worker" },
    },
  ];

  for (const event of events) {
    const output = await routeNativeHook(event, codexPolicy, judge);
    assert.equal(output.hookSpecificOutput?.permissionDecision, "deny");
  }
  assert.equal(calls, 0);
});

test("malformed hook input denies without throwing or calling Jev", async () => {
  let calls = 0;
  const judge: Judge = async () => {
    calls++;
    throw new Error("must not run");
  };

  for (const input of [null, [], "event", 7]) {
    const output = await routeNativeHook(input, codexPolicy, judge);
    assert.equal(output.hookSpecificOutput?.permissionDecision, "deny");
  }
  assert.equal(calls, 0);
});

test("Codex v2 denies opaque tasks without evaluating or falling back", async () => {
  let calls = 0;
  for (const message of ["gAAAAA-encrypted", "Readable but no source discriminator"]) {
    const result = await routeNativeHook(
      {
        hook_event_name: "PreToolUse",
        tool_name: "collaborationspawn_agent",
        tool_input: { agent_type: "worker", message, task_name: "bounded", fork_turns: "none" },
      },
      codexPolicy,
      async () => {
        calls++;
        throw new Error("must not evaluate");
      },
    );
    assert.equal(result.hookSpecificOutput?.permissionDecision, "deny");
    assert.match(result.hookSpecificOutput?.permissionDecisionReason ?? "", /verifiable plaintext/);
    assert.equal(result.hookSpecificOutput?.updatedInput, undefined);
  }
  assert.equal(calls, 0);
});
