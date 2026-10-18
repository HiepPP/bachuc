# TASK-204 Client git, PR, and script calls

Group: C (shared files [packages/desktop-gpui/crates/client/src/core.rs](packages/desktop-gpui/crates/client/src/core.rs), [lib.rs](packages/desktop-gpui/crates/client/src/lib.rs), [model.rs](packages/desktop-gpui/crates/client/src/model.rs))
Class: code

## Brief

Goal: The GPUI client can run every git, PR, branch, stash, and script request that the Electron git UI uses, and reports git status changes.

Change: the client only reads checkout status and diffs -> it also runs git actions, reads full git and PR status, switches branches, stashes, and starts scripts.

Boundaries:

- Request names and fields come from [packages/protocol/src/messages.ts](packages/protocol/src/messages.ts) (line numbers are near, within 1 or 2): `checkout_commit_request` (about line 2304; Electron sends `addAll: true`), `checkout_pull_request` (2329), `checkout_push_request` (2335), `checkout.refresh.request` (2341), `checkout_pr_create_request` (2354; Electron sends no title or body), `checkout_pr_merge_request` (2363; field `mergeMethod`), `checkout.forge.set_auto_merge.request` (2370), `checkout_merge_request` (2312; strategy `merge`, `requireCleanTarget: true`), `checkout_merge_from_base_request` (2321), `checkout_status_request` (2278), `checkout_pr_status_request` (2457), `pull_request_timeline_request` (2463), `checkout_switch_branch_request` (2479), `stash_save_request` (2493), `stash_pop_request` (2501), `stash_list_request` (2509), `start_workspace_script_request` (3070).
- Pull and push runs pull, then push, one after the other (`actions-store.ts` lines 252-256).
- Auto-merge uses the forge request. Use the `checkout.github.set_auto_merge.request` fallback only when `server_info` lacks the forge feature, as `actions-store.ts` lines 305-330 do.
- Status updates: add `checkout_status_update` (about line 5546) and `script_status_update` (line 4473) to `SESSION_EVENTS` (`core.rs` lines 266-273). The daemon sends them only when asked (`session.ts` lines 1390, 2670).
- Emit status changes as a new event type, `GitStatusEvent`, with `impl EventEmitter<GitStatusEvent> for Client`. Do not add a `DirectoryEvent` variant: `sidebar.rs` lines 231-233 and `examples/client_probe.rs` lines 45-48 match it exhaustively, and a new variant breaks them.
- `PrStatus` carries what the git policy needs: the `github` block (`messages.ts` lines 5442-5452: auto-merge request, viewer can enable or disable auto-merge, repository merge policy, `mergeStateStatus`), mapped through Electron's `merge-capability.ts` rules (`policy.ts` lines 53-69).
- The PR `number` is optional on the wire. `aheadBehind` can be null; map null to 0. PR timeline `createdAt` is a number (ms).
- Track workspace scripts from the workspace descriptor `scripts` field and `script_status_update` (`messages.ts` lines 4036-4048).
- Do not change `CheckoutStatus` or `CheckoutDetails`; other crates use them.

How:

- Add each request to `core.rs` with a `Pending::*` reply, and a `lib.rs` wrapper.
- Parse replies into the new model types.
- Add one test per call in `tests/rpcs.rs`, and parse tests for `GitStatus`, `PrStatus`, and the status update event.

Files:

- [packages/desktop-gpui/crates/client/src/core.rs](packages/desktop-gpui/crates/client/src/core.rs) (requests, replies, session events, status updates, scripts)
- [packages/desktop-gpui/crates/client/src/lib.rs](packages/desktop-gpui/crates/client/src/lib.rs) (wrappers, `GitStatusEvent` emitter)
- [packages/desktop-gpui/crates/client/src/model.rs](packages/desktop-gpui/crates/client/src/model.rs) (new types)
- [packages/desktop-gpui/crates/client/tests/rpcs.rs](packages/desktop-gpui/crates/client/tests/rpcs.rs) (tests)

Interfaces:

- Produces: `GitAction` enum `{ Commit, Pull, Push, PullAndPush, CreatePr, MergePr(MergeMethod), SetAutoMerge { enabled: bool, method: Option<MergeMethod> }, MergeBranch, MergeFromBase, Refresh }` for TASK-210 and TASK-212.
- Produces: `MergeMethod` enum `{ Merge, Squash, Rebase }` for TASK-210.
- Produces: `GitActionReply { url: Option<String>, number: Option<u64> }` for TASK-210.
- Produces: `Client::git_action(&mut self, cwd: &str, action: GitAction, cx: &mut Context<Client>) -> Task<anyhow::Result<GitActionReply>>` for TASK-210.
- Produces: `GitStatus { is_git: bool, branch: Option<String>, is_dirty: bool, base_ref: Option<String>, ahead: u32, behind: u32, ahead_of_origin: u32, behind_of_origin: u32, has_remote: bool, remote_url: Option<String>, is_paseo_owned_worktree: bool, has_upstream: bool }` for TASK-210 and TASK-211.
- Produces: `Client::git_status(&mut self, cwd: &str, cx: &mut Context<Client>) -> Task<anyhow::Result<GitStatus>>` for TASK-210.
- Produces: `PrMergeCapability { auto_merge_enabled: bool, auto_merge_method: Option<MergeMethod>, can_enable_auto_merge: bool, can_disable_auto_merge: bool, allowed_methods: Vec<MergeMethod>, merge_state_status: Option<String> }` for TASK-210.
- Produces: `PrStatus { number: Option<u64>, url: String, title: String, state: String, is_merged: bool, is_draft: bool, mergeable: Option<String>, checks: Vec<PrCheck>, review_decision: Option<String>, repo_owner: String, repo_name: String, merge: PrMergeCapability }` and `PrCheck { name: String, status: String, url: Option<String>, workflow: Option<String> }` for TASK-210, TASK-212, and TASK-202.
- Produces: `PrStatusReply { status: Option<PrStatus>, github_features_enabled: bool }` and `Client::pr_status(&mut self, cwd: &str, cx: &mut Context<Client>) -> Task<anyhow::Result<PrStatusReply>>` for TASK-210, TASK-212, and TASK-202.
- Produces: `PrTimelineItem` enum `{ Review { author: String, body: String, created_at_ms: i64, url: Option<String>, state: String }, Comment { author: String, body: String, created_at_ms: i64, url: Option<String>, path: Option<String>, line: Option<u32> } }` and `Client::pr_timeline(&mut self, cwd: &str, number: u64, repo_owner: &str, repo_name: &str, cx: &mut Context<Client>) -> Task<anyhow::Result<Vec<PrTimelineItem>>>` for TASK-212.
- Produces: `Client::switch_branch(&mut self, cwd: &str, branch: &str, cx: &mut Context<Client>) -> Task<anyhow::Result<()>>` for TASK-211.
- Produces: `StashEntry { index: u32, message: String, branch: Option<String>, is_paseo: bool }`, `Client::stash_save(&mut self, cwd: &str, branch: Option<&str>, cx: &mut Context<Client>) -> Task<anyhow::Result<()>>`, `Client::stash_pop(&mut self, cwd: &str, index: u32, cx: &mut Context<Client>) -> Task<anyhow::Result<()>>`, and `Client::stash_list(&mut self, cwd: &str, paseo_only: bool, cx: &mut Context<Client>) -> Task<anyhow::Result<Vec<StashEntry>>>` for TASK-211.
- Produces: `WorkspaceScript { name: String, kind: String, lifecycle: String, health: Option<String>, terminal_id: Option<String>, exit_code: Option<i32> }`, `Client::workspace_scripts(&self, workspace_id: &str) -> Vec<WorkspaceScript>`, and `Client::start_workspace_script(&mut self, workspace_id: &str, script_name: &str, cx: &mut Context<Client>) -> Task<anyhow::Result<String>>` (the terminal id) for TASK-202.
- Produces: `GitStatusEvent` enum `{ Changed { cwd: String }, ScriptsChanged { workspace_id: String } }` emitted by `Client`, for TASK-210, TASK-212, and TASK-202.

Expected result:

- Each new call sends the exact request type and fields, and parses its reply, proven by `tests/rpcs.rs`.
- A reply with `error` set gives `Err` with the daemon message.
- A `checkout_status_update` emits `GitStatusEvent::Changed`; a `script_status_update` emits `GitStatusEvent::ScriptsChanged`.
- `git_status` on a folder outside git gives `is_git: false` and no error. A null `aheadBehind` gives 0 and 0.
- `./cargo.sh build -p bachuc-app` and the `client_probe` example still build.

Anti-goal: the client crate test time must not grow by more than 20%; drift gauge; read `./cargo.sh test -p bachuc-client` wall time before the change and at completion.

## Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-client` -> passes, with a test per new call and the status event tests.
- `./cargo.sh test -p bachuc-client git_status` -> the non-git and null `aheadBehind` cases pass.
- `./cargo.sh build -p bachuc-app --examples` and `./cargo.sh build -p bachuc-client --examples` -> both build.
- Anti-goal: test wall time -> at most 120% of the earlier reading.
