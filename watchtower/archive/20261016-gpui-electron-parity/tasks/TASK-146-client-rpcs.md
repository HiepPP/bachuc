# TASK-146 Client RPCs for parity and a typed server_info

Group: B (writes `crates/client` and `crates/protocol`)
Class: risky

## Brief

Goal: Give the parity views every daemon request and state they need: the project and workspace tree, workspace actions, files and images, git changes and diffs, thinking, slash commands, steer and queue, rich permission responses, project config, host saving, and a typed `server_info`.

Change: the client covers agents, workspaces, providers, timelines, and terminals only -> it also covers the requests below, without breaking the current API.

Boundaries:

- Do not change or remove any current public item of `bachuc-client` (see [TASK-136 outcome](watchtower/archive/20261015-gpui-desktop-client/tasks/TASK-136-outcome.md)). Add only. Do not add variants to `ClientEvent`: emit the new events through a second `EventEmitter` type, `DirectoryEvent`, so `crates/app` keeps compiling.
- Owner answer Q-057 (2026-10-16): export `ServerInfoStatusPayloadSchema` ([packages/protocol/src/messages.ts](packages/protocol/src/messages.ts) line 3653) as a third root in `export-schema.mjs`, and read `server_info` into the typed struct.
- Do not change any file under [packages/protocol](packages/protocol).
- Requests to add, with the schema constant lines in [packages/protocol/src/messages.ts](packages/protocol/src/messages.ts):
  - Tree: `project.list.request` (1362). `fetch_workspaces_request` (1327-1359) has no project, branch, or pin fields; build the tree from its response: branch from `gitRuntime.currentBranch` (near 4052), pin from `pinnedAt` (near 4144; `pinned` is `pinnedAt` not null), and `agent_ids` joined from `AgentSummary.workspace_id`.
  - Workspace actions: `workspace.title.set.request` (1015), `workspace.pin.set.request` (1023), `archive_workspace_request` (2721), and `workspace.mark_unread.request` (2778), as the Electron workspace menu sends them ([packages/app/src/components/sidebar/sidebar-workspace-menu.tsx](packages/app/src/components/sidebar/sidebar-workspace-menu.tsx) lines 184-245).
  - Files: `file_explorer_request` (2840) with `mode` `list` and `file`. Request binary frames (`acceptBinary: true`) as the Electron image loader does ([packages/app/src/assistant-image/file-acquisition.ts](packages/app/src/assistant-image/file-acquisition.ts) line 47), and handle the base64 form too. A file has `kind` `text`, `image`, or `binary` (response near 6124-6135).
  - Git: `checkout_status_request` (2278) gives the branch and `isGit`, but no file list (response near 5349-5413). Take the changed files with `+N -M` counts and `status` (`ok`, `too_large`, `binary`) from the diff path the Electron app uses: `subscribe_checkout_diff_request` (2284) and `checkout.diff.get.request`, as [packages/app/src/git/use-working-diff.ts](packages/app/src/git/use-working-diff.ts) does. A folder that is not a git repo returns `branch: None` and no files, with no error.
  - Agents: `archive_agent_request` (966), `update_agent_request` (979) for rename, `set_agent_thinking_request` (2038), `list_commands_request` (2993).
  - Sending: `send_agent_message_request` (1408-1418) with `activeTurnBehavior` `steer` or `interrupt` (schema at 1254). The Electron default send behavior is `steer` ([packages/app/src/hooks/use-settings/storage.ts](packages/app/src/hooks/use-settings/storage.ts) lines 128 and 204). The alternate send (Cmd+Enter) queues while a turn runs through `agent.queue.enqueue.request` (1952) when the host has the `agentMessageQueue` feature ([packages/app/src/composer/index.tsx](packages/app/src/composer/index.tsx) lines 1489, 1786, 1866). Add enqueue, cancel (1978), and the queued items state; edit, reorder, promote, and resume are out of scope.
  - Permissions: `agent_permission_response` with a full response payload, for question answers, plan approve and reject, and `updatedInput`, as [packages/app/src/tool-calls/detail-level/turn/view.tsx](packages/app/src/tool-calls/detail-level/turn/view.tsx) line 1594 sends it. The current `respond_permission` stays.
  - Project config: `read_project_config_request` (1664) and `write_project_config_request` (1670).
- Hosts: add saving, so settings can add, edit, and remove hosts in `hosts.json` (the current `Vec<HostConfig>` array shape in `crates/client/src/hosts.rs`).
- Follows [ADR-0040](watchtower/decisions/ADR-0040-one-host-plain-ws-over-merged-hosts-and-relay.md): one host at a time.

How:

- Extend the schema export and the protocol crate with the `server_info` root.
- Add each request to `ClientCore` with fake-server tests, then the `Client` wrappers.
- Add the project tree state, the queue state, and `DirectoryEvent`.

Files:

- [packages/desktop-gpui/crates/client](packages/desktop-gpui/crates/client) (`src`, `tests`)
- [packages/desktop-gpui/crates/protocol](packages/desktop-gpui/crates/protocol) (`codegen/export-schema.mjs`, `codegen/protocol.schema.json`, `src/lib.rs`, `tests`)
- [packages/desktop-gpui/Cargo.lock](packages/desktop-gpui/Cargo.lock) (generated)

Interfaces:

- Consumes: `bachuc_client::Client` (a GPUI entity) with `Client::connect(host: HostConfig, cx: &mut gpui_kit::App) -> gpui_kit::Entity<Client>` from TASK-136
- Produces: `bachuc_protocol::ServerInfo` (the generated `server_info` struct) and `Client::server_info(&self) -> Option<&bachuc_protocol::ServerInfo>` for later TASKs
- Produces: `bachuc_client::DirectoryEvent` (enum `ProjectsChanged`, `CheckoutChanged { cwd: String }`) emitted by `Client` for TASK-147, TASK-151
- Produces: `Client::projects(&self) -> &[ProjectNode]` with `ProjectNode { id: String, name: String, root: String, workspaces: Vec<WorkspaceNode> }` and `WorkspaceNode { id: String, name: String, cwd: String, branch: Option<String>, pinned: bool, agent_ids: Vec<String> }` for TASK-147, TASK-153, TASK-155
- Produces: `Client::rename_workspace(&mut self, workspace_id: &str, title: &str, cx: &mut gpui_kit::Context<Client>)`, `Client::pin_workspace(&mut self, workspace_id: &str, pinned: bool, cx: &mut gpui_kit::Context<Client>)`, `Client::archive_workspace(&mut self, workspace_id: &str, cx: &mut gpui_kit::Context<Client>)`, and `Client::mark_workspace_unread(&mut self, workspace_id: &str, cx: &mut gpui_kit::Context<Client>)` for TASK-147
- Produces: `Client::archive_agent(&mut self, agent_id: &str, cx: &mut gpui_kit::Context<Client>)` and `Client::rename_agent(&mut self, agent_id: &str, title: &str, cx: &mut gpui_kit::Context<Client>)` for TASK-155
- Produces: `Client::list_directory(&mut self, cwd: &str, path: &str, cx: &mut gpui_kit::Context<Client>) -> gpui_kit::Task<anyhow::Result<Vec<FileEntry>>>` with `FileEntry { name: String, path: String, is_dir: bool, size: u64 }` for TASK-151
- Produces: `Client::read_file(&mut self, cwd: &str, path: &str, cx: &mut gpui_kit::Context<Client>) -> gpui_kit::Task<anyhow::Result<FileContent>>` with `FileContent { path: String, kind: FileKind, text: Option<String>, bytes: Option<Vec<u8>>, mime_type: Option<String>, size: u64 }` and `bachuc_client::FileKind` (enum `Text`, `Image`, `Binary`) for TASK-149, TASK-151
- Produces: `Client::checkout_status(&mut self, cwd: &str, cx: &mut gpui_kit::Context<Client>) -> gpui_kit::Task<anyhow::Result<CheckoutStatus>>` with `CheckoutStatus { branch: Option<String>, files: Vec<ChangedFile> }` and `ChangedFile { path: String, status: String, added: u32, removed: u32 }`, `Client::watch_checkout(&mut self, cwd: &str, cx: &mut gpui_kit::Context<Client>)`, `Client::unwatch_checkout(&mut self, cwd: &str, cx: &mut gpui_kit::Context<Client>)`, and `Client::file_diff(&mut self, cwd: &str, path: &str, cx: &mut gpui_kit::Context<Client>) -> gpui_kit::Task<anyhow::Result<String>>` (unified diff text) for TASK-151
- Produces: `Client::agent_thinking(&self, agent_id: &str) -> Option<ThinkingControls>` with `ThinkingControls { options: Vec<(String, String)>, current: Option<String> }`, and `Client::set_thinking(&mut self, agent_id: &str, option_id: &str, cx: &mut gpui_kit::Context<Client>)` for TASK-150
- Produces: `Client::list_commands(&mut self, agent_id: &str, cx: &mut gpui_kit::Context<Client>) -> gpui_kit::Task<anyhow::Result<Vec<SlashCommand>>>` with `SlashCommand { name: String, description: String }` for TASK-150
- Produces: `bachuc_client::TurnBehavior` (enum `Interrupt`, `Steer`) and `Client::send_prompt_with(&mut self, agent_id: &str, text: String, images: Vec<serde_json::Value>, behavior: TurnBehavior, cx: &mut gpui_kit::Context<Client>)` for TASK-150
- Produces: `Client::supports_queue(&self) -> bool`, `Client::enqueue_prompt(&mut self, agent_id: &str, text: String, images: Vec<serde_json::Value>, cx: &mut gpui_kit::Context<Client>)`, `Client::queued_prompts(&self, agent_id: &str) -> Vec<QueuedPrompt>` with `QueuedPrompt { id: String, text: String }`, and `Client::cancel_queued(&mut self, agent_id: &str, queued_id: &str, cx: &mut gpui_kit::Context<Client>)` for TASK-150
- Produces: `Client::respond_permission_with(&mut self, agent_id: &str, request_id: &str, response: serde_json::Value, cx: &mut gpui_kit::Context<Client>)` for TASK-149
- Produces: `Client::read_project_config(&mut self, root: &str, cx: &mut gpui_kit::Context<Client>) -> gpui_kit::Task<anyhow::Result<serde_json::Value>>` and `Client::write_project_config(&mut self, root: &str, config: serde_json::Value, cx: &mut gpui_kit::Context<Client>) -> gpui_kit::Task<anyhow::Result<()>>` for TASK-153
- Produces: `bachuc_client::save_hosts(hosts: &[HostConfig]) -> anyhow::Result<()>` for TASK-153

Expected result:

- Each new request has a fake-server test for the request shape and the parsed response.
- The live tests list projects, read a directory, a text file, and an image file, read checkout status and one file diff, list commands, set thinking, and enqueue and cancel a queued prompt on a live agent.
- A checkout change on disk emits `DirectoryEvent::CheckoutChanged` while the folder is watched.
- The current client tests and the app still build and pass unchanged.

Anti-goal: idle CPU of the connected `client_probe` stays at most 1% with project and checkout watching on; drift gauge; read with `measure.sh 60` after the tree state lands and at completion.

## Verify

- `npm run build:client` -> exit 0, then `node packages/desktop-gpui/crates/protocol/codegen/export-schema.mjs` twice -> the same `shasum` for `protocol.schema.json`
- `packages/desktop-gpui/cargo.sh test -p bachuc-protocol` -> pass, including a `server_info` fixture test
- `packages/desktop-gpui/cargo.sh test -p bachuc-client` -> pass, including one fake-server test per new request, a tree test with `pinnedAt` null and set and a workspace with no agents, a non-git checkout test, a binary-frame image read, a `too_large` diff file, and the steer and queue requests
- `packages/desktop-gpui/cargo.sh test -p bachuc-client -- --ignored live_` -> pass on 6768, including projects, files, an image read, checkout status, file diff, commands, thinking, and enqueue and cancel
- `packages/desktop-gpui/cargo.sh test --workspace` -> pass; `crates/app` builds unchanged
- `git diff --stat -- packages/protocol` -> empty
- Anti-goal: `measure.sh 60 probe="pids:<client_probe pid>"` connected and idle with watching on -> CPU at most 1.0
