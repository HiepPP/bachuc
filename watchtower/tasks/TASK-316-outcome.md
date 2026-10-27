# TASK-316 Outcome

## Outcome

Status: DONE (implement: manual checks pending, Q-200)

Changed:

- `ToolShellInputSchema` keeps a trimmed non-empty `description`, and `toShellToolDetail` writes it.
- The protocol shell detail has an optional `description`.
- `presentation` in `tools.rs` shows a non-empty description as the shell summary and falls back to the command.

Contract:

- A shell detail without a description is unchanged. Codex and OpenCode shell details are unchanged.

Verified:

- `npx vitest run packages/server/src/server/agent/providers/claude/tool-call-mapper.test.ts` -> 19 passed, with 2 new cases.
- `npx vitest run packages/protocol/src/messages.wire-compat.test.ts` -> 18 passed.
- Codex and OpenCode `tool-call-mapper.test.ts` -> 45 and 19 passed.
- `./cargo.sh test -p bachuc-timeline --test tools` -> 15 passed, with 2 new cases.
- `npm run build:server`, `npm run typecheck`, and `npm run lint` on the 3 TS files -> exit 0.
- Screen check on live after a daemon restart -> pending.

## UI Checks

- 2026-10-27 16:58: still UNVERIFIED. The release daemon on 6767 is PID 63844, started 2026-10-26 11:31, so it runs the old server code; `~/.bachuc/timeline-cache.sqlite` holds 0 shell details with `description`, and the daily GPUI app shows a raw BASH command (Q-200).
- Live screen check: UNVERIFIED (unattended). The run may not restart live, and the new server code needs a restart (Q-200).

Anti-goal:

- Final: `git diff -U0` on the server mapper tests and `tests/tools.rs` shows 0 removed lines; 2026-10-27.
- Result: PASS against 0 edited expected values.

Lessons:

- The protocol outbound validator `ws-outbound.aot.ts` is generated and git-ignored; `npm run typecheck` regenerates it.
