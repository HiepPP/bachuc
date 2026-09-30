# Native Jev preflight versus direct baseline — 2026-09-21

**For this task, native Jev preflight did not meet the speed/token objective.** Both workflows
passed the same quality checks. The Jev workflow used **5.79× tokens** and **5.58× execution time**
on average. This compares whole workflows, not the isolated evaluator or equal-model inference.

## Task and controls

Improve the real `server/usage.ts` implementation of `summarizeUsage`: validate runtime numeric
counts, reject duplicate IDs, detect unsafe sums, preserve partial/unknown usage, and add role
breakdowns that safely handle keys such as `__proto__`. Each arm edited an identical starting copy;
production `usage.ts` was not changed. This is an enhancement/robustness task with a complete spec,
not a broad repository investigation or production deployment.

Two fresh sessions per arm, serial order **baseline_1 → jev_1 → jev_2 → baseline_2**. Same workspace,
Astra/low parent, auto-review permissions, common task specification, toolchain and frozen assessor.
Baseline implements directly without delegation. Jev parent sends the entire spec through preflight,
spawns exactly one native child and waits. Jev independently selected **Luna/max in both runs**;
each child's transcript confirms that model/effort and its ticket was consumed once.

No result-guided retries, model substitutions, prompt tuning or repairs. Each agent owned only its
assigned candidate and was instructed not to read siblings, results or assessor. Inspected tool calls
showed work on the assigned candidate; the completed candidates were reviewed read-only.

The 18 fixed behavioral groups include empty inputs, duplicate IDs across roles, immutability,
63 invalid-count combinations, strict completeness, partial sums, optional unknowns, safe integer
boundaries/overflow and prototype-safe groups. The original source passed 3/18 before any run;
all four candidates passed **18/18 plus strict TypeScript compilation**. Passing these checks is
not proof of general correctness. There was no quality gain observable in this suite.

## Measured results

| Run        | Implementing model/effort | Quality          | Whole-workflow tokens | Startup s | Parent turn s | Grading s | Execution s |
| ---------- | ------------------------- | ---------------- | --------------------: | --------: | ------------: | --------: | ----------: |
| baseline_1 | gpt-6-astra/low           | 18/18 + tsc pass |                98,814 |     0.443 |        42.842 |     0.276 |      43.561 |
| jev_1      | gpt-5.6-luna/max          | 18/18 + tsc pass |               528,656 |     0.609 |       258.281 |     0.294 |     259.184 |
| jev_2      | gpt-5.6-luna/max          | 18/18 + tsc pass |               616,185 |     1.468 |       235.086 |     0.285 |     236.839 |
| baseline_2 | gpt-6-astra/low           | 18/18 + tsc pass |                98,961 |     0.376 |        44.629 |     0.273 |      45.278 |

| Average of two runs          |    Tokens | Execution time |
| ---------------------------- | --------: | -------------: |
| Direct Astra/low baseline    |  98,887.5 |       44.420 s |
| Jev preflight + native child | 572,420.5 |      248.012 s |
| Jev change                   |  +478.86% |       +458.34% |

All four runs consumed **1,342,616 reported tokens** in total. All collected session usage was terminal
and complete; cumulative snapshots were not added together. Jev usage was deduplicated by request ID.

## Accounting boundary

Tokens include parent, every observed descendant (including provider approval guardian), and Jev.
Cache reads are already included in native input totals, and reasoning is already included in output;
neither is added again. These token counts are **not dollar cost estimates**. Jev does not expose
cache/reasoning breakdown, so aggregate breakdown fields remain null rather than assuming zero.
Native-only uncached input was 27,161 / 33,894 for baseline and 105,974 / 114,541 for Jev workflows;
the result is not solely an increase in cached context reads.

Execution time = persisted Paseo `createdAt` → parent turn start + parent turn wall time + measured
external grading process time. Parent wall time includes routing, approval, spawn, child waiting and
final report. Nested actor times overlap and are **not summed**. Excluded: benchmark fixture setup,
coordinator scheduling/idle gaps before grading, this report-author session and cleanup after grading.
Thus this is task execution latency, not elapsed time to conduct the entire experiment. Provider
startup before persisted `createdAt`, if any, is not observable in this metric.

## Where the overhead appeared

| Run        | Actor    | Model/effort          |  Tokens | Actor wall s |
| ---------- | -------- | --------------------- | ------: | -----------: |
| baseline_1 | parent   | gpt-6-astra/low       |  98,814 |       42.842 |
| jev_1      | parent   | gpt-6-astra/low       | 268,617 |      258.281 |
| jev_1      | guardian | codex-auto-review/low |  14,159 |       11.061 |
| jev_1      | subagent | gpt-5.6-luna/max      | 244,469 |      192.894 |
| jev_2      | parent   | gpt-6-astra/low       | 273,524 |      235.086 |
| jev_2      | guardian | codex-auto-review/low |  14,100 |        8.688 |
| jev_2      | subagent | gpt-5.6-luna/max      | 327,150 |      167.010 |
| baseline_2 | parent   | gpt-6-astra/low       |  98,961 |       44.629 |

Jev itself used **1,411 tokens per run**, with measured evaluation duration **1.540 / 1.541 s**.
Those tokens are included in the whole-workflow totals above. Preflight tool calls also incur the
provider approval review; this is separate from evaluator duration. Guardian is a provider approval
session, not a delegated worker selected by the routing allowlist.

The native child alone took **192.894 / 167.010 s**, versus the complete baseline parent turns of
42.842 / 44.629 s. The Jev parents used 268,617 / 273,524 tokens to delegate, wait and report. Child
outputs included 6,770 / 5,461 reasoning tokens; both models produced passing code, but the child
performed more tool/model rounds. These observations explain where time/tokens accumulated; this
experiment does not isolate model, reasoning effort, context size, approval and delegation effects.

## Conclusion and limits

Use direct Astra/low for this measured class of bounded, explicit-spec work. Automatically treating
such work as a reason to create a Luna/max child is not supported by these results. Jev's fast
selection call does not make the complete delegation workflow fast or token-efficient.

Only one task and two runs per workflow were tested. AB/BA ordering reduces a simple order bias,
but does not control all server load, cache or model variability. Do not generalize this result to
large independent parallel tasks, every model, or paid-dollar savings. No Luna/low was used.

## Reproduction and artifacts

Runner: macOS Darwin arm64, Node 24.18.0, TypeScript 5.9.3, Codex 0.154.0, Paseo 0.8.0.
The unchanged runtime-source hashes, initial source and assessor hashes are in
[manifest.json](artifacts/native-benchmark-20260921/manifest.json). Exact prompts are in
[prompts.json](artifacts/native-benchmark-20260921/prompts.json); results, actor identities,
transcript paths, calls, usage and quality details are in each arm's `result.json` and `quality.json`.
[results.json](artifacts/native-benchmark-20260921/results.json) contains the summary. These local
artifacts are ignored by Git; this report records the conclusions independently.

From the repository root, regenerate the summary without new model calls:

```sh
node --import ./plugins/jev-orchestrator/node_modules/tsx/dist/loader.mjs plugins/jev-orchestrator/artifacts/native-benchmark-20260921/summarize.ts
```

All four Paseo test agents were archived, and their temporary role slots were removed. Production
source/config was not modified, no plugin reload was needed, and no commit/push was performed.
