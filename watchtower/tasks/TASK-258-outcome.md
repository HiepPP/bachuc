# TASK-258 Outcome

## Status

DONE (code). Screen checks are DEFERRED (UI checker).

## Changed

- `packages/desktop-gpui/crates/sidebar/src/prefs.rs` (new): `SidebarPrefs { view: ViewState }` with `parse`, `to_json`, `load_from`, `save_to`, `retain_projects`, and 7 tests.
- `packages/desktop-gpui/crates/sidebar/src/sidebar.rs`: `Sidebar` holds `prefs`, `data_dir`, `pruned`. Loads on create, saves in `toggle_project`, `toggle_pinned`, `toggle_group`. `refresh` prunes once.
- `packages/desktop-gpui/crates/sidebar/src/lib.rs`: `pub mod prefs`.
- `packages/desktop-gpui/crates/sidebar/Cargo.toml`: `serde_json` moved from dev-dependencies to dependencies.
- `packages/desktop-gpui/crates/app/src/shell.rs`: both `Sidebar` creations call `Sidebar::with_data_dir(client, Some(data_dir()), ...)`.
- `model.rs` unchanged: `ViewState` needs no serde, because `prefs.rs` maps it by hand.

## Contract

- `bachuc_sidebar::prefs::SidebarPrefs::load_from(dir: &Path) -> SidebarPrefs` and `save_to(&self, dir: &Path) -> std::io::Result<()>`: as specified.
- `SidebarPrefs` is a plain struct with one field `view: ViewState`. TASK-261 adds display fields beside `view`, and extends `parse` and `to_json`.
- File `sidebar.json` keys: `collapsedProjects` (sorted ids), `pinnedCollapsed`, `expandedGroups` (`"pinned"` or `"project:<id>"`). Each key falls back alone.
- `Sidebar::new` stays (in-memory only, used by `examples/sidebar.rs`). New `Sidebar::with_data_dir(client, Option<PathBuf>, window, cx)` persists.

## Verified

- `./cargo.sh test -p bachuc-sidebar` -> 9 lib tests (7 new), 1, 22, 32 pass; 0 fail.
- New tests: round trip of all three fields, same text for same prefs, missing/empty/bad JSON give defaults, bad field falls back alone, unknown keys ignored, unknown project ids and `Project(id)` groups dropped with `Pinned` kept, save to read-only dir returns `Err`.
- `./cargo.sh fmt --check -p bachuc-sidebar -p bachuc-app` -> exit 0.
- `./cargo.sh clippy -p bachuc-sidebar -p bachuc-app --all-targets`, `./cargo.sh build -p bachuc-app` -> no new warning from this TASK; the `kebab` argument-count warning at `sidebar.rs:2008` is older.
- Screen check (collapse, quit, relaunch -> still collapsed): DEFERRED (UI checker).
- Read-only-dir "sidebar keeps working": `save_prefs` logs `log::warn!` on `Err` and never returns it; read by code only.

## Anti-goal

Writes happen only in `save_prefs`, called from the 3 toggle functions and once from the first prune when it drops ids. `refresh`, scroll, and idle paths never write (read by code, `sidebar.rs` `save_prefs` call sites). The 30 s `stat -f %m sidebar.json` read: DEFERRED (UI checker).

## Lessons

- Keep the old `Sidebar::new` and add a `with_data_dir` constructor: the example and tests then never touch the owner's data dir.
- An empty project list proves nothing (it is also the state before the first answer), so the prune waits for a non-empty list.
- Another group's edits to `crates/app` (`actions.rs`, `main.rs`) made `fmt --check -p bachuc-app` fail for a while; it passed after they formatted.

## UI Checks

Lead, 2026-10-19, check bundle on live (6768), cua_repl:

- Collapse toggle wrote `sidebar.json` once (mtime 1792385669); a relaunch and idle did not rewrite it; the App id stayed after relaunch. PASS.
- The visual collapsed row after relaunch: UNVERIFIED (unattended), because the sidebar opens scrolled to the end and the App project has no rows.
