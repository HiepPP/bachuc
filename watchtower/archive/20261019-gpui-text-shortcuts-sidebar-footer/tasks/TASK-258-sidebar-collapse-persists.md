# TASK-258 Sidebar collapse state persists

Group: B (writes the sidebar crate and [packages/desktop-gpui/crates/app/src/shell.rs](packages/desktop-gpui/crates/app/src/shell.rs); shared files with TASK-261, TASK-262, TASK-263)
Class: code

## Brief

Goal: a collapsed project stays collapsed after the owner quits and reopens GPUI.

Change: `ViewState` lives only in memory -> it loads from and saves to `sidebar.json` in the GPUI data dir.

Boundaries:

- Cause: `ViewState { collapsed_projects, pinned_collapsed, expanded_groups }` ([packages/desktop-gpui/crates/sidebar/src/model.rs](packages/desktop-gpui/crates/sidebar/src/model.rs) lines 305-310) is created with `ViewState::default()` ([packages/desktop-gpui/crates/sidebar/src/sidebar.rs](packages/desktop-gpui/crates/sidebar/src/sidebar.rs) line 265) and never saved. Running Electron at the same time is not the cause: Electron keeps its own `sidebar-collapsed-sections` store, which GPUI never reads.
- Save all three fields. `collapsed_projects` holds GPUI `project.id` values; `expanded_groups` holds `Group` values (`Pinned` or `Project(id)`, [packages/desktop-gpui/crates/sidebar/src/model.rs](packages/desktop-gpui/crates/sidebar/src/model.rs) line 218).
- Add a `SidebarPrefs` type in a new `prefs.rs` with the `LayoutPrefs` pattern (see [watchtower/CONTEXT.md](watchtower/CONTEXT.md)). TASK-261 adds display fields to the same type.
- Save only on a toggle, never per frame.
- The first time the project list arrives, drop project ids and `Project(id)` groups whose project no longer exists. Keep `Pinned`.
- A save error logs a warning, like `save_layout` in [packages/desktop-gpui/crates/app/src/shell.rs](packages/desktop-gpui/crates/app/src/shell.rs) (lines 2941-2945). It never crashes.
- The sidebar crate gets the data dir from the shell, which imports `bachuc_command::data_dir` ([packages/desktop-gpui/crates/app/src/shell.rs](packages/desktop-gpui/crates/app/src/shell.rs) line 39).

Files:

- [packages/desktop-gpui/crates/sidebar/src/prefs.rs](packages/desktop-gpui/crates/sidebar/src/prefs.rs) (new: `SidebarPrefs`, `parse`, `to_json`, `load_from`, `save_to`)
- [packages/desktop-gpui/crates/sidebar/src/sidebar.rs](packages/desktop-gpui/crates/sidebar/src/sidebar.rs) (load on create, save on toggle)
- [packages/desktop-gpui/crates/sidebar/src/model.rs](packages/desktop-gpui/crates/sidebar/src/model.rs) (serde for `ViewState` if needed)
- [packages/desktop-gpui/crates/app/src/shell.rs](packages/desktop-gpui/crates/app/src/shell.rs) (pass `bachuc_command::data_dir()` to the sidebar)

Interfaces:

- Produces: `bachuc_sidebar::prefs::SidebarPrefs` with `load_from(dir: &Path) -> SidebarPrefs` and `save_to(&self, dir: &Path) -> std::io::Result<()>` for TASK-261

Expected result:

- Collapse a project, quit, reopen: the project is still collapsed. The same holds for Pinned and expanded groups.

Anti-goal: the sidebar writes `sidebar.json` at most once per toggle and 0 times while idle or scrolling; read the file mtime before and after 30 s of scrolling.

## Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-sidebar` -> new tests pass: round trip of all three fields; missing, empty, and bad JSON give defaults; unknown keys in the JSON are ignored; ids and `Project(id)` groups of unknown projects are dropped; a save to a read-only dir returns `Err` and the sidebar keeps working.
- Screen check (cua_repl, AX only): collapse a project, quit the check app, relaunch -> the project row is collapsed.
- Scroll the sidebar for 30 s -> `stat -f %m sidebar.json` is unchanged.
