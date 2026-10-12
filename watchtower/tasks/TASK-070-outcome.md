# TASK-070 Outcome

## Outcome

Status: DONE

Changed:

- `shared/lanes.ts`: `needsYouItems(runs)` returns only running threads that wait for input; new `planCounts(plan)` gives "N question(s) · N check(s)" and whether a question blocks a TASK.
- `client/needs-you.tsx`: input items only, each opens its thread; the Watchtower action is gone.
- `client/plan-line.tsx`: shows the counts after `done/total`, amber when a question blocks a TASK.
- `client/page.tsx`: wiring; the `onOpenWatchtower` path is gone.
- `tests/lanes.test.ts`: input-only and count cases. `README.md`: plan line and Needs you bullets, linking ADR-0013.

Contract:

- Follows ADR-0013. The plan line still opens Watchtower on the plan's newest workspace.

Verified:

- `npm run typecheck` -> exit 0; `npm run-script lint` -> 0 warnings, 0 errors; `npm test` -> 128 pass, 0 fail.
- Live check, 2026-10-12, Expo web on daemon 6768, light theme: Needs you held 1 item, the thread "Watchtower target check" waiting on a permission; Q-024 was not listed. The bachuc plan line read "10/13 · 1 question · Watchtower". Screenshot `/tmp/nyu-1.png`.
- Pressing the Needs you item opened that thread (`/workspace/wks_a70c2b9cc272dae9`, title "Watchtower target check").
- Pressing the plan line opened Watchtower with `p.workspaceId=wks_a70c2b9cc272dae9`.

Anti-goal:

- Before: `needsYouItems` returned 3 items for a plan with 1 question and 1 check, 2 of them not threads; `tests/lanes.test.ts`, 2026-10-12.
- Final: 1 item, the thread; 0 non-thread items; `tests/lanes.test.ts`, 2026-10-12.
- Result: PASS against the tripwire of 0.

Lessons:

- `npm run cli -- run` without `--workspace` creates a new workspace each time; pass `--workspace` to keep live test agents together.
