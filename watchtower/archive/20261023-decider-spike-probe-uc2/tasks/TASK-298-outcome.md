# TASK-298 Outcome

## Status

DONE

## Changed

- `packages/desktop-gpui/crates/decider-probe/scripts/lib.ts` (new): `isSecret`, `sample` (mulberry32 Fisher-Yates).
- `packages/desktop-gpui/crates/decider-probe/scripts/lib.test.ts` (new): the red tests, unchanged.
- `packages/desktop-gpui/crates/decider-probe/scripts/extract.ts` (new).
- `packages/desktop-gpui/crates/decider-probe/scripts/jev-run.ts` (new).
- `packages/desktop-gpui/crates/decider-probe/questions.json` (new), generated from `policy.ts` `POLICY` and `QUESTIONS`.
- Data (ignored, not staged): `.dev/decider-probe/events.jsonl`, `labels.draft.jsonl`, `timeline-snapshot.sqlite`.

## Contract

- `questions.json`, `events.jsonl`, `labels.draft.jsonl` and the Jev result line match the Interfaces in the spec. `meta.tier0` holds only `.decision`.
- Result lines: `backend` `"jev"`, `question_id` `"both"`, `server_ms` `null`, `readOnly.p_true`.
- `jev-run.ts` skips ids already in `results-jev.jsonl`; `PROBE_LIMIT` slices the first N events.
- Env: `PROBE_DB`, `PROBE_JEV_CONFIG`, `PROBE_LIMIT`.

## Verified

- Red: `lib.test.ts` before `lib.ts` existed -> fail (missing module). After: 3 pass.
- `extract.ts` -> 150 events, 120 with `tier0` null. Two runs on the same DB -> identical sha (`6565c420...`).
- Secret `rg` over commands -> no match.
- `questions.json`: `readOnly.type` `"noul"`; `readOnly.instructions`, `policy`, and `action` equal `policy.ts`.
- Temp db (long, secret, newline/quote command, short pools) -> long and secret commands absent, newline command round-trips through `jq`, pool warnings printed.
- `PROBE_JEV_CONFIG=/nonexistent/config.json PROBE_LIMIT=1` -> one line, `error` set, `answers` `{}`; line deleted.
- `PROBE_LIMIT=3` -> 3 lines, `backend` `"jev"`, input_tokens 737-770; second run added 0 lines.
- `npm run lint` on the 4 `.ts` files -> exit 0. `npm run format:files` -> clean.
- UI checker: none required.

## Anti-goal

0 committed files hold a command from `events.jsonl`. `git status --short` -> `M watchtower/NEXT.md` (not mine), `?? packages/desktop-gpui/crates/decider-probe/`; no `.dev/` path. `git diff --cached --name-only` -> empty. Read before and at completion (2026-10-23). PASS.

## Lessons

- The live `~/.bachuc/timeline-cache.sqlite` grows during agent runs, so a plain rerun gives a different sample (new commands appear). Determinism holds only for the same DB content. The probe set was extracted from the frozen copy `.dev/decider-probe/timeline-snapshot.sqlite`: `PROBE_DB=.dev/decider-probe/timeline-snapshot.sqlite node --import ./hiep-plugins/plugins/jev-permission-gate/node_modules/tsx/dist/loader.mjs packages/desktop-gpui/crates/decider-probe/scripts/extract.ts`. Rerun extract with that `PROBE_DB`.
- `results-jev.jsonl` from the 3-event check was deleted because the event set changed after it; the file is absent. The check showed Jev answers on the first 3 events: allow, p_true 0.94-0.95, 238-697 ms.
- `jev-run.ts` cannot use top-level await: the crate folder has no `"type": "module"`, so tsx compiles it as CJS. It uses `main()`.
- The `rtk` wrapper hides clean oxlint output and prints an ESLint JSON parse message; use `command npm run lint ... > file; echo $?`.
