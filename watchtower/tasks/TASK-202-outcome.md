# TASK-202 Outcome

## Status

DONE. The UI checks, the owner hand test, and the anti-goal timing are DEFERRED.

## Changed

- `packages/desktop-gpui/crates/app/src/toast.rs` (new): `ToastStack` (push, dismiss, at most 3 notes, a blank message shows nothing) and `render_toasts`. Notes stack at the bottom right with Electron's toast look: `surface0`, a 1 px border (`destructive` for an error), 16 px radius, medium shadow, an 18 px `CircleCheck` or `TriangleAlert` icon, and 480 px maximum width. `TOAST_DURATION` is 3.2 s.
- `packages/desktop-gpui/crates/app/src/main.rs`: `mod toast;`.
- `packages/desktop-gpui/crates/app/src/state.rs`:
  - `load_ship_default` and `save_ship_default`: the ship default per repo cwd in `git-ship-defaults.json` in the app data folder, written through a temp file and a rename. A missing file, entry, or bad value gives `Pr`.
  - `EditorTarget` (VS Code, Cursor, Zed, Finder, Terminal), `editor_targets`, `open_args`, and `github_url`.
  - `script_state` (the state text of a script row) and `pull_request_key`.
- `packages/desktop-gpui/crates/app/src/shell.rs`:
  - `PaneContent::PullRequest(Entity<PullRequestView>)` in `view()`. `owns_focus()` stays false, so the tab gets a `PaneFrame`. `kind()` gives `TabKind::Other`.
  - `OpenWorkspace` keeps the workspace's `GitActionsButton`, made on its first open with the saved ship default. `GitActionsEvent`: `Finished` shows a toast and checks the PR after a success. `OpenUrl` calls `cx.open_url`. `ArchiveWorkspace` runs the existing archive flow with its confirm, for that workspace (`archive_workspace_id`). `ShipDefaultChanged` saves to disk.
  - PR auto-open: `check_pull_request` asks `pr_status` on a workspace's first open, on `ClientEvent::Connected`, after a successful git action, and on `GitStatusEvent::Changed` for its folder (at most once per 10 s). The pure rule `auto_opens_pr_tab` opens the tab once, behind the active tab. `pr_tab_opened` remembers each workspace for the session, so a closed PR tab stays closed. `PullRequestEvent::OpenUrl` calls `cx.open_url`.
  - `GitStatusEvent::ScriptsChanged` for the selected workspace redraws the header.
  - `Shell::show_toast`, and the toast layer in `render`. A host switch clears the PR, menu, and script state.
- `packages/desktop-gpui/crates/app/src/chrome.rs`: `render_header` adds Scripts, Open in editor, and the git split button between the tab menu and the Explorer toggle, only for a workspace with a folder. Each trigger, menu row, and row action has `Role::Button` or `Role::MenuItem`, an aria label, and a click handler plus an `AccessibleAction::Click` handler (`pressable`). The menus are own popovers (`header_popover`), because the gpui-kit dropdown trigger has no AX press.
  - Scripts: shown only when the workspace has scripts. The play icon is blue while a script runs, with labels hidden as Electron's `hideLabels` does. Each row shows the icon (globe for a service, terminal for a script, colored by state), the name, the state (`running`, `running, healthy`, `exit N`, `stopped`), and Run or Stop. Run sends `start_workspace_script`. Stop calls `kill_terminal`. A failure shows Electron's toast text.
  - Open in editor: the primary opens the folder in the last pick of the session, else the first listed app. The caret lists the apps with a check on the pick, plus "Open on GitHub" from the remote URL. The macOS `open` command runs on the background executor. A failure shows "Failed to open workspace".

## Contract

- Produces: `Shell::show_toast(&mut self, ok: bool, message: String, cx: &mut Context<Self>)`, `pub(crate)`.
- Every "Consumes:" line is used with its exact name: `GitActionsButton::new`, `ShipDefault`, `GitActionsEvent`, `GitActionsButton::set_ship_default`, `PullRequestView::new`, `PullRequestEvent`, `PrStatusReply` with `Client::pr_status`, `GitStatusEvent`, `WorkspaceScript`, `Client::workspace_scripts`, and `Client::start_workspace_script`.
- `create_workspace` keeps its signature. Its only change is the new `git_actions: None` field. The button and the PR check are made in `open_workspace`'s first-open branch.
- `on_client_event` keeps the `ClientEvent::TimelineChanged { .. }` early return before any new code.

## Verified

- `CARGO_BUILD_JOBS=4 ./cargo.sh build -p bachuc-app` -> exit 0. The only warning is the existing unused `next_theme` in `state.rs`.
- `CARGO_BUILD_JOBS=4 ./cargo.sh test -p bachuc-app` -> 70 passed, up from 64. New tests: `shell::tests::the_pr_tab_opens_once_for_the_first_pr_found` (the auto-open rule), `state::tests::the_ship_default_saves_per_repo_and_a_missing_value_is_pr` (save and load, a missing, unreadable, and bad value give `Pr`), `state::tests::open_in_editor_lists_installed_editors_and_builds_open_args`, `state::tests::a_github_remote_gives_its_branch_page`, `state::tests::a_script_row_shows_its_lifecycle`, and `toast::tests::toasts_stack_drop_the_oldest_and_leave_by_id`. `state::tests::the_shell_refreshes_logos_only_from_the_theme_observer` passes.
- `./cargo.sh fmt --check -p bachuc-app` -> exit 0.
- `./cargo.sh clippy -p bachuc-app --tests` -> no warnings in the changed files. The existing warnings are in `plugins.rs`, `plugin-host`, and the `bachuc-protocol` build script.
- `./cargo.sh metadata --format-version 1 > /dev/null` -> exit 0.
- GitNexus impact (index 39 commits stale, advisory): `create_workspace` CRITICAL (30 transitive callers), so its signature and behavior stay unchanged. `archive_workspace` LOW, and it keeps its signature. `render_header` and `PaneContent` UNKNOWN; `rg` finds one `render_header` caller, in `Render for Shell`.
- UI check, commit from the header in a scratch repo: a toast shows and Changes empties. DEFERRED (UI checker).
- UI check, start a script and see its state: DEFERRED (UI checker).
- UI check, a 900 px window: the header buttons do not overlap the workspace title. DEFERRED (UI checker). The title group has `flex_1` and `min_w_0` and truncates. Scripts and Open in editor show icons only.
- Owner hand test, push, open a PR, and merge from the header: DEFERRED (owner).
- Anti-goal, workspace click to the first tab paint under 200 ms at both reads: DEFERRED (UI checker). No "before" read was taken. A first open now adds a button entity, three async requests (`git_status`, `pr_status` from the button, and `pr_status` from the shell), and one small file read. A later open adds nothing.

## Lessons

- These views could use `Shell::show_toast` later in place of inline errors: the branch switcher's error notice (TASK-211), the PR view's failed Refresh line (TASK-212), and the shell's `window.push_notification` calls, such as copy notices and reload errors.
- The shell asks `pr_status` apart from the git button, because the button's PR state is private. A public PR getter or event on `GitActionsButton` would remove one request per check.
- Safe defaults to check: Finder and Terminal are always listed, because they are system apps outside `/Applications`. "Open on GitHub" covers github.com remotes only, and it loads when the menu opens. The toast host sits at the bottom right, as the spec says; Electron puts toasts at the top center. All notes last 3.2 s; Electron's success toasts last 2.2 s.
- The Scripts menu has no service URL rows, View terminal, or Restart, and it does not open the terminal tab after a start.
- A header menu closes on `on_mouse_down_out`. A press on its own trigger while open can close and then open it again, as in `GitActionsButton`. The UI checker should confirm the toggle.
- The git split button does not stop mouse-down propagation, so the header wraps it in a div that does. Without it, a press on the button starts a window drag.
