# TASK-002 Coalesce child finish notices

Group: A (shared file with TASK-001 and TASK-003: [packages/server/src/server/agent/agent-prompt.ts](packages/server/src/server/agent/agent-prompt.ts))
Class: risky

## Brief

Goal: Children that finish close together reach the parent as one system message. A parent can opt in to get notices only after its own run settles.

Change: one steer prompt per finished child -> one envelope per parent per window, with an optional settled-only delivery.

Design: T3 Code batches wakes in `apps/server/src/orchestration-v2/Orchestrator.ts:9347-9399` and `apps/server/src/orchestration-v2/NotificationMailbox.ts:7-26`. The `completionWake` values are in `packages/contracts/src/orchestrationV2.ts:3010-3034`.

Boundaries:

- Q-001 answered (default): a 1500 ms window. Keep steer delivery by default. Add an optional `wake: "always" | "settled_only"` input to MCP `create_agent` and `send_agent_prompt`. `settled_only` waits until the parent is idle and has no in-flight run.
- A "needs permission" notice skips the window, because it blocks the child.
- Dedupe entries by child, reason, and permission request id. Give each envelope a deterministic `messageId`. The timeline already dedupes on it ([packages/server/src/server/agent/agent-manager.ts](packages/server/src/server/agent/agent-manager.ts), line 4749).
- The mailbox lives in memory, like the armed map today. It does not survive a daemon restart.
- `disarmFinishNotifications` from TASK-001 must also clear pending mailbox entries.
- Keep the body format of `formatFinishNotificationBody`: status line, optional permission JSON, last reply cut at 4000 chars.

How:

- Run GitNexus `impact` on `setupFinishNotification` and `sendPromptToAgent`.
- Add a per-caller mailbox next to `armedFinishNotifications` in `agent-prompt.ts`.
- Change `notify` so it adds an entry and schedules a flush, instead of sending at once.
- Flush one envelope with one section per child.
- For `settled_only`, flush when the parent reaches idle with no in-flight run.
- Add the `wake` input to the `create_agent` and `send_agent_prompt` schemas in [packages/server/src/server/agent/tools/paseo-tools.ts](packages/server/src/server/agent/tools/paseo-tools.ts). Pass it through [packages/server/src/server/agent/create-agent/create.ts](packages/server/src/server/agent/create-agent/create.ts).
- Add tests. Describe the behavior in [docs/agent-lifecycle.md](docs/agent-lifecycle.md) under Relationships.

Files:

- [packages/server/src/server/agent/agent-prompt.ts](packages/server/src/server/agent/agent-prompt.ts) (mailbox and flush)
- [packages/server/src/server/agent/agent-prompt.test.ts](packages/server/src/server/agent/agent-prompt.test.ts) (coalesce, permission, settled-only, dedupe tests)
- [packages/server/src/server/agent/tools/paseo-tools.ts](packages/server/src/server/agent/tools/paseo-tools.ts) (`wake` input)
- [packages/server/src/server/agent/create-agent/create.ts](packages/server/src/server/agent/create-agent/create.ts) (pass `wake`)
- [packages/server/src/server/agent/create-agent/create.test.ts](packages/server/src/server/agent/create-agent/create.test.ts) (pass-through test)
- [docs/agent-lifecycle.md](docs/agent-lifecycle.md) (finish notice behavior)

Expected result:

- 3 children finish within 1500 ms -> the parent gets 1 prompt with 3 sections.
- 2 children finish 5 s apart -> the parent gets 2 prompts.
- A child asks for permission -> the notice goes out at once, with no window delay.
- `settled_only` and a running parent -> no prompt until the parent is idle, then 1 prompt.
- The same child and reason reported twice -> 1 section.
- Stop on the parent (TASK-001) -> pending entries are dropped and never sent.
- No `wake` input -> today's steer behavior, now batched.

Anti-goal: The delay added to a "needs permission" notice stays at 0 ms; tripwire; read from a fake-timer test in `agent-prompt.test.ts` before changes (current behavior), after the mailbox change, and at completion.

## Verify

- `npx vitest run packages/server/src/server/agent/agent-prompt.test.ts --bail=1` -> pass, with the new cases.
- `npx vitest run packages/server/src/server/agent/create-agent/create.test.ts --bail=1` -> pass.
- `npm run typecheck` -> exit 0.
- `npm run lint -- packages/server/src/server/agent/agent-prompt.ts packages/server/src/server/agent/tools/paseo-tools.ts packages/server/src/server/agent/create-agent/create.ts` -> exit 0.
- Permission test with fake timers -> the notice is sent before any timer advance (anti-goal read).
- Manual live check (needs a human and the live daemon): a live agent starts 3 quick subagents with `notifyOnFinish` -> its timeline shows one combined notice.
