# TASK-108 Outcome

## Status

DONE

## Changed

- `hiep-plugins/plugins/watchtower-board/client/plan-tree.ts` (new)
- `hiep-plugins/plugins/watchtower-board/tests/plan-tree.test.ts` (new)

## Contract

`buildPlanTree({ tasks, run, questions }, started)` returns `{ waves, pace }`.

- `waves[]`: `{ number, header, rows, done, cyclic }`. Rows keep Tracker order.
- `rows[]`: `{ id, title, group, kind, time, over }`. `kind` is `done | running | planned | blocked`, from the Tracker status. `time` is `4m`, `12m / 9m`, `~10:24`, or `null`.
- `pace`: `{ averageMinutes, finishClock, maxParallel, slowest }`, taken from `buildTimeline` (`stats`, `finishMinute`).
- Wave = 1 + highest in-plan dep wave. Deps outside the plan and duplicate TASK IDs are ignored. TASKs in or behind a cycle go in one last wave with `cyclic: true`.
- Headers: wave 1 `N groups in parallel`; later waves `starts when TASK-X and TASK-Y finish` (not-done in-plan deps only); `ready to start` when no dep is left; cyclic wave `waits on a dependency cycle`.

## Verified

- `npx tsx --test tests/plan-tree.test.ts` -> 15 pass, 0 fail.
- `npm run typecheck` -> exit 0.
- `npm run lint` -> exit 0, 0 warnings, 0 errors.
- `npm run format` -> ran; `git status` shows no change outside the new files and the other builder's files.

## Anti-goal

Tripwire: 200 chained TASKs build in under 20 ms. Checkpoint `build time`, value 1.3 ms (test run), source `tests/plan-tree.test.ts` 200-TASK case, time 2026-10-14. Not crossed.

## Lessons

- Safe defaults taken where the spec is silent: "N groups" counts distinct Group values, and a TASK with no group counts as its own group; singular is `1 group`. A planned row has no time when `started` has no parsable clock (a date-only `started` counts from midnight like `timeline.ts`).
- `rtk` wraps `npm run lint` output and prints a misleading "ESLint output (JSON parse failed)" line; the exit code and `oxlint` output are clean.
- The editor diagnostics for the new test were stale until the module existed; `npm run typecheck` is the source of truth.
