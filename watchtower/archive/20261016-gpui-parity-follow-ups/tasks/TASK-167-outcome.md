# TASK-167 Outcome

## Status

DONE

## Changed

- `packages/desktop-gpui/crates/app/src/shell.rs`: subscribes to `ExplorerDiffEvent` and opens `DiffView::new_all` and `DiffView::new_commit` tabs (`open_all_diffs`, `open_commit_diff`, one shared `open_diff_tab`); sets each tab's kind and agent logo in `put_pane`; subscribes to `TabMenuEvent` (`on_tab_menu`); subscribes to `TerminalEvent::Opened` to set the terminal kind; `refresh_logos` and `complete_logos`; `workspace.pane.maximize` runs `toggle_maximize`. `OpenWorkspace` holds a `Vec<Subscription>`. `ExplorerEvent` handling is unchanged.
- `packages/desktop-gpui/crates/app/src/state.rs`: `all_diffs_key`, `commit_diff_key`, `commit_diff_title`, `file_of_key`, `agent_icon`, `LogoFill`, and 4 tests.
- `packages/desktop-gpui/crates/app/src/actions.rs`: `menu` and `shift-f10` bound to `OpenTabMenu` in the workspace key context; 1 test.
- `main.rs` is unchanged: `load_appearance` already runs at startup and now applies fonts and the syntax theme (TASK-160). `Cargo.toml` and `Cargo.lock` are unchanged.

## Contract

- Identities: all-files `diffs:<cwd>` (one tab per folder), commit file `commit:<sha>:<absolute path>`, so a second open shows the open tab. Titles: "Changes" and `<file> (<sha7>)`. Icons: `FileDiff` and `GitCommitHorizontal`.
- `set_item_kind` runs on every open: `Agent { id }` for agent and lazy agent tabs, `Terminal { id }` (id empty until `TerminalEvent::Opened`, which sets it again), `File` for file tabs, `Other` otherwise.
- `set_item_logo` runs on every open from the agent provider (`provider_logo` with the `foreground` token); an agent tab with no logo shows `provider_fallback_icon` (`agent_icon`).
- Logo refresh runs only from (1) the tab open, (2) the `Theme` observer, and only when `tokens.foreground` changed (`LogoFill::changed`), and (3) `complete_logos`, which sets the logo once for an agent tab that opened before its agent was in the list.
- Tab menu: Copy agent id, Copy terminal id, Copy file path, and Rename (agents) are done. Copy resume command, Reload agent, and terminal Rename show a "not available yet" notification (see Questions).
- Maximize: the toolbar button is inside `Workspace`; the shell also runs `toggle_maximize` for the action id `workspace.pane.maximize` (no shortcut in the table uses it).

## Verified

- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test -p bachuc-app` -> exit 0, 31 passed, 0 failed (26 existing, including `every_focus_target_in_this_crate_has_a_role`, plus 5 new).
- `packages/desktop-gpui/cargo.sh fmt --check -p bachuc-app` -> exit 0.
- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh build --release -p bachuc-app` -> exit 0, Finished `release`.
- A click on a changed file or a commit file opening a tab, the Claude logo drawn, and the menu key opening the menu: UNVERIFIED (unattended): code tests only.

## Anti-goal

- Checkpoint: the existing 26 app tests pass (31 passed in total), `cargo.sh test -p bachuc-app`, 2026-10-16, local.
- Checkpoint: test `the_shell_refreshes_logos_only_from_the_theme_observer` asserts one `.refresh_logos(` call site, inside the `Theme` observer, and no logo code before the `TimelineChanged` early return. `the_logos_rebuild_only_when_the_fill_changes` asserts a font, size, or syntax write that leaves the fill does not rebuild. The shell still does not redraw on `TimelineChanged`.

## Lessons

- Both `Explorer` and `Workspace` have two `EventEmitter` impls, so every `subscribe_in` names its event type in the closure.
- `Theme` is written by font, size, and syntax setters too, so the logo refresh compares the fill instead of rebuilding on every write.
- The test for `menu` and `shift-f10` uses `Keystroke::parse` and the binding predicate; both key names parse in GPUI.

## Questions

- Safe default: Copy resume command and Reload agent notify "not available yet". The client has no provider session id (`AgentSummary` lacks it) and no `refresh_agent` request. A client TASK must add both.
- Safe default: terminal Rename notifies "cannot be renamed yet". The client has no terminal rename request.
- A second Changes click with another focus file only shows the open all-files tab. `DiffView` has no public call to scroll to a file after it opens; add one in the explorer crate to scroll to the focus file.
