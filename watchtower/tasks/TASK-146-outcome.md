# TASK-146 Outcome

## Status

DONE

## Changed

- `packages/desktop-gpui/crates/protocol/codegen/export-schema.mjs`: exports `ServerInfoStatusPayloadSchema` as a third root, `ServerInfo` (Q-057).
- `packages/desktop-gpui/crates/protocol/codegen/protocol.schema.json`: regenerated; 295 added lines, the `WsInbound` and `WsOutbound` roots unchanged.
- `packages/desktop-gpui/crates/protocol/src/lib.rs`: `pub use generated::ServerInfo` and `decode_server_info(&Value)`.
- `packages/desktop-gpui/crates/protocol/tests/fixtures.rs`: `server_info_decodes_into_the_typed_struct` on the recorded `server_info` fixture.
- `packages/desktop-gpui/crates/client/Cargo.toml`: adds `base64 = "0.22"` for the base64 file form.
- `packages/desktop-gpui/crates/client/src/files.rs` (new): file transfer frames (`0x10`, `0x11`, `0x12`), the JSON file forms, changed files, and unified diff text.
- `packages/desktop-gpui/crates/client/src/model.rs`: the new public types and `DirectoryEvent`.
- `packages/desktop-gpui/crates/client/src/core.rs`: the new requests, the project tree, queue and thinking state, checkout watches, and project config revisions. It also drops an agent snapshot whose `updatedAt` is older than the stored one (see Lessons).
- `packages/desktop-gpui/crates/client/src/lib.rs`: `EventEmitter<DirectoryEvent>` and the `Client` wrappers.
- `packages/desktop-gpui/crates/client/src/hosts.rs`: `save_hosts`, with a write to `hosts.json.tmp` and a rename.
- `packages/desktop-gpui/crates/client/tests/common/mod.rs`: fake daemon handlers (`FakeServer::on`, `FakeServer::reply`), `Harness::wait`, and `Harness::directory_events`; the fake `server_info` advertises `agentMessageQueue`.
- `packages/desktop-gpui/crates/client/tests/rpcs.rs` (new): 14 fake-server tests.
- `packages/desktop-gpui/crates/client/tests/live.rs`: 2 new live tests.
- `packages/desktop-gpui/crates/client/examples/client_probe.rs`: prints `DirectoryEvent`s and watches the folder in `BACHUC_WATCH_CHECKOUT`, for the anti-goal read.
- `packages/desktop-gpui/Cargo.lock`: updated by cargo.

## Contract

Every existing public item of `bachuc-client` is unchanged; `ClientEvent` has no new variant.

- `bachuc_protocol::ServerInfo` (generated) and `bachuc_protocol::decode_server_info`. `Client::server_info(&self) -> Option<&bachuc_protocol::ServerInfo>`.
- `bachuc_client::DirectoryEvent` (`ProjectsChanged`, `CheckoutChanged { cwd: String }`), emitted by `Client` as a second event type.
- `Client::projects(&self) -> &[ProjectNode]` with `ProjectNode { id, name, root, workspaces: Vec<WorkspaceNode> }` and `WorkspaceNode { id, name, cwd, branch: Option<String>, pinned: bool, agent_ids: Vec<String> }`.
- `Client::rename_workspace`, `pin_workspace`, `archive_workspace`, `mark_workspace_unread`, `archive_agent`, and `rename_agent`, with the spec signatures.
- `Client::list_directory(cwd, path, cx) -> Task<Result<Vec<FileEntry>>>` with `FileEntry { name, path, is_dir, size: u64 }`.
- `Client::read_file(cwd, path, cx) -> Task<Result<FileContent>>` with `FileContent { path, kind: FileKind, text, bytes, mime_type, size: u64 }` and `FileKind` (`Text`, `Image`, `Binary`).
- `Client::checkout_status(cwd, cx) -> Task<Result<CheckoutStatus>>` with `CheckoutStatus { branch, files: Vec<ChangedFile> }` and `ChangedFile { path, status, added: u32, removed: u32 }`. Also `Client::watch_checkout`, `Client::unwatch_checkout`, and `Client::file_diff(cwd, path, cx) -> Task<Result<String>>`.
- `Client::agent_thinking(&self, agent_id) -> Option<ThinkingControls>` with `ThinkingControls { options: Vec<(String, String)>, current: Option<String> }`, and `Client::set_thinking`.
- `Client::list_commands(agent_id, cx) -> Task<Result<Vec<SlashCommand>>>` with `SlashCommand { name, description }`.
- `TurnBehavior` (`Interrupt`, `Steer`) and `Client::send_prompt_with(agent_id, text, images, behavior, cx)`.
- `Client::supports_queue`, `Client::enqueue_prompt`, `Client::queued_prompts(&self, agent_id) -> Vec<QueuedPrompt>` with `QueuedPrompt { id, text }`, and `Client::cancel_queued`.
- `Client::respond_permission_with(agent_id, request_id, response: serde_json::Value, cx)`.
- `Client::read_project_config(root, cx) -> Task<Result<Value>>` and `Client::write_project_config(root, config, cx) -> Task<Result<()>>`.
- `bachuc_client::save_hosts(hosts: &[HostConfig]) -> anyhow::Result<()>`.
- Extra public items: `ClientCore` has the same requests without GPUI, plus `typed_server_info`, `supports_queue`, `projects`, `take_directory_events`, `queued_prompts`, and `agent_thinking`.

## Verified

- `npm run build:client` -> exit 0. Then `node packages/desktop-gpui/crates/protocol/codegen/export-schema.mjs` twice -> the same `shasum` `053a7a12a51e044a825b69b0cb64c0a2707cf60c` both times. PASS.
- `packages/desktop-gpui/cargo.sh test -p bachuc-protocol` -> 18 passed, 1 ignored, including `server_info_decodes_into_the_typed_struct`. That test checks typed fields, a full round trip, and unknown fields. PASS.
- `packages/desktop-gpui/cargo.sh test -p bachuc-client` -> 13 unit, 14 `rpcs`, and 13 `sync` tests passed. PASS.
  - The `rpcs` tests cover one request each and the parsed answers.
  - The tree test covers `pinnedAt` set and null, a workspace with no agents, `emptyProjects`, `project.update` remove, and no event for an agent status change.
  - The checkout tests cover a folder outside git, a `too_large` file, and watch and unwatch.
  - The file test covers a binary-frame image read, the base64 and UTF-8 JSON forms, and an error.
  - The send tests cover steer, interrupt, enqueue, and cancel. The config test checks that a write sends the revision of the last read.
  - `an_older_agent_snapshot_does_not_replace_a_newer_one` covers the `updatedAt` fix.
- `packages/desktop-gpui/cargo.sh test -p bachuc-client -- --ignored live_` on 6768 -> 4 passed. PASS.
  - `live_projects_files_and_checkout` lists projects and reads a directory, a text file, and a PNG through binary frames. It reads the checkout status and one file diff of a temporary git folder, and a folder outside git. A file change on disk emitted `CheckoutChanged` while the folder was watched.
  - `live_agent_commands_thinking_and_queue` uses a mock `five-minute-stream` agent. It checks that mock commands fail with "Agent does not support listing commands". It lists commands of an idle Claude agent that was created with no prompt and ran no turn. It sets thinking and sees `current` change, then enqueues and cancels a queued prompt while the agent runs.
  - The 2 TASK-136 live tests still pass. Each test stopped and archived its agents: `npm run cli -- ls` shows no running test agent, and `terminal ls --all` is empty.
- `packages/desktop-gpui/cargo.sh test --workspace` -> not run: the builder rules allow only `-p` scoped cargo. In its place, `cargo.sh build -p bachuc-app -p bachuc-timeline -p bachuc-composer -p bachuc-terminal` -> exit 0, and `cargo.sh test -p bachuc-app` -> 21 passed. `crates/app` is unchanged. PASS (scoped).
- `git diff --stat -- packages/protocol` -> empty. PASS.
- `packages/desktop-gpui/cargo.sh fmt --check -p bachuc-client -p bachuc-protocol` -> exit 0. PASS.
- `packages/desktop-gpui/cargo.sh clippy -p bachuc-client --all-targets` -> no warning in `bachuc-client`. The 2 warnings in `crates/protocol/build.rs` existed before. PASS.
- `npm run lint -- packages/desktop-gpui/crates/protocol/codegen/export-schema.mjs` -> 0 warnings, 0 errors. `npm run format:check:files` on the script, the schema, and this file -> exit 0. PASS.
- GitNexus `impact` on `ClientCore` -> UNKNOWN, no callers resolved. A text search showed that other crates use only `Client` and the public model types. Those stay unchanged, so the risk is LOW.
- Anti-goal `measure.sh 60 probe="pids:<client_probe pid>"` -> DEFERRED (UI checker).

## Anti-goal

Gauge: idle CPU of the connected `client_probe` with the project tree and checkout watching on, at most 1.0%.

- Checkpoint "tree state landed / completion" (one read, after all code landed), 2026-10-16T03:52:26Z to 03:53:26Z, builder read, not `measure.sh`. Command: `BACHUC_WATCH_CHECKOUT=/tmp/gpui-146-probe target/debug/examples/client_probe ws://127.0.0.1:6768/ws` (pid 35181), with `ps -o time=` sampled 60 s apart. Result: CPU time 0:00.08 -> 0:00.09, about 0.02%, RSS 33 MB. The probe printed `37 projects` and `checkout changed: /tmp/gpui-146-probe` after a file edit. Within limit.
- Official read: DEFERRED (UI checker). Run `packages/desktop-gpui/cargo.sh build -p bachuc-client --example client_probe`. Then start `BACHUC_WATCH_CHECKOUT=/tmp/gpui-146-probe packages/desktop-gpui/target/debug/examples/client_probe ws://127.0.0.1:6768/ws` and run `measure.sh 60 probe="pids:<pid>"`. `/tmp/gpui-146-probe` is a git folder that this TASK left in place.

## Lessons

- Live sends an older agent snapshot after a newer one at agent start: `running` at `updatedAt` 45.016, then `idle` at 44.759. The client stayed `idle` while the agent ran. `ClientCore` now drops an upsert whose `updatedAt` is older than the stored one. TASK-150 needs this to choose steer or queue.
- Every outgoing message passes the generated types, so JSON integers go out as f64 (`1` becomes `1.0`). The daemon accepts both. A fake-server test must compare numbers with `as_f64`.
- `read_file` with `acceptBinary` gets no JSON answer: `FileBegin`, `FileChunk`, and `FileEnd` frames carry the file, keyed by the request id. An error still comes as a JSON `file_explorer_response`. A text file gives `text` and no `bytes`, and an image or binary file gives `bytes` and no `text`.
- `file_diff` fetches the whole uncommitted diff and renders one file. A `too_large` or `binary` file fails with the status in the message, and a file with no change gives `""`. Changed files and diffs use compare mode `uncommitted`.
- `project.update` needs the `project.update` event subscription; the client now asks for it. The tree uses the `project.list` order, then the `emptyProjects` of the workspace list, then any project that only a workspace names. Workspaces keep the daemon's order; nothing sorts pinned workspaces first. `ProjectsChanged` fires only when the tree changes, a few times during startup.
- `write_project_config` sends the revision of the last read or write through this client. Read before a write; otherwise the daemon answers `stale_project_config`, and the error text is the error code.
- `agent_thinking` takes the options of the agent's model, or of the provider's default model when the agent has none. It returns `None` when the model has no options, for example mock `thirty-minute-stream`. Mock `five-minute-stream` has `low`, `medium`, and `high`.
- The mock provider cannot list commands. An idle Claude agent created with no prompt lists commands and runs no turn.
- `ServerInfo.permissions` is a closed enum. A daemon with a new permission name makes `server_info()` return `None` and logs a warning, so `supports_queue` is then false.
- `save_hosts(&[])` writes an empty array, which `load_hosts` reads back as the release and live seeds.

## Questions

- The builder rules allow only `-p` scoped cargo, so `cargo.sh test --workspace` from Verify was replaced by building `bachuc-app`, `bachuc-timeline`, `bachuc-composer`, and `bachuc-terminal`, and testing `bachuc-app`. The lead should run the workspace test after the wave.
- The `updatedAt` guard changes TASK-136 behavior: an upsert with an older `updatedAt` is ignored. Does the daemon ever send a newer state with an older `updatedAt`? None was seen on live.
