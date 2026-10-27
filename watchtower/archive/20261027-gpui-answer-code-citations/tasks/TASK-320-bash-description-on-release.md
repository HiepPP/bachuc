# TASK-320 Bash description reaches the stored timeline

Group: E (Claude provider and timeline merge in `packages/server`; standalone files)
Class: risky

## Brief

Goal: A Claude Bash call on a running daemon stores its `description` on the shell detail, so the GPUI step list shows it (TASK-316 goal).

Change: the mapper keeps `description` in tests, but the stored rows on release have none -> every stored shell detail of a call with a description keeps it.

Boundaries:

- Measured on 2026-10-27: `/Applications/Bachuc.app` installed at 17:36 contains the TASK-316 code (`tool-call-detail-primitives.js` lines 36-42 and 629 in `app.asar`). The release daemon started at 17:36 stored 76 shell rows after it, and 0 have `detail.description`, also for Bash calls that sent one.
- Hypothesis 1: the running event is built from partial streamed input, which has `command` before `description`, and no later event carries the full input.
- Hypothesis 2: the completed event is built without the input, and `mergeToolCallDetail` in `timeline-projection.ts` replaces the running detail with it.
- Code facts (Builder Check): `handleToolInputDelta` in `agent.ts` pushes a running call from partial JSON, so a partial input with `command` and no `description` is emitted; `mergeToolCallDetail` returns `incoming` whole unless one side is `unknown`.
- Test both hypotheses on real events before you change code. Planner call: build the replay test from the event order in `agent.ts` (partial deltas, then completion), not from a captured log. Do not change the protocol or `toShellToolDetail` again unless the evidence points there.
- Follows [docs/protocol-compatibility.md](docs/protocol-compatibility.md): `description` stays optional.

How:

- Find the event path of a Claude Bash call in `agent.ts` from `tool_use` start to completion, and the merge in `timeline-projection.ts`.
- Fix the place that loses `description`, so the stored and the projected detail both keep it.

Files:

- [packages/server/src/server/agent/providers/claude/agent.ts](packages/server/src/server/agent/providers/claude/agent.ts) (only if the cause is there)
- [packages/server/src/server/agent/timeline-projection.ts](packages/server/src/server/agent/timeline-projection.ts) (only if the cause is there)
- The test file next to the changed source, with a replay test of a Bash call with a description and one without.

Expected result:

- WHEN Claude runs Bash with `description` THE stored timeline row of the call SHALL have `detail.description`.
- WHEN the call completes THE projected tool call SHALL keep `detail.description`.
- IF a Bash call has no description THEN THE stored detail SHALL equal today's detail.

Anti-goal: existing expected values in the changed test files stay unchanged (0 edited); tripwire; read with `git diff` before the change and at completion.

Prompt:

```text
Use /solve to find why a Claude Bash description is missing from stored shell details on release, then fix it with a test that replays the real event order.
```

## Verify

- `npx vitest run <changed test file> --bail=1` -> pass, including the replay test.
- `npm run build:server` then `npm run typecheck` -> pass.
- Owner check after `scripts/bachuc-release.sh`: `sqlite3 -readonly ~/.bachuc/timeline-cache.sqlite` shows `detail.description` on new Bash rows, and the GPUI step list shows the description.
