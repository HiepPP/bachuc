# TASK-270 Provider subagent timelines in the GPUI client

Group: A (the `bachuc-client` crate; shares `core.rs`, `lib.rs`, and `tests/rpcs.rs` with TASK-264)
Class: code

## Brief

Goal: The GPUI client loads the live timeline of a provider subagent, so a view can show the child's own steps.

Change: the client lists provider subagents but never fetches their timelines -> it fetches a child timeline by a key, applies its live items, and pages older rows.

Design: [packages/app/src/subagents/provider-store.ts](packages/app/src/subagents/provider-store.ts) (`replaceTimeline`, `applyUpdate`, `observeProviderSubagentTimeline`) and [packages/app/src/panels/provider-subagent-panel.tsx](packages/app/src/panels/provider-subagent-panel.tsx) (`loadOlder`).

Boundaries:

- Follows [ADR-0051](watchtower/decisions/ADR-0051-read-only-provider-subagent-tab-over-jump-to-parent.md).
- Wire: `agent.provider_subagents.timeline.get.request` with `parentAgentId`, `subagentId`, `direction` (`tail`, `before`, or `after`), `cursor`, and `limit` ([packages/protocol/src/messages.ts](packages/protocol/src/messages.ts):1926). The response has `epoch`, `reset`, `hasOlder`, `hasNewer`, `window`, and `rows` of `{item, timestamp, seq}` (`messages.ts:4823`). Live items come in `agent.provider_subagents.update` with `kind: "timeline"` (`messages.ts:4866`).
- Gate on `server_info.features.projectedSubagentTimeline`, as Electron does (`provider-subagent-panel.tsx`, COMPAT comment at `messages.ts:3784`).
- Live item rules, from `applyUpdate`: drop an item whose `epoch` differs from the stored epoch, and drop an item whose `seq` is not above the last seq.
- Hello capability (blocker): the daemon sends `timeline` updates only to a client that advertises `projected_subagent_timeline` (`packages/server/src/server/session.ts:1830-1839`, `:1877-1883`). Without it, the get response holds one fake "Please upgrade the Paseo app" row with `hasOlder` and `hasNewer` false (`session.ts:7998-8016`). GPUI sends 9 capabilities and not this one (`CAPABILITIES`, [packages/desktop-gpui/crates/client/src/connection.rs](packages/desktop-gpui/crates/client/src/connection.rs):63-73). Add it, as named in `packages/protocol/src/client-capabilities.ts:29`, and extend the test at `connection.rs:434`.
- Gate on `server_info.features.projectedSubagentTimeline` too, with a check next to `supports_provider_subagents` (`core.rs:867-873`).
- Planner call: key a child timeline by `provider_subagent_key(parent, child)`, and let `view_agent`, `unview_agent`, `timeline`, and `load_older` accept that key. The timeline view then needs no second data path. Build the key as Electron's stream ID: `provider:` plus the percent-encoded parent ID, `:`, and the percent-encoded child ID (`provider-subagent-panel.tsx`), so IDs with `:` cannot collide.
- A key must never reach a daemon request as an `agentId`. Three paths would send it today: `update_timeline_subscription` sends `viewed` as `agentIds` (`core.rs:822`), and `send_plan` (`core.rs:798`) builds `fetch_agent_timeline_request` through `FetchPlan::request` (`timeline.rs:38`), also on reconnect and epoch-change replans. Keep viewed keys in their own set, and branch `send_plan` for a key.
- Row shapes differ. The agent response has `entries`, which `TimelineSync::on_page` reads (`timeline.rs:248`, `:279`). The child response has `rows` of `{item, timestamp, seq, seqStart?, seqEnd?, sourceSeqRanges?}` without `provider` or `collapsed` (`messages.ts:4844-4860`). The live update is flat (`item`, `seq`, `epoch`, `provider`; `messages.ts:4875-4882`), while `LiveRow::from_stream` (`timeline.rs:86`) reads an `agent_stream` payload. Write an adapter into `TimelineSync`, or a small store of its own.
- `view_agent` of a key can run again on reconnect; it must be idempotent and must refetch the tail after a reconnect.
- A `remove` update for a viewed child drops its timeline, as Electron's `applyUpdate` does.
- The current handler of `agent.provider_subagents.update` reads only `upsert` and `remove` (`core.rs:2475-2499`). Add `timeline` there.

How:

- Add the key functions and the feature gate.
- Route `view_agent`, `unview_agent`, `timeline`, and `load_older` for a key to the child timeline request and store.
- Apply `timeline` updates for a viewed key.
- Add tests to [packages/desktop-gpui/crates/client/tests/rpcs.rs](packages/desktop-gpui/crates/client/tests/rpcs.rs) with the existing harness.

Files:

- [packages/desktop-gpui/crates/client/src/core.rs](packages/desktop-gpui/crates/client/src/core.rs) (request, response, live items, routing by key)
- [packages/desktop-gpui/crates/client/src/lib.rs](packages/desktop-gpui/crates/client/src/lib.rs) (key functions, feature gate, wrappers)
- [packages/desktop-gpui/crates/client/src/connection.rs](packages/desktop-gpui/crates/client/src/connection.rs) (add `projected_subagent_timeline` to `CAPABILITIES`, and its test)
- [packages/desktop-gpui/crates/client/tests/rpcs.rs](packages/desktop-gpui/crates/client/tests/rpcs.rs) (new tests)

Interfaces:

- Consumes: none. TASK-264 is a start order only, because both write the client files.
- Produces: `bachuc_client::provider_subagent_key(parent_agent_id: &str, subagent_id: &str) -> String` for TASK-271 and TASK-272
- Produces: `bachuc_client::parse_provider_subagent_key(key: &str) -> Option<(String, String)>` for TASK-271 and TASK-272
- Produces: `Client::supports_provider_subagent_timeline(&self) -> bool` for TASK-272
- Produces: `Client::view_agent`, `Client::unview_agent`, `Client::timeline`, and `Client::load_older` accept a provider subagent key as `agent_id`, for TASK-271

Expected result:

- Viewing a key sends one tail request for that child and stores its rows; `timeline(key)` returns them.
- A live `timeline` item with a higher seq and the same epoch is added; an older seq or another epoch is dropped.
- `load_older(key)` sends a `before` request with the first seq as the cursor, and the rows go above.
- No request carries a key as its `agentId`.
- A host without `projectedSubagentTimeline` gets no request.

Anti-goal: no existing `bachuc-client` test fails, and the agent timeline path sends the same requests as before; tripwire; read from `./cargo.sh test -p bachuc-client` before changes, after the routing change, and at completion.

## Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-client` -> the new tests and every old test pass.
- New test in `rpcs.rs`: `view_agent(provider_subagent_key("a1", "s1"))` -> the fake server gets `agent.provider_subagents.timeline.get.request` with `parentAgentId: "a1"`, `subagentId: "s1"`, and `direction: "tail"`; its reply rows show in `timeline(key)`.
- New test in `rpcs.rs`: live items with seq 5 then seq 4, and one with another epoch -> only seq 5 is added.
- New test in `rpcs.rs`: `load_older(key)` after a tail with `hasOlder: true` -> a `before` request with the first seq.
- New test in `rpcs.rs`: `server_info` without `projectedSubagentTimeline` -> `view_agent(key)` sends no request.
- Test in `connection.rs`: the hello frame lists `projected_subagent_timeline` -> the capability count is 10.
- New test in `rpcs.rs`: after `view_agent(key)` and a reconnect -> no `agent.timeline.set_subscription.request` and no `fetch_agent_timeline_request` carries the key, and a new tail request goes out.
- New test in `rpcs.rs`: a `remove` update for the viewed child -> `timeline(key)` is `None`.
- `parse_provider_subagent_key(provider_subagent_key("a:1", "s/2"))` -> `Some(("a:1", "s/2"))`; a plain agent ID -> `None`.
- Anti-goal: the old test count before changes equals the old tests still passing at completion; record both counts.
