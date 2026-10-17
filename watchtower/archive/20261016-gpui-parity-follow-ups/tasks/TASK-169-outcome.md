# TASK-169 Outcome

## Status

DONE

## Changed

- `packages/desktop-gpui/crates/client/src/model.rs`: new types `TurnMetrics`, `ActiveTurn`, `AgentUsage`, `TaskEntry`, `TaskStatus`, `SubagentRow`, `SubagentKind`, `OutgoingMessage`, `DraftConfig`, `CreateWorkspace`, `WorkspaceSource`, `WorktreeAction`, `InitialAgent`, `CreatedWorkspace`, `BranchSuggestion`, `BranchValidation`, `AddedProject`, `DiffMode`, `DiffCompare`, and `resume_command`.
- `packages/desktop-gpui/crates/client/src/composer_data.rs` (new): wire decoders (metrics, active turn, usage, `todo` items, provider subagents, branch suggestions) and request builders (draft config, initial agent, `workspace.create.request`, diff compare).
- `packages/desktop-gpui/crates/client/src/core.rs`: private agent extras gain the raw title, `createdAt`, `activeTurn`, `lastUsage`, the provider session id, and the queue `held` flag; a private per-viewed-agent `TasksCache`; provider subagents by parent; `agent.provider_subagents.update` in `SESSION_EVENTS`; `turn_completed` sets `turnMetrics` on the turn's last assistant entry; `Pending::Queue` carries an optional reply; new `ClientCore` methods for every accessor and request below.
- `packages/desktop-gpui/crates/client/src/lib.rs`: the matching `Client` accessors and requests, and the new `pub use` lines.
- `packages/desktop-gpui/crates/client/tests/composer.rs` (new): 15 fake-daemon and model tests.

## Contract

- Accessors: `agent_active_turn`, `agent_last_turn_metrics` (scans back, stops at the first `user_message`), `agent_last_usage`, `agent_tasks` (a kept copy of the newest `todo` by `seqStart`; empty when not viewed), `subagents`, `agent_parent_id` (trimmed label, blank is `None`), `queue_held`, `default_model`, `model_thinking` (current is the model default), `agent_provider_session_id` (`runtimeInfo.sessionId`, else `persistence.sessionId`), `agent_updated_at`.
- Requests: `send_message` and `enqueue_message` (images and attachments; `accepted: false` or an error is an `Err`), `edit_queued`, `reorder_queued`, `promote_queued`, `resume_queue` (through the queue answer path), `provider_modes`, `draft_features` (unknown feature type skipped), `create_workspace`, `create_agent_with`, `branch_suggestions`, `validate_branch`, `add_project`, `refresh_agent` (resolves on status `agent_refreshed`), `rename_terminal` (`success: false` is an `Err`), `checkout_diff_with`.
- `create_workspace` sends `idempotencyKey` only with `workspaceRequestReceipts`, `firstAgentContext` only for a non-empty prompt, and leaves omitted fields out.
- Provider rows come only with `server_info.features.providerSubagents`; a change marks the directory changed, so views get `ClientEvent::AgentsChanged`. No new `ClientEvent` or `DirectoryEvent` variant and no new field on an existing public struct. `send_prompt`, `send_prompt_with`, `enqueue_prompt`, and `checkout_diff` are unchanged.
- Consumes `agent_labels`, `set_agent_labels`, and `close_agent_runtime` from TASK-168 as named.

## Verified

- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test -p bachuc-client` -> exit 0; 72 passed, 0 failed, 5 ignored. `tests/composer.rs`: 15 passed (metrics `None` when the newest turn has none, live `turn_completed` metrics with a growing version and a `TimelineChanged`, newest `todo` kept across an older page and a live row, blank parent label, merged Paseo and provider order, provider upsert with `AgentsChanged`, the list sent again after a reconnect, `providerSubagents` off sends no list and shows no provider row, `activeTurn` present and null, `lastUsage` present and absent, session id from `runtimeInfo` and `persistence`, `updatedAt`, send `accepted: false`, enqueue with attachments, each queue payload and `queue_held`, modes, draft features with an unknown type, `default_model`, `model_thinking`, directory and worktree `create_workspace` payloads, no key without receipts, no context for an empty prompt, an `error` answer, `create_agent_with`, branch suggestions with and without `branchDetails`, `validate_branch` with an error, `add_project` with and without `projectKey`, `refresh_agent` with `rpc_error`, `rename_terminal` with `success: false`, the `Base` plus `ignoreWhitespace` compare, `resume_command`, and the 20,000-entry timing test).
- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh build -p bachuc-app` -> exit 0.
- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test -p bachuc-timeline -p bachuc-sidebar -p bachuc-explorer` -> exit 0; 126 passed, 0 failed.
- `packages/desktop-gpui/cargo.sh fmt --check -p bachuc-client` -> exit 0.
- `git diff --stat main -- packages/protocol` -> empty.

## Anti-goal

- Tasks cache landed: the 20,000-entry timing test passed under its 50 ms limit on the first run with the cache; source `/tmp/wt-a-client2.txt`; 2026-10-16 18:46.
- At completion: 1,000 calls each of `agent_tasks` and `agent_last_turn_metrics` on a 20,000-entry viewed timeline with the last `user_message` 10 entries from the end took 7.02 ms in a debug test (limit 50 ms); source `/tmp/wt-a-v1b.txt`; 2026-10-16 18:47.

## Lessons

- The provider subagent list goes out when `server_info` arrives (`WorkerEvent::Ready`), not in `opened`: the `providerSubagents` flag is unknown at `opened`, and messages sent there wait for `server_info` anyway. A reconnect still sends it again for each viewed agent.
- With explicit event subscriptions, the daemon delivers `agent.provider_subagents.update` to a subscribed client without the `provider_subagents` hello capability (only timeline rows check it), so `CAPABILITIES` stays as it is.
- A `turn_completed` that arrives while its assistant row still waits in the sync buffer finds no entry; the next fetch carries the metrics.
