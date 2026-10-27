# TASK-316 Keep the shell description on the wire

Group: A (protocol shell detail, the shared server shell mapper, and the GPUI tool presentation; no file shared with other groups)
Class: code

## Brief

Goal: A Claude `Bash` call keeps its `description` input, and the GPUI step list shows that description as the call summary instead of the raw command.

Change: the shell detail is `{command, cwd, output, exitCode}` -> it also carries an optional `description` when the tool input has one.

Boundaries:

- Follows the protocol contract in [docs/protocol-compatibility.md](docs/protocol-compatibility.md): the new field is optional, and no field is narrowed or removed. No `.transform()` in the wire schema.
- Measured on 2026-10-27: no stored shell detail in `~/.bachuc/timeline-cache.sqlite` or `.dev/paseo-home/timeline-cache.sqlite` has a `description` key. Old rows stay without it; GPUI falls back to the command.
- Q-198 took the default: add the field.
- Trap: `ToolShellInputSchema` in `tool-call-detail-primitives.ts`: its `.transform()` builds a new object and drops every passthrough key, so `description` is lost there before `toShellToolDetail` runs.
- Trap: the shell schema is shared by Claude, Codex, and OpenCode parsers. Read `description` only when it is a non-empty string, so the other providers keep their current output.

How:

- In `presentation` in `tools.rs`, use a non-empty `detail.description` as the shell summary; keep the command when it is missing. The expanded badge still shows the command and output.
- Add `description: z.string().optional()` to the `shell` branch of the tool detail union in `messages.ts`.
- Carry a trimmed non-empty `description` through `ToolShellInputSchema` and write it in `toShellToolDetail`.

Files:

- [packages/protocol/src/messages.ts](packages/protocol/src/messages.ts) (optional `description` on the shell detail)
- [packages/server/src/server/agent/providers/tool-call-detail-primitives.ts](packages/server/src/server/agent/providers/tool-call-detail-primitives.ts) (keep `description` through the input schema and the detail builder)
- [packages/server/src/server/agent/providers/claude/tool-call-mapper.test.ts](packages/server/src/server/agent/providers/claude/tool-call-mapper.test.ts) (tests)
- [packages/desktop-gpui/crates/timeline/src/tools.rs](packages/desktop-gpui/crates/timeline/src/tools.rs) (shell summary reads `description`)
- [packages/desktop-gpui/crates/timeline/tests/tools.rs](packages/desktop-gpui/crates/timeline/tests/tools.rs) (tests)

Expected result:

- WHEN a Claude `Bash` call has input `{command: "git status", description: "Show working tree status"}` THE mapper SHALL produce a shell detail with `description: "Show working tree status"`.
- IF the input has no `description`, or an empty one, THEN THE mapper SHALL produce the same shell detail as before, with no `description` key.
- WHEN GPUI shows a shell call whose detail has a non-empty `description` THE badge summary SHALL show the description.
- IF the shell detail has no `description` THEN THE badge summary SHALL show the command, as before.
- WHEN an old client parses a shell detail with `description` THE protocol schema SHALL accept it.

Red tests:

File: [packages/server/src/server/agent/providers/claude/tool-call-mapper.test.ts](packages/server/src/server/agent/providers/claude/tool-call-mapper.test.ts)

The builder writes the two cases in the style of the existing Bash cases in this file: one with `description` that expects the field, and one without it that expects the exact old detail object.

File: [packages/desktop-gpui/crates/timeline/tests/tools.rs](packages/desktop-gpui/crates/timeline/tests/tools.rs)

The builder adds two `presentation` cases in the style of the existing shell cases: with `description` the summary is the description; without it the summary is the command. The builder checks the `Presentation` field name for the summary in `tools.rs`.

Anti-goal: the existing expected values in `tool-call-mapper.test.ts` and the Codex and OpenCode parser tests stay unchanged (0 edited expected values); tripwire; read before the change and at completion with `git diff` on those test files.

## Verify

- `npx vitest run packages/server/src/server/agent/providers/claude/tool-call-mapper.test.ts --bail=1` -> pass, including the 2 new cases.
- `npx vitest run packages/protocol/src/messages.wire-compat.test.ts --bail=1` -> pass.
- `npm run build:server` then `npm run typecheck` -> pass.
- `npm run lint -- packages/protocol/src/messages.ts packages/server/src/server/agent/providers/tool-call-detail-primitives.ts` -> pass.
- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-timeline --test tools` -> pass, including the 2 new cases.
- `git diff -U0 -- 'packages/server/src/server/agent/providers/**/*.test.ts' packages/desktop-gpui/crates/timeline/tests/tools.rs | rg '^-\s'` -> no removed expected value lines (anti-goal).
- Screen check on live after a daemon restart (human or UI checker): a new Claude Bash call shows its description in the step list; an old call shows its command.
