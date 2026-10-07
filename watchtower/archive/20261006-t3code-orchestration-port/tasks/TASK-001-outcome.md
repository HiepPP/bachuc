# TASK-001 Outcome

## Outcome

Status: DONE

Changed:

- [packages/server/src/server/agent/agent-prompt.ts](packages/server/src/server/agent/agent-prompt.ts): the armed map stores the caller id. New `disarmFinishNotifications({ agentManager, callerAgentIds })` stops armed notices and cancels in-flight deliveries for those callers. `notify()` skips the send when its delivery was cancelled.
- [packages/server/src/server/agent/lifecycle-command.ts](packages/server/src/server/agent/lifecycle-command.ts): `LifecycleAgentManager` gets `listAgents()`. `cancelAgentRunCommand` disarms the subtree, cancels running live descendants deepest first, cancels the parent, then disarms the subtree again in `finally`.
- [packages/server/src/server/agent/agent-prompt.test.ts](packages/server/src/server/agent/agent-prompt.test.ts) and [packages/server/src/server/agent/lifecycle-command.test.ts](packages/server/src/server/agent/lifecycle-command.test.ts): new tests.
- [docs/agent-lifecycle.md](docs/agent-lifecycle.md): the Cancellation section describes the cascade.

Contract:

- Stop (app, MCP `cancel_agent`, Hub interrupt) cancels the running managed subtree, deepest first, then the parent. It also runs when the parent is idle.
- No notice armed or in flight for the stopped agent or its descendants reaches them after Stop.
- `AgentManager.cancelAgentRun` is unchanged, so reload, replace, rewind, and archive do not cascade.
- A failed descendant cancel is logged and does not block the parent.
- The subtree loses its armed notices even when a cancel fails. This is documented (review finding 3, kept by choice).

Verified:

- `npx vitest run packages/server/src/server/agent/lifecycle-command.test.ts --bail=1` -> 13 passed (cascade order, idle parent, label cycle, failed child cancel).
- `npx vitest run packages/server/src/server/agent/agent-prompt.test.ts --bail=1` -> 19 passed (disarm, end-to-end Stop through `cancelAgentRunCommand`, in-flight delivery race and its control).
- `npm run typecheck` -> exit 0.
- `npm run lint` and `npm run format:check:files` on the 4 changed source files -> exit 0.
- GitNexus impact before edits: `cancelAgentRunCommand` HIGH (3 direct callers), `setupFinishNotification` CRITICAL (45 upstream). The notice logic was kept; only its registry and a cancel check were added.
- Reviewer agent (profile reviewer) -> `VERDICT: ISSUES` with 2 races and a test gap. All three were fixed and tested. Its check of the 3 real callers found that each passes the real `AgentManager`, which has `listAgents()`.
- Manual live check (Stop on a parent with a running subagent) -> UNVERIFIED (autonomous run).

Anti-goal:

- Before changes: 18 passed, 0 failed; `npx vitest run packages/server/src/server/agent/agent-manager.test.ts -t "cascade|cancel" --bail=1`; 2026-10-06 22:54.
- After the cascade change: 18 passed, 0 failed; same command; 2026-10-06 22:56.
- After the review fixes: 18 passed, 0 failed; same command; 2026-10-06 23:03.
- Final: 18 passed, 0 failed; same command; 2026-10-06 23:03.
- Result: PASS against the tripwire of any failure.

Lessons:

- In zsh, `F="a b"; cmd $F` passes one argument. Use an array, `F=(a b); cmd $F`, for file lists. Source: this run's first `format:files` call failed with "Expected at least one target file".
- The lefthook pre-commit hook checks the format of staged `watchtower/*.md` files. The formatter turns a bare `**` glob into emphasis, so put globs in code spans. Source: [lefthook.yml](lefthook.yml) and the ADR-0002 Scope line.
- TASK-002 must keep the in-flight delivery cancel when it replaces `notify()` with a mailbox. Source: the test "disarm cancels a notice whose delivery is already in flight".
