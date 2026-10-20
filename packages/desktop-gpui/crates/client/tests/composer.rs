//! Composer, New workspace, and tab menu data (TASK-169) against the local fake daemon.

mod common;

use bachuc_client::{
    ActiveTurn, AddedProject, AgentFeatureKind, AgentUsage, BranchSuggestion, BranchValidation,
    ClientEvent, CreateWorkspace, CreatedWorkspace, DiffCompare, DiffMode, DraftConfig,
    InitialAgent, OutgoingMessage, SubagentKind, TaskEntry, TaskStatus, ThinkingControls,
    TurnBehavior, TurnMetrics, WorkspaceSource, WorktreeAction, resume_command,
};
use common::{FakeServer, Harness, agent_snapshot, session};
use serde_json::{Value, json};
use std::time::{Duration, Instant};

const WAIT: Duration = Duration::from_secs(5);

fn connect(server: &FakeServer) -> Harness {
    let mut harness = Harness::connect(&server.url, None);
    assert!(harness.connected(), "connects");
    harness
}

fn sent(server: &FakeServer, harness: &mut Harness, kind: &str) -> Value {
    assert!(
        harness.pump_until(WAIT, |_| !server.requests_of(kind).is_empty()),
        "{kind} is sent"
    );
    server.requests_of(kind).pop().unwrap()
}

fn entry(seq: u64, item: Value, turn: &str) -> Value {
    json!({
        "provider": "mock", "item": item, "turnId": turn,
        "timestamp": format!("2026-10-16T00:00:{:02}.000Z", seq % 60),
        "seqStart": seq, "seqEnd": seq,
        "sourceSeqRanges": [{"startSeq": seq, "endSeq": seq}], "collapsed": [],
    })
}

fn user(seq: u64, turn: &str) -> Value {
    entry(seq, json!({"type": "user_message", "text": "hi"}), turn)
}

fn assistant(seq: u64, turn: &str) -> Value {
    entry(
        seq,
        json!({"type": "assistant_message", "text": "ok", "messageId": format!("m{seq}")}),
        turn,
    )
}

fn todo(seq: u64, label: &str) -> Value {
    entry(
        seq,
        json!({"type": "todo", "items": [
            {"text": label, "completed": false, "id": format!("{label}-1"), "status": "in_progress",
             "activeForm": format!("Doing {label}")},
            {"text": format!("{label} done"), "completed": true},
        ]}),
        "t1",
    )
}

/// A projected page of `entries`; the cursors span their seqs.
fn page(request: &Value, entries: Vec<Value>, has_older: bool) -> Value {
    let first = entries
        .first()
        .and_then(|e| e["seqStart"].as_u64())
        .unwrap_or(1);
    let last = entries
        .last()
        .and_then(|e| e["seqEnd"].as_u64())
        .unwrap_or(0);
    json!({
        "requestId": request["requestId"], "agentId": request["agentId"],
        "direction": request["direction"], "projection": "projected", "epoch": "e1",
        "reset": false, "staleCursor": false, "gap": false,
        "window": {"minSeq": 1, "maxSeq": last, "nextSeq": last + 1},
        "startCursor": {"epoch": "e1", "seq": first}, "endCursor": {"epoch": "e1", "seq": last},
        "hasOlder": has_older, "hasNewer": false, "entries": entries, "error": null,
    })
}

fn timeline_reply(server: &FakeServer, tail: Vec<Value>, older: Vec<Value>) {
    server.on("fetch_agent_timeline_request", move |request| {
        let payload = if request["direction"] == "before" {
            page(request, older.clone(), false)
        } else {
            page(request, tail.clone(), !older.is_empty())
        };
        vec![session("fetch_agent_timeline_response", payload)]
    });
}

fn stream(agent_id: &str, event: Value, seq: Option<u64>) -> Value {
    let mut payload = json!({"agentId": agent_id, "event": event,
                             "timestamp": "2026-10-16T00:01:00.000Z"});
    if let Some(seq) = seq {
        payload["seq"] = json!(seq);
        payload["epoch"] = json!("e1");
    }
    json!({"type": "agent_stream", "payload": payload})
}

fn version(harness: &Harness, agent_id: &str) -> u64 {
    harness.core.timeline(agent_id).map_or(0, |t| t.version)
}

#[test]
fn last_turn_metrics_stop_at_the_newest_user_message_and_follow_turn_completed() {
    let server = FakeServer::start();
    server.add_agent("a1", "running");
    let mut metrics_entry = assistant(2, "t1");
    metrics_entry["turnMetrics"] =
        json!({"ttftMs": 120.5, "outputTokens": 40, "outputTokensPerSecond": 22.0});
    timeline_reply(
        &server,
        vec![user(1, "t1"), metrics_entry, user(3, "t2")],
        vec![],
    );
    let mut harness = connect(&server);
    harness.core.view_agent("a1");
    assert!(harness.pump_until(WAIT, |h| h.entries("a1").len() == 3));
    // An older turn has metrics, but the newest user message has none after it.
    assert_eq!(harness.core.agent_last_turn_metrics("a1"), None);

    server.broadcast(stream(
        "a1",
        json!({"type": "timeline", "provider": "mock", "turnId": "t2",
               "item": {"type": "assistant_message", "text": "done", "messageId": "m4"}}),
        Some(4),
    ));
    assert!(harness.pump_until(WAIT, |h| h.entries("a1").len() == 4));
    assert_eq!(harness.core.agent_last_turn_metrics("a1"), None);
    let before = version(&harness, "a1");
    server.broadcast(stream(
        "a1",
        json!({"type": "turn_completed", "provider": "mock", "turnId": "t2",
               "metrics": {"ttftMs": 80, "outputTokens": 12}}),
        None,
    ));
    let expected = TurnMetrics {
        ttft_ms: Some(80.0),
        output_tokens: Some(12),
        output_tokens_per_second: None,
    };
    assert!(
        harness.pump_until(WAIT, |h| h.core.agent_last_turn_metrics("a1")
            == Some(expected))
    );
    assert!(version(&harness, "a1") > before, "the version grows");
    assert!(harness.events.contains(&ClientEvent::TimelineChanged {
        agent_id: "a1".into()
    }));
    // The metrics sit where a fresh fetch puts them.
    assert_eq!(
        harness.entries("a1")[3]["turnMetrics"],
        json!({"ttftMs": 80, "outputTokens": 12})
    );
}

#[test]
fn tasks_keep_the_newest_todo_across_older_pages_and_live_rows() {
    let server = FakeServer::start();
    server.add_agent("a1", "running");
    timeline_reply(
        &server,
        vec![todo(3, "first"), user(4, "t1"), todo(5, "second")],
        vec![user(1, "t1"), todo(2, "oldest")],
    );
    let mut harness = connect(&server);
    assert!(harness.core.agent_tasks("a1").is_empty(), "not viewed");
    harness.core.view_agent("a1");
    assert!(harness.pump_until(WAIT, |h| h.entries("a1").len() == 3));
    let second = vec![
        TaskEntry {
            id: Some("second-1".into()),
            text: "second".into(),
            completed: false,
            status: Some(TaskStatus::InProgress),
            active_form: Some("Doing second".into()),
        },
        TaskEntry {
            id: None,
            text: "second done".into(),
            completed: true,
            status: None,
            active_form: None,
        },
    ];
    assert_eq!(harness.core.agent_tasks("a1"), second);

    harness.core.load_older("a1");
    assert!(harness.pump_until(WAIT, |h| h.entries("a1").len() == 5));
    assert_eq!(harness.core.agent_tasks("a1"), second, "an older todo");

    server.broadcast(stream(
        "a1",
        json!({"type": "timeline", "provider": "mock", "turnId": "t1",
               "item": {"type": "todo", "items": [{"text": "live", "completed": true,
                                                    "status": "completed"}]}}),
        Some(6),
    ));
    assert!(harness.pump_until(WAIT, |h| h.core.agent_tasks("a1").len() == 1));
    assert_eq!(
        harness.core.agent_tasks("a1")[0].status,
        Some(TaskStatus::Completed)
    );
    harness.core.unview_agent("a1");
    assert!(harness.core.agent_tasks("a1").is_empty(), "unviewed");
}

fn child(id: &str, parent: &str, created_at: &str) -> Value {
    let mut agent = agent_snapshot(id, "running");
    agent["labels"] = json!({"paseo.parent-agent-id": parent});
    agent["createdAt"] = json!(created_at);
    agent
}

fn provider_subagent(id: &str, created_at: &str, status: &str) -> Value {
    json!({"id": id, "parentAgentId": "p", "provider": "claude", "title": format!("task {id}"),
           "description": "explore", "status": status, "createdAt": created_at,
           "updatedAt": created_at, "toolCallId": null, "subtitle": "3 tools"})
}

#[test]
fn subagents_merge_paseo_and_provider_rows_by_created_at() {
    let server = FakeServer::start();
    server.with(|s| {
        s.features.insert("providerSubagents".into(), json!(true));
        let mut blank = child("c3", "  ", "2026-10-16T00:00:01.000Z");
        blank["title"] = Value::Null;
        s.agents = vec![
            agent_snapshot("p", "running"),
            child("c2", " p ", "2026-10-16T00:00:05.000Z"),
            child("c1", "p", "2026-10-16T00:00:02.000Z"),
            blank,
        ];
    });
    server.on("agent.provider_subagents.list.request", |request| {
        vec![session(
            "agent.provider_subagents.list.response",
            json!({"requestId": request["requestId"], "parentAgentId": request["parentAgentId"],
                   "subagents": [provider_subagent("s1", "2026-10-16T00:00:03.000Z", "running")],
                   "error": null}),
        )]
    });
    let mut harness = connect(&server);
    assert!(harness.pump_until(WAIT, |h| h.core.agents().len() == 4));
    assert_eq!(harness.core.agent_parent_id("c3"), None, "a blank label");
    assert_eq!(harness.core.agent_parent_id("c2").as_deref(), Some("p"));
    assert_eq!(harness.core.agent_parent_id("missing"), None);

    harness.core.view_agent("p");
    let request = sent(
        &server,
        &mut harness,
        "agent.provider_subagents.list.request",
    );
    assert_eq!(request["parentAgentId"], "p");
    assert!(harness.pump_until(WAIT, |h| h.core.subagents("p").len() == 3));
    let rows = harness.core.subagents("p");
    let ids: Vec<&str> = rows.iter().map(|row| row.id.as_str()).collect();
    assert_eq!(ids, ["c1", "s1", "c2"]);
    assert_eq!(rows[0].kind, SubagentKind::Paseo);
    assert_eq!(rows[0].title.as_deref(), Some("agent c1"));
    assert_eq!(rows[1].kind, SubagentKind::Provider);
    assert_eq!(rows[1].subtitle.as_deref(), Some("3 tools"));
    assert_eq!(rows[1].description.as_deref(), Some("explore"));
    assert!(!rows[1].requires_attention);

    // A provider upsert adds a row and marks the directory changed.
    harness.events.clear();
    server.broadcast(json!({"type": "agent.provider_subagents.update", "payload": {
        "kind": "upsert", "subagent": provider_subagent("s2", "2026-10-16T00:00:04.000Z", "failed")}}));
    assert!(harness.pump_until(WAIT, |h| h.core.subagents("p").len() == 4));
    let rows = harness.core.subagents("p");
    let ids: Vec<&str> = rows.iter().map(|row| row.id.as_str()).collect();
    assert_eq!(ids, ["c1", "s1", "s2", "c2"]);
    assert!(rows[2].requires_attention, "a failed provider row");
    assert!(harness.events.contains(&ClientEvent::AgentsChanged));

    // After a reconnect the viewed agent's list is asked for again.
    server.drop_connections();
    assert!(
        harness.pump_until(Duration::from_secs(10), |_| server
            .requests_of("agent.provider_subagents.list.request")
            .len()
            >= 2),
        "lists again after the reconnect"
    );
    let subscription = server
        .requests_of("session.events.set_subscription.request")
        .pop()
        .unwrap();
    assert!(
        subscription["events"]
            .as_array()
            .unwrap()
            .contains(&json!("agent.provider_subagents.update"))
    );
}

#[test]
fn without_provider_subagents_no_list_is_sent_and_no_provider_rows_show() {
    let server = FakeServer::start();
    server.with(|s| {
        s.agents = vec![
            agent_snapshot("p", "running"),
            child("c1", "p", "2026-10-16T00:00:02.000Z"),
        ];
    });
    let mut harness = connect(&server);
    harness.core.view_agent("p");
    server.broadcast(json!({"type": "agent.provider_subagents.update", "payload": {
        "kind": "upsert", "subagent": provider_subagent("s1", "2026-10-16T00:00:03.000Z", "running")}}));
    harness.pump_for(Duration::from_millis(300));
    assert!(
        server
            .requests_of("agent.provider_subagents.list.request")
            .is_empty()
    );
    let rows = harness.core.subagents("p");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, "c1");
}

#[test]
fn snapshot_fields_active_turn_usage_session_and_updated_at() {
    let server = FakeServer::start();
    server.with(|s| {
        let mut busy = agent_snapshot("a1", "running");
        busy["activeTurn"] = json!({"turnId": "t9", "startedAt": "2026-10-16T00:00:00.000Z"});
        busy["lastUsage"] = json!({"inputTokens": 1200, "cachedInputTokens": 800,
            "outputTokens": 90, "totalCostUsd": 0.25, "contextWindowMaxTokens": 200000,
            "contextWindowUsedTokens": 15000});
        busy["runtimeInfo"] = json!({"provider": "claude", "sessionId": "runtime-1"});
        busy["persistence"] = json!({"provider": "claude", "sessionId": "stored-1"});
        busy["updatedAt"] = json!("2026-10-16T01:00:00.000Z");
        let mut idle = agent_snapshot("a2", "idle");
        idle["activeTurn"] = Value::Null;
        idle["runtimeInfo"] = json!({"provider": "claude", "sessionId": null});
        idle["persistence"] = json!({"provider": "claude", "sessionId": "stored-2"});
        s.agents = vec![busy, idle, agent_snapshot("a3", "idle")];
    });
    let mut harness = connect(&server);
    assert!(harness.pump_until(WAIT, |h| h.core.agents().len() == 3));
    assert_eq!(
        harness.core.agent_active_turn("a1"),
        Some(ActiveTurn {
            turn_id: "t9".into(),
            started_at: Some("2026-10-16T00:00:00.000Z".into())
        })
    );
    assert_eq!(harness.core.agent_active_turn("a2"), None);
    assert_eq!(
        harness.core.agent_last_usage("a1"),
        Some(AgentUsage {
            input_tokens: Some(1200),
            cached_input_tokens: Some(800),
            output_tokens: Some(90),
            total_cost_usd: Some(0.25),
            context_window_max_tokens: Some(200000),
            context_window_used_tokens: Some(15000),
        })
    );
    assert_eq!(harness.core.agent_last_usage("a2"), None);
    assert_eq!(
        harness.core.agent_provider_session_id("a1").as_deref(),
        Some("runtime-1")
    );
    assert_eq!(
        harness.core.agent_provider_session_id("a2").as_deref(),
        Some("stored-2")
    );
    assert_eq!(harness.core.agent_provider_session_id("a3"), None);
    assert_eq!(
        harness.core.agent_updated_at("a1").as_deref(),
        Some("2026-10-16T01:00:00.000Z")
    );
    assert_eq!(harness.core.agent_updated_at("missing"), None);
}

#[test]
fn send_and_enqueue_carry_attachments_and_resolve_on_the_answer() {
    let server = FakeServer::start();
    server.add_agent("a1", "running");
    server.on("send_agent_message_request", |request| {
        let accepted = request["text"] != "refuse";
        vec![session(
            "send_agent_message_response",
            json!({"requestId": request["requestId"], "agentId": request["agentId"],
                   "accepted": accepted,
                   "error": if accepted { Value::Null } else { json!("Agent is closed") }}),
        )]
    });
    server.on("agent.queue.enqueue.request", |request| {
        vec![session(
            "agent.queue.enqueue.response",
            json!({"requestId": request["requestId"], "agentId": request["agentId"],
                   "itemId": "q1", "error": null,
                   "queue": {"held": false, "items": [{"id": "q1", "text": request["text"],
                             "attachmentKinds": ["text"], "createdAt": "now"}]}}),
        )]
    });
    let mut harness = connect(&server);
    let message = |text: &str| OutgoingMessage {
        text: text.into(),
        images: vec![json!({"data": "aGk=", "mimeType": "image/png"})],
        attachments: vec![json!({"type": "text", "mimeType": "text/plain", "text": "chip"})],
    };
    let reply = harness
        .core
        .send_outgoing("a1", message("go"), TurnBehavior::Steer);
    harness.wait(reply, WAIT).expect("accepted");
    let request = server.requests_of("send_agent_message_request").remove(0);
    assert_eq!(request["text"], "go");
    assert_eq!(request["activeTurnBehavior"], "steer");
    assert_eq!(
        request["images"],
        json!([{"data": "aGk=", "mimeType": "image/png"}])
    );
    assert_eq!(
        request["attachments"],
        json!([{"type": "text", "mimeType": "text/plain", "text": "chip"}])
    );
    let reply = harness
        .core
        .send_outgoing("a1", message("refuse"), TurnBehavior::Interrupt);
    let error = harness.wait(reply, WAIT).expect_err("refused");
    assert_eq!(error.to_string(), "Agent is closed");

    let reply = harness.core.enqueue_outgoing("a1", message("later"));
    harness.wait(reply, WAIT).expect("queued");
    let request = server.requests_of("agent.queue.enqueue.request").remove(0);
    assert_eq!(request["attachments"][0]["text"], "chip");
    assert_eq!(request["images"][0]["mimeType"], "image/png");
    assert_eq!(harness.core.queued_prompts("a1")[0].text, "later");
}

#[test]
fn queue_requests_and_the_held_flag() {
    let server = FakeServer::start();
    server.with(|s| {
        let mut agent = agent_snapshot("a1", "idle");
        agent["queue"] = json!({"held": true, "items": [
            {"id": "q1", "text": "one", "attachmentKinds": [], "createdAt": "now"},
            {"id": "q2", "text": "two", "attachmentKinds": [], "createdAt": "now"}]});
        s.agents = vec![agent];
    });
    for kind in [
        "agent.queue.edit",
        "agent.queue.reorder",
        "agent.queue.promote",
        "agent.queue.resume",
    ] {
        let response = format!("{kind}.response");
        server.on(&format!("{kind}.request"), move |request| {
            vec![session(
                &response,
                json!({"requestId": request["requestId"], "agentId": request["agentId"],
                       "error": null, "queue": {"held": false, "items": [
                           {"id": "q2", "text": "two", "attachmentKinds": [], "createdAt": "now"}]}}),
            )]
        });
    }
    let mut harness = connect(&server);
    assert!(harness.pump_until(WAIT, |h| !h.core.queued_prompts("a1").is_empty()));
    assert!(harness.core.queue_held("a1"));
    assert!(!harness.core.queue_held("missing"));

    harness.core.edit_queued("a1", "q1", "uno");
    harness.core.reorder_queued("a1", "q2", Some("q1"));
    harness.core.reorder_queued("a1", "q1", None);
    harness.core.promote_queued("a1", "q1");
    harness.core.resume_queue("a1");
    assert!(harness.pump_until(WAIT, |h| !h.core.queue_held("a1")));
    let strip = |mut request: Value| {
        request.as_object_mut().unwrap().remove("requestId");
        request
    };
    assert_eq!(
        strip(sent(&server, &mut harness, "agent.queue.edit.request")),
        json!({"type": "agent.queue.edit.request", "agentId": "a1", "itemId": "q1", "text": "uno"})
    );
    assert!(harness.pump_until(
        WAIT,
        |_| server.requests_of("agent.queue.reorder.request").len() == 2
    ));
    let reorders: Vec<Value> = server
        .requests_of("agent.queue.reorder.request")
        .into_iter()
        .map(strip)
        .collect();
    assert_eq!(
        reorders,
        vec![
            json!({"type": "agent.queue.reorder.request", "agentId": "a1", "itemId": "q2",
                   "beforeItemId": "q1"}),
            json!({"type": "agent.queue.reorder.request", "agentId": "a1", "itemId": "q1",
                   "beforeItemId": null}),
        ]
    );
    assert_eq!(
        strip(sent(&server, &mut harness, "agent.queue.promote.request")),
        json!({"type": "agent.queue.promote.request", "agentId": "a1", "itemId": "q1"})
    );
    assert_eq!(
        strip(sent(&server, &mut harness, "agent.queue.resume.request")),
        json!({"type": "agent.queue.resume.request", "agentId": "a1"})
    );
    let ids: Vec<String> = harness
        .core
        .queued_prompts("a1")
        .into_iter()
        .map(|q| q.id)
        .collect();
    assert_eq!(ids, ["q2"]);
}

#[test]
fn modes_draft_features_default_model_and_model_thinking() {
    let server = FakeServer::start();
    server.add_agent("a1", "idle");
    server.on("list_provider_models_request", |request| {
        vec![session(
            "list_provider_models_response",
            json!({"requestId": request["requestId"], "fetchedAt": "now",
                   "provider": request["provider"], "models": [
                {"provider": "mock", "id": "m1", "label": "Model 1"},
                {"provider": "mock", "id": "m2", "label": "Model 2", "isDefault": true,
                 "thinkingOptions": [{"id": "low", "label": "Low"},
                                     {"id": "high", "label": "High", "isDefault": true}]},
            ]}),
        )]
    });
    server.on("list_provider_modes_request", |request| {
        vec![session(
            "list_provider_modes_response",
            json!({"requestId": request["requestId"], "fetchedAt": "now",
                   "provider": request["provider"],
                   "modes": [{"id": "plan", "label": "Plan"}, {"id": "auto", "label": "Auto"}]}),
        )]
    });
    server.on("list_provider_features_request", |request| {
        vec![session(
            "list_provider_features_response",
            json!({"requestId": request["requestId"], "fetchedAt": "now", "provider": "mock",
                   "features": [
                {"type": "toggle", "id": "fast", "label": "Fast", "value": true},
                {"type": "slider", "id": "later", "label": "From a newer daemon", "value": 3},
            ]}),
        )]
    });
    let mut harness = connect(&server);
    assert!(harness.pump_until(WAIT, |h| h.core.default_model("mock").is_some()));
    assert_eq!(harness.core.default_model("mock").as_deref(), Some("m2"));
    assert_eq!(harness.core.default_model("pi"), None);
    assert_eq!(
        harness.core.model_thinking("mock", "m2"),
        Some(ThinkingControls {
            options: vec![("low".into(), "Low".into()), ("high".into(), "High".into())],
            current: Some("high".into()),
        })
    );
    assert_eq!(harness.core.model_thinking("mock", "m1"), None);

    let reply = harness.core.provider_modes("mock", Some("/repo"));
    assert_eq!(
        harness.wait(reply, WAIT).unwrap(),
        vec![
            ("plan".to_string(), "Plan".to_string()),
            ("auto".to_string(), "Auto".to_string())
        ]
    );
    let request = server.requests_of("list_provider_modes_request").remove(0);
    assert_eq!(request["cwd"], "/repo");

    let reply = harness.core.draft_features(DraftConfig {
        provider: "mock".into(),
        cwd: "/repo".into(),
        model: Some("m2".into()),
        mode_id: None,
        thinking_option_id: Some("high".into()),
    });
    let features = harness.wait(reply, WAIT).unwrap();
    assert_eq!(features.len(), 1, "the unknown feature type is skipped");
    assert_eq!(features[0].kind, AgentFeatureKind::Toggle { value: true });
    let request = server
        .requests_of("list_provider_features_request")
        .remove(0);
    assert_eq!(
        request["draftConfig"],
        json!({"provider": "mock", "cwd": "/repo", "model": "m2", "thinkingOptionId": "high"})
    );
}

fn initial_agent() -> InitialAgent {
    InitialAgent {
        config: DraftConfig {
            provider: "claude".into(),
            cwd: "/repo".into(),
            model: Some("opus".into()),
            mode_id: Some("plan".into()),
            thinking_option_id: None,
        },
        feature_values: [("fast".to_string(), json!(true))].into(),
        env: [("PASEO_DRAFT".to_string(), "1".to_string())].into(),
        initial_prompt: "build it".into(),
        images: vec![json!({"data": "aGk=", "mimeType": "image/png"})],
        attachments: vec![json!({"type": "text", "mimeType": "text/plain", "text": "chip"})],
        labels: [("team".to_string(), "ui".to_string())].into(),
    }
}

#[test]
fn create_workspace_sends_the_directory_and_worktree_payloads() {
    let server = FakeServer::start();
    server.add_agent("a1", "idle");
    server.on("workspace.create.request", |request| {
        let payload = if request["title"] == "fail" {
            json!({"requestId": request["requestId"], "workspace": null, "setupTerminalId": null,
                   "error": "Directory not found"})
        } else {
            json!({"requestId": request["requestId"], "setupTerminalId": "term-9", "error": null,
                   "workspace": {"id": "wks_1"}, "agent": {"id": "agent-9"}})
        };
        vec![session("workspace.create.response", payload)]
    });
    let mut harness = connect(&server);
    let reply = harness.core.create_workspace(CreateWorkspace {
        source: WorkspaceSource::Directory {
            path: "/repo".into(),
            project_id: Some("p1".into()),
        },
        title: None,
        first_agent_prompt: Some(String::new()),
        agent: None,
    });
    assert_eq!(
        harness.wait(reply, WAIT).unwrap(),
        CreatedWorkspace {
            workspace_id: "wks_1".into(),
            agent_id: Some("agent-9".into()),
            setup_terminal_id: Some("term-9".into()),
        }
    );
    let mut request = server.requests_of("workspace.create.request").remove(0);
    request.as_object_mut().unwrap().remove("requestId");
    // No receipts feature: no idempotency key; an empty prompt: no first agent context.
    assert_eq!(
        request,
        json!({"type": "workspace.create.request",
               "source": {"kind": "directory", "path": "/repo", "projectId": "p1"}})
    );

    let reply = harness.core.create_workspace(CreateWorkspace {
        source: WorkspaceSource::Directory {
            path: "/missing".into(),
            project_id: None,
        },
        title: Some("fail".into()),
        first_agent_prompt: None,
        agent: None,
    });
    let error = harness.wait(reply, WAIT).expect_err("error answer");
    assert_eq!(error.to_string(), "Directory not found");
}

#[test]
fn create_workspace_with_receipts_sends_a_key_the_prompt_and_the_agent() {
    let server = FakeServer::start();
    server.add_agent("a1", "idle");
    server.with(|s| {
        s.features
            .insert("workspaceRequestReceipts".into(), json!(true));
    });
    server.reply(
        "workspace.create.request",
        "workspace.create.response",
        json!({"setupTerminalId": null, "error": null, "workspace": {"id": "wks_2"}}),
    );
    let mut harness = connect(&server);
    let reply = harness.core.create_workspace(CreateWorkspace {
        source: WorkspaceSource::Worktree {
            cwd: "/repo".into(),
            project_id: Some("p1".into()),
            action: Some(WorktreeAction::BranchOff),
            ref_name: None,
            base_branch: Some("main".into()),
            branch_name: Some("feat/x".into()),
            worktree_slug: Some("bright-owl".into()),
        },
        title: Some("Feature".into()),
        first_agent_prompt: Some("build it".into()),
        agent: Some(initial_agent()),
    });
    assert_eq!(
        harness.wait(reply, WAIT).unwrap(),
        CreatedWorkspace {
            workspace_id: "wks_2".into(),
            agent_id: None,
            setup_terminal_id: None,
        }
    );
    let request = server.requests_of("workspace.create.request").remove(0);
    assert!(
        request["idempotencyKey"]
            .as_str()
            .is_some_and(|key| !key.is_empty())
    );
    assert_eq!(
        request["source"],
        json!({"kind": "worktree", "cwd": "/repo", "projectId": "p1", "action": "branch-off",
               "baseBranch": "main", "branchName": "feat/x", "worktreeSlug": "bright-owl"})
    );
    assert_eq!(request["title"], "Feature");
    assert_eq!(request["firstAgentContext"], json!({"prompt": "build it"}));
    let agent = &request["agent"];
    assert_eq!(
        agent["config"],
        json!({"provider": "claude", "cwd": "/repo", "model": "opus", "modeId": "plan",
               "featureValues": {"fast": true}})
    );
    assert_eq!(agent["env"], json!({"PASEO_DRAFT": "1"}));
    assert_eq!(agent["initialPrompt"], "build it");
    assert!(agent["clientMessageId"].is_string());
    assert_eq!(agent["labels"], json!({"team": "ui"}));
    assert_eq!(agent["images"][0]["mimeType"], "image/png");
    assert_eq!(agent["attachments"][0]["text"], "chip");
}

#[test]
fn create_workspace_resolves_when_its_agent_is_ready() {
    let server = FakeServer::start();
    server.add_agent("a1", "idle");
    server.with(|s| {
        s.features
            .insert("workspaceRequestReceipts".into(), json!(true));
        s.features.insert("creationLifecycle".into(), json!(true));
    });
    // The daemon answers only after the first turn starts; the agent exists from `agent_ready`.
    server.on("workspace.create.request", |request| {
        let update = |key: &Value, phase: &str| {
            session(
                "workspace.create.update",
                json!({"kind": "workspace", "idempotencyKey": key, "revision": 2, "phase": phase,
                       "workspaceId": "wks_3", "agentId": "agent-3", "agent": {"id": "agent-3"},
                       "error": null}),
            )
        };
        vec![
            update(&json!("other-key"), "agent_ready"),
            update(&request["idempotencyKey"], "workspace_ready"),
            update(&request["idempotencyKey"], "agent_ready"),
        ]
    });
    let mut harness = connect(&server);
    let reply = harness.core.create_workspace(CreateWorkspace {
        source: WorkspaceSource::Directory {
            path: "/repo".into(),
            project_id: None,
        },
        title: None,
        first_agent_prompt: None,
        agent: Some(initial_agent()),
    });
    assert_eq!(
        harness.wait(reply, WAIT).unwrap(),
        CreatedWorkspace {
            workspace_id: "wks_3".into(),
            agent_id: Some("agent-3".into()),
            setup_terminal_id: None,
        }
    );
    let request = server.requests_of("workspace.create.request").remove(0);
    assert_eq!(request["subscribe"], true);
}

#[test]
fn create_agent_with_sends_the_full_config() {
    let server = FakeServer::start();
    server.add_agent("a1", "idle");
    server.reply(
        "create_agent_request",
        "status",
        json!({"status": "agent_created", "agentId": "new-1"}),
    );
    let mut harness = connect(&server);
    let reply = harness.core.create_agent_with(initial_agent());
    assert_eq!(harness.wait(reply, WAIT).unwrap(), "new-1");
    let request = server.requests_of("create_agent_request").remove(0);
    assert_eq!(request["config"]["featureValues"], json!({"fast": true}));
    assert_eq!(request["config"]["modeId"], "plan");
    assert_eq!(request["env"], json!({"PASEO_DRAFT": "1"}));
    assert_eq!(request["labels"], json!({"team": "ui"}));
    assert_eq!(request["initialPrompt"], "build it");
    assert_eq!(request["attachments"][0]["type"], "text");
}

#[test]
fn branch_helpers_and_add_project() {
    let server = FakeServer::start();
    server.add_agent("a1", "idle");
    server.on("branch_suggestions_request", |request| {
        let payload = if request["query"] == "detail" {
            json!({"requestId": request["requestId"], "error": null, "branches": ["main"],
                   "branchDetails": [{"name": "main", "committerDate": 1700.0, "hasLocal": true,
                                      "hasRemote": false, "localAhead": 2, "localBehind": 0}]})
        } else {
            json!({"requestId": request["requestId"], "error": null,
                   "branches": ["main", "feat/x"]})
        };
        vec![session("branch_suggestions_response", payload)]
    });
    server.on("validate_branch_request", |request| {
        let payload = if request["branchName"] == "main" {
            json!({"requestId": request["requestId"], "exists": true,
                   "resolvedRef": "refs/heads/main", "isRemote": false, "error": null})
        } else {
            json!({"requestId": request["requestId"], "exists": false, "resolvedRef": null,
                   "isRemote": false, "error": "Not a git repository"})
        };
        vec![session("validate_branch_response", payload)]
    });
    server.on("project.add.request", |request| {
        let project = |id: &str, key: Option<&str>| {
            let mut project = json!({"projectId": id, "projectDisplayName": "Repo",
                "projectRootPath": request["cwd"], "projectKind": "git"});
            if let Some(key) = key {
                project["projectKey"] = json!(key);
            }
            project
        };
        let payload = match request["cwd"].as_str() {
            Some("/keyed") => json!({"requestId": request["requestId"], "error": null,
                                     "project": project("p8", Some("remote:github.com/a/b"))}),
            Some("/plain") => json!({"requestId": request["requestId"], "error": null,
                                     "project": project("p9", None)}),
            _ => json!({"requestId": request["requestId"], "project": null,
                        "error": "Directory not found", "errorCode": "directory_not_found"}),
        };
        vec![session("project.add.response", payload)]
    });
    let mut harness = connect(&server);

    let reply = harness.core.branch_suggestions("/repo", None, Some(20));
    let plain = harness.wait(reply, WAIT).unwrap();
    assert_eq!(
        plain,
        vec![
            BranchSuggestion {
                name: "main".into(),
                committer_date: None,
                has_local: None,
                has_remote: None,
                local_ahead: None,
                local_behind: None,
            },
            BranchSuggestion {
                name: "feat/x".into(),
                committer_date: None,
                has_local: None,
                has_remote: None,
                local_ahead: None,
                local_behind: None,
            },
        ]
    );
    let request = server.requests_of("branch_suggestions_request").remove(0);
    assert_eq!(request["limit"], 20);
    assert!(request.get("query").is_none());
    let reply = harness
        .core
        .branch_suggestions("/repo", Some("detail"), None);
    assert_eq!(
        harness.wait(reply, WAIT).unwrap(),
        vec![BranchSuggestion {
            name: "main".into(),
            committer_date: Some(1700.0),
            has_local: Some(true),
            has_remote: Some(false),
            local_ahead: Some(2),
            local_behind: Some(0),
        }]
    );

    let reply = harness.core.validate_branch("/repo", "main");
    assert_eq!(
        harness.wait(reply, WAIT).unwrap(),
        BranchValidation {
            exists: true,
            resolved_ref: Some("refs/heads/main".into()),
            is_remote: false,
        }
    );
    let reply = harness.core.validate_branch("/tmp", "x");
    let error = harness.wait(reply, WAIT).expect_err("validation error");
    assert_eq!(error.to_string(), "Not a git repository");

    let reply = harness.core.add_project("/keyed");
    assert_eq!(
        harness.wait(reply, WAIT).unwrap(),
        AddedProject {
            id: "p8".into(),
            name: "Repo".into(),
            root: "/keyed".into(),
            view_key: "remote:github.com/a/b".into(),
        }
    );
    let reply = harness.core.add_project("/plain");
    assert_eq!(
        harness.wait(reply, WAIT).unwrap().view_key,
        json!(["fake", "p9"]).to_string()
    );
    let reply = harness.core.add_project("/missing");
    let error = harness.wait(reply, WAIT).expect_err("missing folder");
    assert_eq!(error.to_string(), "Directory not found");
}

#[test]
fn refresh_rename_and_diff_compare() {
    let server = FakeServer::start();
    server.add_agent("a1", "idle");
    server.on("refresh_agent_request", |request| {
        if request["agentId"] == "a1" {
            vec![session(
                "status",
                json!({"status": "agent_refreshed", "agentId": "a1",
                       "requestId": request["requestId"], "timelineSize": 3}),
            )]
        } else {
            vec![session(
                "rpc_error",
                json!({"requestId": request["requestId"], "requestType": request["type"],
                       "error": "Agent not found", "code": "agent_refresh_failed"}),
            )]
        }
    });
    server.on("terminal.rename.request", |request| {
        let ok = request["terminalId"] == "term-1";
        vec![session(
            "terminal.rename.response",
            json!({"requestId": request["requestId"], "success": ok,
                   "error": if ok { Value::Null } else { json!("Terminal not found") }}),
        )]
    });
    server.reply(
        "checkout.diff.get.request",
        "checkout.diff.get.response",
        json!({"cwd": "/repo", "files": [], "error": null}),
    );
    let mut harness = connect(&server);

    let reply = harness.core.refresh_agent("a1");
    harness.wait(reply, WAIT).expect("refreshed");
    let reply = harness.core.refresh_agent("gone");
    let error = harness.wait(reply, WAIT).expect_err("rpc_error");
    assert_eq!(error.to_string(), "Agent not found");

    let reply = harness.core.rename_terminal("term-1", "Build");
    harness.wait(reply, WAIT).expect("renamed");
    let request = server.requests_of("terminal.rename.request").remove(0);
    assert_eq!(request["title"], "Build");
    let reply = harness.core.rename_terminal("term-2", "Build");
    let error = harness.wait(reply, WAIT).expect_err("success false");
    assert_eq!(error.to_string(), "Terminal not found");

    let reply = harness.core.checkout_diff_with(
        "/repo",
        DiffCompare {
            mode: DiffMode::Base,
            base_ref: None,
            ignore_whitespace: true,
        },
    );
    assert!(harness.wait(reply, WAIT).unwrap().is_empty());
    let reply = harness.core.checkout_diff_with(
        "/repo",
        DiffCompare {
            mode: DiffMode::Uncommitted,
            base_ref: Some("main".into()),
            ignore_whitespace: false,
        },
    );
    harness.wait(reply, WAIT).unwrap();
    let requests = server.requests_of("checkout.diff.get.request");
    assert_eq!(
        requests[0]["compare"],
        json!({"mode": "base", "ignoreWhitespace": true})
    );
    assert_eq!(
        requests[1]["compare"],
        json!({"mode": "uncommitted", "baseRef": "main"})
    );
}

#[test]
fn resume_command_follows_the_provider_templates() {
    assert_eq!(
        resume_command("claude", "abc").as_deref(),
        Some("claude --resume abc")
    );
    assert_eq!(
        resume_command("codex", "abc").as_deref(),
        Some("codex resume abc")
    );
    assert_eq!(
        resume_command("opencode", "s1").as_deref(),
        Some("opencode --session s1")
    );
    assert_eq!(resume_command("mock", "abc"), None);
}

#[test]
fn tasks_and_turn_metrics_stay_fast_on_a_20000_entry_timeline() {
    let server = FakeServer::start();
    server.add_agent("a1", "idle");
    let total = 20_000u64;
    let entries: Vec<Value> = (1..=total)
        .map(|seq| {
            if seq == total - 10 {
                user(seq, "t2")
            } else {
                assistant(seq, if seq < total - 10 { "t1" } else { "t2" })
            }
        })
        .collect();
    timeline_reply(&server, entries, vec![]);
    let mut harness = connect(&server);
    harness.core.view_agent("a1");
    assert!(
        harness.pump_until(Duration::from_secs(30), |h| h
            .core
            .timeline("a1")
            .is_some_and(|t| t.entries.len() == total as usize)),
        "the 20,000-entry page loads"
    );
    let started = Instant::now();
    for _ in 0..1000 {
        assert!(harness.core.agent_tasks("a1").is_empty());
        assert!(harness.core.agent_last_turn_metrics("a1").is_none());
    }
    let elapsed = started.elapsed();
    eprintln!("1,000 calls each of agent_tasks and agent_last_turn_metrics: {elapsed:?}");
    assert!(
        elapsed < Duration::from_millis(50),
        "1,000 calls each took {elapsed:?}"
    );
}
