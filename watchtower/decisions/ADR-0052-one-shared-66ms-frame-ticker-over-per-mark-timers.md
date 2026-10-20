# ADR-0052 One shared 66 ms frame ticker over per-mark timers and 60 fps animation frames

Status: accepted
Date: 2026-10-19
Supersedes: -
Scope: `packages/desktop-gpui/crates/theme/src/motion.rs`, `packages/desktop-gpui/crates/timeline/src/motion.rs`, `packages/desktop-gpui/crates/sidebar/src/ring.rs`, `packages/desktop-gpui/plugins/*/src/**`, `packages/desktop-gpui/crates/explorer/src/**`

## Context

- GPUI read a median 42.6% CPU on main `74c6266ad` during a running turn, with the release plugin set, against the 10% cap of [ADR-0039](ADR-0039-incremental-timeline-sync-over-throttle-or-budget.md) (Q-175, [TASK-266 outcome](../tasks/TASK-266-outcome.md) Round 4).
- GPUI 0.3.9 marks every ancestor view of a notified view dirty, so each frame of any running mark re-renders the window tree that holds it (MEMORY learning of TASK-266 fix round 1).
- On main, the stop-orb face calls `window.request_animation_frame()` on every render while not cancelling (`packages/desktop-gpui/plugins/stop-orb/src/face.rs:105`), and each `RunningRing` runs its own 33 ms timer (`packages/desktop-gpui/crates/sidebar/src/ring.rs:31`). Out-of-phase timers coalesce only per platform frame.
- The timeline `Motion` marks already share one 66 ms ticker with quantized states (`packages/desktop-gpui/crates/timeline/src/motion.rs`).

## Decision

- Every repeating mark of the GPUI app redraws from one shared ticker at 66 ms (15 fps), with one shared clock. A frame whose quantized state did not change notifies nothing. The ticker sleeps when no mark was painted in the last 200 ms.
- The ticker lives in `bachuc-theme`, the one crate every GPUI crate and plugin crate depends on.
- Hiep chose this in the `new` interview on 2026-10-19.

## Consequences

- The stop orb drops from about 60 fps to 15 fps, and the sidebar ring from 30 fps to 15 fps.
- A new repeating animation must use the shared ticker. `window.request_animation_frame()` in a render, a per-view repeat timer, and `with_animation(..).repeat()` on a mark that shows during a turn break this decision.
- One-shot transitions, such as the sidebar slide and the composer drop fade, may keep `request_animation_frame` because they end.

## Options Considered

- One shared ticker at 33 ms (30 fps): rejected. It doubles the frame count, and each frame re-renders the window tree, so the cap of 10% is less likely to hold.
- Keep per-mark timers and `request_animation_frame`: rejected. It is the measured state of 42.6% on main.

## Revisit If

- A 60 s read with the release plugin set during a running turn stays at or below 5% with the 66 ms ticker, so a 33 ms rate may fit the cap.
- GPUI stops marking ancestor views dirty on a child notify, so a mark frame no longer re-renders the window tree.
