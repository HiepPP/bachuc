# TASK-002 Outcome

## Outcome

Status: DONE

Changed:

- [packages/server/src/server/agent/agent-prompt.ts](packages/server/src/server/agent/agent-prompt.ts): finish notices go into one mailbox per caller and wake mode. The mailbox flushes `FINISH_NOTICE_WINDOW_MS` (1500 ms) after its first notice, as one system prompt with one section per child. A permission notice flushes at once and skips the `settled_only` hold. `settled_only` waits until the caller is not running. A duplicate child and reason in one window keeps the newest section. `disarmFinishNotifications` also clears mailboxes, timers, and settle listeners. New optional setup params: `wake` and `coalesceWindowMs`.
- [packages/server/src/server/agent/tools/paseo-tools.ts](packages/server/src/server/agent/tools/paseo-tools.ts) and [packages/server/src/server/agent/create-agent/create.ts](packages/server/src/server/agent/create-agent/create.ts): optional `wake: "always" | "settled_only"` on agent-scoped `create_agent` and `send_agent_prompt`, passed to `setupFinishNotification`.
- Tests: [packages/server/src/server/agent/agent-prompt.test.ts](packages/server/src/server/agent/agent-prompt.test.ts) (new mailbox scenario), [packages/server/src/server/agent/create-agent/create.test.ts](packages/server/src/server/agent/create-agent/create.test.ts) (wake pass-through), [packages/server/src/server/agent/mcp-server.test.ts](packages/server/src/server/agent/mcp-server.test.ts) (two finish-notice waits allow for the window).
- [docs/agent-lifecycle.md](docs/agent-lifecycle.md): Relationships describes the mailbox and `wake`.

Contract:

- Without `wake`, notices still steer into a running caller turn, now batched.
- Q-001 answered: a 1500 ms window, steer by default, and opt-in `settled_only`.
- No deterministic `messageId` per envelope. This deviates from the spec on purpose: nothing retries a send, and a fixed id could make the timeline drop a real repeat notice. The reviewer agreed.
- The mailbox lives in memory and does not survive a daemon restart.
- TASK-001 behavior is kept: Stop clears armed, in-flight, and mailbox notices.

Verified:

- `npx vitest run packages/server/src/server/agent/agent-prompt.test.ts` -> 28 passed (batching, separate notices, permission skip, settled_only hold and release, permission under settled_only, re-arm replaces the watch, dedupe, disarm of a pending and of a held notice).
- `npx vitest run packages/server/src/server/agent/create-agent/create.test.ts` -> 11 passed.
- `npx vitest run packages/server/src/server/agent/lifecycle-command.test.ts` -> 13 passed.
- `npx vitest run packages/server/src/server/agent/mcp-server.test.ts` -> 121 passed. The two finish-notice tests passed 3 runs in a row.
- `npm run typecheck` -> exit 0. `npm run lint` and `npm run format:check:files` on the changed files -> exit 0.
- Reviewer agent (profile reviewer) -> `VERDICT: ISSUES`, no blockers. Fixed: permission under `settled_only`, a re-arm test, dead `sending` chain, a re-check after a failed caller turn start, the top-level `wake` label, and two test gaps. Not done: an MCP-level `wake` pass-through test; the command-level test covers it.
- Manual live check (3 quick subagents give one combined notice) -> UNVERIFIED (autonomous run).

Anti-goal:

- Before changes: 0 ms; source read of `notify()` at `da7a1c2`, which called `sendPromptToAgent` with no timer; 2026-10-06 23:10.
- After the mailbox change: 0 ms; test "a permission notice skips the coalescing window" passed with no timer advance; 2026-10-06 23:18.
- After the review fixes: 0 ms; same test plus "settled_only still sends a permission notice to a running caller at once"; 2026-10-06 23:26.
- Final: 0 ms; same tests; 2026-10-06 23:26.
- Result: PASS against the 0 ms tripwire.

Lessons:

- `vi.waitFor` times out after 1000 ms by default. A test that waits for a finish notice must allow `FINISH_NOTICE_WINDOW_MS`. Source: two `mcp-server.test.ts` failures at about 1014 ms.
- A fake timer created before `vi.useRealTimers()` never fires afterwards. Keep fake timers on through `vi.waitFor`, which advances them on each check. Source: the "blocking agent-scoped prompt outlasts the wait" test.
- TASK-003 holds the queue on Stop in `cancelAgentRunCommand`; keep its two disarm calls in place. Source: [packages/server/src/server/agent/lifecycle-command.ts](packages/server/src/server/agent/lifecycle-command.ts).
