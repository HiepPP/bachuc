# TASK-320 Outcome

## Outcome

Status: DONE (no code change)

Changed:

- Nothing. The spec premise was false.

Verified:

- The raw Claude transcripts show that no Bash call after the 17:36 release sent a `description` in its input: 0 of 11 in agent `dd136da8`, and this agent's calls sent only `command`. So "0 of 76 stored rows have a description" was not a bug.
- Probe on release, 2026-10-27 17:58: a Bash call with `description: "Print a probe line for the description check"` stored it on all 3 rows (seq 2357 and 2358 running, 2359 completed with output) in `~/.bachuc/timeline-cache.sqlite`.
- The built mapper in `packages/server/dist` returns `{"type":"shell","command":"ls","description":"List files"}` for a Bash input with a description.

Anti-goal:

- Final: no test file changed; 2026-10-27.
- Result: PASS.

Lessons:

- Before calling a missing field a bug, check the raw provider input (the Claude transcript JSONL) for that field.
