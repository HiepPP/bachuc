# TASK-240 outcome

Status: DONE

Changed:

- `packages/desktop-gpui/crates/app/src/plugin_views.rs`: `OVERLAY_WIDTH` 0.92 -> 0.96, `OVERLAY_HEIGHT` 0.88 -> 0.94, removed `OVERLAY_MAX_WIDTH` and its `.max_w`.
- `packages/desktop-gpui/plugins/watchtower-board/src/decision_modal.rs`: `CARD_WIDTH` 520 -> 820 (doc comment updated).

Contract: overlay is 96% wide, no max width, 94% high; dialog max 820 px.

Verified:

- `CARGO_BUILD_JOBS=4 ./cargo.sh build -p bachuc-app` -> builds (one dead_code warning, not in these files).
- `cargo.sh fmt --check` on bachuc-app and the board crate -> clean.
- UI checker lines: DEFERRED (UI checker).

Anti-goal: at 900 px, 2% per side = 18 px margin (>= 16 px) by arithmetic (source: constant 0.96). No clamp added. Screenshot read: DEFERRED (UI checker). Below ~800 px the margin drops under 16 px.

Lessons: gpui has no calc, so a pixel margin floor needs a different layout; the percentage margin was enough at 900 px.
