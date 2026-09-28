import { EvaluationError } from "../server/evaluation-error";
import { spawn } from "node:child_process";
import { createBridge } from "../server/bridge";
import type { Engine } from "../server/engine";
import assert from "node:assert/strict";
import {
  mkdtemp,
  mkdir,
  writeFile,
  readFile,
  readdir,
  unlink,
  rmdir,
  realpath,
} from "node:fs/promises";
import path from "node:path";
import { tmpdir } from "node:os";
import test from "node:test";
import { parse } from "smol-toml";
import { NativeTickets } from "../server/native-tickets";
import { prepareNativeLaunch, defaultNativeSettings } from "../server/native-launch";
import type { Judge } from "../server/types";

async function cleanup(dir: string) {
  for (const e of await readdir(dir, { withFileTypes: true })) {
    const f = path.join(dir, e.name);
    if (e.isDirectory()) await cleanup(f);
    else await unlink(f);
  }
  await rmdir(dir);
}
async function fixture(
  judge: Judge,
  now = Date.now,
  runtime?: ConstructorParameters<typeof NativeTickets>[3],
) {
  const root = await realpath(await mkdtemp(path.join(tmpdir(), "jev-tickets-")));
  const base = path.join(root, "paseo/plugin-data/jev-orchestrator/native");
  await mkdir(base, { recursive: true });
  const settings = path.join(base, "settings.json");
  await writeFile(settings, JSON.stringify(defaultNativeSettings()));
  const env = await prepareNativeLaunch(path.join(root, "paseo"), root, root, "codex");
  const tickets = new NativeTickets(root, judge, now, runtime);
  await tickets.bind("parent", root, env.PASEO_JEV_NATIVE_POLICY);
  const input = {
    requestId: "task_1",
    task: "Compute 17*19. Reply EXACT:323. No tools.",
    sourceRole: "default",
    forkTurns: "none",
    shareWithJev: true,
  };
  const intent = (value = input) =>
    tickets.handle("native_intent", "parent", root, {
      sessionId: "native-session",
      cwd: root,
      input: value,
    });
  const prepare = (value = input) =>
    tickets.handle("native_prepare", "parent", root, value) as Promise<any>;
  const consume = (ticket: any, changes = {}) =>
    tickets.handle("native_consume", "parent", root, {
      sessionId: "native-session",
      cwd: root,
      input: {
        task_name: ticket.taskName,
        agent_type: "default",
        fork_turns: "none",
        message: "gAAAAA-ciphertext",
        model: "outside",
        reasoning_effort: "high",
        ...changes,
      },
    }) as Promise<any>;
  return {
    root,
    env,
    tickets,
    input,
    settings,
    intent,
    prepare,
    consume,
    cleanup: () => cleanup(root),
  };
}
const choose: Judge = async () => ({
  profileId: "c1",
  discovery: false,
  risk: "low",
  category: "bounded",
});

test("preflight binds plaintext, evaluates once, and racing consumes grant exactly one spawn", async () => {
  let calls = 0;
  const f = await fixture(async (phase, state, profiles, signal) => {
    calls++;
    assert.equal(phase, "direct");
    assert.equal(state.task, "Compute 17*19. Reply EXACT:323. No tools.");
    assert.equal(profiles[1].model, "gpt-5.6-luna");
    assert.equal(profiles[1].thinkingOptionId, "max");
    return choose(phase, state, profiles, signal);
  });
  try {
    await assert.rejects(f.prepare()); // No trusted hook intent.
    await f.intent();
    const [a, b] = await Promise.all([f.prepare(), f.prepare()]);
    assert.deepEqual(a, b);
    assert.equal(calls, 1);
    assert.equal(a.model, "gpt-5.6-luna");
    assert.match(a.taskName, /^[a-z0-9_]+$/);
    const role = parse(
      await readFile(path.join(f.root, ".codex/agents", a.routedAgentType + ".toml"), "utf8"),
    );
    assert.equal(role.model_reasoning_effort, "max");
    assert.match(String(role.developer_instructions), /Compute 17\*19/);
    await assert.rejects(f.consume(a, { fork_turns: "all" }));
    await assert.rejects(
      f.tickets.handle("native_consume", "parent", f.root, {
        sessionId: "other",
        cwd: f.root,
        input: { task_name: a.taskName },
      }),
    );
    const results = await Promise.allSettled([f.consume(a), f.consume(a)]);
    assert.equal(results.filter((r) => r.status === "fulfilled").length, 1);
    const output = (results.find((r) => r.status === "fulfilled") as PromiseFulfilledResult<any>)
      .value;
    assert.equal(output.hookSpecificOutput.updatedInput.agent_type, a.routedAgentType);
    assert.equal(output.hookSpecificOutput.updatedInput.message, "gAAAAA-ciphertext");
    assert.equal(output.hookSpecificOutput.updatedInput.model, undefined);
    await assert.rejects(f.intent());
    await assert.rejects(f.consume(a));
  } finally {
    await f.cleanup();
  }
});

test("changed task, workspace, policy, definition, expired ticket and revoked scope reject", async () => {
  let now = 1000;
  const f = await fixture(choose, () => now);
  try {
    await f.intent();
    await assert.rejects(f.prepare({ ...f.input, task: "Different task" }));
    const a = await f.prepare();
    await assert.rejects(f.tickets.handle("native_status", "parent", "/", {}));
    now += 300001;
    await assert.rejects(f.consume(a));
    now = 1000;
    const file = path.join(f.root, ".codex/agents", a.routedAgentType + ".toml");
    await writeFile(file, "tampered");
    await assert.rejects(f.consume(a));
    const original = await readFile(f.settings, "utf8");
    await writeFile(f.settings, original + " ");
    await assert.rejects(f.tickets.handle("native_status", "parent", f.root, {}));
    await writeFile(f.settings, original);
    await f.tickets.archive("parent");
    await assert.rejects(readFile(file), /ENOENT/);
    await assert.rejects(f.tickets.handle("native_status", "parent", f.root, {}));
  } finally {
    await f.cleanup();
  }
});

test("failure has no fallback/re-evaluation, and three slots are never recycled", async () => {
  let calls = 0;
  const f = await fixture(async () => {
    calls++;
    throw new Error("API failure");
  });
  try {
    await f.intent();
    await assert.rejects(f.prepare());
    await assert.rejects(f.prepare());
    assert.equal(calls, 1);
    await f.intent({ ...f.input, requestId: "task_2" });
    await f.intent({ ...f.input, requestId: "task_3" });
    await assert.rejects(f.intent({ ...f.input, requestId: "task_4" }), /capacity/);
  } finally {
    await f.cleanup();
  }
});

test("real command hook registers root intent and consumes over the scoped bridge", async () => {
  const f = await fixture(choose);
  let scopeError: Error | undefined;
  const bridge = createBridge(
    {} as Engine,
    async () => {
      if (scopeError) throw scopeError;
      return { cwd: f.root };
    },
    (action, parent, cwd, input) => f.tickets.handle(action, parent, cwd, input),
  );
  const token = bridge.issue(f.root);
  bridge.bind(token, "parent", f.root);
  const url = await bridge.ready;
  const sessions = path.join(f.root, ".codex/sessions");
  await mkdir(sessions);
  const transcript = path.join(sessions, "root.jsonl");
  await writeFile(
    transcript,
    JSON.stringify({ type: "session_meta", payload: { id: "native-session", source: "vscode" } }) +
      "\n",
  );
  const env = {
    ...process.env,
    ...f.env,
    CODEX_HOME: path.join(f.root, ".codex"),
    PASEO_ORCH_URL: url,
    PASEO_ORCH_TOKEN: token,
  };
  const hook = (tool: string, input: unknown) =>
    new Promise<any>((resolve, reject) => {
      const child = spawn(
        process.execPath,
        [
          "--import",
          path.resolve("node_modules/tsx/dist/loader.mjs"),
          path.resolve("server/native-hook-command.ts"),
        ],
        { env },
      );
      let out = "";
      child.stdout.on("data", (data) => (out += data));
      child.stderr.resume();
      child.once("error", reject);
      child.once("close", () => {
        try {
          resolve(JSON.parse(out));
        } catch (error) {
          reject(error);
        }
      });
      child.stdin.end(
        JSON.stringify({
          hook_event_name: "PreToolUse",
          tool_name: tool,
          session_id: "native-session",
          transcript_path: transcript,
          cwd: f.root,
          tool_input: input,
        }),
      );
    });
  try {
    scopeError = new Error("Transport not connected (status: disconnected)");
    const disconnected = await hook("mcp__jev_orchestrator__prepare_native_delegate", f.input);
    assert.equal(disconnected.hookSpecificOutput.permissionDecision, "deny");
    assert.match(
      disconnected.hookSpecificOutput.permissionDecisionReason,
      /daemon transport disconnected/,
    );
    assert.match(
      disconnected.hookSpecificOutput.permissionDecisionReason,
      /paseo plugin reload jev-orchestrator/,
    );
    scopeError = new Error("private internal error must not escape");
    const rejected = await hook("mcp__jev_orchestrator__prepare_native_delegate", f.input);
    assert.match(rejected.hookSpecificOutput.permissionDecisionReason, /ticket rejected/);
    assert.doesNotMatch(JSON.stringify(rejected), /private internal/);
    scopeError = undefined;
    assert.deepEqual(await hook("mcp__jev_orchestrator__prepare_native_delegate", f.input), {});
    const res = await fetch(url, {
      method: "POST",
      headers: { Authorization: `Bearer ${token}`, "Content-Type": "application/json" },
      body: JSON.stringify({ action: "native_prepare", input: f.input }),
    });
    assert.equal(res.status, 200);
    const ticket = (await res.json()) as any;
    const input = {
      task_name: ticket.taskName,
      agent_type: "default",
      fork_turns: "none",
      message: "ciphertext",
    };
    assert.equal(
      (await hook("collaborationspawn_agent", input)).hookSpecificOutput.permissionDecision,
      "allow",
    );
    assert.equal(
      (await hook("collaborationspawn_agent", input)).hookSpecificOutput.permissionDecision,
      "deny",
    );
    const rootTranscript = await readFile(transcript, "utf8");
    await writeFile(
      transcript,
      JSON.stringify({
        type: "session_meta",
        payload: { id: "native-session", parent_thread_id: "other-root", source: { subagent: {} } },
      }) + "\n",
    );
    assert.equal(
      (
        await hook("mcp__jev_orchestrator__prepare_native_delegate", {
          ...f.input,
          requestId: "child_attempt",
        })
      ).hookSpecificOutput.permissionDecision,
      "deny",
    );
    await writeFile(transcript, rootTranscript);
    bridge.revoke("parent");
    const revoked = await hook("mcp__jev_orchestrator__prepare_native_delegate", f.input);
    assert.equal(revoked.hookSpecificOutput.permissionDecision, "deny");
    assert.match(revoked.hookSpecificOutput.permissionDecisionReason, /session binding expired/);
    bridge.close();
    const offline = await hook("mcp__jev_orchestrator__prepare_native_delegate", f.input);
    assert.equal(offline.hookSpecificOutput.permissionDecision, "deny");
    assert.match(offline.hookSpecificOutput.permissionDecisionReason, /bridge unreachable/);
  } finally {
    bridge.close();
    await f.cleanup();
  }
});

test("Astra/low self decision is deduplicated and cannot authorize a spawn", async () => {
  let calls = 0;
  const f = await fixture(
    async (_phase, _state, profiles) => {
      calls++;
      assert.ok(profiles.some((p) => p.id === "self"));
      return { profileId: "self", discovery: false, risk: "low", category: "bounded" };
    },
    Date.now,
    async () => ({ model: "gpt-6-astra", thinkingOptionId: "low" }),
  );
  try {
    await f.intent();
    const result = await f.prepare();
    assert.equal(result.action, "self");
    assert.equal(result.state, "self");
    assert.equal(result.taskName, undefined);
    assert.equal(result.routedAgentType, undefined);
    await f.intent();
    assert.deepEqual(await f.prepare(), result);
    assert.equal(calls, 1);
    await assert.rejects(f.consume(result));
    const files = await readdir(path.join(f.root, ".codex/agents"));
    for (const file of files.filter((f) => f.startsWith("jev-native-ticket-")))
      assert.match(
        await readFile(path.join(f.root, ".codex/agents", file), "utf8"),
        /Unissued Jev ticket/,
      );
  } finally {
    await f.cleanup();
  }
});

test("self is rejected without verified Astra/low and if runtime changes during evaluation", async () => {
  for (const changed of [false, true]) {
    let reads = 0;
    const f = await fixture(
      async (_phase, _state, profiles) => {
        assert.equal(
          profiles.some((p) => p.id === "self"),
          changed,
        );
        return { profileId: "self", discovery: false, risk: "low", category: "bounded" };
      },
      Date.now,
      async () =>
        ++reads === 1 && changed
          ? { model: "gpt-6-astra", thinkingOptionId: "low" }
          : { model: "gpt-5.6-luna", thinkingOptionId: "max" },
    );
    try {
      await f.intent();
      await assert.rejects(f.prepare());
    } finally {
      await f.cleanup();
    }
  }
});

test("preflight exposes safe stage/code through bridge and status without retrying", async () => {
  let calls = 0;
  const secret = "private-task-token-path";
  const f = await fixture(async () => {
    calls++;
    throw new EvaluationError("JEV_GATEWAY_HTTP", { httpStatus: 429, processExitCode: 1 });
  });
  const bridge = createBridge(
    {} as Engine,
    async () => ({ cwd: f.root }),
    (action, parent, cwd, input) => f.tickets.handle(action, parent, cwd, input),
  );
  const token = bridge.issue(f.root);
  bridge.bind(token, "parent", f.root);
  try {
    await f.intent();
    const response = await fetch(await bridge.ready, {
      method: "POST",
      headers: { Authorization: `Bearer ${token}`, "Content-Type": "application/json" },
      body: JSON.stringify({ action: "native_prepare", input: f.input }),
    });
    assert.equal(response.status, 400);
    const body = (await response.json()) as any;
    assert.equal(body.failureStage, "evaluation");
    assert.equal(body.failureCode, "NATIVE_EVALUATION_FAILED");
    assert.deepEqual(body.evaluationError, {
      workerCode: "JEV_GATEWAY_HTTP",
      httpStatus: 429,
      processExitCode: 1,
    });
    assert.doesNotMatch(JSON.stringify(body), new RegExp(secret));
    const status = (await f.tickets.handle("native_status", "parent", f.root, {})) as any;
    assert.equal(status.records[0].state, "failed");
    assert.equal(status.records[0].failureStage, body.failureStage);
    assert.equal(status.records[0].failureCode, body.failureCode);
    assert.deepEqual(status.records[0].evaluationError, body.evaluationError);
    assert.doesNotMatch(JSON.stringify(status), new RegExp(secret));
    await assert.rejects(f.prepare());
    await assert.rejects(f.consume(status.records[0]));
    assert.equal(calls, 1);
  } finally {
    bridge.close();
    await f.cleanup();
  }
});

test("runtime, invalid decision and role-write failures have distinct stages", async () => {
  for (const scenario of ["runtime", "decision", "write"] as const) {
    let root = "";
    const f = await fixture(
      async (...args) => {
        if (scenario === "decision")
          return { profileId: "invalid", discovery: false, risk: "low", category: "lookup" };
        if (scenario === "write") {
          const dir = path.join(root, ".codex/agents");
          const role = (await readdir(dir)).find(
            (name) => name.startsWith("jev-native-ticket-") && name.endsWith("-0.toml"),
          )!;
          await writeFile(path.join(dir, role + ".pending"), "private-content");
        }
        return choose(...args);
      },
      Date.now,
      scenario === "runtime"
        ? async () => {
            throw new Error("private-runtime");
          }
        : undefined,
    );
    root = f.root;
    try {
      await f.intent();
      await assert.rejects(f.prepare());
      const status = (await f.tickets.handle("native_status", "parent", f.root, {})) as any;
      const expected = {
        runtime: ["parent_runtime", "NATIVE_PARENT_RUNTIME_FAILED"],
        decision: ["decision_validation", "NATIVE_DECISION_INVALID"],
        write: ["ticket_write", "NATIVE_TICKET_WRITE_FAILED"],
      }[scenario];
      assert.equal(status.records[0].failureStage, expected[0]);
      assert.equal(status.records[0].failureCode, expected[1]);
      assert.equal(status.records[0].state, "failed");
      assert.doesNotMatch(JSON.stringify(status), /private-/);
    } finally {
      await f.cleanup();
    }
  }
});
