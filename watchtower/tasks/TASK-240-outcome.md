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

## UI Checks

- Line 33 (1800 px, board width about 96%): PASS at 1710 px. The window cannot be set to 1800, so the zoom action gave 1710 px. The Watchtower overlay card spans 40 to 1959 of 2000 screenshot px (96.0%) and 36 to 1147 of 1184 (93.8%, about 94%). `/tmp/wt-ui/chk-240-wt-1710.png`.
- Line 34 (900 px, board and a TASK dialog fit): UNVERIFIED (unattended): window width 900 cannot be set through AX, and no workspace on live has a `watchtower/NEXT.md`, so there is no TASK dialog to open. By constant, 4% of 900 is 18 px per side.
