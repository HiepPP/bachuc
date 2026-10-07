# TASK-003 Outcome

## Outcome

Status: DONE

Changed:

- Added [hiep-plugins/plugins/watchtower-board/client/lifecycle.ts](hiep-plugins/plugins/watchtower-board/client/lifecycle.ts) with `lifecyclePhases`, `needsYou`, `phaseNames`, and the `Phase` and `NeedsYouRow` types. It reuses `plural` from [hiep-plugins/plugins/watchtower-board/client/dashboard.ts](hiep-plugins/plugins/watchtower-board/client/dashboard.ts).
- Added [hiep-plugins/plugins/watchtower-board/tests/lifecycle.test.ts](hiep-plugins/plugins/watchtower-board/tests/lifecycle.test.ts) with 8 tests.

Contract:

- The current phase follows `ARCHIVED` -> Archive, no tasks -> Plan, `Finished:` set -> Review, every task `DONE` -> Verify, otherwise Implement. Earlier phases are `done`; later ones are `todo`.
- A question is blocking when its status is `OPEN` and `blocks` is not empty. A question with a default other than `none` gets the `warning` tone.
- Needs you rows: `kind` (`question`, `adr`, or `checks`), `id`, `title`, `meta`, and `tone`. The checks row has the id `manual-checks`.

Verified:

- `npx tsx --test tests/lifecycle.test.ts` -> 8 pass, 0 fail.
- `npm test` in the plugin folder -> 34 pass, 0 fail.
- `npm run typecheck` -> exit 0. `npm run lint` -> exit 0, 0 warnings.
- `rg -n "react-native|from \"react\"" client/lifecycle.ts` -> no match.

Anti-goal:

- Final: 0.0193 ms average over 100 runs on a 200-task overview with 40 questions and 50 checks; `t.diagnostic` in the timing test, 2026-10-07 12:45.
- Result: PASS against 5 ms.

Lessons:

- The implement phase progress bar can reuse `dashboardSummary` from [hiep-plugins/plugins/watchtower-board/client/dashboard.ts](hiep-plugins/plugins/watchtower-board/client/dashboard.ts); it counts tasks by status without React.
