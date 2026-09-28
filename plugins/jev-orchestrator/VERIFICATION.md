# Jev orchestrator verification — 2026-09-21

Implemented TASK-007–012 in one standalone plugin. Installed and running on macOS,
Paseo daemon/client/SDK 0.8.0. No Paseo application source changes, commit, push or worktree.

## Reproduce

From `plugins/jev-orchestrator/`:

```sh
npm run format
npm run typecheck
npm run lint
npm test
paseo plugin ls jev-orchestrator --json
# Billable synthetic acceptance, requires installed plugin and existing saved profiles:
npm run test:live
```

Format, typecheck and lint pass; 24 tests pass. Tests cover routing/profile preservation,
bounded escalation, discovery, review, dependency/path/resource locks, permission pauses,
restart without replay, comparable history, real process checks, authenticated bridge and
real stdio MCP submission/status/deduplication. Escalation and scheduler failure scenarios
use controlled drivers; no production reliability claim follows from them.

## Live results

The final run used real Jev Gateway calls and real saved Paseo profiles. Both fixtures failed
external checks before work; each final result passed the unchanged independent command.
Times include creation, routing, child work and polling, but exclude cleanup. One sample per
case/arm; fixture order is baseline→routed for intervals and routed→baseline for cache.

| Case            | Direct baseline |    Routed | External checks |
| --------------- | --------------: | --------: | --------------- |
| Merge intervals |        45.272 s | 150.765 s | Both pass       |
| Expiring cache  |        41.840 s | 316.600 s | Both pass       |

- Baseline: saved `architect`, Codex `gpt-6-astra`, `low`, `auto-review`.
- Routed implementation: saved `logic`, Codex `gpt-5.6-luna`, `max`, `auto-review`.
- Cache discovery and review: saved `reviewer`, Claude `claude-opus-4-8`, `max`, `auto`.
- Codex runtime snapshots confirmed model/effort/mode. Claude runtimeInfo omitted effort;
  separate host snapshots confirmed `thinkingOptionId` and `effectiveThinkingOptionId` as `max`.
- Interval routed worker: 142.774 s. Cache phases: discovery 92.262 s, implementation
  140.750 s, review 45.594 s. These are observed child intervals, not inferred CPU time.
- No coding escalation, human permission intervention or repair retry was needed in the final pairs.
- The cache routed arm deliberately forced discovery and review; baseline had neither.
  This compares complete workflows with different review depth, not isolated model speed.

**Measured outcome:** autonomous stage transitions and externally verified completion work.
On these small fixtures, routing was slower: 3.33× and 7.57× baseline wall time. Worker `max`
effort and additional review work are visible contributors; these samples cannot establish causality
or general model quality. Do not enable extra stages for simple work expecting a speed benefit.

Full cost cannot be compared: Codex and Jev cost totals were unavailable. Claude reported
$0.6244315 for discovery and $0.432832 for review in the final run; these are partial reported
usage, not complete workflow cost. Token fields are provider-reported latest usage, not a
normalized billing comparison. No savings or learned-routing improvement is claimed.
The small synthetic history is insufficient to calibrate routing on real project work.

## Integration and preserved failures

- Real Codex parent `5192e6f0-54ff-4684-ba3c-690def61b4d6` called
  `jev_orchestrator.orchestrator_status` once. Timeline recorded `completed`, `error: null`,
  output `{"jobs":[],"history":[]}`. This verifies fresh-agent injection and session binding.
- Desktop `/delegations` opened the expected agent-scoped panel and empty state. A later
  populated-panel check was inconclusive: computer-use exposed only the Workspace scripts
  popover and could not capture a screenshot. Populated, mobile and theme acceptance remain unverified.
- Initial load failed with `Cannot read properties of undefined (reading 'AccessTokenMissingError')`.
  Isolating the AI SDK in a standalone Node worker fixed the installed plugin bundle failure.
  The current plugin log ends with `Plugin ready` at 2026-09-20 19:03:17 UTC, with no later load error.
- First run retained: interval baseline 40.260 s/pass; routed interval interrupted by the
  operator's plugin reload at 35.719 s/check failure. This is test interference, excluded from
  the final comparison, not discarded. Cache routed 295.902 s/pass; baseline 41.436 s/pass.
- Final run followed discovery-readiness and metric-validation fixes. Runtime source stayed
  unchanged during that run. No failed answer was retried to improve a reported score.

## Evidence and cleanup

Final runtime source SHA-256 (server/shared/client, entrypoints and lockfile):
`057d1ff1938351f4f6084baf4f96850fa0484262ca9cf02931ccfe5f855fc83a`.

Local ignored artifacts contain synthetic fixtures, outputs, independent checks, parent/child IDs,
observed runtime settings and usage:

- [Final run](artifacts/live-2026-09-20T19-03-43-633Z/results.json)
- [First run including interruption](artifacts/live-2026-09-20T18-56-18-500Z/results.json)
- [Cleanup ownership audit](artifacts/cleanup-audit.json)

All 17 test agents and nine test-only local workspace records are archived. Archive responses
confirmed `removedDirectory: false`; a fresh workspace listing contains none of those nine IDs.
Fixtures and evidence remain ignored for reproducibility. Plugin remains enabled. The live harness
now cleans its own workspace records after validating agent ownership; this cleanup addition passed
typecheck/lint but was not used to rerun the billable benchmark. Actual cleanup above was independently
performed and verified against the installed host. No production workload was exercised.

## Whole-workflow token accounting follow-up — 2026-09-21

This follow-up replaces the earlier `lastUsage` comparison for token measurement. It does not
retroactively turn old snapshots into full-workflow totals. The same two fixture definitions,
profile allowlist, model/effort settings, checks and arm order are retained.

Accounting methods:

- Codex: final native `token_usage_record.thread_token_usage`, with older cumulative `token_count`
  support. Repeated snapshots are not summed; `response_id` identifies distinct reported requests.
- Claude: deduplicate `requestId`/`message.id`, then sum each distinct request's input, cache-read,
  cache-write and output. Reasoning remains an output subset. This follows the
  [Claude API input-token definition](https://platform.claude.com/docs/en/api/cli/beta/messages/create)
  and [streaming usage semantics](https://platform.claude.com/docs/en/build-with-claude/streaming).
- Jev: persist every evaluator invocation and the AI SDK's returned input/output usage, including
  an invalid judgment after a successful response. Failed/missing usage remains unknown.
- Parent: real MCP submission and the final completion-notification response are included. Each
  benchmark parent is fresh, so prior unrelated conversation usage cannot contaminate its total.
- The harness waits for notification submission and the parent's final turn before collection.
  It sums each parent, child and evaluation once. Missing coverage yields a null complete total
  and an explicitly separate known subtotal; cache/reasoning fields remain null when unreported.

Scope: one sample per case/arm, provider-reported tokens, with synthetic tasks. The supplied task
contract does not measure parent planning/decomposition from an ambiguous natural-language goal.
Cross-provider token counts are not equivalent to money or subscription quota. Cache and model rates
are not priced. Provider-hidden operations that produce no accounting record cannot be inferred.
Native transcript lookup currently supports local provider homes; alternate long/symlinked Claude
project-directory encodings may return incomplete coverage. No fallback to lastUsage is permitted.

Validation: 34 tests, typecheck and lint pass. New tests cover cumulative counters, repeated events,
request deduplication, cache/reasoning subsets, archived logs, identity mismatch, missing coverage,
failed evaluator usage and post-notification capture. The installed plugin reloaded successfully.

### Final measured tokens

All four arms passed the unchanged independent checks and have complete provider-reported total
coverage after their agents were archived. No failed answer was retried. Token totals include cached
input; cache is a subset, not additional tokens added on top. Per-model tokenizers differ.

| Case            | Baseline total | Orchestrator total |  Ratio | Baseline/routed wall time |
| --------------- | -------------: | -----------------: | -----: | ------------------------- |
| Merge intervals |         90,057 |            456,093 |  5.06× | 44.968 / 157.221 s        |
| Expiring cache  |         90,179 |          1,006,360 | 11.16× | 40.739 / 317.393 s        |

| Routed component                       |      Intervals |            Cache |
| -------------------------------------- | -------------: | ---------------: |
| Parent (submission and final response) |        204,430 |          207,801 |
| Discovery                              |    Not invoked |          385,587 |
| Implementation                         |        249,633 |          216,688 |
| Review                                 |    Not invoked |          183,860 |
| All Jev calls                          | 2,030 (1 call) | 12,424 (5 calls) |

The baseline made three reported model requests per task. Intervals routing made six parent requests,
eight worker requests and one evaluator call. Cache routing made six parent, eight discovery, seven
worker, four review requests and five evaluator calls. The measured overhead is predominantly repeated
agent inference and its context; Jev itself is a small part of the reported token totals. This does
not prove which model/effort would win in a controlled same-model comparison.

Cache detail for agent calls (excluding Jev, which does not expose cache/reasoning breakdown):

| Case/arm                | Cache-read input | Cache-write input | Reasoning subset of output |
| ----------------------- | ---------------: | ----------------: | -------------------------: |
| Intervals baseline      |           65,280 |                 0 |                          0 |
| Intervals routed agents |          368,896 |                 0 |                      3,000 |
| Cache baseline          |           58,624 |                 0 |                          0 |
| Cache routed agents     |          824,681 |            82,125 |                      7,795 |

Whole-workflow cache/reasoning breakdown remains `null` for the routed arms because Jev omits those
fields; input/output/total coverage is complete. No token-price or subscription-quota savings claim.
The cache routed arm still does extra discovery and review, so it is a workflow comparison with
unequal review depth. Both small fixtures were solved correctly by the direct baseline.

**Conclusion:** no token-saving benefit on these samples. Use direct execution for comparable small
tasks; additional autonomous stages need a demonstrated quality or reduced-intervention benefit to
justify their overhead. These results do not establish behavior on larger production work.

### Late transcript flush and cleanup

The first idle-time Claude readings missed a final request: discovery initially reported 334,449,
review 136,316. Re-reading after completion yielded 385,587 and 183,860. The parser now requires a
terminal assistant `stop_reason` before marking coverage complete; the harness reconciles all actor
usage after archiving. A regression test covers an idle snapshot with a pending final log record.
The final totals above are independently re-collected from closed sessions and match the pre-archive
whole-workflow totals. Intermediate attempt snapshots remain preserved in the raw artifact.

All eight benchmark agents and four test-only workspace records are archived. A fresh host listing
contains none of those workspace IDs. No directory was deleted. Plugin remains enabled.

- [Raw run](artifacts/live-tokens-2026-09-21T03-04-41-403Z/results.json)
- [Final accounting and closed-agent snapshots](artifacts/live-tokens-2026-09-21T03-04-41-403Z/usage-reconciled.json)
- Execution source SHA-256: `e3918aa6693480c492b15c73b10009200b2c7308e1de4c67b0d7e450c75c8574`.
- Final accounting source SHA-256: `2aae692a74f6c4e56e17e2994a8da4bc6f4e4533eb79dd64a95cff12001edab5`.

The final accounting fix was verified against the same closed native sessions and offline tests;
model answers were not rerun. The billed benchmark excludes plugin implementation and this analysis.

## Direct model–effort routing — 2026-09-21

Implemented an opt-in workspace panel (`/route`) and `direct.run` RPC. One Jev choice selects
an allowed, live-supported model/effort pair; one agent receives the original prompt directly.
There is no parent LLM, automatic discovery/review, or completion-notification turn. Profile
permission modes/features remain unchanged. Default effort candidates are low/medium/high.
The plugin uses supported Paseo 0.8.0 APIs; no Paseo application source was changed.

Format, typecheck, lint and all 43 offline tests passed. Independent review found prompt trimming;
that was fixed and the exact-prompt regression now includes surrounding whitespace. Tests cover
concurrent/persisted idempotency, changed request input, invalid candidates, profile/workspace drift,
failed evaluator usage, shutdown and interrupted creation. Installed reload returned `running`.
Desktop UI verification covered the populated profile list, explicit selection, prompt input,
enabled/disabled submission button and cleanup of the temporary panel. Actual creation was verified
through the same RPC in the live harness; clicking the UI's submit/open-agent buttons and native
mobile behavior were not separately exercised.

### Matched live experiment

Command: `npm --prefix plugins/jev-orchestrator run test:direct` (billable).
Two fixtures, two repetitions each, alternating baseline/direct order: eight total runs.
Both arms use the exact same prompt, reset solution path, checks, workspace cwd and permission mode.
Fresh directory workspace records do not create Git worktrees. External checks fail before each
run and their hashes remain unchanged afterward. No failed model answers were discarded or rerun.

Baseline: `codex/gpt-6-astra`, effort `low`, saved architect settings.
Direct candidates: saved logic and architect models, each with supported low/medium/high efforts.
Jev selected `codex/gpt-5.6-luna/low` in all four routed runs. Runtime provider/model/effort/mode and
explicit feature settings matched the requested selections in every run. Saved profiles were not edited.

| Fixture  | Repeat | Arm        | External checks | Whole-workflow tokens | Task time (s) | Jev time (s) |
| -------- | -----: | ---------- | --------------- | --------------------: | ------------: | -----------: |
| Interval |      1 | Baseline   | Pass            |                90,393 |        48.346 |            — |
| Interval |      1 | Direct Jev | Pass            |               176,611 |        42.249 |        2.182 |
| Interval |      2 | Direct Jev | Pass            |               206,796 |        52.660 |        1.625 |
| Interval |      2 | Baseline   | Pass            |                90,146 |        45.337 |            — |
| Cache    |      1 | Baseline   | Pass            |                90,583 |        43.016 |            — |
| Cache    |      1 | Direct Jev | Pass            |               208,131 |        52.818 |        1.956 |
| Cache    |      2 | Direct Jev | Pass            |               177,107 |        80.190 |        1.653 |
| Cache    |      2 | Baseline   | Pass            |                90,541 |        45.747 |            — |

| Aggregate                 |  Baseline | Direct Jev |
| ------------------------- | --------: | ---------: |
| Passing runs              |       4/4 |        4/4 |
| Total reported tokens     |   361,663 |    768,645 |
| Mean task time            | 45.6115 s | 56.97925 s |
| Coding-model requests     |        12 |         26 |
| Jev tokens included above |         0 |      4,324 |
| Mean Jev routing time     |         — |    1.854 s |

Direct routing used **2.1253× tokens** and **24.923% more time on average**. Interval mean time
was 46.8415 vs 47.4545 seconds; cache was 44.3815 vs 66.504 seconds. One interval run was faster;
that isolated result does not establish a speed benefit.

### Accounting, interpretation and limits

Task time starts immediately before baseline creation or direct RPC and ends after the external
check and its integrity check. It includes routing, candidate loading/revalidation, agent creation
and execution. Fixture setup, workspace creation, archive and token-log collection are excluded
symmetrically and separately retained as `harnessWallMs`. The whole experiment ran sequentially
on the existing host; other user work and provider latency were not controlled.

Every workflow has complete input/output/total coverage. The coding agent uses final native Codex
cumulative counters after archive, and direct workflows add exactly one reported Jev evaluation.
All eight closed-session totals were independently re-read and matched the saved results. Input
includes cache reads; output includes reasoning. Neither subset is added twice. Jev omits cache
and reasoning breakdown, so those full-workflow dimensions remain null. These are reported tokens,
not invoiced cost; implementation, test-authoring and this analysis are outside the benchmark.

The baseline used three model requests per run; Luna used six or seven. In the first matched pair,
native records show two `exec` tool calls for Astra versus five for Luna. Growing context is sent
on each request, including cached context. That explains the token increase despite only one
coding agent. Jev itself accounts for 4,324 of 768,645 routed tokens; its decision latency is small.

This is a comparison of the complete routing policy against a fixed-model baseline. Model choice
and subsequent tool behavior are part of the treatment; it does not isolate Jev's causal effect.
The sample contains two small synthetic tasks and cannot establish broad quality, hard-task effort
selection, production calibration or statistical significance. Live low-effort selection and offline
high-effort forwarding were verified; live high-effort task quality was not measured.

**Conclusion:** the direct integration works, but this routing policy does not meet the token/speed
objective on these fixtures. Keep the fixed baseline for comparable tasks. A useful next iteration
must rank candidates using measured whole-workflow behavior, including model/tool request counts,
and validate on held-out tasks. A catalog description such as fast/affordable and lower effort is
not evidence of fewer total tokens. The plugin remains opt-in; ordinary composer sends are unchanged.

### Evidence and cleanup

- [Raw run and all eight outputs](artifacts/direct-live-2026-09-21T03-35-22-378Z/results.json).
- [Verified totals, source identity and cleanup](artifacts/direct-live-2026-09-21T03-35-22-378Z/summary-verified.json).
- [Real duplicate-request check](artifacts/direct-live-2026-09-21T03-35-22-378Z/dedupe-check.json).
- Source SHA-256 before/after: `1ca86c37faf31a3eab82a6889cd24c0be2dd052c5c1b06f9f6166642d99a5279`.

Replaying a completed direct RPC with the identical UUID/input returned the same archived agent
and left the evaluator ledger unchanged. No second model call was made for that check. All eight
test agents are archived; all eight test workspace IDs are absent from the live workspace listing.
No directory was removed. Synthetic artifacts remain ignored. No commit or push.

## Per-model effort allowlist — 2026-09-21

Direct routing now requires `allowedModels` entries containing provider, model and effort IDs.
The UI groups selected profiles by model and requires explicit effort selections for each group.
Empty, missing, extra or unsupported entries fail before Jev. Server validation also resolves
catalog aliases and rejects Luna at any effort other than max. Exact candidates are revalidated
before agent creation; no unlisted model or effort is used as a fallback.

Format, typecheck, lint and all 47 tests passed. Installed plugin reload returned running.
A read-only `direct.profiles` RPC on the live host returned Luna `[max]` and Astra
`[low, medium, high, xhigh, max]`. No evaluator/model request or paid benchmark was run for this
change. Updated UI controls passed typecheck/lint; their desktop interactions were not re-exercised.
The earlier Luna/low results above remain historical evidence, not measurements of this policy.
Allowlist selections are per request; they are not saved as global defaults.

## Paseo-scoped native hooks — 2026-09-21

Registered one guarded PreToolUse command in each native runtime, preserving existing groups
and backing up local configuration. Native policy persists outside Git in
`~/.paseo/plugin-data/jev-orchestrator/native/settings.json`. Session-open injection applies only
to interactive built-in Codex/Claude sessions; history opens receive no native marker.

`npm run format`, `npm run typecheck`, `npm run lint`, and all 59 offline tests passed.
Plugin reload returned `running`; its latest log says `Plugin ready`. Direct execution of both
installed hook commands verified outside-Paseo `{}` and wrong-workspace `deny` without evaluating.
All generated definition hashes matched their manifests.

Used the native Codex hook-review UI to trust only the new Jev hook. No trust-bypass flags were used.
A fresh app-server `hooks/list` reported `enabled: true`, `trustStatus: trusted`, and hash
`sha256:1b75831073cb63a03509e788ad4dccf0f7f323e5537e1d76c5b63428d928551a`.
The temporary configuration-only CLI session was closed without sending a model prompt.

Created no-prompt sessions in the existing workspace through the installed Paseo SDK:
Codex `9e689a79-9d0b-49d1-97b6-3de96cfce2d8` and Claude
`da947d06-b264-48f3-b376-563fe4b6fcf6`. Both became idle with no reported error and were archived.
This verifies session initialization, not an actual native subagent's final model/effort.
No paid evaluator run or new performance benchmark was performed. Generated role descriptions
add unmeasured context overhead; unsupported/invalid source definitions are not mapped.

## Native v2 bypass investigation — 2026-09-21

Preserved failed acceptance: Paseo agent `63ce82c0-6007-40df-84da-45e7c3925538`,
parent Codex session `01a0c27c-eca9-7291-aaf8-37675447812f`, child
`01a0c27d-168a-7d02-a225-5aaab4831966`. The child returned `NATIVE_JEV_OK:323`,
but its turn_context was **gpt-5.6-sol/high**, outside the allowlist. Parent markers were present;
installation/trust alone did not establish enforcement. The ignored synthetic evidence is
`artifacts/native-verification-63ce82c0.json`.

Version-pinned Codex 0.154.0 source confirms hook names use `collaborationspawn_agent` for v2,
while the old matcher allowed only `spawn_agent|Agent`. The v2 message remains encrypted and the
hook payload lacks its plaintext/encrypted source discriminator. Correcting the name alone would
send ciphertext to Jev, not a meaningful task.

Changed the matcher and exact legacy-registration migration; v2 now returns an explicit unsupported
plaintext-contract denial before evaluation. This is a bypass fix, not successful v2 routing.
All 60 tests, format, typecheck and lint passed. Installed the updated registration, reviewed/trusted
only that modified hook through Codex's native UI, and reloaded the plugin (`running`).

Fresh Paseo probe `f390a228-cde8-4769-bd86-1137ac3cfa7c` used Astra/low and made
exactly one native v2 spawn call. Codex transcript `01a0c28d-c222-7121-bed1-928c5557777f`
line 14 records the call; line 16 records `Tool call blocked by PreToolUse hook` with the
explicit plaintext-contract denial and tool name `collaborationspawn_agent`. No child session
with this parent ID was found in that day's session files. The test agent was archived and the
configuration-only TUI exited. Evidence: `artifacts/native-v2-denial-f390a228.json` (ignored).
This confirms runtime interception/denial, not successful Jev routing or token/time savings.

## Codex native v2 preflight ticket acceptance — 2026-09-21

Implemented plaintext `prepare_native_delegate` and `native_delegation_status` over the existing
parent-scoped MCP bridge. A root-transcript PreToolUse intent binds the native session to the task.
Jev evaluates once, a pre-registered role receives the original instructions and authorized task,
and the v2 spawn hook consumes a five-minute ticket atomically and applies the pinned role.
No Paseo application changes, global model-default changes or substitute Paseo child launches.
Three non-reusable slots per fresh parent; only `fork_turns:none` and root-parent delegation.

Validation: format, typecheck, lint and 64 tests passed, including a real command-hook subprocess
against the scoped HTTP bridge, duplicate concurrent consumes, exact task matching, expiry,
policy/role mutation, descendant transcript rejection, failed evaluation and archival cleanup.
Updated and trusted only the modified Codex hook through its native review UI; plugin reload is running.

Retained failed first acceptance: Paseo `948df1ef-c47b-409f-9bcb-aba72e3b524b`, native parent
`01a0c29e-4a79-73f3-a919-1be2cb429753`. Jev chose Luna/max (658 tokens, 1,935 ms), but
Codex rejected the hyphenated task_name. Corrected by separating underscore-only ticket task names
from role names. The consumed failed ticket was not replayed; the agent was archived and its three
owned role slots removed. Evidence: `artifacts/native-preflight-failed-948df1ef.json` (ignored).

Successful acceptance after the format fix: Paseo `4f6e8186-c411-41d3-92fd-891ce60a142f`.
Parent `01a0c2a0-e94f-7583-9c05-514ad00240c1` made one native spawn (transcript line 23).
Child `01a0c2a1-78e3-79d2-ac68-128f52978b35` independently confirms
**gpt-5.6-luna / max** in turn_context and returns exactly `NATIVE_JEV_PREFLIGHT_OK:323`.
The parent passed only the returned transport pointer as message; the arithmetic task arrived in
its selected role's developer instructions. Status reports consumed, two remaining slots.

| Measured component                 |  Tokens |
| ---------------------------------- | ------: |
| Parent Astra/low                   | 193,154 |
| Delegated child Luna/max           |  33,294 |
| Provider guardian review           |  12,156 |
| Jev, one evaluation                |     658 |
| Observed acceptance workflow total | 239,262 |

Jev: 624 input + 34 output, 1,526 ms. Parent turn wall time: 53,691 ms, including tool discovery,
approval/review, evaluation, spawn, waiting and reporting. Child execution: 7,628 ms.
These are the observed acceptance sessions, not the entire implementation conversation and not an
A/B benchmark. They do not establish savings or production-task accuracy. Guardian review is a
provider approval session, outside the delegation model allowlist; its tokens are included above.

Evidence: `artifacts/native-preflight-success-4f6e8186.json` records session paths, choices,
usage, timings, source HEAD and hashes. The acceptance parent was archived; all three role slots
were confirmed absent afterward. The temporary configuration TUI exited without a model request.
Reload/crash revokes tickets but retains live role files; orphan cleanup after such events remains
manual. Do not reload the plugin while relying on outstanding tickets.

## Native preflight code-task benchmark — 2026-09-21

See [NATIVE-BENCHMARK.md](NATIVE-BENCHMARK.md) and ignored
`artifacts/native-benchmark-20260921/results.json`. Four serial AB/BA runs on identical copies of
real usage-accounting code all passed 18/18 fixed behavior groups and strict tsc. Both native Jev
children were independently verified as Luna/max. Mean direct Astra/low baseline: 98,887.5 tokens,
44.420 s including startup and external grading; mean Jev workflow: 572,420.5 tokens, 248.012 s.
Whole-workflow tokens include parent, child, approval guardian and evaluator. Jev used 1,411 tokens
and about 1.54 s per run. No quality improvement was observed; this task's token/time goal failed.
No result-guided reruns or product-source edits. Agents archived and role slots cleaned.

## Native routing recovery — 2026-09-22

The reported failed session returned the generic native-hook denial while `native_status`
returned HTTP 400 and `direct.profiles` reported `Transport not connected (status: disconnected)`.
The bridge and hook now distinguish SDK disconnection, invalid session binding, unreachable
bridge and rejected ticket using fixed, redacted messages. Authorization remains fail-closed.
This is diagnostic correction plus operational recovery, not a permanent SDK reconnect fix.

Regression: `node --import tsx --test --test-name-pattern='real command hook' tests/native-tickets.test.ts`
failed against the old generic message, then passed. `npm run format`, `npm run typecheck`,
`npm run lint`, `npm test` (66/66), and `git diff --check` passed.

`paseo plugin reload jev-orchestrator` completed; status was `running`, and the log recorded
`Plugin ready` at 2026-09-22T04:29:24.404Z. Fresh smoke parent
`8b6bf454-f693-45f0-8c19-74003fe7e09d`, Codex session
`01a0c760-6c76-7f80-af83-adccc64b48d0`, called preflight once. Its actual tool result reports
`action:self`, `state:self`, `gpt-6-astra/low`, one Jev request, 840 evaluator tokens and 1,359 ms.
This verifies post-reload preflight, not post-reload child spawning. The smoke parent was archived.
Existing parents retain stale bindings after reload and require fresh sessions.

## Direct Typesafe migration — 2026-09-28

All three installed Jev plugins now use `@ai-sdk/typesafe-ai` 3.0.8 with
`jev-latest` at `https://api.typesafe.ai/v1/systemone`. The server reads the key
from `$PASEO_HOME/typesafe-ai.json` (0600); no key is copied into agent settings
or Git. Gateway remains an unrelated configured chat provider, not a Jev fallback.

Validation on the uncommitted migration diff:

- Each plugin: `npm run format`, `npm run typecheck`, `npm run lint`, `npm test`.
- Orchestrator: 87 tests; evaluator: 5 tests plus 3 benchmark scorer tests;
  permission gate: 5 tests. All 100 passed. Gate lint retains one pre-existing
  no-useless-escape warning in `tests/policy.test.ts:63`; no lint errors.
- `tests/typesafe-worker.test.ts` executes the real worker with mocked HTTP,
  checking the direct URL, authorization, model, successful choice/usage,
  401/429 diagnostics, malformed answers, missing key, and no retry/fallback.
- Evaluator `npm run smoke`: one live MCP request returned boolean, choice and
  score; usage 379 input + 64 output = 443 tokens.
- Permission worker: one synthetic classification, 738 ms including worker startup;
  readOnly probability 0.88 and allow probability 0.91. The unchanged 0.9 threshold
  would escalate this verdict. No classified shell command was executed.
- All three plugin reloads returned running; orchestrator ready at
  2026-09-28T07:49:16.385Z. No daemon restart.
- Credential scan found no exact key in tracked/untracked nonignored repository files.

Fresh native acceptance parent: `5c88bdd1-efeb-4f83-ae18-caee0fe5f1e6`;
request `typesafe_native_acceptance_20260928_once`. The actual parent transcript records
one issued then consumed ticket, selecting `gpt-6-luna / max`, 859 ms evaluation,
1,075 input + 41 output = 1,116 tokens. Child `01a0e6fe-951a-7151-969e-e0f143d85fe7`
has matching model/effort in its own `turn_context` and parent linkage to native session
`01a0e6fd-aa01-7721-ab9f-faa42a63ae1b`. Both transcripts now contain `task_complete`;
the child final confirms the credential guard and reports no remaining actionable
findings in the scoped integration. Review compared source and baseline; it did not
rerun tests. Exactly one preflight and one native spawn were used. The acceptance
parent was archived after reading its final result.

Follow-up credential regression: `tier0('cat ~/.paseo/typesafe-ai.json')` initially
returned allow. A new test failed before adding the exact credential filename to
the deny rules and Jev policy. Permission-gate checks now pass 6/6; formatting,
typecheck, lint (same existing escape warning), and plugin reload completed.
This adds one credential protection; probability thresholds remain unchanged.
No secret content was read by the regression probe.
