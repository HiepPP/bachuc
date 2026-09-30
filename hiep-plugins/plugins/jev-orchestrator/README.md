# Jev orchestrator

Delegate bounded tasks to real Paseo children using saved profiles, Jev routing,
and independently executed checks. Requires Paseo daemon/client 0.8.0 and Node 22+.
No Paseo application source changes or worktree creation.

**Codex native v2 uses `prepare_native_delegate` before native spawn:** Jev evaluates the plaintext
task once; a scoped, one-use ticket pins the child model/effort. Initial scope: fresh Paseo parent,
`fork_turns:none`, three non-reusable tickets per session. Calls without a valid ticket are denied.

The [native subagent hook adapter](NATIVE-HOOKS.md) targets Codex `spawn_agent` and Claude
`Agent`/`Task`. Register it with `npm run native:install`, review the Codex hook, then reload this
plugin. Automatic routing applies only to interactive provider sessions opened through Paseo.

## Direct model and effort routing

Open `/route` in a workspace, enter a task, select the allowed saved profiles and effort options for each model, then choose
**Route and run**. The task is sent to Jev; one choice question selects a supported model–effort
pair and the plugin creates one agent directly in that workspace. Open that agent to follow its
normal progress and permission requests. There is no parent LLM, discovery/review stage, result
notification turn, or automatic escalation on this path. Ordinary composer sends are unchanged:
the installed `agent.create` hook does not receive the task prompt.

The selected profile supplies the provider, model, permission mode and features. Only the effort
is selected per task; saved profiles are never edited. Candidates use the live catalog's model
and effort descriptions and profile notes. Every selected model needs an explicit effort allowlist.
The UI offers only supported options; Luna only offers `max`. No prices, latency measurements or capability rankings
are inferred from a model name. Jev's choice is a heuristic, not a quality or optimality guarantee.
Direct routing validates the chosen option without applying the legacy .5 probability threshold.

The trusted-host RPC `direct.run` accepts `workspaceId`, a UUID `requestId`, `prompt`,
`allowedProfileIds`, required `allowedModels`, and `shareWithJev: true`. The effort allowlist
supports `low`, `medium`, `high`, `xhigh`, `max`; `ultra` is excluded because its catalog describes
automatic delegation. A singleton profile plus singleton effort pins the exact pair. Arbitrary
model/effort instructions inside prose are not parsed as settings: constrain the allowlists instead.
The server revalidates the workspace and exact candidate settings after the Jev call.

Example `allowedModels` (model IDs must match the selected profiles):

```json
[
  { "provider": "codex", "model": "gpt-5.6-luna", "effortIds": ["max"] },
  { "provider": "codex", "model": "gpt-6-astra", "effortIds": ["low", "high"] }
]
```

The model allowlist is configured per request in `/route` or RPC; it does not modify global profiles
or persist a default across panel sessions. Efforts are never shared across models. Missing models,
extra models without a selected profile, duplicate entries, unsupported efforts and Luna below `max`
are rejected before evaluating. The Luna rule also checks canonical catalog IDs for aliases.
Jev receives only these candidate pairs and cannot launch an unlisted choice. Empty allowlists never
mean unrestricted access. The old shared `allowedEffortIds` input is rejected rather than migrated
silently; existing request records remain available through `direct.status`.

Future `test:direct` runs explicitly allow Luna/max. Historical low-effort benchmark artifacts are
unchanged and do not measure this new policy.

`direct.profiles` lists available profiles; `direct.status` returns per-workspace request records.
The direct ledger is `$PASEO_HOME/plugin-data/jev-orchestrator/direct.json` (0600), separate from
legacy jobs. It retains settings, timestamps, evaluator usage and a request hash, not task text.
The created agent's ordinary Paseo session still contains its task. Repeat a request with the same
UUID and identical input to recover its existing result without another Jev call or agent.
Errors never trigger a fallback or retry. An ambiguous creation or plugin restart stays interrupted;
inspect agents labeled `jev-direct-request` before considering a new request. The router does not
replay it. The ledger fails closed at 10,000 requests rather than forgetting idempotency records.

`npm run test:direct` is billable and compares this path with a direct saved-profile baseline.
It uses fresh sessions, equal prompts and independent checks, alternating arm order, and includes
Jev plus native agent tokens after session closure. End-to-end time includes routing and validation.
Results and failed attempts remain under ignored `artifacts/`. This measures the supplied fixtures,
not universal savings. The older `test:live` exercises the separate multi-agent workflow below.

## Install

```sh
npm ci
npm run format
npm run typecheck
npm run lint
npm test
paseo plugin install "$PWD" --id jev-orchestrator
paseo plugin ls jev-orchestrator --json
```

Uses Typesafe AI directly at `https://api.typesafe.ai/v1/systemone`, model `jev-latest`.
Store `{ "apiKey": "<your Typesafe key>" }` in `$PASEO_HOME/typesafe-ai.json`
(default `~/.paseo/typesafe-ai.json`) with permissions `0600`. No Gateway fallback.
The key is read only in the server; it never enters agent MCP settings or client code.
Plugins must already be enabled. This is trusted unsandboxed code.

## Delegate

Create a fresh built-in Codex or Claude agent. Read saved profiles and their notes.
Call `delegate_task` with a `tasks` array. Every task needs:

- A stable `id`, `goal`, `acceptance`, and `kind` (`research`, `implementation`, or `review`).
- `files`: exact workspace-relative ownership paths, not globs. `.` owns the workspace.
- `allowedProfileIds`: the profiles the caller permits. Optional `profileId` fixes implementation selection.
- `checks`: explicit `{argv, timeoutMs}` commands. They run without a shell in the creating workspace.
- `shareWithJev: true`: explicit consent to send this task and bounded child/check evidence to Typesafe AI.

Optional `resources` name shared fixtures, ports, or databases. `dependsOn` names other tasks
under the same parent. Submit a whole dependency graph in one call; cycles and unknown IDs fail.
Defaults: `discovery: auto`, `review: auto`, `maxAttempts: 2`, `maxDurationMs: 600000`.
Discovery/review also accept `always` or `never`. At most two jobs run concurrently on this host.

A submission starts work and returns IDs immediately. Read `orchestrator_status`, or open
`/delegations` for an agent-scoped panel. Final status is appended to the parent's timeline.
An idle parent also receives a completion message. A busy parent reads the card or status tool.

## Decisions and authority

Jev chooses among saved, available provider/model/mode/effort combinations. The plugin copies
profile settings, including features, without rewriting them. An explicit implementation profile
never silently changes. Discovery/review may use other allowed profiles.

The route asks separate questions for profile, missing context, risk, and task category.
Unclear implementation can run read-only discovery first. Risky implementation can run a separate
review profile. A failed check may route to a different untried profile, within the attempt limit.
Environment failures and missing information pause; identical blind retries do not run.

Child prompts prohibit edits outside ownership, commits, pushes, deployments, worktrees,
recursive delegation, and changes to validation tests. These are instructions, not an OS sandbox.
The selected profile's existing permission mode remains intact. Review/discovery use read-only
instructions; provider-specific enforcement is not claimed. Declare all shared resources.

The scheduler enforces dependency and declared path/resource locks across jobs. Paths cannot escape
through existing symlinks. A job passes only when the configured checks exit zero. No checks means
`unverified`, never measured success. A positive Jev judgment cannot turn a failed check into a pass.
A review finding leaves the job needing input; it does not trigger an unbounded repair loop.

## Evidence, history, and limits

State lives outside Git at `$PASEO_HOME/plugin-data/jev-orchestrator/state.json` with mode 0600.
It includes submitted tasks and bounded child/check output. Send only content you intend to share.
Basic token redaction is not a complete secret detector. Never submit credentials or unrelated context.

Up to 200 jobs and 500 check measurements are kept. Model/profile changes invalidate history matches.
History compares the same task category and implementation phase. After at least three check-backed
samples, a candidate with at least 80% passes can replace a near-tied Jev choice (probability gap <= .1).
Pass rate is ranked before elapsed time. This is a conservative heuristic, not calibrated learning,
causal model benchmarking, or a claim that provider efforts are comparable. Usage stays missing when
providers omit it; missing cost is never treated as zero.

Jev calls use a 20-second timeout, no retries, and 26-second request spacing. Unavailable or uncertain
routing pauses without guessing. The .5 selected-option probability floor is a heuristic, not confidence.

Permission/deadline/uncertain jobs retain ownership. Inspect the child, then use `cancel_delegation`
before submitting a new ID. Cancellation archives only known children. A plugin stop/restart leaves
unfinished work `interrupted`; it does not replay or relaunch it. Inspect children on ambiguous launch
failure. Existing MCP processes have stale bindings after reload; create fresh agents.

Interactive root sessions require a ready bridge with a valid token, URL and workspace lease before
native policy generation. Missing bindings identify the missing environment variable names without
printing values. Stale or rejected bindings stop session opening with fresh-agent guidance; they do
not silently disable routing. Managed children skip parent-only native policy generation.

Native preflight failures include `failureStage` and `failureCode` in both the failed status record
and the HTTP 400 response. Stages identify parent runtime lookup, evaluation, decision validation,
policy revalidation, definition read/render, ticket write/publish, or final issuance validation.
Codes are fixed `NATIVE_*` identifiers, with `NATIVE_PREFLIGHT_TIMEOUT` for an aborted preflight.
Raw exceptions, paths, task contents and credentials are not returned. Missing-intent/scope denials
remain generic; failed tickets remain unusable and cannot be retried under a new request ID.
These fields diagnose future failures; errors discarded by older builds cannot be recovered.

Evaluation failures additionally expose `evaluationError` with an allowlisted `workerCode`, optional
integer `httpStatus` (100–599), and `processExitCode` from the actual worker process (0–255, or null
when no exit code exists). Missing fields mean unknown, not success. Worker codes distinguish config,
credential, input, Typesafe HTTP/non-HTTP, invalid response, spawn, exit, invalid output, output limit,
timeout and cancellation. Worker-reported exit codes are ignored. Raw stderr, response bodies,
exception messages and credentials stay redacted; no automatic retries or fallback are added.
Offline tests use a fake worker subprocess and mocked preflight judge, never the live evaluator.

The session-scoped MCP bridge binds random tokens to a parent ID and canonical workspace. It listens
only on loopback, rejects browser Origin headers and oversized requests, and revokes archived parents.
Managed children do not receive the orchestrator MCP. Ordinary plugin RPCs use Paseo's trusted host
connection and allow the connected user to inspect/control their selected parent.

## Whole-workflow token accounting

Each attempt records `usage` from its identified native provider session. Codex uses final
cumulative thread counters, never a sum of snapshots. Claude sums distinct API requests,
deduplicates streaming records, and includes cache read/write tokens in input. Reasoning
is an output subset and is not added twice. Missing dimensions remain `null`.

Each job keeps an `evaluations` ledger, including failed/invalid Jev judgments. An evaluation
without provider token counts stays incomplete. A completed judgment is not evidence of zero usage.
The metrics token field is populated only when native accounting is complete.

The live harness records `workflowUsage` for a fresh parent, all managed children and all Jev calls.
The parent submits via MCP and finishes its result-notification turn before collection. Native counts
are reconciled after archiving test actors; Claude requires a terminal assistant record before
reporting complete coverage. It emits
both a complete total (only when every component is covered) and a separately labeled known subtotal.
It retains per-component source, counts, request count when available, and coverage notes.
Provider-hidden calls are not estimated. These are reported token counts, not a billing invoice.

Native accounting runs on the daemon host and reads only the identified session file under
`CODEX_HOME`/`~/.codex` or `CLAUDE_CONFIG_DIR`/`~/.claude`; unrelated transcript contents are not scanned.
Different/remote provider homes, missing logs, malformed records or unsupported providers return
incomplete coverage. Reused parent sessions include earlier work, so only fresh sessions provide the
benchmark's task attribution. Historical `lastUsage` measurements are not comparable to this ledger.

## Verify

`npm test` covers routing, discovery, escalation, review, dependencies, locks, permission, duplicate
submission, cancellation, restart, history, path boundaries, real command execution, and bridge scoping.

`npm run test:live` is billable: it runs synthetic coding tasks through the installed host and Jev.
It uses only task-owned paths, records all attempts and failures under ignored `artifacts/`, and archives
its test agents afterward. Live sample results measure those cases only, not production workloads.

### Codex parent execution policy

An Astra/low parent should handle small bounded changes with explicit acceptance and local checks directly, without preflight or a child, unless independent parallel work materially helps. A clear spec alone is not a reason to delegate. This follows the narrow single-task evidence in `NATIVE-BENCHMARK.md`, not a universal model ranking.

If preflight is called, Jev may choose `action: self` only when Paseo reports the current parent runtime as Astra/low. This returns no spawn ticket; the parent executes the submitted task. Runtime is checked again after evaluation. Other parents retain delegation-only choices. Existing model–effort allowlists are unchanged, including Luna/max only. A self preflight still uses one evaluation and one of the three reserved request slots; obvious small tasks should skip preflight. New sessions receive the updated tool description after plugin reload.

### Direct Typesafe access

`JEV_TYPESAFE_HTTP` includes the sanitized HTTP status from Typesafe; `JEV_TYPESAFE_FAILED`
means a non-HTTP provider failure. No response body or credential is returned. Historical Gateway
codes remain readable for older records. Direct access uses a Typesafe key, independently of
Gateway credits. Neither routing allowlists nor permission thresholds change with this provider.
