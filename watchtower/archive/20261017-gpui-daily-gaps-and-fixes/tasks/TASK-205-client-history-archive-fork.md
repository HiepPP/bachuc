# TASK-205 Client history, archived agents, and fork context

Group: C (shared files [packages/desktop-gpui/crates/client/src/core.rs](packages/desktop-gpui/crates/client/src/core.rs), [lib.rs](packages/desktop-gpui/crates/client/src/lib.rs), [model.rs](packages/desktop-gpui/crates/client/src/model.rs))
Class: risky

## Brief

Goal: The client can page agent history with search, keep archived agents it has seen, unarchive an agent, and build a fork context.

Change: archived agents are dropped and lost -> they live in a separate archived cache, and history, unarchive, and fork context calls exist.

Boundaries:

- History: `fetch_agent_history_request` ([packages/protocol/src/messages.ts](packages/protocol/src/messages.ts) line 1368, response line 4278). Send `search` only when not empty, `sort: [{key: "updated_at", direction: "desc"}]`, `page: {limit: 200, cursor?}`, and no `filter`, as `use-agent-history.ts` lines 74-86 do. The daemon includes archived agents for this request (`session.ts` lines 5449-5453).
- Gate search on `server_info.features.agentHistorySearch`. Without it, send no `search`.
- Archived cache: today `upsert_agent` removes an agent whose `archivedAt` is set (`core.rs` lines 2568-2574). Keep that removal from the live directory, so the sidebar and workspace trees stay as they are. Also store the agent's `AgentSummary` in a new archived map. History rows also fill the map. An `agent_update` upsert with `archivedAt` null moves the agent back to the live directory and out of the map.
- `remove_agent` also clears `extras`; the archived map keeps its own entries.
- Unarchive: `Client::refresh_agent` already sends `refresh_agent_request` (`core.rs` line 1147). The daemon restores the workspace and unarchives (`session.ts` lines 4605-4656). Add `unarchive_agent` as a thin wrapper.
- Fork context: `agent.fork_context.request` (line 1941) with `agentId`, optional `boundaryCursor` (an `{epoch, seq}` object, line 1883) and `boundaryMessageId`; response line 4925. Gate the call on `server_info.features.agentForkContext`, and sending a cursor on `features.agentForkContextCursor`.
- Keep the reply attachment as raw JSON, so the app sends it unchanged with its `contextKind: "chat_history"` (`messages.ts` lines 1154-1166). A reply with `attachment: null` gives `Err("Nothing to fork")`.
- History reply `searchTruncated` maps to `search_truncated`.
- Do not add a field to `AgentSummary`; many crates build it.

How:

- Add the archived map and route archived upserts into it.
- Add the three calls with `Pending::*` replies and `lib.rs` wrappers.
- Add tests in `tests/rpcs.rs`: history request shape, archived routing both ways, fork context parse.

Files:

- [packages/desktop-gpui/crates/client/src/core.rs](packages/desktop-gpui/crates/client/src/core.rs) (archived map, history, fork context)
- [packages/desktop-gpui/crates/client/src/lib.rs](packages/desktop-gpui/crates/client/src/lib.rs) (wrappers and lookups)
- [packages/desktop-gpui/crates/client/src/model.rs](packages/desktop-gpui/crates/client/src/model.rs) (new types)
- [packages/desktop-gpui/crates/client/tests/rpcs.rs](packages/desktop-gpui/crates/client/tests/rpcs.rs) (tests)

Interfaces:

- Consumes: nothing from TASK-204; it only runs after it because both write the same files.
- Produces: `HistoryEntry { agent_id: String, title: String, provider: String, workspace_id: Option<String>, workspace_name: Option<String>, project_name: String, branch: Option<String>, cwd: String, archived: bool, last_activity_at: Option<String>, pending_permissions: usize }` for TASK-218.
- Produces: `HistoryPage { entries: Vec<HistoryEntry>, next_cursor: Option<String>, has_more: bool, search_truncated: bool }` and `Client::fetch_agent_history(&mut self, search: Option<&str>, cursor: Option<&str>, cx: &mut Context<Client>) -> Task<anyhow::Result<HistoryPage>>` for TASK-218.
- Produces: `Client::history_search_supported(&self) -> bool` for TASK-218.
- Produces: `Client::archived_agent(&self, agent_id: &str) -> Option<&AgentSummary>` and `Client::is_archived(&self, agent_id: &str) -> bool` for TASK-200.
- Produces: `Client::unarchive_agent(&mut self, agent_id: &str, cx: &mut Context<Client>) -> Task<anyhow::Result<()>>` for TASK-200.
- Produces: `ForkCursor { epoch: String, seq: u64 }` and `ForkBoundary { cursor: Option<ForkCursor>, message_id: Option<String> }` for TASK-208 and TASK-201.
- Produces: `Client::fork_supported(&self) -> bool` and `Client::fork_cursor_supported(&self) -> bool` for TASK-208 and TASK-201.
- Produces: `ForkContext { attachment: serde_json::Value, title: Option<String>, item_count: u32 }` and `Client::fork_context(&mut self, agent_id: &str, boundary: ForkBoundary, cx: &mut Context<Client>) -> Task<anyhow::Result<ForkContext>>` for TASK-201.

Expected result:

- An archived upsert removes the agent from `agents()` and adds it to the archived map.
- An upsert with `archivedAt` null for an archived agent moves it back.
- `fetch_agent_history` sends the exact request and parses entries, cursor, and `hasMore`.
- `fork_context` keeps the raw attachment, gives `Err` with the daemon message on `error`, and gives `Err("Nothing to fork")` on a null attachment.
- Without `agentForkContextCursor`, the request carries no `boundaryCursor`.

Anti-goal: the live directory must not change for live agents; tripwire; every existing `bachuc-client` test passes unchanged, read before the change and at completion.

## Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-client` -> passes, including the new history, archived, and fork tests.
- `./cargo.sh build -p bachuc-app` -> builds.
- Anti-goal: the count of passing existing client tests before and after -> equal, with no test edited.
