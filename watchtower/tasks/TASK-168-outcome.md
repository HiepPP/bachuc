# TASK-168 Outcome

## Status

DONE

## Changed

- `packages/desktop-gpui/crates/client/src/plugins.rs` (new): `PluginEvent`, `PluginStatus`, `PluginListItem`, `PluginSettingsRead`, `PluginSettingsWrite`, and per-item decoders for the plugin list, the catalog ids, and the settings answers.
- `packages/desktop-gpui/crates/client/src/core.rs`: new private `Pending` variants `PluginList`, `PluginCatalog`, `PluginRpc` (a boxed decode callback), and `Accepted`; `labels` in the private agent extras; `status.plugin_catalog_changed` and `status.plugin_settings_changed` in `SESSION_EVENTS`; `status` messages without a request id read into `PluginEvent`s; new `ClientCore` methods `list_plugins`, `plugin_catalog_ids`, `plugin_rpc`, `read_plugin_settings`, `write_plugin_settings`, `reset_plugin_settings`, `agent_labels`, `set_agent_labels`, `close_agent_runtime`, and `take_plugin_events`.
- `packages/desktop-gpui/crates/client/src/lib.rs`: `impl EventEmitter<PluginEvent> for Client`, `publish` emits plugin events, and the `Client` request methods in the `file_diff` calling form.
- `packages/desktop-gpui/crates/client/tests/plugins.rs` (new): 8 fake-daemon tests.
- `packages/desktop-gpui/crates/client/tests/common/mod.rs`: `Harness::plugin_events`, and `State::features` merged into the fake `server_info.features`.
- `packages/desktop-gpui/crates/client/tests/live.rs`: ignored test `live_plugin_list_matches_catalog`.

## Contract

- Produces `bachuc_client::PluginEvent` (`CatalogChanged { plugin_id }`, `SettingsChanged { plugin_id, settings_id }`; `Clone, Debug, PartialEq, Eq`), emitted through a third `EventEmitter` impl. No variant added to `ClientEvent` or `DirectoryEvent`.
- Produces `PluginListItem`, `PluginStatus` (`Running`, `Disabled`, `Failed`, `Unknown`), and `Client::list_plugins`. Items decode one by one; an unknown status is `Unknown`, and an item without `id` is left out.
- Produces `Client::plugin_catalog_ids` (reads only `id`), `Client::invoke_plugin_rpc` (resolves to `output`, `null` gives `Value::Null`), and `Client::plugin_rpc<I, O>` (an output that does not fit `O` is an `Err`).
- Produces `PluginSettingsRead`, `PluginSettingsWrite`, and `Client::read_plugin_settings`, `write_plugin_settings`, `reset_plugin_settings` over `settings.<id>.read|write|reset`; an unknown `status` is an `Err`.
- Produces `Client::agent_labels` (`None` for an unknown agent, empty map without `labels`), `Client::set_agent_labels` (sends `labels` only), and `Client::close_agent_runtime`; `accepted: false` is an `Err` with `error`.
- Every new request is `Err("not connected")` when it cannot be sent and `Err(<daemon text>)` on `rpc_error`.
- No existing public item or public struct field changed. No `crates/protocol` change.

## Verified

- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test -p bachuc-client` -> exit 0; 72 passed, 0 failed, 5 ignored (live). `tests/plugins.rs`: 8 passed (list order with `starting` -> `Unknown` and an item without id skipped, catalog ids, RPC payload and output, `output: null`, `rpc_error` text, `plugin_rpc` decode error, the two `PluginEvent`s, an unrelated `daemon_config_changed` status that emits nothing, the subscription holding both names, the subscription sent again after a reconnect and an event read after it, settings read `ready`/`invalid`/unknown, write `saved`/`conflict`/`invalid`/unknown, reset `saved`, every request while disconnected, labels present and absent, `accepted: false` for labels and runtime close).
- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test -p bachuc-protocol` -> exit 0; 20 passed, 1 ignored (crate unchanged).
- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh build -p bachuc-app` -> exit 0.
- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test -p bachuc-timeline -p bachuc-sidebar` -> exit 0; 86 passed, 0 failed (same as the baseline).
- `packages/desktop-gpui/cargo.sh fmt --check -p bachuc-client -p bachuc-protocol` -> exit 0.
- `git diff --stat main -- packages/protocol` -> empty.
- `live_plugin_list_matches_catalog` -> UNVERIFIED (unattended): live 6768 is down (`nc -z 127.0.0.1 6768` refused); no daemon started.

## Anti-goal

- Checkpoint 1, before the first change: `build -p bachuc-app` exit 0 and `test -p bachuc-timeline -p bachuc-sidebar` 86 passed; source `/tmp/wt-a-base-build.txt`, `/tmp/wt-a-base-test.txt`; 2026-10-16 18:31.
- Checkpoint 2, after `PluginEvent` landed: `build -p bachuc-app` exit 0 and timeline plus sidebar 86 passed; source `/tmp/wt-a-cp2-build.txt`, `/tmp/wt-a-cp2-test.txt`; 2026-10-16 18:39.
- Checkpoint 3, at completion (after TASK-169 too): `build -p bachuc-app` exit 0 and timeline plus sidebar 86 passed; source `/tmp/wt-a-v3.txt`, `/tmp/wt-a-v4.txt`; 2026-10-16 18:48. No edit outside `crates/client`.

## Lessons

- Every outgoing message passes through `session_frame`, which decodes it into the generated `WsInbound`; a new request must match the generated schema exactly (`codegen/protocol.schema.json` shows each shape).
- With `explicit_event_subscriptions`, the daemon starts its plugin change producers only when a subscription names one of the two `status.plugin_*` events (`refreshCatalogProducers`).
