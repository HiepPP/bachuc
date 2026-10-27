# TASK-305 Outcome

## Status

DONE. The live probe line is `DEFERRED (UI checker)`.

## Changed

- `packages/server/src/server/agent/providers/claude/hook-events.ts` (new): `readClaudeHookMessage`, `hookToolName`, `truncateHookOutput`, `slashCommandSkill`, and `hookRowItem` (the own `hook` row of a hook with no tool call).
- `packages/server/src/server/agent/providers/claude/hook-events.test.ts` (new): the red tests of the spec, unchanged.
- `packages/server/src/server/agent/providers/claude/agent.ts`: `includeHookEvents: true`; hook messages handled in `appendSystemMessageEvents`; `turnToolCalls` keeps the newest item per call id (max 64); `trackTimelineEvents` runs at the end of `translateMessageToEvents` only and counts each `Skill` call once; `emitSlashCommandSkillUsed` runs after `input.push` in `startTurn`; `finishTurnHooks` re-emits running hooks as `cancelled` before every terminal event.
- `packages/server/src/server/agent/providers/claude/agent.test.ts`: 10 stream tests (nesting, late hook, own row, error status and exit code, 2048 cut, parallel calls, cancel at turn end, Skill once per call id, slash command, history replay).
- `packages/server/src/server/agent/agent-sdk-types.ts` and `packages/protocol/src/agent-types.ts`: `ToolCallHook`, `hooks?` on the tool call item; the server file also has the `skill_used` and `hook_used` stream events.
- `packages/protocol/src/messages.ts`: `ToolCallHookPayloadSchema` and optional `hooks` on `ToolCallBasePayloadSchema`. `TOOL_CALL_ICON_NAMES` is unchanged.
- `packages/protocol/src/messages.wire-compat.test.ts`: tool call item with and without `hooks` parses; an old schema drops `hooks`. It also holds the snapshot test of TASK-306.

## Contract

- Produces: `ToolCallHook`, `hooks?: ToolCallHook[]`, the own row (`name: "hook"`, `callId: "hook:<hook_id>"`, `plain_text` detail with the hook name as label and `icon: "wrench"`), and the `skill_used` and `hook_used` stream events, all with the exact names and types of the spec.
- `hook_used` is sent once per hook, on `hook_response`, with status `success`, `error`, or `cancelled`. A hook cancelled at turn end sends no `hook_used`.
- The hook of an own row is re-emitted with the same `callId`, so the manager replaces it.
- Slash command: when no init message has arrived yet (the first prompt of a session), the typed text waits and the init message resolves it. Without this, the first `/<skill>` of a new agent would not count.
- Impact (GitNexus): `AgentStreamEvent` in `agent-sdk-types.ts` is HIGH (125 callers or users; two union members added, the build and typecheck stay green). `notifySubscribers` (Claude) is MEDIUM and unchanged. `appendSystemMessageEvents` is LOW.

## Verified

- Red: `hook-events.test.ts` failed with "Cannot find module './hook-events.js'" before any other edit.
- `npx vitest run packages/server/src/server/agent/providers/claude/hook-events.test.ts --bail=1` -> 12 passed.
- `npx vitest run packages/server/src/server/agent/providers/claude/agent.test.ts --bail=1` -> 97 passed. Also green: `agent.redesign`, `agent.interrupt-restart-regression`, `agent.sub-agent-sidechain`, `agent.background-subagent`.
- `npx vitest run packages/protocol/src/messages.wire-compat.test.ts --bail=1` -> 18 passed.
- Error case: the stream test shows the hook with status `error` and `exitCode` 2, and `hook_used` with the same fields.
- Two Bash calls in one assistant message with one `PreToolUse:Bash` hook each: each hook lands on a different call.
- Group run (also counts for TASK-306): `npm run build:server` -> exit 0; `npm run typecheck` -> exit 0; `rtk proxy npm run lint -- packages/server/src/server/agent packages/protocol/src` -> 0 errors; `npm run format:check:files` on every changed file -> clean.
- Live probe, agent-run: `DEFERRED (UI checker)`. No debug log was added, so none was removed.

## Anti-goal

Tripwire: tool call identities of a one-Bash turn with its `PreToolUse` and `PostToolUse` hooks. Source: the stream test "nests tool hooks in their tool call without adding a timeline row" in `agent.test.ts`, 2026-10-27.

- Before changes: 1 identity (`toolu_b1`). Measured with the hook branch switched off, so hook messages were dropped as before; the test failed only on the missing hooks.
- After the nesting change: 1 identity, with 2 hooks on the call.
- At completion: 1 identity (test green in the final `agent.test.ts` run, 97 passed).

## Behavior change for existing input

- Intended by the spec and ADR-0068: every Claude agent now gets a `hook` row for a hook with no tool (`SessionStart`, `UserPromptSubmit`, `Stop`), and tool hooks add `hooks` to their tool call. Electron and mobile show the `hook` row as a generic tool row. Before: hooks did not reach the timeline.

## Lessons

- Paseo callback hooks (`buildSubagentEffortHooks`): not proven. The raw `hook_started` log needs a real Claude turn, which is the deferred live probe. No rule tells them apart in the code, so all hooks stay. If the SDK emits events for the callbacks, the `Pre/PostToolUse` rows will show them; the UI checker should read the first live turn.
- Limit: a hook message has no tool call id, so a tool hook goes to the newest call of its tool. Parallel calls of one tool take one hook of an event each, newest first, so with two parallel calls the hook order can be swapped. A subagent's `PostToolUse:Bash` can attach to an older main-thread Bash call.
- `rtk proxy` is needed to see `console.log` and the lint output in agent shells; plain `npx vitest` and `npm run lint` hide them.
- The lint rule `max-nested-callbacks` (3) forced the new stream tests to sit directly in the existing `describe`, with named helper functions.

## Live Probe

Run by the lead on 2026-10-27 at 01:27, on live (6768) with commit 002d06e34. Agent `cd36194f` (claude-haiku-5-5) ran `ls` once in `/tmp/wt-probe`. Rows read from `.dev/paseo-home/timeline-cache.sqlite`.

- The final Bash tool call item carries 3 `PreToolUse:Bash` and 3 `PostToolUse:Bash` hooks, all with `status: "success"`, `exitCode: 0`, and output. One output is the RTK rewrite reason. PASS.
- Non-tool hooks became `hook` rows: 3 `SessionStart:startup`, 6 `UserPromptSubmit`, and 10 `Stop` (the Stop hooks ran twice, because a Stop hook asked for more text). PASS.
- No hook named after a Paseo callback showed. The 3 `PreToolUse:Bash` hooks match the owner's own hooks. PASS, with the limit that the SDK gives no field that marks a callback.
- One `Stop` hook was re-emitted as `cancelled` at turn end, then as `success` when its late response came. The last row wins by `callId`, so the final state is right.
- Observation: one short turn added 19 `hook` rows to the timeline, because the owner has many non-tool hooks.
