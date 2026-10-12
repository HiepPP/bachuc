# ADR-0010 Task lanes over status columns on Board

Status: superseded by ADR-0012
Date: 2026-10-12
Supersedes: -
Scope: `hiep-plugins/plugins/board/**`

## Context

- Board showed two status columns, Running and Just finished, per host. It did not show which Watchtower TASK a thread worked on.
- Autorun labels its subagents `watchtower.task=TASK-NNN`, and Board already reads agent labels in `hiep-plugins/plugins/board/server/snapshot.ts`.
- The owner compared three designs in the Claude Design file `Board Watchtower Options.dc.html`: 1a light touch, 1b plan band, and 1c task lanes.

## Decision

- Each project is one grid: TASK rows of its active plan, then one Ad-hoc row, across Todo, Running, Finished, and Verified. Needs you is a left column.
- Ad-hoc threads use Running and Finished only; Remove hides them as before.
- The owner chose this on 2026-10-12, as design 2a.

## Consequences

- Board's server reads `watchtower/NEXT.md`, `QUESTIONS.md`, and `RUN.md` for each project.
- A project without a plan is a single Ad-hoc row, so Board keeps one layout.
- Opening Watchtower on the right workspace needs `params` in `openSurface`.

## Options Considered

- 1a, a Needs you strip and a plan line over the old columns: rejected. Each autorun iteration stays a separate card, so a 6-iteration run fills Just finished.
- 1b, a plan band with autorun iterations folded into one card: rejected. It still hides which TASK each thread served, which the owner wants to trace.
- 1c as first drawn, with lanes only for plan projects: rejected. Board would switch between two layouts per project.

## Revisit If

- A project's grid needs more than 30 rows on screen at once, so the lanes stop fitting a desktop window.
- Watchtower stops labelling agents with `watchtower.task`.
