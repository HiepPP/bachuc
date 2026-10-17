# ADR-0039 Incremental timeline sync over a throttle or a higher budget

Status: accepted
Date: 2026-10-16
Supersedes: -
Scope: `packages/desktop-gpui/crates/timeline/**`, `packages/desktop-gpui/crates/app/**`

## Context

- The GPUI app read 9.7% CPU on a 2-minute mock turn and 13.4% on a 15-minute turn, so the cost grew with the turn (Q-065).
- `TimelineView::sync` regrouped and hashed every entry on each 50 ms flush, and the turn header fingerprint hashed every step ([TASK-144 outcome](watchtower/archive/20261015-gpui-desktop-client/tasks/TASK-144-outcome.md)).
- The plan budget is 10% CPU and 250 MB, from [ADR-0034](ADR-0034-rust-gpui-client-over-swift-and-electron.md).

## Decision

- Keep per-flush timeline work in proportion to the change: `grouping::TurnGrouper` regroups only changed turns, and the view hashes only changed entries.
- Keep the 10% budget and the 50 ms flush.
- The owner chose this in the Q-065 interview on 2026-10-16. TASK-144 brought the 15-minute read to 8.9%.

## Consequences

- The incremental grouper must stay equal to a full `group_turns` regroup; `incremental_rows_match_full_regroup` guards it.
- A new row kind or grouping rule must update the grouper cache rules too.

## Options Considered

- Raise the app CPU budget to 15%: rejected. It hides a cost that keeps growing with the turn.
- Redraw the timeline at most every 100 to 150 ms while a turn streams: rejected. Streamed text looks less smooth ([docs/agent-stream-performance.md](docs/agent-stream-performance.md)).
- Render only the last N steps of a running turn and fold older steps: rejected. It breaks [ADR-0019](ADR-0019-always-open-step-list-over-folding-group.md).

## Revisit If

- A 30-minute mock turn reads over 10% CPU in the app.
- The client starts to report changed ranges (Q-068), so the view can drop its own change scan.
