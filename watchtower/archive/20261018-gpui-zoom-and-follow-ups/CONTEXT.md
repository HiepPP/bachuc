# Plan Context

## Shared Context

- Scope is the Rust GPUI client in [packages/desktop-gpui](packages/desktop-gpui) only. Do not change the Electron app in [packages/app](packages/app) or [packages/desktop](packages/desktop). Read Electron source to measure.
- Lessons of the last plan: [watchtower/archive/20261018-gpui-polish-and-fixes/LEARN.md](watchtower/archive/20261018-gpui-polish-and-fixes/LEARN.md). Stay inside `Files:`; if a fix needs another file, record it in the outcome with a Q-ID.
- Run cargo only through `packages/desktop-gpui/cargo.sh`. Run several `-p` crates in one cargo call, because each separate call builds its own gpui-kit copy.
- The window rem size is 16 x zoom, not the 14 px body font. Convert a px gap to rems with `window_rems(zoomed(X, cx), rem_size(cx))`, as [crates/timeline/src/lib.rs](packages/desktop-gpui/crates/timeline/src/lib.rs) line 1599 does (MEMORY 2026-10-18).
- GPUI views have no headless test context here. Put each rule in a pure function and test it in the crate `tests/` folder. Check views by reading them, then by a UI checker screenshot.
- In a file with `use gpui_kit::*`, a test module with `use super::*` picks up GPUI's test macro. Put such tests in the crate `tests/` folder.
- Do not toggle `display` from hover; hide by opacity (MEMORY 2026-10-16).
- UI checks: build the check bundle under its own name, open it with `open -n -g --env BACHUC_NO_ACTIVATE=1 --env BACHUC_WINDOW=1200x800 <bundle>`, and never take the cursor or the front. `BACHUC_WINDOW` sets the window size ([crates/app/src/state.rs](packages/desktop-gpui/crates/app/src/state.rs) line 634).

## Zoom Conversion Rule (TASK-246 to TASK-251)

- Replace a fixed size `px(X)` with `bachuc_theme::zoomed(X, cx)`. This covers paddings, margins, gaps, icon sizes, control heights and widths, `text_size`, `line_height`, radii, and layout widths such as `MAX_CONTENT_WIDTH`.
- Keep `px(..)` for: 1 px hairlines and borders, a value already zoomed, a width the user dragged and saved, a width or position read from a GPUI layout or mouse event, and test code.
- When a helper has no `cx`, pass the zoom factor or a `&App` in; do not read a global another way.
- At zoom 100%, `zoomed(X, cx) == px(X)`, so the look at 100% must not change.
- Planner call: no new metric constants and no renames. Keep each crate's `metrics` names.

## Decisions

- New TASK IDs start at TASK-243 and Q-155. The owner chose to skip the archive ID migration again on 2026-10-18.
- Q-144, Q-148, and Q-151 were answered on 2026-10-18 (archived [QUESTIONS.md](watchtower/archive/20261018-gpui-polish-and-fixes/QUESTIONS.md)).
- Vendor patches follow [ADR-0044](watchtower/decisions/ADR-0044-vendored-gpui-base-patch-over-git-fork-or-log.md).

## Open Decisions

- None.

## References

- [packages/desktop-gpui/crates/theme/src/lib.rs](packages/desktop-gpui/crates/theme/src/lib.rs) (`zoom`, `zoomed`, `rem_size`)
- [watchtower/archive/20261018-gpui-polish-and-fixes/tasks/TASK-234-outcome.md](watchtower/archive/20261018-gpui-polish-and-fixes/tasks/TASK-234-outcome.md) (list of unscaled sites)
- [watchtower/archive/20261018-gpui-polish-and-fixes/tasks/TASK-233-outcome.md](watchtower/archive/20261018-gpui-polish-and-fixes/tasks/TASK-233-outcome.md) (list gap table)
