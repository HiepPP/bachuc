# TASK-210 Git actions policy and split button

Group: E (shared file [packages/desktop-gpui/crates/explorer/src/lib.rs](packages/desktop-gpui/crates/explorer/src/lib.rs) and the explorer crate)
Class: risky

## Brief

Goal: A git split button with Electron's primary pick, secondary list, and enable rules, ready for the workspace header.

Change: no git actions in GPUI -> a `GitActionsButton` view that runs commit, pull, push, PR, and merge actions.

Boundaries:

- Port the policy from [packages/app/src/git/policy.ts](packages/app/src/git/policy.ts):
  - Action ids (lines 9-24): commit, pull, push, pull-and-push, pr, merge-pr-{squash,merge,rebase}, enable-pr-auto-merge-{squash,merge,rebase}, disable-pr-auto-merge, merge-branch, merge-from-base, archive-workspace.
  - Primary pick, first match wins (lines 315-357): archive when promoted; commit when dirty; pull; push; merge-pr; enable auto-merge; pr when auto-merge is on; pr when the ship default is pr; merge-branch when ahead of base; merge-from-base; view PR; archive for a Paseo-owned worktree.
  - Secondary list (lines 300-310) and enable rules (lines 512-546, 608-698). All actions are disabled when not git, while status loads, or while an action runs.
- A disabled item with an unavailable message still shows that message (`use-actions.tsx` lines 40-57).
- The overflow `menu` is always empty in Electron; do not build it.
- Ship default (pr or merge) is kept per repo in Electron (`use-actions.tsx` lines 352-393). Owner answer Q-111: save it to disk. The explorer crate does not write files: the button takes the saved value through `set_ship_default` and emits `ShipDefaultChanged` when the user picks; the app (TASK-202) saves it.
- Archive is out of this button for now: emit an event and let the app run its existing archive flow.
- Results: the button emits `GitActionsEvent::Finished` with a success or error message. The app shows it (TASK-202). The PR action with an existing PR emits `OpenUrl`.
- Refresh status after every action and on `GitStatusEvent::Changed` for this cwd. Coalesce a burst of events into one refresh per 500 ms.

How:

- Write `git_actions.rs` as pure policy functions over `GitStatus` and `PrStatusReply`, with tests that mirror Electron cases.
- Write `git_actions_ui.rs`: the split button, caret menu, spinner while pending.
- Add the `pub mod` lines to `lib.rs`.

Files:

- [packages/desktop-gpui/crates/explorer/src/git_actions.rs](packages/desktop-gpui/crates/explorer/src/git_actions.rs) (new: pure policy)
- [packages/desktop-gpui/crates/explorer/src/git_actions_ui.rs](packages/desktop-gpui/crates/explorer/src/git_actions_ui.rs) (new: the view)
- [packages/desktop-gpui/crates/explorer/src/lib.rs](packages/desktop-gpui/crates/explorer/src/lib.rs) (module lines and exports)

Interfaces:

- Consumes: `GitAction` enum `{ Commit, Pull, Push, PullAndPush, CreatePr, MergePr(MergeMethod), SetAutoMerge { enabled: bool, method: Option<MergeMethod> }, MergeBranch, MergeFromBase, Refresh }` from TASK-204.
- Consumes: `MergeMethod` enum `{ Merge, Squash, Rebase }` from TASK-204.
- Consumes: `GitActionReply { url: Option<String>, number: Option<u64> }` from TASK-204.
- Consumes: `Client::git_action(&mut self, cwd: &str, action: GitAction, cx: &mut Context<Client>) -> Task<anyhow::Result<GitActionReply>>` from TASK-204.
- Consumes: `GitStatus { is_git: bool, branch: Option<String>, is_dirty: bool, base_ref: Option<String>, ahead: u32, behind: u32, ahead_of_origin: u32, behind_of_origin: u32, has_remote: bool, remote_url: Option<String>, is_paseo_owned_worktree: bool, has_upstream: bool }` from TASK-204.
- Consumes: `Client::git_status(&mut self, cwd: &str, cx: &mut Context<Client>) -> Task<anyhow::Result<GitStatus>>` from TASK-204.
- Consumes: `PrStatusReply { status: Option<PrStatus>, github_features_enabled: bool }` and `Client::pr_status(&mut self, cwd: &str, cx: &mut Context<Client>) -> Task<anyhow::Result<PrStatusReply>>` from TASK-204.
- Consumes: `PrMergeCapability { auto_merge_enabled: bool, auto_merge_method: Option<MergeMethod>, can_enable_auto_merge: bool, can_disable_auto_merge: bool, allowed_methods: Vec<MergeMethod>, merge_state_status: Option<String> }` from TASK-204.
- Consumes: `PrStatus { number: Option<u64>, url: String, title: String, state: String, is_merged: bool, is_draft: bool, mergeable: Option<String>, checks: Vec<PrCheck>, review_decision: Option<String>, repo_owner: String, repo_name: String, merge: PrMergeCapability }` and `PrCheck { name: String, status: String, url: Option<String>, workflow: Option<String> }` from TASK-204.
- Consumes: `GitStatusEvent` enum `{ Changed { cwd: String }, ScriptsChanged { workspace_id: String } }` emitted by `Client`, from TASK-204.
- Produces: `GitActionsButton::new(client: Entity<Client>, cwd: String, cx: &mut Context<GitActionsButton>) -> GitActionsButton` for TASK-202.
- Produces: `ShipDefault` enum `{ Pr, Merge }` for TASK-202.
- Produces: `GitActionsEvent` enum `{ Finished { ok: bool, message: String }, OpenUrl { url: String }, ArchiveWorkspace, ShipDefaultChanged { cwd: String, default: ShipDefault } }` for TASK-202.
- Produces: `GitActionsButton::set_ship_default(&mut self, default: ShipDefault, cx: &mut Context<GitActionsButton>)` for TASK-202.

Expected result:

- Dirty tree -> primary is Commit. Clean, behind origin -> Pull. Clean, ahead of origin -> Push. Open PR, clean, mergeable -> Merge PR.
- Not a git folder -> the button is disabled.
- A failed push emits `Finished { ok: false }` with the daemon message.

Anti-goal: the explorer crate must not depend on the app crate; tripwire; `./cargo.sh tree -p bachuc-explorer` shows no `bachuc-app`, read before the change and at completion.

## Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-explorer git_actions` -> passes, with at least the 5 primary cases above.
- `./cargo.sh test -p bachuc-explorer git_actions` -> a status-loading case and a not-git case give every action disabled.
- `./cargo.sh build -p bachuc-explorer` -> builds.
- Anti-goal: `./cargo.sh tree -p bachuc-explorer | rg bachuc-app` -> no match at both reads.
