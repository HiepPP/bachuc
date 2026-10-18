# TASK-202 Git actions, PR tab, Scripts, and Open in editor in the app

Group: S (shared files [packages/desktop-gpui/crates/app/src/shell.rs](packages/desktop-gpui/crates/app/src/shell.rs) and [chrome.rs](packages/desktop-gpui/crates/app/src/chrome.rs))
Class: code

## Brief

Goal: The workspace header shows Scripts, Open in editor, and the git split button, a toast reports results, and a PR tab opens once by itself when the workspace has a PR.

Change: the header has only the menu and the explorer toggle (`chrome.rs` line 248 `render_header`) -> Electron's header buttons and the PR tab.

Boundaries:

- Header order in Electron: plugin buttons, Scripts, Open in editor, git actions, explorer toggle (`workspace-screen.tsx` lines 3775-3807). Desktop only, and only for a workspace with a directory.
- Toast: GPUI has no toast host. Add a small one in the app: success and error variants, about 3.2 s, stacked at the bottom right. Show `GitActionsEvent::Finished` messages with Electron's strings (`workspace.git.actions.*`).
- PR tab: add `PaneContent::PullRequest` and update `view()`, `owns_focus()`, `kind()`. Owner choice: open the tab once by itself when a PR is first found for the workspace, as `workspace-screen.tsx` lines 1773-1783 do. Remember per workspace that it was opened, so a closed PR tab does not come back in that session.
- Scripts button: Electron [packages/app/src/screens/workspace/workspace-scripts-button.tsx](packages/app/src/screens/workspace/workspace-scripts-button.tsx). Refresh it on `GitStatusEvent::ScriptsChanged`. Show it only when the workspace has scripts. Start sends `start_workspace_script`; stop uses the existing `kill_terminal`. Show each script's lifecycle state.
- Open in editor: Electron [packages/app/src/workspace/open-in-editor/button.tsx](packages/app/src/workspace/open-in-editor/button.tsx) uses the desktop bridge, not the daemon. In GPUI, open the workspace folder with the macOS `open` command for a chosen app (VS Code, Cursor, Zed, Finder, Terminal: list only apps that exist under `/Applications`), plus "Open on GitHub" from the remote URL. Remember the last choice in memory.
- Ship default (owner answer Q-111): save `GitActionsEvent::ShipDefaultChanged` per repo cwd in the app state on disk ([packages/desktop-gpui/crates/app/src/state.rs](packages/desktop-gpui/crates/app/src/state.rs) already writes state). When a button is made, pass the saved value with `set_ship_default`. A missing or unreadable value means `Pr`.
- `GitActionsEvent::OpenUrl` and `PullRequestEvent::OpenUrl` open the URL with `cx.open_url`. `GitActionsEvent::ArchiveWorkspace` runs the existing workspace archive flow with its confirm.

How:

- Add the toast host.
- Mount the three header buttons in `render_header`.
- Add the PR pane kind and the auto-open rule as a pure function with a test.

Files:

- [packages/desktop-gpui/crates/app/src/shell.rs](packages/desktop-gpui/crates/app/src/shell.rs) (PR pane kind, auto-open, event handlers)
- [packages/desktop-gpui/crates/app/src/chrome.rs](packages/desktop-gpui/crates/app/src/chrome.rs) (header buttons)
- [packages/desktop-gpui/crates/app/src/toast.rs](packages/desktop-gpui/crates/app/src/toast.rs) (new: toast host)
- [packages/desktop-gpui/crates/app/src/main.rs](packages/desktop-gpui/crates/app/src/main.rs) (module line)
- [packages/desktop-gpui/crates/app/src/state.rs](packages/desktop-gpui/crates/app/src/state.rs) (ship default per repo, saved to disk)

Interfaces:

- Consumes: `GitActionsButton::new(client: Entity<Client>, cwd: String, cx: &mut Context<GitActionsButton>) -> GitActionsButton` from TASK-210.
- Consumes: `ShipDefault` enum `{ Pr, Merge }` from TASK-210.
- Consumes: `GitActionsEvent` enum `{ Finished { ok: bool, message: String }, OpenUrl { url: String }, ArchiveWorkspace, ShipDefaultChanged { cwd: String, default: ShipDefault } }` from TASK-210.
- Consumes: `GitActionsButton::set_ship_default(&mut self, default: ShipDefault, cx: &mut Context<GitActionsButton>)` from TASK-210.
- Consumes: `PullRequestView::new(client: Entity<Client>, cwd: String, cx: &mut Context<PullRequestView>) -> PullRequestView` from TASK-212.
- Consumes: `PullRequestEvent` enum `{ OpenUrl { url: String } }` from TASK-212.
- Consumes: `PrStatusReply { status: Option<PrStatus>, github_features_enabled: bool }` and `Client::pr_status(&mut self, cwd: &str, cx: &mut Context<Client>) -> Task<anyhow::Result<PrStatusReply>>` from TASK-204.
- Consumes: `GitStatusEvent` enum `{ Changed { cwd: String }, ScriptsChanged { workspace_id: String } }` emitted by `Client`, from TASK-204.
- Consumes: `WorkspaceScript { name: String, kind: String, lifecycle: String, health: Option<String>, terminal_id: Option<String>, exit_code: Option<i32> }`, `Client::workspace_scripts(&self, workspace_id: &str) -> Vec<WorkspaceScript>`, and `Client::start_workspace_script(&mut self, workspace_id: &str, script_name: &str, cx: &mut Context<Client>) -> Task<anyhow::Result<String>>` from TASK-204.
- Produces: `Shell::show_toast(&mut self, ok: bool, message: String, cx: &mut Context<Self>)` for TASK-201 and TASK-203.

Expected result:

- A git workspace header shows the split button; a dirty tree shows Commit as primary.
- Commit succeeds -> a success toast; the button refreshes.
- A workspace with an open PR -> a Pull request tab opens once; closing it keeps it closed.
- A workspace with scripts -> the Scripts button lists them; Start runs one and shows its state.
- Open in editor -> the folder opens in the chosen app.

Anti-goal: workspace switch time must not grow; drift gauge; sidebar workspace click to the first painted tab stays under 200 ms, read with log timestamps before the change and at completion.

## Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-app` -> passes, including the auto-open rule test and a ship default save-and-load test with a missing value giving `Pr`.
- UI check on live in a scratch repo: edit a file, press Commit -> a toast shows and Changes empties.
- UI check on live: a workspace with scripts -> Start a script, and its state shows.
- UI check on live: shrink the window to 900 px wide -> the header buttons do not overlap the workspace title; labels may hide, as Electron's `hideLabels` does.
- Owner hand test: push, open a PR, and merge from the header on a real repo.
- Anti-goal: workspace click to first tab paint -> under 200 ms at both reads.
