# TASK-309 Client reads attentionTimestamp

Group: A (packages/desktop-gpui/crates/client/src/core.rs, packages/desktop-gpui/crates/client/src/lib.rs, packages/desktop-gpui/crates/client/tests/rpcs.rs)
Class: code

## Brief

Goal: The GPUI client keeps the daemon `attentionTimestamp` of each agent, so the unread queue can put the oldest first.

Change: `AgentExtra` drops `attentionTimestamp` -> it stores it and exposes `agent_attention_timestamp`.

Boundaries:

- The protocol already sends `attentionTimestamp: string | null` on agent snapshots in [packages/protocol/src/messages.ts](packages/protocol/src/messages.ts). Do not change the protocol.
- Trap: the upsert in `core.rs` returns early on an older `updatedAt`. Set the new field in the same block that sets `requires_attention`, so an old snapshot never writes it.
- A null, absent, or blank value gives `None`.

How:

- Add `attention_timestamp: Option<String>` to `AgentExtra` in `core.rs`, set beside `attention_reason`.
- Add `ClientCore::agent_attention_timestamp` in `core.rs` and a forwarding `Client::agent_attention_timestamp` in `lib.rs`, next to `agent_attention`.

Files:

- [packages/desktop-gpui/crates/client/src/core.rs](packages/desktop-gpui/crates/client/src/core.rs) (field, parse, accessor)
- [packages/desktop-gpui/crates/client/src/lib.rs](packages/desktop-gpui/crates/client/src/lib.rs) (forwarding accessor)
- [packages/desktop-gpui/crates/client/tests/rpcs.rs](packages/desktop-gpui/crates/client/tests/rpcs.rs) (red test)

Interfaces:

- Produces: `ClientCore::agent_attention_timestamp(&self, agent_id: &str) -> Option<String>` for TASK-312
- Produces: `Client::agent_attention_timestamp(&self, agent_id: &str) -> Option<String>` for TASK-312

Expected result:

- WHEN a snapshot carries `attentionTimestamp` THE client SHALL return it from `agent_attention_timestamp`.
- IF the value is null, blank, or absent THEN THE client SHALL return `None`.
- IF the agent is unknown THEN THE client SHALL return `None`.

Red tests:

File: [packages/desktop-gpui/crates/client/tests/rpcs.rs](packages/desktop-gpui/crates/client/tests/rpcs.rs)

```rust
#[test]
fn agent_attention_timestamp_reads_the_snapshot_value() {
    let server = FakeServer::start();
    server.with(|s| {
        let mut stamped = agent_snapshot("a1", "idle");
        stamped["requiresAttention"] = json!(true);
        stamped["attentionReason"] = json!("finished");
        stamped["attentionTimestamp"] = json!("2026-10-27T05:00:00.000Z");
        let mut null = agent_snapshot("a2", "idle");
        null["attentionTimestamp"] = Value::Null;
        let mut blank = agent_snapshot("a3", "idle");
        blank["attentionTimestamp"] = json!(" ");
        s.agents = vec![stamped, null, blank, agent_snapshot("a4", "idle")];
    });
    let mut harness = connect(&server);
    assert!(harness.pump_until(WAIT, |h| h.core.agents().len() == 4));
    assert_eq!(
        harness.core.agent_attention_timestamp("a1").as_deref(),
        Some("2026-10-27T05:00:00.000Z")
    );
    assert_eq!(harness.core.agent_attention_timestamp("a2"), None);
    assert_eq!(harness.core.agent_attention_timestamp("a3"), None);
    assert_eq!(harness.core.agent_attention_timestamp("a4"), None, "absent");
    assert_eq!(harness.core.agent_attention_timestamp("missing"), None);
}
```

Anti-goal: the existing `bachuc-client` tests stay green, 0 new failures; tripwire; read from `./cargo.sh test -p bachuc-client` before the change and at completion.

## Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-client --test rpcs agent_attention` -> both attention tests pass.
- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-client` -> all tests pass, same count of failures as the baseline (0).
