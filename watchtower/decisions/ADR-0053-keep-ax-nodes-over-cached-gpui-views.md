# ADR-0053 Keep AX nodes over cached GPUI views for the running-turn CPU

Status: accepted
Date: 2026-10-20
Supersedes: -
Scope: `packages/desktop-gpui/crates/**`, `packages/desktop-gpui/plugins/**`

## Context

- After the shared 66 ms ticker ([ADR-0052](ADR-0052-one-shared-66ms-frame-ticker-over-per-mark-timers.md)), GPUI read a median 14.9% CPU during a running turn against the 10% cap of [ADR-0039](ADR-0039-incremental-timeline-sync-over-throttle-or-budget.md) ([TASK-275 outcome](../tasks/TASK-275-outcome.md)). The sample shows whole-window layout per frame: `_platform_memmove` 132, taffy `compute_flexbox_layout` 64.
- In gpui-pre 0.3.9 a reused `.cached` view skips `window.a11y` in `reuse_prepaint` and `reuse_paint` (`window.rs:3996`, `:4061`; `view.rs:530-545`), and `a11y.rs:275` clears the AX nodes each frame, so the cached subtree loses its AX nodes and click actions while AX is on.
- The UI checker drives every screen check through AX (`cua_repl`), and MEMORY records the same AX loss from an earlier plan.

## Decision

- Do not use `.cached(..)` view boundaries in the GPUI client. Cut the per-frame cost by removing copies and allocations in render and layout paths instead.
- Hiep chose this in the TASK-276 planning ask on 2026-10-20.

## Consequences

- A mark frame still re-renders the window tree on its path; each render must stay cheap.
- AX automation and VoiceOver keep every node.

## Options Considered

- `.cached` views only when no AX client is active: rejected. No proven way to detect an active AX client in gpui-pre 0.3.9, and UI checks would measure a different build path than daily use.
- `.cached` views with AX loss accepted: rejected. It breaks the AX-driven UI checks and VoiceOver for the cached subtrees.
- Accept 14.9% and raise the cap: rejected here; ADR-0039 already rejects a higher budget.

## Revisit If

- A gpui upgrade keeps AX nodes for a reused cached view.
- TASK-276 cuts per-frame copies and the median still reads over 10%.
