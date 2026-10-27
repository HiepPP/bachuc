# TASK-306 Outcome

## Status

DONE. No screen or live check was in this spec.

## Changed

- `packages/server/src/server/agent/skill-hook-usage.ts` (new): `applySkillHookEvent`, `SkillHookEvent`, `hasSkillHookUsage`. Entries newest first, 50 at most, key `name`; a hook with status `running` is not counted.
- `packages/server/src/server/agent/skill-hook-usage.test.ts` (new): the red tests of the spec, unchanged.
- `packages/server/src/server/agent/agent-manager.ts`: `skillHookUsage?` on `ManagedAgentBase`; `dispatchSessionEvent` handles `skill_used` and `hook_used` through `applySkillHookUsage` (`emitState` with `{ persist: false }`); `agentsWithUnsavedUsage` marks the unsaved agents; a terminal event persists once if the mark is still set; `persistSnapshot` clears the mark. `registerSession` loads `skillHookUsage` from the stored record; `dispatchStoredAgentState` copies it; `prepareAgentForClosure` keeps it through its spread, so close and reload write it.
- `packages/server/src/server/agent/agent-manager.test.ts`: 4 tests (snapshot update and omitted field, write count, restart, history replay).
- `packages/server/src/server/agent/agent-storage.ts`: optional `skillHookUsage` in `STORED_AGENT_SCHEMA`.
- `packages/server/src/server/agent/agent-projections.ts`: non-empty `skillHookUsage` goes to the stored record, the live snapshot, and the stored-agent payload.
- `packages/server/src/server/agent/agent-sdk-types.ts`: `AgentSkillHookUsage`, `SkillHookUsageEntry`.
- `packages/protocol/src/messages.ts`: `AgentSkillHookUsageSchema`; optional `skillHookUsage` on `AgentSnapshotPayloadSchema`.
- `packages/desktop-gpui/crates/protocol/codegen/protocol.schema.json`: regenerated. +1622 lines, because the snapshot schema is inlined in many messages. It now has `skillHookUsage` and `hooks`.

## Contract

- Consumes: `ToolCallHook`, `skill_used`, and `hook_used` from TASK-305 with the exact names and types.
- Produces: snapshot field `skillHookUsage?: { skills: SkillHookUsageEntry[]; hooks: SkillHookUsageEntry[] }` with `SkillHookUsageEntry = { name; event?; count; errorCount; lastUsedAt }`. A hook entry carries `event`; a skill entry has none.
- No `server_info.features` flag; the field is optional read-only data.
- The turn-end write path: `finalizeForegroundTurn` already persists through `emitState`, so a foreground turn adds no write. The new terminal check covers a turn that ends with no state write (an autonomous turn).
- Impact (GitNexus): `persistSnapshot` HIGH (70 callers; the change is one `Set.delete`), `toAgentPayload` CRITICAL (44 callers; the change adds one optional field and only when the summary is non-empty), `dispatchSessionEvent` LOW. Behavior for existing input is unchanged: an agent with no use gets no new field and the same write count.

## Verified

- Red: `skill-hook-usage.test.ts` ran 0 tests because `./skill-hook-usage.js` did not exist.
- `npx vitest run packages/server/src/server/agent/skill-hook-usage.test.ts --bail=1` -> 5 passed.
- `npx vitest run packages/server/src/server/agent/agent-manager.test.ts --bail=1` -> 199 passed (4 new).
- `npx vitest run packages/server/src/server/agent/agent-storage.test.ts --bail=1` -> 24 passed. Every existing record lacks the field, so they prove a record without it loads. I added no test there, because the file is not in `Files:`.
- `npx vitest run packages/server/src/server/agent/agent-projections.test.ts --bail=1` -> 20 passed.
- `npx vitest run packages/protocol/src/messages.wire-compat.test.ts --bail=1` -> 18 passed, with a snapshot with and without `skillHookUsage`. The test sits in the file that TASK-305 owns.
- `node packages/desktop-gpui/crates/protocol/codegen/export-schema.mjs` -> wrote the schema; `rg` finds `skillHookUsage` (18 matches) and `"hooks"` (56 matches). `./cargo.sh check -p bachuc-protocol` (jobs 4) -> exit 0, so the Rust type generation accepts it. I ran `./cargo.sh clean --profile dev` after (no other cargo ran).
- Group run (also counts for TASK-305): `npm run build:server` -> exit 0; `npm run typecheck` -> exit 0; `rtk proxy npm run lint -- packages/server/src/server/agent packages/protocol/src` -> 0 errors; `npm run format:check:files` on every changed file -> clean.

## Anti-goal

Tripwire: agent record writes of a turn with 20 `hook_used` events. Source: `storage.applySnapshot` spy calls for the agent during one `streamAgent` turn, in `agent-manager.test.ts` ("a turn with 20 hook events adds at most one agent record write"), 2026-10-27. Baseline is the same turn with no skill or hook event; the busy turn has 1 `skill_used` and 20 `hook_used`.

- Before changes: baseline 2, busy 2. Measured with the new event branch switched off, so the events were ignored as before.
- After the save change: baseline 2, busy 2 (+0).
- At completion: not read as a number again. The test asserts `busy - baseline <= 1` and passed in the final full `agent-manager.test.ts` run.

## Lessons

- `emitState` persists on every call by default, so each usage event passes `{ persist: false }`. A foreground turn already persists once in `finalizeForegroundTurn`.
- The server resolves `@getpaseo/protocol` from `dist`: a new protocol schema fails server tests with "Cannot read properties of undefined" until `npm run build:server` runs.
- Each usage event still sends one `agent_state` to clients (`persist: false` only skips the write). A turn with 20 hooks sends 20 snapshots. If that costs too much, send one at turn end.
- The restored agent reads `skillHookUsage` from the registry in `registerSession`, so `agent-loading.ts` needs no change.

## Live Probe

Run by the lead on 2026-10-27 at 01:27, on live (6768) with commit 002d06e34. The record `.dev/paseo-home/agents/tmp-wt-probe/cd36194f-7bf7-43a5-90f8-2c95131a1b5e.json` holds `skillHookUsage` with 5 hook entries, newest first: `Stop` 10, `PostToolUse:Bash` 3, `PreToolUse:Bash` 3, `UserPromptSubmit` 6, `SessionStart:startup` 3. All `errorCount` values are 0, and `skills` is empty. PASS.
