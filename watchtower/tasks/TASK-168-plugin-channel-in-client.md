# TASK-168 Plugin channel in the client

Group: A (writes `crates/client`, and `crates/protocol` only if a generated type must change; TASK-169 follows in the same group)
Class: risky

## Brief

Goal: Give the GPUI client a typed path to plugins on the active host: the plugin list, plugin RPCs, plugin settings, plugin change events, and the agent label and runtime calls that plugins use.

Change: the client sends no `plugin.*` message, drops `status` messages, and keeps no agent labels -> plugin list, catalog ids, RPC, settings helpers, a `PluginEvent` stream, and agent label and runtime-close calls.

Design: the wire in [packages/protocol/src/messages.ts](packages/protocol/src/messages.ts) (`plugin.catalog.get.request` line 1471, `plugin.list.request` line 1476, `plugin.rpc.invoke.request` line 1578, the two status events lines 3939-3948, `PluginListItemSchema` line 6748, `plugin.rpc.invoke.response` line 6877); the daemon side in [packages/server/src/server/session.ts](packages/server/src/server/session.ts) (`dispatchPluginMessage` line 2404, `subscribeToPluginChanges` line 2571, the `wantsEvent` gate in `refreshCatalogProducers` line 1683); the settings RPC names in [packages/plugin/src/settings.ts](packages/plugin/src/settings.ts) (`settingsRpc`, line 33); the TS calls in [packages/client/src/daemon-client.ts](packages/client/src/daemon-client.ts) (`getPluginCatalog` 5504, `listPlugins` 5514, `invokePluginRpc` 5727) and the agent handle in [packages/client/src/index.ts](packages/client/src/index.ts) (`closeRuntime` and `setLabels`, lines 384-386 and 987-991).

Boundaries:

- Follows [ADR-0045](watchtower/decisions/ADR-0045-native-rust-plugin-clients-over-webview-panes-or-shared-logic.md): plugin client parts are native Rust and call their server part only through plugin RPCs. This TASK is the RPC path.
- Add-only. Keep every existing public item and every existing public struct field: `AgentSummary`, `WorkspaceNode`, and other model types are built with struct literals in other crates (see TASK-157 Boundaries). Store labels in the private agent extras, not on `AgentSummary`.
- `ClientEvent` and `DirectoryEvent` are matched exhaustively in `crates/timeline/src/lib.rs:301-315`, `crates/sidebar/src/sidebar.rs:152-160`, and `crates/app/src/state.rs:368`. Add no variant to them. Emit plugin events as a third type, `PluginEvent`, through a third `EventEmitter` impl, as `DirectoryEvent` does (Planner call).
- The client sends raw JSON, as every other request does (`core.rs`). Decode each `plugin.list` item on its own: a `status` value this build does not know gives `PluginStatus::Unknown` and never fails the list. The generated `PluginEnableResponsePayloadPluginStatus` is a closed enum, so do not decode the list through it (Planner call). No `crates/protocol` change is expected; if one is needed, change `crates/protocol/build.rs` and regenerate, and never hand-edit generated output. Never edit `packages/protocol` (TypeScript).
- The client uses `explicit_event_subscriptions` (`connection.rs:67`), so add `status.plugin_catalog_changed` and `status.plugin_settings_changed` to `SESSION_EVENTS` in `core.rs:184`. Read both from `status` messages whose `payload.status` is `plugin_catalog_changed` or `plugin_settings_changed`. Ignore any other `status` payload without a `requestId`.
- A failed request answers with `rpc_error` (`session.ts:4662` shows the shape), which `core.rs:1313` already reads. Every new request resolves to `Err` with the daemon's `error` text on `rpc_error`, and to `Err("not connected")` when it cannot be sent. Every plugin message needs the `daemon.manage` permission; a refused request is an `Err`, never a panic.
- Add `Client::plugin_catalog_ids(&mut self, cx) -> Task<anyhow::Result<Vec<String>>>`. It reads only `id` from `plugin.catalog.get.response` and ignores `clientBundle` (JS). Planner call: the catalog is the loaded set (`packages/server/src/server/plugins/runtime.ts:356`), and TASK-170 gates on `plugin.list`, so no later TASK reads the catalog; it serves the live cross-check in Verify.
- Settings helpers call `plugin.rpc.invoke` with methods `settings.<settings_id>.read` (input `{}`), `settings.<settings_id>.write` (input `{revision, values}`), and `settings.<settings_id>.reset` (input `{revision}`). Decode the output by `status` per `SettingsReadResultSchema` and `SettingsWriteResultSchema` (`settings.ts:23-32`). An unknown `status` is an `Err`.
- `set_agent_labels` sends `update_agent_request` with `labels` only. The daemon keeps the other labels (`index.ts:385`). `close_agent_runtime` sends `agent.runtime.close.request`. Both read `AgentActionResponsePayloadSchema` (`messages.ts:2012`): `accepted: false` is an `Err` with `error`.
- Request methods use the calling form of `file_diff` (`crates/client/src/lib.rs:380`): `&mut self`, the arguments, `cx: &mut Context<Client>`, returning `Task<anyhow::Result<..>>`. In the Interfaces lines, `cx` means `cx: &mut Context<Client>`.

Files:

- [packages/desktop-gpui/crates/client](packages/desktop-gpui/crates/client) (`src`, `tests`)
- [packages/desktop-gpui/crates/protocol](packages/desktop-gpui/crates/protocol) (`build.rs`, `src`, `tests`, only if a generated type must change)

Interfaces:

- Produces: `bachuc_client::PluginEvent` (enum `CatalogChanged { plugin_id: String }`, `SettingsChanged { plugin_id: String, settings_id: String }`, derives `Clone, Debug, PartialEq, Eq`) emitted by `Client` through a third `EventEmitter` for TASK-170
- Produces: `bachuc_client::PluginListItem` (`id: String`, `description: Option<String>`, `enabled: bool`, `status: PluginStatus`, `error: Option<String>`), `bachuc_client::PluginStatus` (enum `Running`, `Disabled`, `Failed`, `Unknown`), and `Client::list_plugins(&mut self, cx) -> Task<anyhow::Result<Vec<PluginListItem>>>` for TASK-170
- Produces: `Client::invoke_plugin_rpc(&mut self, plugin_id: &str, method: &str, input: serde_json::Value, cx) -> Task<anyhow::Result<serde_json::Value>>` and `Client::plugin_rpc<I: serde::Serialize, O: serde::de::DeserializeOwned + Send + 'static>(&mut self, plugin_id: &str, method: &str, input: &I, cx) -> Task<anyhow::Result<O>>` for TASK-170 and the plugin TASKs
- Produces: `bachuc_client::PluginSettingsRead` (enum `Ready { revision: String, values: serde_json::Value }`, `Invalid { revision: String, error: String }`), `bachuc_client::PluginSettingsWrite` (enum `Saved { revision: String, values: serde_json::Value }`, `Conflict { error: String }`, `Invalid { error: String }`), `Client::read_plugin_settings(&mut self, plugin_id: &str, settings_id: &str, cx) -> Task<anyhow::Result<PluginSettingsRead>>`, `Client::write_plugin_settings(&mut self, plugin_id: &str, settings_id: &str, revision: &str, values: serde_json::Value, cx) -> Task<anyhow::Result<PluginSettingsWrite>>`, and `Client::reset_plugin_settings(&mut self, plugin_id: &str, settings_id: &str, revision: &str, cx) -> Task<anyhow::Result<PluginSettingsWrite>>` for TASK-170
- Produces: `Client::agent_labels(&self, agent_id: &str) -> Option<std::collections::BTreeMap<String, String>>` (`None` for an unknown agent), `Client::set_agent_labels(&mut self, agent_id: &str, labels: std::collections::BTreeMap<String, String>, cx) -> Task<anyhow::Result<()>>`, and `Client::close_agent_runtime(&mut self, agent_id: &str, cx) -> Task<anyhow::Result<()>>` for TASK-169, TASK-178, and the plugin TASKs

Expected result:

- `list_plugins` returns each listed plugin in the daemon's order; `plugin_catalog_ids` returns the catalog ids.
- `invoke_plugin_rpc` sends one `plugin.rpc.invoke.request` with `pluginId`, `method`, and `input`, and resolves to `output`.
- A `status` message `plugin_catalog_changed` or `plugin_settings_changed` emits one matching `PluginEvent`.
- Cases:
  - A list item with `status: "starting"` -> `PluginStatus::Unknown`; the other items still decode.
  - `rpc_error` for an RPC -> `Err` with the daemon text; `output: null` -> `Ok(Value::Null)`.
  - `plugin_rpc` with an output that does not fit `O` -> `Err`, not a panic.
  - Settings read `ready` and `invalid`, write `saved`, `conflict`, and `invalid`, reset `saved`, and an unknown `status` -> `Err`.
  - Any request while disconnected -> `Err("not connected")`.
  - An agent snapshot with `labels` -> `agent_labels` returns them; a snapshot without `labels` -> an empty map.
  - `set_agent_labels` or `close_agent_runtime` answered with `accepted: false` -> `Err` with `error`.

Anti-goal: no existing public item changes; tripwire: `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh build -p bachuc-app` and `cargo.sh test -p bachuc-timeline -p bachuc-sidebar` pass with no edit outside this group's files; read before the first change, after the event type lands, and at completion.

## Verify

- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test -p bachuc-client` -> pass, with one fake-daemon test per expected case: the list in order with an unknown status, the catalog ids, the RPC payload and its `output`, an RPC `rpc_error`, a `plugin_rpc` decode error, the two `PluginEvent`s from `status` messages, an unrelated `status` payload that emits nothing, the subscription request that holds both event names, each settings read, write, and reset result, a request while disconnected, labels present and absent, and `accepted: false` for labels and runtime close, and a reconnect mid-test after which the plugin subscription is sent again (Review Focus)
- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test -p bachuc-protocol` -> pass unchanged
- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh build -p bachuc-app` -> exit 0 with no edit outside this group's files
- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test -p bachuc-timeline -p bachuc-sidebar` -> pass unchanged
- `packages/desktop-gpui/cargo.sh fmt --check -p bachuc-client -p bachuc-protocol` -> exit 0
- `git diff --stat main -- packages/protocol` -> empty
- Only when live already runs on `ws://127.0.0.1:6768/ws`: `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test -p bachuc-client -- --ignored live_plugin` -> a new ignored test `live_plugin_list_matches_catalog` passes: every catalog id is listed as enabled and `Running`; when live does not run, report it as UNVERIFIED and never start a daemon
