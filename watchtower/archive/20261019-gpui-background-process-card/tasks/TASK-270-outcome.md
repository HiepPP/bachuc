# TASK-270 outcome

Status: DONE

Changed:

- `packages/desktop-gpui/crates/client/src/connection.rs`: `projected_subagent_timeline` in `CAPABILITIES` (now 10) and its test.
- `packages/desktop-gpui/crates/client/src/lib.rs`: `provider_subagent_key`, `parse_provider_subagent_key`, the percent codec, `Client::supports_provider_subagent_timeline`.
- `packages/desktop-gpui/crates/client/src/core.rs`: `viewed_provider_subagents`, child request/response routing, live items, page adapter, remove handling, reconnect refetch.
- `packages/desktop-gpui/crates/client/tests/rpcs.rs`: seven tests.

Contract:

- `bachuc_client::provider_subagent_key(parent_agent_id: &str, subagent_id: &str) -> String` builds `provider:<parent>:<child>` with both ids percent-encoded as `encodeURIComponent` does (`provider_subagent_key("a:1", "s/2")` gives `provider:a%3A1:s%2F2`).
- `bachuc_client::parse_provider_subagent_key(key: &str) -> Option<(String, String)>`; a plain agent id gives `None`.
- `Client::supports_provider_subagent_timeline(&self) -> bool` reads `features.projectedSubagentTimeline == true`.
- `Client::view_agent`, `Client::unview_agent`, `Client::timeline`, and `Client::load_older` accept a key as `agent_id`.
- `send_plan` routes a key to `agent.provider_subagents.timeline.get.request`; keys never enter `agentIds` or an agent request.
- The child page maps into `TimelineSync`: rows gain `provider`, `collapsed`, and a default `sourceSeqRanges`. Live items apply only for a viewed key, with the same epoch and a seq above the last.

Verified:

- `./cargo.sh test -p bachuc-client` (group-level, after both TASKs): PASS. 135 passed, 0 failed, 5 ignored.
- `view_agent(key)` sends one tail request and stores the reply rows: PASS (`viewing_a_provider_subagent_key_fetches_and_stores_its_tail`).
- Live seq 5 then seq 4 and another epoch: only seq 5 is added: PASS (`provider_subagent_live_items_ignore_an_old_seq_or_another_epoch`).
- `load_older(key)` sends `before` with the first seq and prepends rows: PASS (`loading_older_provider_rows_sends_a_before_request_and_prepends_them`).
- Host without `projectedSubagentTimeline` sends no child request: PASS (`a_host_without_projected_subagent_timeline_sends_no_child_request`).
- Hello capability count is 10 and lists `projected_subagent_timeline`: PASS (`connection::tests::hello_carries_the_capabilities_and_password`).
- Reconnect: no key in `agent.timeline.set_subscription.request` or `fetch_agent_timeline_request`, and a new tail goes out: PASS (`a_reconnect_refetches_the_child_tail_and_never_sends_its_key_as_an_agent`).
- `remove` for the viewed child clears `timeline(key)`: PASS (`a_remove_update_drops_a_viewed_child_timeline`).
- Key round trip and plain-id rejection: PASS (`provider_subagent_keys_round_trip_and_plain_agent_ids_do_not`).
- `./cargo.sh build`: PASS, exit 0.
- `./cargo.sh fmt --check -p bachuc-client`: PASS, exit 0.
- Anti-goal: before changes 123 tests, 122 passed plus the pre-existing `one_thousand_workspace_updates_stay_cheap` load flake (passes alone). At completion all 123 old tests pass; the agent timeline path sends the same requests (the 22 `sync` tests stay green).

Anti-goal: no existing `bachuc-client` test fails; the agent timeline path is unchanged because `send_plan`, `update_timeline_subscription`, and `load_older` branch only for a parsed key.

Lessons:

- The child `rows` lack `provider` and `collapsed`, so the page adapter adds them before `TimelineSync::on_page`; the live update is flat and becomes a `LiveRow`.
- A child timeline reuses `TimelineSync` under the key in `timelines`, so `timeline(key)` needs no second path.
- A live item of another epoch is dropped before `on_live`, as the app's `applyUpdate` does; a gap still fetches an `after` page for the child.
- TASK-271 and TASK-272 can call `view_agent(key)` and read `timeline(key)` without new client state.
