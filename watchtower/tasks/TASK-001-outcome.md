# TASK-001 Outcome

## Outcome

Status: DONE

Changed:

- Added [hiep-plugins/plugins/watchtower-board/shared/overview.ts](hiep-plugins/plugins/watchtower-board/shared/overview.ts) with `overviewSchema` and `readOverviewRpc` (`watchtower.overview.read`).
- Added `readOverview` and `parseManualChecks` in [hiep-plugins/plugins/watchtower-board/server/overview.ts](hiep-plugins/plugins/watchtower-board/server/overview.ts). It reuses `readBoard` for tasks, then reads `NEXT.md` for plan meta, Tracker groups, Plan Verify bullets, and manual checks.
- Exported `inside`, `readFile`, `plain`, and `tableRows` from [hiep-plugins/plugins/watchtower-board/server/board.ts](hiep-plugins/plugins/watchtower-board/server/board.ts), with no behavior change.
- Added `loadWorkspaceOverview` to [hiep-plugins/plugins/watchtower-board/server/handlers.ts](hiep-plugins/plugins/watchtower-board/server/handlers.ts) and registered the RPC in [hiep-plugins/plugins/watchtower-board/index.server.ts](hiep-plugins/plugins/watchtower-board/index.server.ts).
- Added [hiep-plugins/plugins/watchtower-board/tests/overview.test.ts](hiep-plugins/plugins/watchtower-board/tests/overview.test.ts) with 6 tests.
- Outside the spec: added `--disable-nested-config` to the plugin `lint` script in [hiep-plugins/plugins/watchtower-board/package.json](hiep-plugins/plugins/watchtower-board/package.json). Without it, `npm run lint` failed before this TASK, because oxlint read the root `.oxlintrc.json` as a nested config. The `pr-watch` plugin uses the same flag.

Contract:

- `watchtower.read` and `boardSchema` are unchanged.
- `watchtower.overview.read` returns `plan`, `tasks` (with `depIds` and `group`), `planVerify`, `manualChecks`, `message`, and `warnings`. A missing `NEXT.md` gives an empty plan with a `message`.

Verified:

- `npx tsx --test tests/overview.test.ts` -> 6 pass, 0 fail.
- `npx tsx --test tests/board.test.ts` -> 10 pass, 0 fail.
- `npm test` in the plugin folder -> 21 pass, 0 fail.
- `npm run typecheck` -> exit 0. `npm run lint` -> exit 0, 0 warnings, after the script fix.
- `rg -n "watchtower.overview.read" shared/overview.ts` -> one match, line 34.

Anti-goal:

- Before the first edit: 10 of 10 pass; `npx tsx --test tests/board.test.ts`, 2026-10-07 12:33.
- After the `board.ts` export change: 10 of 10 pass; same command, 2026-10-07 12:33.
- Final: 10 of 10 pass; same command, 2026-10-07 12:34.
- Result: PASS against 10 of 10 passing.

Lessons:

- The `watchtower-board` lint script needs `--disable-nested-config`, like `pr-watch`. Source: `npm run lint` output, "options.typeAware option is only supported in the root config".
- `readBoard` with `{ runState: false }` gives tasks without reading `QUESTIONS.md`, `RUN.md`, or `DECISIONS.md`. Source: [hiep-plugins/plugins/watchtower-board/server/board.ts](hiep-plugins/plugins/watchtower-board/server/board.ts).
