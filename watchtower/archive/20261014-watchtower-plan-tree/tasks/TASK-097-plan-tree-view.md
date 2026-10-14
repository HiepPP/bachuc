# TASK-097 Plan tree view

Group: C (writes only the new view file)
Class: code

## Brief

Goal: Draw the plan tree of mock D v2 as a React Native component that takes the model of TASK-095 and the builders of TASK-096.

Change: No tree view exists -> `PlanTree` draws the lead row, the waves on a spine, and one row per TASK.

Design: mock D v2, see Shared Context in [watchtower/CONTEXT.md](watchtower/CONTEXT.md).

Boundaries:

- Lead row: a small square icon, "Lead agent, implement", one line "<model>, running for 14m, waiting on 2 builders", and Open thread. With no lead, show the plan title and the run state from the autorun log, with no Open thread button ([Q-036](watchtower/QUESTIONS.md), answered 2026-10-14).
- Wave: a header ("Wave 1" plus its text from TASK-095) and rows hanging off one vertical spine. A done wave collapses to its header with a "Show" toggle.
- Row: a state icon (done check, running spinner, waiting hand, planned dashed circle), the TASK ID in monospace, the title, one secondary line, and the time text on the right in monospace.
- Secondary line: done "Builder A committed <sha>" when the log names a commit, else "Group A, code"; running "Builder C on <model>, active 30s ago"; waiting "Builder D asks permission"; planned "Group E, code".
- A waiting row has an amber tint and an Open thread button. Use the plugin theme: running uses the accent color, waiting uses the warning color, everything else uses foreground and muted colors. No fixed hex values.
- Open thread uses the same open-agent call the page already uses for links to agents.
- Below 520 px, drop the spine and stack waves as plain lists.
- Pressing a row selects that TASK in the left rail, as the task list does.
- Use the existing `MONO` font constant and spacing from the page. No new dependency.

How:

- Build `client/plan-tree-view.tsx` with `PlanTree({ tree, builders, lead, narrow, theme, onSelectTask, onOpenAgent })`.
- Keep layout logic out of render where it can be tested, and add pure helpers to `client/plan-tree.ts` only through TASK-095's file owner (do not edit it here).

Files:

- [hiep-plugins/plugins/watchtower-board/client/plan-tree-view.tsx](hiep-plugins/plugins/watchtower-board/client/plan-tree-view.tsx) (new)

Expected result:

- With the Fleet map plan data, the view shows three waves, four rows in wave 1, and the TASK-089 row amber with Open thread when its builder waits.
- A done wave is collapsed and opens with Show.
- Narrow width shows the stacked list.

Anti-goal: The view does not add work per frame. Tripwire: the spinner is the only animation, and it stops under reduced motion; read from the code at the end of the TASK.

## Verify

- `cd hiep-plugins/plugins/watchtower-board && npm run typecheck` -> exit 0.
- `cd hiep-plugins/plugins/watchtower-board && npm run lint` -> exit 0.
- `rg -n "#[0-9A-Fa-f]{6}" hiep-plugins/plugins/watchtower-board/client/plan-tree-view.tsx` -> no match.
- Anti-goal: `rg -n "Animated|useNativeDriver|requestAnimationFrame" hiep-plugins/plugins/watchtower-board/client/plan-tree-view.tsx` -> only the spinner, gated by reduced motion.
- Needs a human or a browser session: checked through TASK-098 on live.
