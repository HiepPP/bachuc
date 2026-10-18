# Plan Context

## Shared Context

- Scope is the Rust GPUI client in [packages/desktop-gpui](packages/desktop-gpui) only. Do not change the Electron app in [packages/app](packages/app) or [packages/desktop](packages/desktop).
- The Electron app is the reference look and behavior. Read its source to measure, but do not edit it.
- Run cargo only through `packages/desktop-gpui/cargo.sh`. Run several `-p` crates in one cargo call, because each separate `-p` call builds its own gpui-kit copy (MEMORY 2026-10-17).
- GPUI views have no headless test context here. Put each rule (clamp, threshold, layout choice, key filter) in a pure function and test it in the crate `tests/` folder. Check view code by reading it, then by a UI checker screenshot.
- In a file with `use gpui_kit::*`, a test module with `use super::*` picks up GPUI's test macro. In such a file, put tests in the crate `tests/` folder. `bachuc-composer`, `bachuc-settings`, and `workspace-spaces` keep inline tests in files without that import; follow the crate's pattern.
- Give every pressable element an explicit `AccessibleAction::Click` handler (MEMORY, plugin port lessons).
- Do not toggle `display` from hover or `group_hover`; it panics with "must call prepaint before paint". Hide by opacity (MEMORY 2026-10-16).
- A flex child with a negative main-axis margin inside a content-sized container makes taffy measure the container at 0 width (MEMORY 2026-10-18).
- UI checks: build the check bundle under its own name, open it with `open -n -g --env BACHUC_NO_ACTIVATE=1 <bundle>`, and never take the cursor or the front. `cua_repl` cannot make a real scroll or swipe, so those are owner hand checks.
- After the last cargo check, run `./cargo.sh clean --profile dev` if no other cargo or rustc runs.

## Decisions

- New TASK IDs start at TASK-232 and Q-144. The owner chose to skip the archive ID migration on 2026-10-18.
- Overflow (TASK-232) runs first because it may touch any view crate. Dropdown (TASK-242) runs last because it touches files of groups S, C, W, and T.

## Open Decisions

- None.

## References

- [docs/plugins.md](docs/plugins.md) section "GPUI native clients"
- [packages/desktop-gpui/crates/theme/src/lib.rs](packages/desktop-gpui/crates/theme/src/lib.rs)
- [packages/desktop-gpui/crates/settings/src/appearance.rs](packages/desktop-gpui/crates/settings/src/appearance.rs)
