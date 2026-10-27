# TASK-306 Count, store, and send the skill and hook summary

Group: A (shared file with TASK-305: `packages/protocol/src/messages.ts`, `packages/server/src/server/agent/agent-sdk-types.ts`)
Class: risky

## Brief

Goal: The daemon keeps a per-agent count of skills and hooks, stores it in the agent record, and sends it on the agent snapshot.

Change: no summary exists -> `skill_used` and `hook_used` events update `skillHookUsage`, which survives a daemon restart.

Boundaries:

- Follows [ADR-0068](watchtower/decisions/ADR-0068-hooks-in-tool-calls-and-stored-usage-summary.md): store the summary in the agent record JSON; do not scan the stored timeline to rebuild it.
- Follows [ADR-0032](watchtower/decisions/ADR-0032-sqlite-served-old-pages-over-memory-only-serving.md): add no SQL read for this feature.
- Use the shapes in `## Wire Shapes` of [watchtower/CONTEXT.md](watchtower/CONTEXT.md) exactly. Keep entries newest first, at most 50 each.
- A skill entry key is `name`. A hook entry key is `name`, and it also carries `event`. `errorCount` counts hooks with status `error`; skills keep `errorCount: 0`.
- Planner call: send no `server_info.features` flag. The field is optional read-only data.
- Planner call: write the agent record for a usage change at turn end and on agent close, not per event. A turn with 20 hook events adds at most one record write.
- Trap: `emitState` in `agent-manager.ts`: it calls `enqueueBackgroundPersist` on every call unless it gets `{ persist: false }`. The `skill_used` and `hook_used` cases must pass `{ persist: false }`. Check that the `turn_completed` path reaches `persistSnapshot`; add the call there if it does not.
- Trap: `ManagedAgent` in `agent-manager.ts`: it is built in several places, like `backgroundProcesses: new Map()`. Set `skillHookUsage` at every build site, and load it from the stored record when an agent is restored.
- The provider never emits `hook_used` with status `running`. The reducer counts any other status, and counts `error` in `errorCount`.
- Ties: the entry applied last is the newest.
- Trap: `AgentSnapshotPayloadSchema` in [packages/protocol/src/messages.ts](packages/protocol/src/messages.ts): add the field as optional. A required field breaks an old daemon's snapshots in a new client.
- Trap: `STORED_AGENT_SCHEMA` in [packages/server/src/server/agent/agent-storage.ts](packages/server/src/server/agent/agent-storage.ts): the field must be optional, so every stored agent without it still loads.

How:

- Add a pure reducer `applySkillHookEvent(usage: AgentSkillHookUsage | undefined, event: SkillHookEvent, at: string): AgentSkillHookUsage` in a new `skill-hook-usage.ts`. `SkillHookEvent` is `{ kind: "skill"; name: string } | { kind: "hook"; name: string; event: string; status: ToolCallHook["status"] }`. It returns a new object and never changes its input.
- In `agent-manager.ts`, handle `skill_used` and `hook_used` stream events on the managed agent, and load and save `skillHookUsage` with the agent record.
- In `agent-projections.ts`, copy a non-empty `skillHookUsage` to the snapshot payload, as `backgroundProcesses` does.
- Refresh the GPUI schema with `packages/desktop-gpui/crates/protocol/codegen/export-schema.mjs`.

Files:

- [packages/server/src/server/agent/skill-hook-usage.ts](packages/server/src/server/agent/skill-hook-usage.ts) (new reducer)
- [packages/server/src/server/agent/skill-hook-usage.test.ts](packages/server/src/server/agent/skill-hook-usage.test.ts) (new tests)
- [packages/server/src/server/agent/agent-manager.ts](packages/server/src/server/agent/agent-manager.ts) (event handling, load, save at turn end)
- [packages/server/src/server/agent/agent-manager.test.ts](packages/server/src/server/agent/agent-manager.test.ts) (events update the snapshot; one write per turn; restart keeps counts)
- [packages/server/src/server/agent/agent-storage.ts](packages/server/src/server/agent/agent-storage.ts) (optional `skillHookUsage` in the stored record)
- [packages/server/src/server/agent/agent-projections.ts](packages/server/src/server/agent/agent-projections.ts) (snapshot field)
- [packages/server/src/server/agent/agent-sdk-types.ts](packages/server/src/server/agent/agent-sdk-types.ts) (`AgentSkillHookUsage`, `SkillHookUsageEntry`)
- [packages/protocol/src/messages.ts](packages/protocol/src/messages.ts) (`AgentSkillHookUsageSchema`, optional `skillHookUsage` on `AgentSnapshotPayloadSchema`)
- [packages/desktop-gpui/crates/protocol/codegen/protocol.schema.json](packages/desktop-gpui/crates/protocol/codegen/protocol.schema.json) (regenerated)

Interfaces:

- Consumes: `ToolCallHook = { id: string; name: string; event: string; status: "running" | "success" | "error" | "cancelled"; exitCode?: number; output?: string }` from TASK-305
- Consumes: `AgentStreamEvent` member `{ type: "skill_used"; provider: AgentProvider; name: string; source: "tool" | "slash_command" }` from TASK-305
- Consumes: `AgentStreamEvent` member `{ type: "hook_used"; provider: AgentProvider; hook: ToolCallHook }` from TASK-305
- Produces: agent snapshot field `skillHookUsage?: { skills: SkillHookUsageEntry[]; hooks: SkillHookUsageEntry[] }` with `SkillHookUsageEntry = { name: string; event?: string; count: number; errorCount: number; lastUsedAt: string }` for TASK-308

Expected result:

- WHEN a `skill_used` event arrives THE agent manager SHALL add 1 to that skill's `count`, set `lastUsedAt`, and move the entry to the front.
- WHEN a `hook_used` event with status `error` arrives THE agent manager SHALL add 1 to that hook's `count` and `errorCount`.
- WHEN the daemon restarts THE agent snapshot SHALL carry the same `skillHookUsage` as before the restart.
- IF the agent has no skill or hook use THEN THE agent snapshot SHALL omit `skillHookUsage`.
- IF a stored agent record has no `skillHookUsage` THEN THE daemon SHALL load it without error.
- WHEN more than 50 distinct skills are used THE agent manager SHALL keep the 50 newest.

Red tests:

File: [packages/server/src/server/agent/skill-hook-usage.test.ts](packages/server/src/server/agent/skill-hook-usage.test.ts)

```ts
import { describe, expect, it } from "vitest";
import { applySkillHookEvent } from "./skill-hook-usage.js";

const T1 = "2026-10-27T10:00:00.000Z";
const T2 = "2026-10-27T10:01:00.000Z";

describe("applySkillHookEvent", () => {
  it("starts a summary from nothing", () => {
    expect(applySkillHookEvent(undefined, { kind: "skill", name: "watchtower" }, T1)).toEqual({
      skills: [{ name: "watchtower", count: 1, errorCount: 0, lastUsedAt: T1 }],
      hooks: [],
    });
  });

  it("counts a skill again and moves it to the front", () => {
    let usage = applySkillHookEvent(undefined, { kind: "skill", name: "a" }, T1);
    usage = applySkillHookEvent(usage, { kind: "skill", name: "b" }, T1);
    usage = applySkillHookEvent(usage, { kind: "skill", name: "a" }, T2);
    expect(usage.skills.map((entry) => [entry.name, entry.count])).toEqual([
      ["a", 2],
      ["b", 1],
    ]);
    expect(usage.skills[0]?.lastUsedAt).toBe(T2);
  });

  it("counts hook errors", () => {
    let usage = applySkillHookEvent(
      undefined,
      { kind: "hook", name: "PreToolUse:Bash", event: "PreToolUse", status: "success" },
      T1,
    );
    usage = applySkillHookEvent(
      usage,
      { kind: "hook", name: "PreToolUse:Bash", event: "PreToolUse", status: "error" },
      T2,
    );
    expect(usage.hooks).toEqual([
      { name: "PreToolUse:Bash", event: "PreToolUse", count: 2, errorCount: 1, lastUsedAt: T2 },
    ]);
  });

  it("keeps the 50 newest entries", () => {
    let usage = applySkillHookEvent(undefined, { kind: "skill", name: "s0" }, T1);
    for (let index = 1; index <= 55; index += 1) {
      usage = applySkillHookEvent(usage, { kind: "skill", name: `s${index}` }, T1);
    }
    expect(usage.skills).toHaveLength(50);
    expect(usage.skills[0]?.name).toBe("s55");
    expect(usage.skills.some((entry) => entry.name === "s0")).toBe(false);
  });

  it("does not change the input summary", () => {
    const before = applySkillHookEvent(undefined, { kind: "skill", name: "a" }, T1);
    applySkillHookEvent(before, { kind: "skill", name: "a" }, T2);
    expect(before.skills[0]?.count).toBe(1);
  });
});
```

Anti-goal: agent record writes; a turn with 20 `hook_used` events adds at most one agent record write; tripwire; read from a storage spy in [packages/server/src/server/agent/agent-manager.test.ts](packages/server/src/server/agent/agent-manager.test.ts) before changes, after the save change, and at completion.

## Verify

- `npx vitest run packages/server/src/server/agent/skill-hook-usage.test.ts --bail=1` -> all tests pass.
- `npx vitest run packages/server/src/server/agent/agent-manager.test.ts --bail=1` -> new tests pass: events update the snapshot, an agent without use omits the field, a reload from storage keeps the counts, and a provider history replay adds no count.
- `npx vitest run packages/server/src/server/agent/agent-storage.test.ts --bail=1` -> a stored record without `skillHookUsage` still loads.
- `npx vitest run packages/protocol/src/messages.wire-compat.test.ts --bail=1` -> a snapshot with and without `skillHookUsage` both parse.
- `node packages/desktop-gpui/crates/protocol/codegen/export-schema.mjs` then `git diff --stat packages/desktop-gpui/crates/protocol/codegen/protocol.schema.json` -> the schema shows `skillHookUsage` and `hooks`.
- `npm run build:server && npm run typecheck` -> exit 0.
- `npm run lint -- packages/server/src/server/agent packages/protocol/src` -> exit 0.
- Anti-goal read: the 20-hook turn test with a storage spy -> at most one extra write at each checkpoint.
