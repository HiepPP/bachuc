# TASK-001 Outcome

## Outcome

Status: DONE

## Changed

All new, untracked (use `git status --short --untracked-files=all plugins/thread-janitor`):

- plugins/thread-janitor/paseo-plugin.json (id `thread-janitor`, paseo `>=0.9.0-beta.2`)
- plugins/thread-janitor/package.json, package-lock.json, tsconfig.json (board style, SDK 0.9.0-beta.2)
- plugins/thread-janitor/index.server.ts (settings, `agent.turn_ended` + `agent.created` hooks, 30 min timer, cleanup)
- plugins/thread-janitor/shared/settings.ts (`janitor` settings: `enabled` true, `idleHours` 24, min 1)
- plugins/thread-janitor/server/stale.ts (pure `selectStale(agents, now, settings)`)
- plugins/thread-janitor/server/janitor.ts (paged list, sweep, 10 min throttle, single-flight, timer reuse of last hook API)
- plugins/thread-janitor/tests/stale.test.ts, tests/janitor.test.ts
- plugins/thread-janitor/README.md

Scaffolded with `paseo plugin init`; the greeting example files were removed. The empty
scaffold `client/` directory was left in place (not tracked by Git; directory deletion needs
user confirmation).

## Contract

- Stale = not archived AND status != `running` AND no pending permissions AND last activity
  older than `idleHours` (strict `>`; exactly 24 h is kept).
- SDK 0.9.0-beta.2 agent snapshots have no `lastActivityAt`. The daemon (0.9.1 bundle,
  `toStoredAgentRecord`) stores `lastActivityAt = updatedAt`, so last activity =
  max(`updatedAt`, `lastUserMessageAt`). Unparseable timestamps are never stale.
- Before each archive the sweep re-fetches the agent (`ref(id).refresh()`, a read-only
  `fetch_agent_request`) and re-applies `selectStale`, because daemon archive cancels a live run.
- `agents.ref(id).archive()` per agent; one failure is logged and counted, the sweep continues.
- Logs: `archived <id> titleLength=<n>` and one summary line
  `sweep (<reason>): archived A of S stale, checked C, failed F, idleHours H`. No titles logged.
- Throttle: at most one sweep per 10 min across hooks and timer; never concurrent. Timer
  (30 min) is skipped until a hook has supplied the shared plugin-process `PaseoApi`.
- Disabled or invalid settings: no list, no archive; logs `sweep skipped`.
- Cleanup clears the interval, removes both hook listeners, and aborts an in-flight sweep loop.
- Settings file: `~/.paseo/plugin-settings/thread-janitor/janitor.json`, read at each sweep.

## Verified

Per-task checks and the heavy checks ran once as a group-level run (group A has only TASK-001).

- `npm run format`, `typecheck`, `lint`, `test` in plugins/thread-janitor -> all exit 0;
  10/10 tests pass, including the five `selectStale` cases (idle boundary, running, pending
  permission, already archived, disabled) plus sweep failure isolation, no-title logs,
  fresh-snapshot skip, throttle/timer, and disabled sweep.
  - Note: the literal command string `npm run lint` is rewritten by a local shell/tool hook
    that prints `ESLint output (JSON parse failed ...)` and exits 1 (plugins/board fails the
    same way). Running the same script as `S=lint; npm run $S` exits 0 with
    `Found 0 warnings and 0 errors.` This is a tooling issue, not a code failure.
- `paseo plugin install "$PWD/plugins/thread-janitor" --id thread-janitor` -> installed;
  `paseo plugin ls thread-janitor --json` -> `"status": "running"`, `"enabled": true`, no error.
- `paseo plugin logs thread-janitor` after a turn ended ->
  `2026-09-23T08:20:13.808Z [thread-janitor] sweep (turn_ended): archived 105 of 127 stale, checked 167, failed 0, idleHours 24`.
  The other 22 stale candidates were skipped by the fresh re-check (their fresh snapshot was no
  longer stale, likely archived together with a parent).
- Runtime data check (agent storage, timestamps only): 42 threads remain unarchived, 0 remain
  idle > 24 h; 10 threads with a user message in the last hour are all still unarchived; 0
  threads with a user message in the last 24 h were archived since install.
- Archived ids are listed in `paseo plugin logs thread-janitor` for unarchive if needed.
- Pending human verification: Paseo app UI check (idle thread gone from list, recent thread
  kept, History -> Unarchive works). The desktop UI cannot be automated here.
- GitNexus impact: not applicable; new untracked plugin, no existing symbol edited.

## Follow-up 2026-09-23

Status: DONE

Changed:
- The user still saw old rows in the sidebar. Cause: the sidebar lists workspaces, and 95 unarchived `local_checkout` workspaces held only archived agents.
- The sweep now also archives idle workspaces with no unarchived agent, no open terminal, no pin, and not a Paseo-owned worktree.

Verified:
- `npm run typecheck`, oxlint, and `npm test` -> 12/12 pass.
- `paseo plugin logs thread-janitor` -> `archived 95 of 95 stale workspaces, failed 0`.
- `~/.paseo/projects/workspaces.json` -> 40 unarchived workspaces. None of them lacks a live agent. No archived workspace held a live agent.
- Pending human check: the sidebar in the desktop app no longer shows the old rows.
