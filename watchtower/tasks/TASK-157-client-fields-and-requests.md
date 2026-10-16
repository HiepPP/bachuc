# TASK-157 Client fields and requests for the follow-ups

Group: A (writes `crates/client`, and `crates/protocol` only for the feature decode)
Class: risky

## Brief

Goal: Give the follow-up views every daemon field and request they need, without changing any existing public item.

Change: the client drops `requiresAttention`, `projectKey`, `diffStat`, `pinnedAt`, `isGit`, and the per-file `isNew`/`isDeleted`/`oldPath`, and has no agent features, no all-files diff, and no commits -> accessors and requests that expose them.

Design: the daemon fields in [packages/protocol/src/messages.ts](packages/protocol/src/messages.ts) (see References in [watchtower/CONTEXT.md](watchtower/CONTEXT.md)); the Electron project view key in [packages/app/src/projects/workspace-structure.ts](packages/app/src/projects/workspace-structure.ts) (`createProjectViewKey`, line 115).

Boundaries:

- Add-only, and no new field on an existing public struct: `AgentSummary`, `ProjectNode`, `WorkspaceNode`, `CheckoutStatus`, and `ChangedFile` are built with struct literals in other crates (`crates/explorer/src/explorer.rs:340`, `crates/explorer/tests/models.rs:145`, `crates/sidebar/tests/model.rs`, `crates/app/src/state.rs:424`), so a new field breaks them. Expose the new data through new accessors and new types instead (Planner call from the Builder Check).
- Every crate that builds today still builds with no edit outside this group. Never edit `packages/protocol` (TypeScript).
- Agent features: the snapshot `features` is a discriminated union on `type` (`messages.ts:308`). Decode it so an unknown `type` is skipped and never fails the agent snapshot, the same compat rule as Q-073. If the generated type is a closed union, change the codegen in `crates/protocol/build.rs` (for example decode `features` as raw JSON) and regenerate; never hand-edit generated output.
- `checkout_diff` sends one `checkout.diff.get` request and returns every file (owner). Keep `file_diff` and `checkout_status` as they are.
- Request methods use the calling form of `file_diff` (`crates/client/src/lib.rs:380`): `&mut self`, the arguments, `cx`, returning `Task<anyhow::Result<..>>`.
- `project_view_key` returns `projectKey` when present, else the JSON string `["<serverId>","<projectId>"]`, as `createProjectViewKey` does.

Files:

- [packages/desktop-gpui/crates/client](packages/desktop-gpui/crates/client) (`src`, `tests`)
- [packages/desktop-gpui/crates/protocol](packages/desktop-gpui/crates/protocol) (`build.rs`, `src`, `tests`, only for the feature decode)

Interfaces:

- Produces: `bachuc_client::AttentionReason` (enum `Finished`, `Error`, `Permission`) and `Client::agent_attention(&self, agent_id: &str) -> Option<(bool, Option<AttentionReason>)>` (requires attention, reason) for TASK-161
- Produces: `Client::project_view_key(&self, project_id: &str) -> Option<String>` for TASK-161
- Produces: `bachuc_client::DiffStat` (struct `additions: u32`, `deletions: u32`), `Client::workspace_diff_stat(&self, workspace_id: &str) -> Option<DiffStat>`, and `Client::workspace_pinned_at(&self, workspace_id: &str) -> Option<String>` for TASK-161 and TASK-163
- Produces: `bachuc_client::CheckoutDetails` (`is_git: bool`, `branch: Option<String>`, `files: Vec<ChangedFileDetails>`), `bachuc_client::ChangedFileDetails` (`path: String`, `old_path: Option<String>`, `is_new: bool`, `is_deleted: bool`, `status: String`, `additions: u32`, `deletions: u32`), and `Client::checkout_details(&mut self, cwd: &str, cx) -> Task<anyhow::Result<CheckoutDetails>>` for TASK-162
- Produces: `bachuc_client::AgentFeature` (`id: String`, `label: String`, `description: Option<String>`, `tooltip: Option<String>`, `icon: Option<String>`, `kind: AgentFeatureKind`), `bachuc_client::AgentFeatureKind` (enum `Toggle { value: bool }`, `Select { value: Option<String>, options: Vec<AgentFeatureOption> }`), `bachuc_client::AgentFeatureOption` (fields as the daemon sends), `Client::agent_features(&self, agent_id: &str) -> Vec<AgentFeature>`, and `Client::set_agent_feature(&mut self, agent_id: &str, feature_id: &str, value: serde_json::Value, cx) -> Task<anyhow::Result<()>>` for TASK-163
- Produces: `bachuc_client::FileDiff` (`path: String`, `old_path: Option<String>`, `is_new: bool`, `is_deleted: bool`, `status: FileDiffStatus`, `additions: u32`, `deletions: u32`, `text: String` as `file_diff` renders it), `bachuc_client::FileDiffStatus` (enum `Ok`, `TooLarge`, `Binary`), and `Client::checkout_diff(&mut self, cwd: &str, cx) -> Task<anyhow::Result<Vec<FileDiff>>>` for TASK-162
- Produces: `bachuc_client::CommitList` (`base_ref: Option<String>`, `commits: Vec<CommitSummary>`), `bachuc_client::CommitSummary` (`sha`, `short_sha`, `subject`, `author_name`, `author_date: String`, `is_on_remote: bool`, `is_on_base: Option<bool>`, `files: Vec<CommitFile>`), `bachuc_client::CommitFile` (`path`, `additions: u32`, `deletions: u32`, `status: Option<String>`), `Client::commits(&mut self, cwd: &str, cx) -> Task<anyhow::Result<CommitList>>`, and `Client::commit_file_diff(&mut self, cwd: &str, sha: &str, path: &str, cx) -> Task<anyhow::Result<Option<FileDiff>>>` for TASK-162

Expected result:

- Each accessor reads the daemon value, and an absent optional field gives `None`.
- `set_agent_feature` sends one `set_agent_feature_request` and resolves on its response, with an error for a failed response.

Anti-goal: no existing public item changes; tripwire: `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh build -p bachuc-app` and `cargo.sh test -p bachuc-sidebar -p bachuc-explorer` pass with no edit outside this group's files; read at completion.

## Verify

- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test -p bachuc-client -p bachuc-protocol` -> pass, with one fake-daemon test per accessor and request: attention set and absent, `projectKey` present and absent (fallback key), `diffStat` object and null, `pinnedAt`, `isGit: false`, `isNew`/`isDeleted`/`oldPath`, a toggle and a select feature, an unknown feature `type` that leaves the snapshot decoded, the `set_agent_feature` payload and an error response, `checkout_diff` with `ok`, `too_large`, and `binary` files, `commits` with an error and with commits, and `commit_file_diff` returning `null`
- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh build -p bachuc-app` -> exit 0 with no edit outside this group's files
- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test -p bachuc-sidebar -p bachuc-explorer` -> pass unchanged
- `packages/desktop-gpui/cargo.sh fmt --check -p bachuc-client -p bachuc-protocol` -> exit 0
- `git diff --stat main -- packages/protocol` -> empty
