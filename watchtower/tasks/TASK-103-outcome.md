# TASK-103 Outcome

## Status

BLOCKED

Blocked: the running row shows, but the amber waiting row never shows: a builder that waits for a permission is drawn as running ([Q-041](watchtower/QUESTIONS.md)).

## Setup

- 2026-10-14, 05:45Z to 05:54Z, on live (daemon 6768, Electron with CDP on port 9223), plugin reloaded after commits 08dd9210e and 47daaecc0.
- Fixture `/tmp/wt-fixture-103` (`/tmp/wt-fixture` already held an older fixture, so it was left alone): `watchtower/NEXT.md` with TASK-001 DONE, TASK-002 IN PROGRESS, TASK-003 TODO, and two commits, one naming `(TASK-001)`. No `RUN.md`.
- Live workspace `wks_7e50d57cb45cea91`. Claude Haiku agents: "Fixture lead"; "build B (running)" labelled `watchtower.task=TASK-002` with `paseo.parent-agent-id` set to the lead; "build B (waiting)" with the same labels in mode `default`, stopped on a Bash permission (`paseo permit ls` listed it).

## Results

- Running row -> PASS. Screenshot `/tmp/fleet/wt7-running.png`: TASK-002 has a spinner, "Builder B, active 0s ago", and `1m / 0m`.
- Git run timing (TASK-100) -> PASS. "Autorun running, runner implement (git), started 12:51", and the pace line shows `0m`, `~12:52`, `1`, `TASK-002 +1m`. The average is 0m because the fixture's TASK-001 commit came 2 s after the plan commit.
- Rail bar (TASK-101) -> PASS. Three segments and "1 done · 1 running · 1 not started".
- Waiting row -> FAIL. Screenshot `/tmp/fleet/wt7-waiting.png`: with "build B (waiting)" blocked on a permission, TASK-002 still shows the spinner and "Builder B, active 5s ago"; Needs you says "Nothing waits on you". Likely cause: `client/builders.ts:40` marks `waiting` only when `requiresAttention` and `attentionReason` are `permission`, and a pending permission does not set them. `PaseoAgent` carries `pendingPermissions`, which the sidebar uses for its needs-input bucket.
- Wave header -> finding. Wave 2 says "ready to start" while its TASK is running.

## Anti-goal

- Agents on live after the check: all three archived (`npm run cli -- ls -g` shows none). Source: CLI, 2026-10-14T05:53:52Z.
- Agents on release: none match (`PASEO_HOME=~/.bachuc paseo ls -g`). Source: CLI, 2026-10-14T05:53:52Z.
- Result: PASS.

## Lessons

- A pending permission does not show through `requiresAttention` in the plugin's agent list; read `pendingPermissions`.
- `paseo run --mode default` with Claude Haiku and a Bash prompt is a quick way to make an agent wait for a permission on live.
