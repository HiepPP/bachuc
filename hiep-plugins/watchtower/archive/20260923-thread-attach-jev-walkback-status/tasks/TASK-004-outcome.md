# TASK-004 Outcome

## Outcome

Status: DONE

Changed:
- `plugins/thread-context-attach/README.md` — added `## Jev mode (opt-in)` section (setting, status line, 1-3 reply walk-back, first-search-shows-plain behavior, what is sent to Vercel and its 4,000-char cut, `jev-evaluator` + Gateway key requirement, fallback to the current format on off/missing/failing/slow).

Contract: README matches shipped behavior in `server/snapshot.ts`, `server/threads.ts`, `server/jev.ts`, `server/jev-worker.mjs`, `shared/settings.ts`.

Verified:
- `npx oxfmt --check README.md` — exit 0, "All matched files use the correct format."
- `paseo plugin reload thread-context-attach` — reload succeeded.
- `paseo plugin ls thread-context-attach --json` — `"enabled": true`, `"status": "running"`, no error field present.
- Manual live check (turn setting on, open + → Thread twice, attach a thread, check `Status:` line and `paseo plugin logs` for one Jev usage line with no reply text) is PENDING-USER: needs the Paseo desktop app UI and makes a billable Jev call, which this agent cannot perform.

Group-level heavy checks (this is the only/last TASK in group C, so run once here):
- `npm run typecheck` — clean, no output/errors.
- `npm test` — 16/16 pass.
- `S=lint; npm run $S` — 0 warnings, 0 errors.
This is a group-level run; README.md is docs-only and not itself exercised by these commands, but they confirm the group's shipped code (which the README documents) is unbroken.
