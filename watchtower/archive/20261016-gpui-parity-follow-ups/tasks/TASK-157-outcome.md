# TASK-157 Outcome

## Status

DONE

## Changed

- `packages/desktop-gpui/crates/client/src/model.rs`: new types `AttentionReason`, `DiffStat`, `CheckoutDetails`, `ChangedFileDetails`, `AgentFeature`, `AgentFeatureKind`, `AgentFeatureOption`, `FileDiff`, `FileDiffStatus`, `CommitList`, `CommitSummary`, `CommitFile`.
- `packages/desktop-gpui/crates/client/src/core.rs`: the agent snapshot keeps `requiresAttention`, `attentionReason`, and `features`; projects keep `projectKey`; workspaces keep `pinnedAt` and `diffStat`. New `ClientCore` accessors and requests with the same names as on `Client`. `checkout_status` and `checkout_details` share one private `CheckoutReply`, so `checkout_status` answers as before.
- `packages/desktop-gpui/crates/client/src/files.rs`: parsers for the detailed changed files, one diff file, and the commit list; `impl From<ChangedFileDetails> for ChangedFile`.
- `packages/desktop-gpui/crates/client/src/lib.rs`: re-exports and the `Client` methods.
- `packages/desktop-gpui/crates/client/tests/rpcs.rs`: 9 fake-daemon tests.
- `packages/desktop-gpui/crates/protocol/tests/fixtures.rs`: 1 decode test for an unknown feature type.
- `crates/protocol/build.rs` and `src` are unchanged: the generated `UpsertAgentFeaturesItem` is already an internally tagged enum with the `#[serde(other)] Unrecognized` fallback.

## Contract

- `Client::agent_attention(&self, agent_id) -> Option<(bool, Option<AttentionReason>)>`: `None` for an unknown agent; an absent `requiresAttention` is `false`; an absent, null, or unknown reason is `None`.
- `Client::project_view_key(&self, project_id) -> Option<String>`: `projectKey` from `project.list` or `project.update` or `emptyProjects`, else `["<serverId>","<projectId>"]`; `None` for an unknown project or before `server_info`.
- `Client::workspace_diff_stat(&self, workspace_id) -> Option<DiffStat>` and `Client::workspace_pinned_at(&self, workspace_id) -> Option<String>`.
- `Client::agent_features(&self, agent_id) -> Vec<AgentFeature>`: each feature decodes through the generated `UpsertAgentFeaturesItem`; an `Unrecognized` or malformed feature is skipped.
- `Client::set_agent_feature(&mut self, agent_id, feature_id, value: serde_json::Value, cx) -> Task<anyhow::Result<()>>`: one `set_agent_feature_request`; the daemon's `error` string, an `rpc_error`, or `accepted: false` fails it.
- `Client::checkout_details(&mut self, cwd, cx) -> Task<anyhow::Result<CheckoutDetails>>`: the same two requests as `checkout_status`.
- `Client::checkout_diff(&mut self, cwd, cx) -> Task<anyhow::Result<Vec<FileDiff>>>`: one `checkout.diff.get` request (`uncommitted`). `text` is the `file_diff` rendering for `Ok` and empty for `TooLarge` and `Binary`.
- `Client::commits(&mut self, cwd, cx) -> Task<anyhow::Result<CommitList>>`: a checkout error fails with its message.
- `Client::commit_file_diff(&mut self, cwd, sha, path, cx) -> Task<anyhow::Result<Option<FileDiff>>>`: `file: null` gives `None`.
- No existing public item changed. The only removals are private items and imports.

## Verified

- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test -p bachuc-client -p bachuc-protocol` -> exit 0; rpcs 23 passed (9 new), protocol fixtures 20 passed and 1 ignored (`live_hello`), others pass; live tests ignored.
- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh build -p bachuc-app` -> exit 0, with no edit outside this group's files.
- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test -p bachuc-sidebar -p bachuc-explorer` -> exit 0; 21 and 16 passed, unchanged.
- `packages/desktop-gpui/cargo.sh fmt --check -p bachuc-client -p bachuc-protocol` -> exit 0.
- `git diff --stat main -- packages/protocol` -> empty.
- Live client tests on 6768: UNVERIFIED (unattended), live is down.

## Anti-goal

- Checkpoint: `cargo.sh build -p bachuc-app` -> exit 0, source the command run, 2026-10-16 15:59.
- Checkpoint: `cargo.sh test -p bachuc-sidebar -p bachuc-explorer` -> 21 and 16 passed, source the command run, 2026-10-16 15:59.
- Checkpoint: `git diff -U0` of the client and protocol crates -> removed lines are only private items and imports, 2026-10-16 16:00.

## Lessons

- Agent snapshots reach `ClientCore` as raw JSON, so only the generated per-item decode decides which features are known. The generated `UpsertAgentFeaturesItem` already has `Unrecognized` from `add_fallback_variants`, so the feature decode needed no codegen change.
- Workspace descriptors carry no `projectKey`; only project descriptors do.

## Questions

- Safe default: `checkout_diff` gives an empty list for `NOT_GIT_REPO`, as `checkout_status` does; other checkout errors fail.
- Safe default: a diff file `status` from a newer daemon maps to `FileDiffStatus::Binary`, so the file shows without a diff.
