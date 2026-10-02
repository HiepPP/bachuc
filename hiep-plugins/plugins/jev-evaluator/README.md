# Jev evaluator for Paseo

Expose Typesafe AI’s direct `jev-latest` Evaluation API as the MCP tool `jev_evaluate`.
New Paseo Codex and Claude agents, including subagents created through Paseo's `create_agent`,
receive the tool while enabled. Provider-internal subagents are not covered by this hook.
Use your normal Codex or Claude provider to run the agent. Jev evaluates decisions; it cannot
run a coding-agent session through the Responses API.

## Install

Requires Node 22+, Paseo 0.8+, enabled trusted plugins, and a Typesafe AI key stored as
`{ "apiKey": "<your Typesafe key>" }` in `$PASEO_HOME/typesafe-ai.json`
(default `~/.paseo/typesafe-ai.json`, permissions `0600`). Requests go directly to
`https://api.typesafe.ai/v1/systemone`; no Gateway fallback.
The MCP subprocess reads that file; the key is not copied into agent prompts or MCP configuration.
Paseo uses its own Node executable, including the Electron helper on desktop.
Run `npm run smoke` for a real, billable evaluation of synthetic state through MCP.

```sh
cd hiep-plugins/plugins/jev-evaluator   # from the HiepPP/paseo checkout
npm ci
npm run typecheck
npm run lint
npm test
PASEO_HOME=~/.paseo-dev paseo plugin install "$PWD" --id jev-evaluator   # release
PASEO_HOME=~/.paseo-dev paseo plugin ls jev-evaluator --json
```

Keep this directory and its dependencies available on the daemon machine. Install under the exact
ID `jev-evaluator`; the plugin resolves its MCP entry from that directory-source configuration.
No global Codex or Claude configuration changes are needed. Existing agents do not gain the tool
retroactively: create a new agent after installation. Existing MCP entries named `jev` are preserved.

## Delegate an evaluation

Ask the main agent:

> Create a subagent using the normal Codex provider. Ask it to call `jev_evaluate` with the task
> state and typed questions below, then return Jev's answers and usage to me. Do not substitute
> its own judgment if the tool fails.

Example tool arguments:

```json
{
  "state": "The backend change passed its focused tests. A database migration still needs review.",
  "questions": {
    "nextStep": {
      "type": "choice",
      "instructions": "Choose the next action based on the evidence.",
      "criteria": {
        "review": "Review the pending database migration.",
        "finish": "All required work and checks are complete."
      }
    },
    "complete": {
      "type": "boolean",
      "instructions": "Is all required work complete?"
    },
    "risk": {
      "type": "score",
      "instructions": "Score the remaining release risk.",
      "criteria": ["No unresolved risk", "Review needed", "Known unsafe change"]
    }
  }
}
```

Answers keep question IDs. Boolean answers contain a probability; choice answers contain a selected
key; scores index the ordered rubric and may be fractional. Let the main agent interpret the result
and decide what to delegate. The tool never starts agents, approves permissions, or changes files.
Probabilities are model judgments, not proof that a check passed.

Requests have a 45-second timeout and no automatic retries. Failures remain tool errors.
Only send state you intend to share with Vercel. The wrapper sends no workspace files automatically.

## Update or disable

After source edits, run the checks above and `PASEO_HOME=~/.paseo-dev paseo plugin reload jev-evaluator` (release), then `npm run cli -- plugin reload jev-evaluator` from the repo root (live).
Run `PASEO_HOME=~/.paseo-dev paseo plugin disable jev-evaluator` to stop injecting the tool into new agents.
Previously created agents retain their stored MCP entry; archive them when no longer needed.
No daemon restart is required.

API contract: [AI SDK evaluation](https://ai-sdk.dev/docs/ai-sdk-core/evaluation).

## Known-answer benchmark

`npm run benchmark` sends the fixed synthetic cases in `benchmark/cases.json` through the same
MCP entry. Expected answers and their proofs are kept locally and never sent to Jev. Review
`benchmark/labels.md` before changing a label; freeze labels before each run.

Requests are spaced 25 seconds apart. Only a `429` gets one retry after 70 seconds; model answers
are never retried. The runner saves a dataset snapshot/hash, every attempt, metrics, and a Markdown
report under this plugin’s `artifacts/jev-benchmark-<timestamp>/` directory. These reports contain
synthetic inputs and answers, not credentials. Run `npm run test:benchmark` to check the scorer.
To resume an interrupted run, pass its absolute artifact directory to `npm run benchmark -- <directory>`.
Resume verifies the saved dataset/runtime hashes, skips answered batches, retains prior failures,
and spaces remaining requests 65 seconds apart. It never retries an answer to improve the score.

Report choice accuracy, boolean accuracy/Brier, and ordinal score error separately. Option
probabilities and TypeSafe confidence are different fields. Results from this small, correlated,
synthetic set are not production calibration or a universal reliability estimate.

## Optional task-aware preflight

Install [workspace-preflight](../workspace-preflight/README.md) independently to give new Paseo
Codex/Claude agents `workspace_preflight` alongside `jev_evaluate`. Neither plugin imports the other.
Collect task requirements, call preflight with `{}`, preserve raw measurements, then send only
sanitized necessary evidence to Jev. Use a `nextStep` question with `type: "choice"` and criteria:

```json
{
  "proceed": "All task-critical prerequisites are evidenced; unrelated failures may remain.",
  "prepare_environment": "A known failure blocks a task-critical prerequisite.",
  "need_more_evidence": "Required evidence or task coverage is missing or unknown."
}
```

Include required check IDs, their task relevance and coverage gaps in shared `state` alongside
sanitized observations. Never put credentials, unrelated repository text or private paths in state.
Jev cannot modify measurements, execute repairs or approve permissions. Missing tool, timeout,
API error or invalid choice means evaluation unavailable, never an implicit `proceed`.
The preflight panel remains usable without Jev; no per-turn evaluation loop is installed.
