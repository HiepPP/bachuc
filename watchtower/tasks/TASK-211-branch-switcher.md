# TASK-211 Branch switcher

Group: E (shared file [packages/desktop-gpui/crates/explorer/src/lib.rs](packages/desktop-gpui/crates/explorer/src/lib.rs) and the explorer crate)
Class: code

## Brief

Goal: The branch label in the Changes header becomes a searchable branch switcher with Electron's stash flow.

Change: a passive branch label (`explorer.rs` lines 1236-1278) -> a combobox that switches branches.

Boundaries:

- Electron: [packages/app/src/components/branch-switcher.tsx](packages/app/src/components/branch-switcher.tsx) and `use-branch-switcher.ts`. It hides when there is no current branch (line 76). It loads branches when it opens, limit 200, kept for 15 s (lines 55-75). Picking the current branch does nothing.
- If the switch error contains "uncommitted", confirm "stash and switch": `stash_save` with the current branch name, then switch (lines 121-150, 160-163).
- After a switch, list Paseo stashes for the target branch. If one exists, confirm "restore stash?" and run `stash_pop` (lines 93-119).
- No create-branch: Electron has none, and the daemon has no create request.
- `Client::branch_suggestions` already exists ([packages/desktop-gpui/crates/client/src/lib.rs](packages/desktop-gpui/crates/client/src/lib.rs) line 860).
- Planner call: build the two confirms as a small inline dialog in the explorer crate. No app dialog exists.

How:

- Add `branch_switcher.rs` with the view and a pure function for the stash decision, with tests.
- Replace the label in `render_changes_header` with the switcher.

Files:

- [packages/desktop-gpui/crates/explorer/src/branch_switcher.rs](packages/desktop-gpui/crates/explorer/src/branch_switcher.rs) (new)
- [packages/desktop-gpui/crates/explorer/src/explorer.rs](packages/desktop-gpui/crates/explorer/src/explorer.rs) (use the switcher in the Changes header)
- [packages/desktop-gpui/crates/explorer/src/lib.rs](packages/desktop-gpui/crates/explorer/src/lib.rs) (module line)

Interfaces:

- Consumes: `Client::switch_branch(&mut self, cwd: &str, branch: &str, cx: &mut Context<Client>) -> Task<anyhow::Result<()>>` from TASK-204.
- Consumes: `StashEntry { index: u32, message: String, branch: Option<String>, is_paseo: bool }`, `Client::stash_save(&mut self, cwd: &str, branch: Option<&str>, cx: &mut Context<Client>) -> Task<anyhow::Result<()>>`, `Client::stash_pop(&mut self, cwd: &str, index: u32, cx: &mut Context<Client>) -> Task<anyhow::Result<()>>`, and `Client::stash_list(&mut self, cwd: &str, paseo_only: bool, cx: &mut Context<Client>) -> Task<anyhow::Result<Vec<StashEntry>>>` from TASK-204.
- Consumes: `GitStatus { is_git: bool, branch: Option<String>, is_dirty: bool, base_ref: Option<String>, ahead: u32, behind: u32, ahead_of_origin: u32, behind_of_origin: u32, has_remote: bool, remote_url: Option<String>, is_paseo_owned_worktree: bool, has_upstream: bool }` from TASK-204.

Expected result:

- Click the branch -> a searchable list of up to 200 branches.
- Pick a branch on a clean tree -> the branch changes and the label updates.
- Pick a branch on a dirty tree -> the stash confirm shows; yes stashes and switches; no keeps the branch.
- Switch back to a branch with a Paseo stash -> the restore confirm shows.

Anti-goal: opening the Changes tab must not get slower; drift gauge; branches load only when the switcher opens, proven by no `branch_suggestions_request` in the daemon log on tab open, read before the change and at completion.

## Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-explorer branch` -> passes, including the stash decision tests.
- UI check on live: switch branches on a clean test repo -> the label updates.
- UI check on live: switch on a dirty test repo, accept the stash -> the switch succeeds; switch back -> the restore confirm shows.
- Anti-goal: open the Changes tab, then grep the live `daemon.log` for `branch_suggestions_request` -> none until the switcher opens.
