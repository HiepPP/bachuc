# TASK-240 Wider watchtower board

Group: B ([packages/desktop-gpui/crates/app/src/plugin_views.rs](packages/desktop-gpui/crates/app/src/plugin_views.rs), [packages/desktop-gpui/plugins/watchtower-board](packages/desktop-gpui/plugins/watchtower-board))
Class: mechanical

## Brief

Goal: The watchtower board and its TASK dialog use more of the window.

Change: overlay 92% wide, max 1180 px, 88% high, and dialog max 520 px -> overlay 96% wide, no max width, 94% high, and dialog max 820 px.

Boundaries:

- Owner chose both on 2026-10-18.
- `OVERLAY_WIDTH`, `OVERLAY_MAX_WIDTH`, `OVERLAY_HEIGHT` in [packages/desktop-gpui/crates/app/src/plugin_views.rs](packages/desktop-gpui/crates/app/src/plugin_views.rs) about lines 23-25 apply to every plugin overlay, not only the board. That is accepted.
- `CARD_WIDTH` in [packages/desktop-gpui/plugins/watchtower-board/src/decision_modal.rs](packages/desktop-gpui/plugins/watchtower-board/src/decision_modal.rs) about line 22.

Files:

- [packages/desktop-gpui/crates/app/src/plugin_views.rs](packages/desktop-gpui/crates/app/src/plugin_views.rs) (overlay size)
- [packages/desktop-gpui/plugins/watchtower-board/src/decision_modal.rs](packages/desktop-gpui/plugins/watchtower-board/src/decision_modal.rs) (dialog width)

Expected result:

- On a 1800 px window, the board is about 1730 px wide.
- The TASK dialog is up to 820 px wide.

Anti-goal: on a 900 px window, the overlay still leaves a visible margin of at least 16 px each side; read from a UI checker screenshot.

## Verify

- `cd packages/desktop-gpui && ./cargo.sh build -p bachuc-app` -> builds.
- UI checker at 1800 px: open the board -> width about 96% of the window.
- UI checker at 900 px: open the board and a TASK dialog -> both fit with a margin.
