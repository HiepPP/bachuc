# ADR-0014 TASK chips only over plan state on Board

Status: accepted
Date: 2026-10-12
Supersedes: ADR-0012
Scope: `hiep-plugins/plugins/board/**`

## Context

- ADR-0012 kept Board on Running and Just finished, and added a Needs you column, search, keys, and one plan line per project. That Board work was never committed, and on 2026-10-12 the owner chose not to rebuild it.
- The Watchtower page already shows TASK status, deps, blockers, questions, checks, and a timeline per plan.
- Board's own job is triage across every project and host: what runs, what just finished, what to remove. A card that waits for input already shows a warning badge.

## Decision

- Board keeps two status columns, Running and Just finished, per host lane.
- Its only Watchtower link is the TASK chip, read from the `watchtower.task` agent label, on cards and subagent rows. The chip opens Watchtower on the thread's workspace at that TASK.
- Board reads no plan files. It has no plan line, no Needs you column, and no plan counts.
- The Timeline tab looks back over 24 hours; Runs stays the triage view.

## Consequences

- Plan depth, questions, and checks live only on the Watchtower page.
- Board needs no copy of the Watchtower plan parser.

## Options Considered

- Task lanes, ADR-0010: rejected. Measured on live, most cells were empty and the grid repeated the Watchtower page.
- A plan line, a Needs you column, search, and keys on Board, ADR-0012: rejected. The owner dropped the work; the Watchtower page and the card badges cover it.
- A Gantt as Board's main view: rejected. Bars drop star, Remove, model, and subagents; the Timeline tab covers looking back.

## Revisit If

- The owner misses a plan question or a waiting thread because Board shows no plan counts.
- A plan regularly runs 3 or more TASKs at once in one project, so the chip and the Watchtower page no longer show what each thread is doing.
- The Watchtower page is removed or stops showing TASK status.
