//! Client behavior against a local fake daemon.

mod common;

use bachuc_client::{ClientEvent, TerminalFrame};
use common::{FakeServer, Harness, Log, agent_snapshot};
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::time::Duration;

const WAIT: Duration = Duration::from_secs(5);

fn user(text: &str) -> Value {
    json!({"type": "user_message", "text": text, "messageId": format!("u-{text}")})
}

fn chunk(text: &str, id: &str) -> Value {
    json!({"type": "assistant_message", "text": text, "messageId": id})
}

fn tool(call: &str, status: &str) -> Value {
    json!({"type": "tool_call", "callId": call, "name": "read", "status": status,
           "detail": {"type": "read", "filePath": "a.rs"}, "error": null})
}

/// A server with agent `a1` and a short turn.
fn server_with_turn() -> FakeServer {
    let server = FakeServer::start();
    server.add_agent("a1", "running");
    server.append("a1", user("hi"), "t1", false);
    server.append("a1", chunk("Hel", "m1"), "t1", false);
    server.append("a1", chunk("lo", "m1"), "t1", false);
    server.append("a1", tool("c1", "running"), "t1", false);
    server
}

fn view(server: &FakeServer, harness: &mut Harness, agent_id: &str) {
    assert!(harness.connected(), "connects");
    harness.core.view_agent(agent_id);
    let expected = server.projected(agent_id);
    assert!(
        harness.pump_until(WAIT, |h| h.entries(agent_id) == expected),
        "first tail page loads"
    );
}

/// The client's entries equal a fresh fetch of the same range, with no duplicates.
fn assert_matches_fresh(server: &FakeServer, harness: &Harness, agent_id: &str) {
    let entries = harness.entries(agent_id);
    let first = entries
        .first()
        .and_then(|e| e["seqStart"].as_u64())
        .unwrap_or(0);
    let fresh: Vec<Value> = server
        .projected(agent_id)
        .into_iter()
        .filter(|e| e["seqStart"].as_u64().unwrap() >= first)
        .collect();
    assert_eq!(entries, fresh);
    let starts: BTreeSet<u64> = entries
        .iter()
        .map(|e| e["seqStart"].as_u64().unwrap())
        .collect();
    assert_eq!(starts.len(), entries.len(), "no duplicate entries");
}

fn timeline_requests(server: &FakeServer, direction: &str) -> usize {
    server
        .requests_of("fetch_agent_timeline_request")
        .iter()
        .filter(|r| r["direction"] == direction)
        .count()
}

#[test]
fn queue_before_hello_holds_session_messages_until_server_info() {
    let server = FakeServer::start();
    server.add_agent("a1", "idle");
    server.with(|s| s.server_info_delay = Duration::from_millis(400));
    let mut harness = Harness::connect(&server.url, None);
    assert!(harness.connected());
    harness.pump_for(Duration::from_millis(300));
    let log = server.log();
    assert!(
        !log.iter().any(|e| matches!(e, Log::Violation(_))),
        "{log:?}"
    );
    let info = log
        .iter()
        .position(|e| matches!(e, Log::ServerInfoSent))
        .unwrap();
    let first_message = log
        .iter()
        .position(|e| matches!(e, Log::Message(_)))
        .unwrap();
    assert!(first_message > info);
    // The socket stayed open: one connection, no disconnect.
    assert_eq!(server.connect_times().len(), 1);
    assert!(
        !harness
            .events
            .iter()
            .any(|e| matches!(e, ClientEvent::Disconnected { .. }))
    );
    let hello = log.iter().find_map(|e| match e {
        Log::Hello(hello) => Some(hello.clone()),
        _ => None,
    });
    let hello = hello.unwrap();
    assert_eq!(hello["capabilities"]["owned_subscriptions"], true);
    assert_eq!(hello["capabilities"]["selective_agent_timeline"], true);
    assert_eq!(hello["capabilities"]["plugin_timeline_items"], true);
}

#[test]
fn wrong_password_disconnects_with_the_reason_and_backs_off() {
    let server = FakeServer::start();
    server.add_agent("a1", "idle");
    server.with(|s| s.password = Some("secret".into()));
    let mut harness = Harness::connect(&server.url, Some("wrong"));
    harness.pump_for(Duration::from_millis(3800));
    assert!(harness.events.contains(&ClientEvent::Disconnected {
        reason: "incorrect_password".into()
    }));
    assert!(!harness.events.contains(&ClientEvent::Connected));
    let times = server.connect_times();
    assert!(
        times.len() >= 3 && times.len() <= 4,
        "attempts: {}",
        times.len()
    );
    let floors = [0.5, 1.0, 2.0];
    for (index, pair) in times.windows(2).enumerate() {
        let gap = pair[1].duration_since(pair[0]).as_secs_f64();
        assert!(gap >= floors[index] * 0.95, "gap {index} was {gap:.3}s");
    }
}

#[test]
fn correct_password_connects() {
    let server = FakeServer::start();
    server.add_agent("a1", "idle");
    server.with(|s| s.password = Some("secret".into()));
    let mut harness = Harness::connect(&server.url, Some("secret"));
    assert!(harness.connected());
    assert!(harness.events.contains(&ClientEvent::Connected));
    assert_eq!(harness.core.agents()[0].id, "a1");
}

#[test]
fn directory_and_controls_come_from_the_host() {
    let server = FakeServer::start();
    server.add_agent("a1", "running");
    let mut harness = Harness::connect(&server.url, None);
    assert!(harness.connected());
    assert!(harness.pump_until(WAIT, |h| {
        h.core.providers().iter().any(|p| !p.models.is_empty()) && !h.core.workspaces().is_empty()
    }));
    let agent = &harness.core.agents()[0];
    assert_eq!(
        (agent.provider.as_str(), agent.model.as_deref()),
        ("mock", Some("m1"))
    );
    assert_eq!(agent.workspace_id.as_deref(), Some("ws1"));
    assert_eq!(harness.core.workspaces()[0].cwd, "/tmp/fake");
    let providers = harness.core.providers();
    assert_eq!(providers[0].label, "Mock");
    assert_eq!(
        providers[0].models,
        vec![("m1".to_string(), "Model 1".to_string())]
    );
    assert!(!providers[1].available);
    let controls = harness.core.agent_controls("a1").unwrap();
    assert_eq!(controls.current_mode.as_deref(), Some("load-test"));
    assert_eq!(controls.modes.len(), 2);
    assert!(controls.running);
    // Status and permission updates.
    let mut finished = agent_snapshot("a1", "idle");
    finished["pendingPermissions"] =
        json!([{"id": "p1", "provider": "mock", "name": "Bash", "kind": "tool"}]);
    server.broadcast(
        json!({"type": "agent_update", "payload": {"kind": "upsert", "agent": finished}}),
    );
    assert!(harness.pump_until(WAIT, |h| h.core.agents()[0].pending_permissions == 1));
    assert!(harness.events.contains(&ClientEvent::AgentFinished {
        agent_id: "a1".into()
    }));
    assert!(harness.events.contains(&ClientEvent::PermissionRequested {
        agent_id: "a1".into()
    }));
    server.broadcast(json!({"type": "agent_permission_resolved", "payload": {
        "agentId": "a1", "requestId": "p1", "resolution": {"behavior": "allow"}}}));
    assert!(harness.pump_until(WAIT, |h| h.core.agents()[0].pending_permissions == 0));
}

#[test]
fn live_rows_merge_in_seq_order() {
    let server = server_with_turn();
    let mut harness = Harness::connect(&server.url, None);
    view(&server, &mut harness, "a1");
    let subscribe = server.requests_of("agent.timeline.set_subscription.request");
    assert_eq!(subscribe[0]["agentIds"], json!(["a1"]));
    server.append("a1", chunk(" there", "m2"), "t1", true);
    server.append("a1", tool("c1", "completed"), "t1", true);
    server.append("a1", json!({"type": "reasoning", "text": "a"}), "t1", true);
    server.append("a1", json!({"type": "reasoning", "text": "b"}), "t1", true);
    let expected = server.projected("a1");
    assert!(harness.pump_until(WAIT, |h| h.entries("a1") == expected));
    assert!(harness.events.contains(&ClientEvent::TimelineChanged {
        agent_id: "a1".into()
    }));
    assert_eq!(timeline_requests(&server, "after"), 0);
}

#[test]
fn end_gap_fetches_after_pages_until_has_newer_is_false() {
    let server = server_with_turn();
    server.with(|s| s.page_cap = Some(2));
    let mut harness = Harness::connect(&server.url, None);
    harness.connected();
    harness.core.view_agent("a1");
    assert!(harness.pump_until(WAIT, |h| !h.entries("a1").is_empty()));
    harness.pump_for(Duration::from_millis(100));
    for index in 0..5 {
        server.append("a1", user(&format!("missed {index}")), "t1", false);
    }
    server.append("a1", user("live"), "t1", true);
    let expected = server.projected("a1");
    assert!(harness.pump_until(WAIT, |h| {
        h.entries("a1").last() == expected.last() && h.entries("a1").len() >= 6
    }));
    assert!(
        timeline_requests(&server, "after") >= 3,
        "paged after the gap"
    );
    assert_matches_fresh(&server, &harness, "a1");
}

#[test]
fn middle_gap_replaces_the_state_with_a_tail_page() {
    let server = server_with_turn();
    let mut harness = Harness::connect(&server.url, None);
    view(&server, &mut harness, "a1");
    for index in 0..4 {
        server.append("a1", user(&format!("trimmed {index}")), "t2", false);
    }
    // The daemon no longer holds the rows right after the client's end cursor.
    server.with(|s| s.timelines.get_mut("a1").unwrap().min_seq = 7);
    server.append("a1", user("live"), "t2", true);
    let expected = server.projected("a1");
    assert!(harness.pump_until(WAIT, |h| h.entries("a1") == expected));
    assert_eq!(timeline_requests(&server, "after"), 1);
    assert_matches_fresh(&server, &harness, "a1");
}

#[test]
fn epoch_change_replaces_the_state_with_a_tail_page() {
    let server = server_with_turn();
    let mut harness = Harness::connect(&server.url, None);
    view(&server, &mut harness, "a1");
    server.with(|s| {
        let timeline = s.timelines.get_mut("a1").unwrap();
        timeline.epoch = "e2".into();
        timeline.rows.clear();
    });
    server.append("a1", user("after rewind"), "t9", false);
    server.append("a1", chunk("new answer", "m9"), "t9", true);
    let expected = server.projected("a1");
    assert!(harness.pump_until(WAIT, |h| h.entries("a1") == expected));
    assert_eq!(expected.len(), 2);
    assert_eq!(timeline_requests(&server, "tail"), 2);
}

#[test]
fn replacement_message_replaces_the_timeline() {
    let server = server_with_turn();
    let mut harness = Harness::connect(&server.url, None);
    view(&server, &mut harness, "a1");
    server.with(|s| {
        let timeline = s.timelines.get_mut("a1").unwrap();
        timeline.epoch = "e3".into();
        timeline.rows.clear();
    });
    server.append("a1", user("replaced"), "t5", false);
    server.broadcast(
        json!({"type": "agent.timeline.replacement", "payload": {"agentId": "a1", "epoch": "e3"}}),
    );
    let expected = server.projected("a1");
    assert!(harness.pump_until(WAIT, |h| h.entries("a1") == expected));
    assert_eq!(harness.entries("a1").len(), 1);
}

#[test]
fn reconnect_resubscribes_and_refetches_without_duplicates() {
    let server = server_with_turn();
    let mut harness = Harness::connect(&server.url, None);
    view(&server, &mut harness, "a1");
    server.drop_connections();
    assert!(harness.pump_until(WAIT, |h| !h.core.is_connected()));
    server.append("a1", chunk(" more", "m1"), "t1", false);
    server.append("a1", tool("c1", "completed"), "t1", false);
    server.append("a1", user("next"), "t2", false);
    assert!(harness.pump_until(WAIT, |h| h.core.is_connected()));
    let expected = server.projected("a1");
    assert!(harness.pump_until(WAIT, |h| h.entries("a1") == expected));
    assert_matches_fresh(&server, &harness, "a1");
    assert_eq!(server.connect_times().len(), 2);
    assert_eq!(server.requests_of("fetch_agents_request").len(), 2);
    assert_eq!(
        server
            .requests_of("agent.timeline.set_subscription.request")
            .len(),
        2
    );
    assert_eq!(
        timeline_requests(&server, "after"),
        1,
        "resumed from the end cursor"
    );
    // Live rows still apply after the reconnect.
    server.append("a1", chunk("!", "m3"), "t2", true);
    let expected = server.projected("a1");
    assert!(harness.pump_until(WAIT, |h| h.entries("a1") == expected));
}

#[test]
fn older_pages_prepend_until_history_starts() {
    let server = FakeServer::start();
    server.add_agent("a1", "idle");
    for index in 0..25 {
        server.append(
            "a1",
            user(&format!("m{index}")),
            &format!("t{index}"),
            false,
        );
    }
    server.with(|s| s.page_cap = Some(10));
    let mut harness = Harness::connect(&server.url, None);
    harness.connected();
    harness.core.view_agent("a1");
    assert!(harness.pump_until(WAIT, |h| h.entries("a1").len() == 10));
    assert!(harness.core.timeline("a1").unwrap().has_older);
    while harness.core.timeline("a1").unwrap().has_older {
        let before = harness.entries("a1").len();
        harness.core.load_older("a1");
        harness.core.load_older("a1"); // a second call while one is in flight is ignored
        assert!(harness.pump_until(WAIT, |h| h.entries("a1").len() > before));
    }
    assert_eq!(harness.entries("a1"), server.projected("a1"));
    assert_eq!(timeline_requests(&server, "before"), 2);
}

#[test]
fn attach_terminal_gives_a_snapshot_first_then_output() {
    let server = FakeServer::start();
    server.add_agent("a1", "idle");
    let mut harness = Harness::connect(&server.url, None);
    assert!(harness.connected());
    let mut reply = harness.core.attach_terminal("term-1");
    let mut handle = None;
    assert!(harness.pump_until(WAIT, |_| {
        if let Ok(Some(result)) = reply.try_recv() {
            handle = Some(result.unwrap());
        }
        handle.is_some()
    }));
    let mut handle = handle.unwrap();
    assert_eq!(handle.id, "term-1");
    let mut frames = Vec::new();
    assert!(harness.pump_until(WAIT, |_| {
        while let Ok(frame) = handle.frames.try_recv() {
            frames.push(frame);
        }
        frames.len() >= 2
    }));
    assert!(matches!(&frames[0], TerminalFrame::Snapshot(state) if state["rows"] == 2));
    assert_eq!(frames[1], TerminalFrame::Output(b"ok".to_vec()));
    let request = &server.requests_of("subscribe_terminal_request")[0];
    assert!(request.get("restore").is_none());
    harness.core.terminal_input("term-1", b"ls\r");
    harness.core.terminal_resize("term-1", 30, 100, true);
    assert!(harness.pump_until(WAIT, |_| {
        server
            .log()
            .iter()
            .filter(|e| matches!(e, Log::Binary(_)))
            .count()
            >= 2
    }));
    let binaries: Vec<Vec<u8>> = server
        .log()
        .into_iter()
        .filter_map(|e| match e {
            Log::Binary(bytes) => Some(bytes),
            _ => None,
        })
        .collect();
    assert_eq!(binaries[0], b"\x02\x07ls\r".to_vec());
    assert_eq!(binaries[1][..2], [0x03, 7]);
    let resize: Value = serde_json::from_slice(&binaries[1][2..]).unwrap();
    assert_eq!(resize, json!({"rows": 30, "cols": 100, "intent": "claim"}));
}

#[test]
fn create_terminal_passes_the_workspace_of_its_folder() {
    let server = FakeServer::start();
    server.add_agent("a1", "idle");
    let mut harness = Harness::connect(&server.url, None);
    assert!(harness.connected());
    assert!(harness.pump_until(WAIT, |h| !h.core.workspaces().is_empty()));
    let mut reply = harness.core.create_terminal("/tmp/fake");
    let mut handle = None;
    assert!(harness.pump_until(WAIT, |_| {
        if let Ok(Some(result)) = reply.try_recv() {
            handle = Some(result.unwrap());
        }
        handle.is_some()
    }));
    let request = &server.requests_of("create_terminal_request")[0];
    assert_eq!(request["workspaceId"], "ws1");
}
