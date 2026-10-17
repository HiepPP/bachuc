# TASK-169 Client data for the composer, New workspace, and tab menu

Group: A (writes `crates/client`; runs after TASK-168 in the same group)
Class: risky

## Brief

Goal: Give the composer track strip, the draft composer, the New workspace screen, the tab menu, and the explorer diff mode every daemon field and request they need, without changing any existing public item.

Change: the client has no turn metrics, usage, tasks, subagents, send result, queue edits, draft controls, workspace create, branch helpers, project add, agent refresh, resume id, terminal rename, or diff compare mode -> accessors and requests for each.

Design: Electron reads its data here:

- Track strip: [packages/app/src/panels/agent-tracks.tsx](packages/app/src/panels/agent-tracks.tsx) (`AgentTracks`, line 32), fed by [packages/app/src/panels/agent-panel.tsx](packages/app/src/panels/agent-panel.tsx) lines 1164-1167: subagents from `useSubagentsForParent` ([packages/app/src/subagents/select.ts](packages/app/src/subagents/select.ts) lines 55-170) and tasks from `agentTasks`, the newest `todo` item of the stream (`latestTasksFromStream`, [packages/app/src/stores/session-store.ts](packages/app/src/stores/session-store.ts) line 346).
- Turn metrics: [packages/app/src/composer/turn-metrics-pill.tsx](packages/app/src/composer/turn-metrics-pill.tsx) (line 35) with `findLastTurnMetrics` in [packages/app/src/timeline/turn-time.ts](packages/app/src/timeline/turn-time.ts) (line 214). Usage: [packages/app/src/composer/index.tsx](packages/app/src/composer/index.tsx) lines 296-298. Queue actions: same file, lines 2261-2311.
- New workspace create: [packages/app/src/screens/new-workspace-screen.tsx](packages/app/src/screens/new-workspace-screen.tsx) (`createMultiplicityWorkspace` line 811, the initial agent lines 978-992) and the draft env in [packages/app/src/plugins/composer/draft.ts](packages/app/src/plugins/composer/draft.ts) (line 24).
- Resume command: [packages/app/src/screens/workspace/workspace-screen.tsx](packages/app/src/screens/workspace/workspace-screen.tsx) (line 2730) and the templates in [packages/app/src/utils/provider-command-templates.ts](packages/app/src/utils/provider-command-templates.ts) (line 10).
- Wire, all in [packages/protocol/src/messages.ts](packages/protocol/src/messages.ts): `TurnMetricsSchema` 392, `turn_completed` 737, entry `turnMetrics` 4714, `AgentUsageSchema` 383, snapshot `activeTurn`/`persistence`/`runtimeInfo`/`lastUsage` 860-875, `todo` 689, `send_agent_message_request` 1408 and its response 5103, queue requests 1951-1995, provider subagent list 1919, descriptor 4790, update 4866, `providerSubagents` feature 3783, modes 1798, draft features 2987 with `ListCommandsDraftConfigSchema` 2978, `workspace.create.request` 2730 and response 5059, `validate_branch_request` 2473 and response 6008, `branch_suggestions_request` 2518 and response 6019, `project.add.request` 2681 and response 4545, `refresh_agent_request` 1856, `terminal.rename.request` 3064 and response 6528, `CheckoutDiffCompareSchema` 2272. The parent label is `paseo.parent-agent-id` ([packages/protocol/src/agent-labels.ts](packages/protocol/src/agent-labels.ts) line 1).

Boundaries:

- Add-only, and no new field on an existing public struct (`AgentSummary`, `QueuedPrompt`, `TimelineState`, and the others; see TASK-157 Boundaries). Expose new data through new accessors and new types. Every crate that builds today still builds with no edit outside this group.
- Add no variant to `ClientEvent` or `DirectoryEvent` (exhaustive matches, see TASK-168). A provider subagent change marks the directory changed, so it reaches views as `ClientEvent::AgentsChanged`.
- Keep per-flush work in proportion to the change ([ADR-0039](watchtower/decisions/ADR-0039-incremental-timeline-sync-over-throttle-or-budget.md)). `agent_tasks` returns a kept copy of the newest `todo` item, updated as pages and live rows arrive; it never scans the timeline. `agent_last_turn_metrics` scans back from the end and stops at the first `user_message`, as `findLastTurnMetrics` does.
- Live turn metrics: GPUI ignores the `turn_completed` stream event today (`core.rs:1955`). On `turn_completed` with `metrics`, set `turnMetrics` on the last `assistant_message` entry of that `turnId` in the viewed timeline, bump its version, and mark it dirty. A fresh fetch sets the same field on the same entry (`session.ts:7743`), so a synced state still equals a fetch.
- Active turn comes from the snapshot `activeTurn`; usage from the snapshot `lastUsage`. The live "waiting" and "streaming" estimates of Electron stay in the composer (TASK-171), which reads `Client::timeline`.
- Subagents: Paseo rows are agents whose `paseo.parent-agent-id` label (trimmed, not empty) names the parent, sorted by `createdAt`. Provider rows come from `agent.provider_subagents.list.request` and `agent.provider_subagents.update` events with `kind: "upsert"`, only when `server_info.features.providerSubagents` is true. Planner call: `view_agent` sends the list request for the viewed agent, and `opened` sends it again for each viewed agent after a reconnect. Add `agent.provider_subagents.update` to `SESSION_EVENTS`. Rows are Paseo rows then provider rows, merged and sorted by `createdAt` when both exist, as `select.ts:163-169` does. Detach (`agent.detach.request`) stays out of this plan.
- Send: `send_message` and `enqueue_message` carry `images` and `attachments` (raw `AgentAttachment` JSON objects, for plugin chips) and resolve on the response: `accepted: false` or an `error` is an `Err` with the text, so the composer can show the send error. Keep `send_prompt`, `send_prompt_with`, and `enqueue_prompt` as they are.
- Queue edits use the `agent.queue.*` requests and the existing `Pending::Queue` answer path, which replaces the stored queue. `queue_held` reads the queue `held` flag.
- `create_workspace` sends one `workspace.create.request`. Send `idempotencyKey` (a new UUID) only when `server_info.features.workspaceRequestReceipts` is true. Send `firstAgentContext: {prompt}` only for a non-empty prompt. The initial agent is `{config: {provider, cwd, model?, modeId?, thinkingOptionId?, featureValues?}, env?, initialPrompt, clientMessageId, images?, attachments?, labels}`, with omitted fields left out. A response with `error` or a null `workspace` is an `Err`.
- `add_project` sends `project.add.request`. `AddedProject.view_key` follows the `project_view_key` rule of TASK-157.
- `refresh_agent` resolves on the `status` payload `agent_refreshed` with the same `requestId` (`session.ts:4650`); `rpc_error` is an `Err`.
- `agent_provider_session_id` returns `runtimeInfo.sessionId`, else `persistence.sessionId`. `resume_command` copies the six templates of `provider-command-templates.ts` and returns `None` for any other provider.
- `checkout_diff_with` sends `checkout.diff.get.request` with `compare: {mode, baseRef?, ignoreWhitespace?}`. Keep `checkout_diff` as the uncommitted call it is.
- `draft_features` sends `list_provider_features_request` with `draftConfig` and decodes features with the same lenient rule as `agent_features` (unknown `type` skipped). `provider_modes` sends `list_provider_modes_request`. `default_model` and `model_thinking` read the model data the client already keeps.
- Request methods use the calling form of `file_diff` (`crates/client/src/lib.rs:380`). In the Interfaces lines, `cx` means `cx: &mut Context<Client>`.

Files:

- [packages/desktop-gpui/crates/client](packages/desktop-gpui/crates/client) (`src`, `tests`)

Interfaces:

- Consumes: `Client::agent_labels(&self, agent_id: &str) -> Option<std::collections::BTreeMap<String, String>>` (`None` for an unknown agent), `Client::set_agent_labels(&mut self, agent_id: &str, labels: std::collections::BTreeMap<String, String>, cx) -> Task<anyhow::Result<()>>`, and `Client::close_agent_runtime(&mut self, agent_id: &str, cx) -> Task<anyhow::Result<()>>` from TASK-168
- Produces: `bachuc_client::TurnMetrics` (`ttft_ms: Option<f64>`, `output_tokens: Option<u64>`, `output_tokens_per_second: Option<f64>`), `bachuc_client::ActiveTurn` (`turn_id: String`, `started_at: Option<String>`), `Client::agent_active_turn(&self, agent_id: &str) -> Option<ActiveTurn>`, and `Client::agent_last_turn_metrics(&self, agent_id: &str) -> Option<TurnMetrics>` for TASK-171
- Produces: `bachuc_client::AgentUsage` (`input_tokens: Option<u64>`, `cached_input_tokens: Option<u64>`, `output_tokens: Option<u64>`, `total_cost_usd: Option<f64>`, `context_window_max_tokens: Option<u64>`, `context_window_used_tokens: Option<u64>`) and `Client::agent_last_usage(&self, agent_id: &str) -> Option<AgentUsage>` for TASK-171
- Produces: `bachuc_client::TaskEntry` (`id: Option<String>`, `text: String`, `completed: bool`, `status: Option<TaskStatus>`, `active_form: Option<String>`), `bachuc_client::TaskStatus` (enum `Pending`, `InProgress`, `Completed`), and `Client::agent_tasks(&self, agent_id: &str) -> Vec<TaskEntry>` (empty for an agent that is not viewed) for TASK-171
- Produces: `bachuc_client::SubagentRow` (`id: String`, `kind: SubagentKind`, `provider: String`, `title: Option<String>`, `description: Option<String>`, `subtitle: Option<String>`, `status: String`, `requires_attention: bool`, `created_at: String`), `bachuc_client::SubagentKind` (enum `Paseo`, `Provider`), `Client::subagents(&self, parent_agent_id: &str) -> Vec<SubagentRow>`, and `Client::agent_parent_id(&self, agent_id: &str) -> Option<String>` for TASK-171 and TASK-189
- Produces: `bachuc_client::OutgoingMessage` (`text: String`, `images: Vec<serde_json::Value>`, `attachments: Vec<serde_json::Value>`), `Client::send_message(&mut self, agent_id: &str, message: OutgoingMessage, behavior: TurnBehavior, cx) -> Task<anyhow::Result<()>>`, and `Client::enqueue_message(&mut self, agent_id: &str, message: OutgoingMessage, cx) -> Task<anyhow::Result<()>>` for TASK-171 and TASK-172
- Produces: `Client::queue_held(&self, agent_id: &str) -> bool`, `Client::edit_queued(&mut self, agent_id: &str, queued_id: &str, text: &str, cx)`, `Client::reorder_queued(&mut self, agent_id: &str, queued_id: &str, before_id: Option<&str>, cx)`, `Client::promote_queued(&mut self, agent_id: &str, queued_id: &str, cx)`, and `Client::resume_queue(&mut self, agent_id: &str, cx)` for TASK-171
- Produces: `bachuc_client::DraftConfig` (`provider: String`, `cwd: String`, `model: Option<String>`, `mode_id: Option<String>`, `thinking_option_id: Option<String>`), `Client::provider_modes(&mut self, provider: &str, cwd: Option<&str>, cx) -> Task<anyhow::Result<Vec<(String, String)>>>`, `Client::draft_features(&mut self, config: DraftConfig, cx) -> Task<anyhow::Result<Vec<AgentFeature>>>`, `Client::default_model(&self, provider: &str) -> Option<String>`, and `Client::model_thinking(&self, provider: &str, model: &str) -> Option<ThinkingControls>` (current is the model default) for TASK-172 and TASK-173
- Produces: `bachuc_client::CreateWorkspace` (`source: WorkspaceSource`, `title: Option<String>`, `first_agent_prompt: Option<String>`, `agent: Option<InitialAgent>`), `bachuc_client::WorkspaceSource` (enum `Directory { path: String, project_id: Option<String> }`, `Worktree { cwd: String, project_id: Option<String>, action: Option<WorktreeAction>, ref_name: Option<String>, base_branch: Option<String>, branch_name: Option<String>, worktree_slug: Option<String> }`), `bachuc_client::WorktreeAction` (enum `BranchOff`, `Checkout`), `bachuc_client::InitialAgent` (`config: DraftConfig`, `feature_values: std::collections::BTreeMap<String, serde_json::Value>`, `env: std::collections::BTreeMap<String, String>`, `initial_prompt: String`, `images: Vec<serde_json::Value>`, `attachments: Vec<serde_json::Value>`, `labels: std::collections::BTreeMap<String, String>`), `bachuc_client::CreatedWorkspace` (`workspace_id: String`, `agent_id: Option<String>`, `setup_terminal_id: Option<String>`), and `Client::create_workspace(&mut self, request: CreateWorkspace, cx) -> Task<anyhow::Result<CreatedWorkspace>>` for TASK-173
- Produces: `bachuc_client::BranchSuggestion` (`name: String`, `committer_date: Option<f64>`, `has_local: Option<bool>`, `has_remote: Option<bool>`, `local_ahead: Option<u32>`, `local_behind: Option<u32>`), `Client::branch_suggestions(&mut self, cwd: &str, query: Option<&str>, limit: Option<u32>, cx) -> Task<anyhow::Result<Vec<BranchSuggestion>>>`, `bachuc_client::BranchValidation` (`exists: bool`, `resolved_ref: Option<String>`, `is_remote: bool`), and `Client::validate_branch(&mut self, cwd: &str, branch_name: &str, cx) -> Task<anyhow::Result<BranchValidation>>` for TASK-173
- Produces: `bachuc_client::AddedProject` (`id: String`, `name: String`, `root: String`, `view_key: String`) and `Client::add_project(&mut self, cwd: &str, cx) -> Task<anyhow::Result<AddedProject>>` for TASK-173 and TASK-175
- Produces: `Client::refresh_agent(&mut self, agent_id: &str, cx) -> Task<anyhow::Result<()>>`, `Client::agent_provider_session_id(&self, agent_id: &str) -> Option<String>`, `bachuc_client::resume_command(provider: &str, session_id: &str) -> Option<String>`, and `Client::rename_terminal(&mut self, terminal_id: &str, title: &str, cx) -> Task<anyhow::Result<()>>` for TASK-174
- Produces: `Client::create_agent_with(&mut self, agent: InitialAgent, cx) -> Task<anyhow::Result<String>>` (one `create_agent_request` with the full config, feature values, env, images, attachments, and labels of `InitialAgent`; resolves to the new agent id) for TASK-173
- Produces: `Client::agent_updated_at(&self, agent_id: &str) -> Option<String>` (the agent snapshot `updatedAt`, `None` for an unknown agent) for TASK-190 and TASK-194
- Produces: `bachuc_client::DiffCompare` (`mode: DiffMode`, `base_ref: Option<String>`, `ignore_whitespace: bool`), `bachuc_client::DiffMode` (enum `Uncommitted`, `Base`), and `Client::checkout_diff_with(&mut self, cwd: &str, compare: DiffCompare, cx) -> Task<anyhow::Result<Vec<FileDiff>>>` for TASK-179

Expected result:

- Each accessor reads the daemon value, and an absent optional field gives `None` or an empty list.
- Each request sends one message with the fields above and resolves on its own response.
- Cases:
  - A timeline whose newest `user_message` has no later metrics -> `agent_last_turn_metrics` is `None`, even when an older turn has metrics.
  - A live `turn_completed` with `metrics` after the last assistant row -> `agent_last_turn_metrics` returns them, and the agent's timeline version grows.
  - Two `todo` items, then an older page that holds a third `todo` -> `agent_tasks` stays the newest by `seqStart`.
  - A label `paseo.parent-agent-id` of `"  "` -> no parent.
  - `providerSubagents` false -> no list request and no provider rows.
  - `send_message` answered `accepted: false` -> `Err` with `error`.
  - `create_workspace` without `workspaceRequestReceipts` -> no `idempotencyKey`; with an empty prompt -> no `firstAgentContext`; an `error` response -> `Err`.
  - `branch_suggestions` answered without `branchDetails` -> one suggestion per `branches` name with the other fields `None`.
  - `refresh_agent` answered with `rpc_error` -> `Err`.
  - `resume_command("claude", "abc")` -> `Some("claude --resume abc")`; an unknown provider -> `None`.
  - `checkout_diff_with` with `Base` and `ignore_whitespace: true` -> `compare` is `{"mode":"base","ignoreWhitespace":true}`.

Anti-goal: on a 20,000-entry viewed timeline with no `todo` and its last `user_message` 10 entries from the end, 1,000 calls each of `agent_tasks` and `agent_last_turn_metrics` take under 50 ms in a debug test; tripwire; read when the tasks cache lands and at completion.

## Verify

- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test -p bachuc-client` -> pass, with one fake-daemon or model test per expected case, plus: `activeTurn` present and null, `lastUsage` present and absent, a provider subagent upsert event, the merged subagent order, each queue request payload and `queue_held`, `provider_modes` and `draft_features` with an unknown feature type, `default_model` and `model_thinking`, a directory and a worktree `create_workspace` payload, `validate_branch` with an error, `add_project`, `agent_provider_session_id` from `runtimeInfo` and from `persistence`, `rename_terminal` with `success: false`, and the 20,000-entry timing test
- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh build -p bachuc-app` -> exit 0 with no edit outside this group's files
- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test -p bachuc-timeline -p bachuc-sidebar -p bachuc-explorer` -> pass unchanged
- `packages/desktop-gpui/cargo.sh fmt --check -p bachuc-client` -> exit 0
- `git diff --stat main -- packages/protocol` -> empty
