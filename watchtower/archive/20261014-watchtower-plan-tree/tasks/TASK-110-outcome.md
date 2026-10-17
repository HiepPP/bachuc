# TASK-110 Outcome

## Status

DONE (live check UNVERIFIED, unattended)

## Changed

- `hiep-plugins/plugins/watchtower-board/client/plan-tree-view.tsx` (new)

## Contract

`PlanTree({ tree, builders, lead, narrow, theme, onSelectTask, onOpenAgent, planTitle?, runState?, classes?, commits? })`.

- `tree` is the `buildPlanTree` result. `builders` is `matchBuilders(...).byTask`. `lead` is `matchBuilders(...).lead`.
- Draws a lead row, then one block per wave: a header ("Wave N" plus the TASK-108 text) and one row per TASK on a left spine. A done wave starts collapsed; a Show/Hide toggle opens it. Other waves have no toggle.
- Row: state icon (done check, accent spinner, amber hand, dashed circle; blocked uses an alert icon), mono TASK ID, title, secondary line, mono time on the right. Pressing the row calls `onSelectTask(id)`.
- A running row whose builder state is `waiting` gets the amber tint and an Open thread button that calls `onOpenAgent(builder.id)`. The lead row has Open thread only when a lead exists.
- `narrow` drops the spine and the connector ticks, and stacks the waves as plain lists.
- No-lead (Q-043 default): the lead row shows `planTitle` and `runState`, with no button.
- Optional extras, because the model and builder types carry no such data: `planTitle`, `runState` (lead row without a lead), `classes` (TASK ID to Tracker class, for "Group A, code"), `commits` (TASK ID to short hash). The file exports the pure helper `commitShas(log)`, which finds a 7+ hex hash with a digit in each run-log row's `detail` or `result`.
- The builder `model` and the lead's run duration are not in `RunAgent`, so the lines read "Builder C, active 30s ago" and "<title>, running, active 1m ago, waiting on 2 builders" with no model or duration.
- Caller to do (page task): pass `classes` from the overview tasks, `commits` from `commitShas(run.log)`, and `onOpenAgent` from `navigation.openAgent({ agentId, serverId })`.

## Verified

- `npm run typecheck` -> exit 0.
- `npx oxlint --disable-nested-config index.server.ts index.client.tsx server client shared tests` (the `lint` script body) -> exit 0, 0 warnings, 0 errors. `npm run lint` is intercepted in this shell by an ESLint wrapper (see TASK-109 outcome); not a code failure.
- `npm run format` -> `git status` shows only `client/plan-tree-view.tsx` changed; `npx oxfmt --check` on it passes.
- `rg -n "#[0-9A-Fa-f]{6}" client/plan-tree-view.tsx` -> no match.
- `rg -n "Animated|useNativeDriver|requestAnimationFrame" client/plan-tree-view.tsx` -> only the `Spinner` component (lines 53 to 72) and the import.
- Live check (three waves, four rows in wave 1, TASK-089 amber): UNVERIFIED (unattended).

## Anti-goal

Tripwire: the spinner is the only animation and it stops under reduced motion. Checkpoint `animation sites`, value 1 (`Spinner`, `client/plan-tree-view.tsx`), source `rg` above, 2026-10-14. Checkpoint `reduced motion gate`, value: `useReducedMotion` starts `true` (static) and the loop effect returns early while it is `true`; it follows `AccessibilityInfo` `reduceMotionChanged`. Source: read from the code at the end of the TASK. Not crossed. There is no per-frame state, timer, or `requestAnimationFrame`; one native-driver loop per running row (JS driver on web).

## Lessons

- Editor diagnostics lagged and showed stale `Cannot find name` errors after the edit; `npm run typecheck` (exit 0) is the source of truth.
- A row's amber tint needs `overflow: hidden`, which clips an absolutely positioned spine tick, so the tick sits on an outer wrapper.
- Waiting applies only to a running row: a Group builder matches every TASK of its group, and a planned sibling must not turn amber.
