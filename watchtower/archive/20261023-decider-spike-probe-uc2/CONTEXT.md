# Plan Context

## Shared Context

- Goal: measure if local Decider can replace or back hosted Jev in [hiep-plugins/plugins/jev-permission-gate](hiep-plugins/plugins/jev-permission-gate) (UC2). No UI work. UC1 (needs attention) is a later plan.
- Crate root: [packages/desktop-gpui/crates/decider-probe](packages/desktop-gpui/crates/decider-probe). The workspace `members = ["crates/*", ...]` in [packages/desktop-gpui/Cargo.toml](packages/desktop-gpui/Cargo.toml) picks it up. Run cargo only through `packages/desktop-gpui/cargo.sh`.
- Data dir: `.dev/decider-probe/` at the repo root. `.gitignore` already ignores `.dev/`. Command text never goes into git.
- Source of shell commands: `~/.bachuc/timeline-cache.sqlite`, table `timeline_rows`, rows with `item_type = 'tool_call'` and `json_extract(item, '$.detail.type') = 'shell'`. The command is `$.detail.command`. About 34,000 distinct commands exist on 2026-10-23. Open the file read-only; the release daemon writes it.
- Decider endpoint default: `http://127.0.0.1:8080/v1/decisions`, model `pplx-decider-flash-v1.2`. Endpoint comes from `--endpoint` so the Mac mini can run later. The server log is `~/Library/Application Support/ai.perplexity.decider/decider.log`; each request appends `decided questions=N images=0 passes=N input_tokens=T ms=M`.
- Decider question types are `noul`, `choice`, `score`. `boolean` is rejected (live check, 2026-10-23). A `noul` answer is `{"type":"noul","noul":<P(yes)>}`. A `choice` answer is `{"type":"choice","choice":<key>,"confidence":<number>,"probabilities":{<key>:<number>}}`. `input_tokens` comes from the response field `usage.input_tokens`.
- Cold start: the first requests after idle took 3,102 ms (87 tokens) and 2,721 ms (94 tokens) on 2026-10-23, against about 290 ms warm in the handoff. Latency stats must exclude a warm-up request.
- Jev types use `boolean` and `evaluateWithTypesafe` in [hiep-plugins/plugins/jev-evaluator/server/typesafe.mjs](hiep-plugins/plugins/jev-evaluator/server/typesafe.mjs). Each Jev call is billable.
- The gate rule to replay is `fromVerdict` in [hiep-plugins/plugins/jev-permission-gate/server/policy.ts](hiep-plugins/plugins/jev-permission-gate/server/policy.ts): deny when action is `deny` at P >= t; allow when action is `allow` at P >= t and readOnly is true with confidence >= t; else escalate. Production t is `THRESHOLD = 0.9`.

## Decisions

- Probe crate is standalone Rust; extraction and the Jev bridge are Node scripts that import `tier0` and `evaluateWithTypesafe`. See [ADR-0066](watchtower/decisions/ADR-0066-standalone-rust-decider-probe-over-jev-evaluator-backend.md).
- Questions are the gate's two questions, copied as version `uc2-v1`, with `boolean` renamed to `noul`.
- Decider gets one question per request. Jev gets both questions in one request, as the gate sends them.
- Hiep labels every event. An agent may pre-fill drafts; the human label is ground truth ([watchtower/QUESTIONS.md](watchtower/QUESTIONS.md) Q-190).

## Open Decisions

- None.

## References

- Decider docs: https://docs.perplexity.ai/docs/decisions/decider-for-mac
- Go criteria, three in total: (1) in the full gate view, auto-resolve >= 50% of events at precision >= 0.95; (2) beat the regex baseline: at the same threshold, the full gate auto-resolves more events than regex alone, with precision >= 0.95; (3) Decider p95 server latency per request <= 2,000 ms for state <= 400 tokens on the Air.
