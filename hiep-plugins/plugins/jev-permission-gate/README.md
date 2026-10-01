# Jev permission gate

Answers shell permission requests for Paseo agents before they reach the user.
Regex handles the certain cases; Jev classifies the rest; anything uncertain stays with the user.
It uses the `agent.permission` before hook, so an answered request never shows a prompt or sends a
push notification. Requires the Paseo fork `>=0.10.2-beta.900`; the stable 0.10.1 host refuses it.
While the gate decides, other events of that agent wait, up to the 25-second budget.

Order per `agent.permission` request with a shell command:

1. Regex deny: `rm -rf`, `git push`, `git reset --hard`, `git checkout --`, `git clean -f`, `sudo`,
   pipe into a shell, `find -delete`, secret files (`.env`, `*.pem`, `credentials`, `typesafe-ai.json`).
2. Regex escalate: quotes, escapes, wildcards, expansions, or embedded newlines require human
   review; raw command text cannot establish their filesystem targets. This also applies to
   benign quoted arguments. These commands never reach Jev for automatic approval.
   An interpreter running a script file also escalates: its contents are invisible to the gate.
3. Regex allow: a single non-compound `ls/cat/head/tail/wc/rg/grep/find/pwd/which/stat`, a
   read-only `git` subcommand, or exactly `npm test`, `npm run typecheck|lint|check`, `npx tsc --noEmit`.
4. Jev: boolean `readOnly` plus choice `allow | deny | escalate`. Allow needs both at
   probability >= 0.9; deny needs >= 0.9; otherwise no answer and the user decides.
5. Jev timeout, API error, or invalid answer: no answer, user decides.

Non-tool requests (plan, question, mode) and tools without a shell command are ignored.

Decisions append to `$PASEO_HOME/plugin-data/jev-permission-gate/decisions.jsonl` (0600) with a
command hash, decision, source, probabilities, and timing; command text is not stored. Only the
command, tool name, and workspace-relative cwd are sent directly to Typesafe AI (`https://api.typesafe.ai/v1/systemone`, `jev-latest`).
The server reads `{ "apiKey": "<your Typesafe key>" }` from `$PASEO_HOME/typesafe-ai.json`
(default `~/.paseo/typesafe-ai.json`, mode `0600`). No Gateway fallback.
The worker sends the request through `../jev-evaluator/server/typesafe.mjs`, so `jev-evaluator`
must sit beside this plugin with its dependencies installed.
Confidence thresholds are unchanged. Direct access to `typesafe-ai.json` is denied before
the read-only allowlist or Jev evaluation.

Evidence: 30 labeled shell commands scored 30/30 in
`plugins/jev-evaluator/artifacts/jev-benchmark-2026-09-22T11-51-03-364Z/report.md`, a synthetic set
with the policy stated in the state. Real workloads are not measured. Without the command list context of that benchmark, Jev rated
`npm test` allow at only 0.72 because script contents are unknown, so package scripts other than the
exact regex forms end with the user. Jev cannot see script
contents, heredocs, or the filesystem; the regex tiers stay in front of it.

## Install

```sh
npm ci
npm run typecheck
npm run lint
npm test
paseo plugin install "$PWD" --id jev-permission-gate
paseo plugin ls jev-permission-gate --json
```

Reload after edits with `paseo plugin reload jev-permission-gate`; disable with
`paseo plugin disable jev-permission-gate`. Hooks apply to every agent, including existing ones.
