# TASK-300 Run the probe and write results

Group: C (writes `.dev/decider-probe/**` and `packages/desktop-gpui/crates/decider-probe/RESULTS.md`)
Class: code

## Brief

Goal: Run both backends on the 150 labeled events and record the go or no-go result for UC2 on the Air.

Change: no measurement -> `report.md` (ignored) and a committed `RESULTS.md` with numbers only.

Boundaries:

- Blocked until Hiep answers [Q-190](watchtower/QUESTIONS.md): `labels.jsonl` must exist with no `null` field.
- Keep the Mac awake and on power. Do not run other heavy jobs during `run`, because latency is part of the result.
- Jev costs money: run `jev-run.ts` once on the full set. Do not rerun it after a crash; it resumes.
- `RESULTS.md` holds numbers, the host (MacBook Air M4, 24 GB), Decider model, question version, and date. It holds no command text. The top-10 errors list stays in the ignored `report.md`.
- Planner call: when a go criterion fails on latency only, write that the Mac mini rerun is next. When it fails on accuracy, write that the hosted 27B is next, as the handoff says.

How:

- Run the commands in the order of [packages/desktop-gpui/crates/decider-probe/README.md](packages/desktop-gpui/crates/decider-probe/README.md): `run` for Decider, `jev-run.ts` for Jev, then `report`.
- Copy the go/no-go table and the summary numbers from `report.md` into `RESULTS.md`.

Files:

- `.dev/decider-probe/results-decider.jsonl`, `results-jev.jsonl`, `report.md` (generated, ignored)
- [packages/desktop-gpui/crates/decider-probe/RESULTS.md](packages/desktop-gpui/crates/decider-probe/RESULTS.md) (new)

Interfaces:

- Consumes: `.dev/decider-probe/labels.jsonl`, one line per event = `{"id":"e001","uc":"uc2","label":"allow"|"deny"|"escalate","read_only":boolean}` from TASK-298
- Consumes: `.dev/decider-probe/events.jsonl`, one line per event = `{"id":"e001","uc":"uc2","state":{"policy":string,"command":string,"tool":"Bash","cwd_relative":"."},"meta":{"tier0":"allow"|"deny"|"escalate"|null,"agent_id":string,"ts":string}}` from TASK-298
- Consumes: result line = `{"backend":"decider"|"jev","id":"e001","question_id":"readOnly"|"action"|"both","question_version":"uc2-v1","input_tokens":number|null,"client_ms":number,"server_ms":number|null,"answers":{"readOnly"?:{"p_true":number},"action"?:{"choice":"allow"|"deny"|"escalate","probabilities":{"allow":number,"deny":number,"escalate":number}}},"error":string|null}` from TASK-298
- Consumes: `scripts/jev-run.ts`, which writes `.dev/decider-probe/results-jev.jsonl`, from TASK-298
- Consumes: `.dev/decider-probe/results-decider.jsonl` with the result line shape above, `backend` `"decider"`, and `question_id` `"readOnly"` or `"action"`. from TASK-299
- Consumes: `.dev/decider-probe/report.md`. from TASK-299
- Consumes: commands `./cargo.sh run -p decider-probe -- run --data <dir>` and `./cargo.sh run -p decider-probe -- report --data <dir>`, run from `packages/desktop-gpui`. from TASK-299

Expected result:

- WHEN the runs finish THE results files SHALL hold 300 Decider lines and 150 Jev lines, with `error` `null` on at least 98% of lines.
- WHEN `report` runs THE `RESULTS.md` file SHALL state, per backend, the values of the three go criteria and pass or fail for each.

Anti-goal: the share of Decider lines whose `client_ms` exceeds `server_ms` by more than 200 ms stays below 5%; drift gauge; read from `results-decider.jsonl` at completion. A larger gap means the client or the machine, not the model, sets the latency.

## Verify

- `jq -s length .dev/decider-probe/results-decider.jsonl` -> 300; `jq -s length .dev/decider-probe/results-jev.jsonl` -> 150.
- `jq -s 'map(select(.error!=null))|length' .dev/decider-probe/results-decider.jsonl` -> 6 or fewer.
- `RESULTS.md` -> has a go/no-go table for Decider and Jev with the three criteria from [watchtower/CONTEXT.md](watchtower/CONTEXT.md).
- `rg -F -f <(jq -r .state.command .dev/decider-probe/events.jsonl) packages/desktop-gpui/crates/decider-probe/RESULTS.md` -> no match.
- Anti-goal: `jq -s 'map(select(.server_ms!=null and .client_ms-.server_ms>200))|length' .dev/decider-probe/results-decider.jsonl` -> below 15.
- Human check: Hiep reads `report.md` and the 10 most confident wrong answers, and agrees with the go or no-go call.
