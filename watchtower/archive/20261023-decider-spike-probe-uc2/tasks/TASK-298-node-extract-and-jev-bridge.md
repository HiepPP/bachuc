# TASK-298 Node extract and Jev bridge

Group: A (writes `packages/desktop-gpui/crates/decider-probe/scripts/**` and `questions.json`; shared file with no other group)
Class: code

## Brief

Goal: Build the event set for UC2 from real shell commands, with the regex baseline per event, and run hosted Jev on it.

Change: no probe data -> `events.jsonl`, `labels.draft.jsonl`, and `results-jev.jsonl` in `.dev/decider-probe/`.

Boundaries:

- Follows [ADR-0066](watchtower/decisions/ADR-0066-standalone-rust-decider-probe-over-jev-evaluator-backend.md): import `tier0`, `POLICY`, and `QUESTIONS` from [hiep-plugins/plugins/jev-permission-gate/server/policy.ts](hiep-plugins/plugins/jev-permission-gate/server/policy.ts); do not copy the regex.
- Read `~/.bachuc/timeline-cache.sqlite` with `node:sqlite` `DatabaseSync` and `{ readOnly: true }`. Node is v24.
- Sample deterministically: take the first row per distinct command (its `agent_id` and `timestamp` fill `meta`), sort by command text, dedupe, drop secrets and commands over 2,000 characters, then take 120 commands where `tier0` returns `null` and 30 where it returns a decision. Use seed `20261023` with a seeded Fisher-Yates shuffle (for example mulberry32), so a rerun gives the same set. If a pool is short, take all of it and print a warning with the counts.
- Secret filter: drop a command that matches `/(sk-[A-Za-z0-9]|ghp_|gho_|xox[bp]-|AKIA[0-9A-Z]|BEGIN [A-Z ]*PRIVATE|api[_-]?key\s*[=:]|token\s*[=:]|password\s*[=:]|Bearer\s)/i`.
- Trap: `tier0` in `policy.ts` returns `{ decision, reason } | null`. Write only `.decision` into `meta.tier0`.
- Resolve the data dir from `import.meta.url` to the repo root, not from the cwd, and create it when absent.
- Planner call: `cwd_relative` is `"."` and `tool` is `"Bash"` for every event. The timeline row has no workspace-relative cwd; most commands start with their own `cd`.
- Planner call: use `questions.json`, not YAML, so Rust needs no YAML crate.
- `jev-run.ts` maps `noul` back to `boolean` and sends both questions in one request, as `jev-worker.ts` does. It skips ids already in `results-jev.jsonl`. Jev's `readOnly` answer field `probability` is P(true), as the comment in `jev-worker.ts` says; write it as `p_true`. Jev `input_tokens` is `null` unless the result carries usage. It needs `$HOME/.bachuc/typesafe-ai.json`; pass `configFile` as `~/.bachuc/config.json`.
- Trap: `evaluateWithTypesafe` in [hiep-plugins/plugins/jev-evaluator/server/typesafe.mjs](hiep-plugins/plugins/jev-evaluator/server/typesafe.mjs): import it by file URL, as `jev-worker.ts` does. Its SDK resolves from `jev-evaluator/node_modules`, so that folder needs `npm ci` first.
- Trap: the AI SDK fails inside the plugin subprocess (comment in `jev-worker.ts`). Run the scripts with plain `node`, not through the daemon.

How:

- `scripts/extract.ts`: read the db path from env `PROBE_DB` (default `~/.bachuc/timeline-cache.sqlite`), query shell commands, filter, sample, write `events.jsonl` and `labels.draft.jsonl`. Pre-fill `label` with the `tier0` decision when it is not null, else `null`; pre-fill `read_only` with `null`.
- `scripts/lib.ts`: pure helpers `isSecret(command: string): boolean` and `sample<T>(items: T[], count: number, seed: number): T[]`.
- `scripts/jev-run.ts`: read `events.jsonl` and `questions.json`, call `evaluateWithTypesafe` per event in sequence, append one result line per event, and time each call with `performance.now()`.
- `questions.json`: version `uc2-v1`, the gate's `readOnly` and `action` questions with exact wording from `QUESTIONS`, and `readOnly.type` set to `noul`. Also hold `policy` set to the exact `POLICY` text.

Files:

- [packages/desktop-gpui/crates/decider-probe/scripts/extract.ts](packages/desktop-gpui/crates/decider-probe/scripts/extract.ts) (new)
- [packages/desktop-gpui/crates/decider-probe/scripts/jev-run.ts](packages/desktop-gpui/crates/decider-probe/scripts/jev-run.ts) (new)
- [packages/desktop-gpui/crates/decider-probe/scripts/lib.ts](packages/desktop-gpui/crates/decider-probe/scripts/lib.ts) (new)
- [packages/desktop-gpui/crates/decider-probe/scripts/lib.test.ts](packages/desktop-gpui/crates/decider-probe/scripts/lib.test.ts) (new)
- [packages/desktop-gpui/crates/decider-probe/questions.json](packages/desktop-gpui/crates/decider-probe/questions.json) (new)

Interfaces:

- Note: this TASK writes only `labels.draft.jsonl`, in the labels shape with `null` allowed. Hiep saves the final `labels.jsonl` (Q-190) and labels `read_only` apart from `label`.
- Produces for TASK-299 and TASK-300: `questions.json` = `{"version":"uc2-v1","policy":string,"questions":{"readOnly":{"type":"noul","instructions":string},"action":{"type":"choice","instructions":string,"criteria":{"allow":string,"deny":string,"escalate":string}}}}`
- Produces for TASK-299 and TASK-300: `.dev/decider-probe/events.jsonl`, one line per event = `{"id":"e001","uc":"uc2","state":{"policy":string,"command":string,"tool":"Bash","cwd_relative":"."},"meta":{"tier0":"allow"|"deny"|"escalate"|null,"agent_id":string,"ts":string}}`
- Produces for TASK-299 and TASK-300: `.dev/decider-probe/labels.jsonl`, one line per event = `{"id":"e001","uc":"uc2","label":"allow"|"deny"|"escalate","read_only":boolean}`.
- Produces for TASK-299 and TASK-300: result line = `{"backend":"decider"|"jev","id":"e001","question_id":"readOnly"|"action"|"both","question_version":"uc2-v1","input_tokens":number|null,"client_ms":number,"server_ms":number|null,"answers":{"readOnly"?:{"p_true":number},"action"?:{"choice":"allow"|"deny"|"escalate","probabilities":{"allow":number,"deny":number,"escalate":number}}},"error":string|null}`. Jev writes `.dev/decider-probe/results-jev.jsonl` with `question_id` `"both"` and `server_ms` `null`.

Expected result:

- WHEN `extract.ts` runs THE script SHALL write 150 events to `events.jsonl`: 120 with `meta.tier0` `null` and 30 with a decision.
- WHEN `extract.ts` runs twice THE script SHALL write byte-identical `events.jsonl` files.
- IF a command matches the secret filter or is longer than 2,000 characters THEN THE script SHALL drop it.
- WHEN `jev-run.ts` runs THE script SHALL append one result line per event that is not yet in `results-jev.jsonl`.
- IF a Jev call fails THEN THE script SHALL write a line with `error` set and `answers` `{}`, and go on.

Red tests:

File: [packages/desktop-gpui/crates/decider-probe/scripts/lib.test.ts](packages/desktop-gpui/crates/decider-probe/scripts/lib.test.ts)

```ts
import assert from "node:assert/strict";
import { test } from "node:test";
import { isSecret, sample } from "./lib.ts";

test("isSecret flags keys and tokens", () => {
  assert.equal(isSecret("curl -H 'Authorization: Bearer abc' x"), true);
  assert.equal(isSecret("export OPENAI_API_KEY=sk-abc123"), true);
  assert.equal(isSecret("git push https://ghp_abc@github.com/x"), true);
  assert.equal(isSecret("echo password=hunter2"), true);
});

test("isSecret passes ordinary commands", () => {
  assert.equal(isSecret("rg -n tokenize src"), false);
  assert.equal(isSecret("cargo test -p decider-probe"), false);
});

test("sample is deterministic and bounded", () => {
  const items = Array.from({ length: 50 }, (_, i) => i);
  const a = sample(items, 10, 42);
  assert.deepEqual(a, sample(items, 10, 42));
  assert.equal(a.length, 10);
  assert.equal(new Set(a).size, 10);
  assert.equal(sample(items, 80, 42).length, 50);
});
```

Anti-goal: the count of committed files that contain a command from `events.jsonl` stays 0; tripwire; read from `git status --short` and `git diff --cached --name-only` before commit and at completion.

## Verify

- `node --import ./hiep-plugins/plugins/jev-permission-gate/node_modules/tsx/dist/loader.mjs --test packages/desktop-gpui/crates/decider-probe/scripts/lib.test.ts` -> 3 tests pass.
- Run `extract.ts` with the same loader -> `wc -l .dev/decider-probe/events.jsonl` prints 150; `jq -s 'map(select(.meta.tier0==null))|length'` prints 120.
- Run `extract.ts` again and compare with `shasum` -> the hash is unchanged.
- `jq -r .state.command .dev/decider-probe/events.jsonl | rg -i 'sk-[a-z0-9]|ghp_|bearer |password\s*='` -> no match.
- `jq '.questions.readOnly.type' packages/desktop-gpui/crates/decider-probe/questions.json` -> `"noul"`; its `instructions` text equals `QUESTIONS.readOnly.instructions` in `policy.ts`.
- With `PROBE_DB` pointed at a temp sqlite that has one command over 2,000 characters, one secret command, one command with a newline and quotes, and too few rows for the pools -> the long and secret commands are absent, the newline command round-trips through `jq`, and a pool warning prints.
- Run `jev-run.ts` with an unreadable `typesafe-ai.json` path (env `PROBE_JEV_CONFIG=/nonexistent/config.json`, `PROBE_LIMIT=1`) -> one line with `error` set and `answers` `{}`; delete that line after the check.
- Run `jev-run.ts` on the first 3 events (env `PROBE_LIMIT=3`) -> 3 lines in `results-jev.jsonl` with `backend` `"jev"`; a second run adds 0 lines.
- Anti-goal: `git status --short` -> no path under `.dev/`.
