//! The parity requests (TASK-146) against the local fake daemon: request shapes and parsed
//! answers.

mod common;

use bachuc_client::{
    AgentFeature, AgentFeatureKind, AgentFeatureOption, AttentionReason, ChangedFile,
    ChangedFileDetails, CheckoutDetails, CheckoutStatus, ClientEvent, CommitFile, CommitList,
    CommitSummary, DiffStat, DirectoryEvent, DirectorySuggestion, FileDiff, FileDiffStatus,
    FileKind, ForkBoundary, ForkCursor, GitAction, GitActionReply, GitStatus, GitStatusEvent,
    HistoryEntry, HistoryPage, MatchMode, MergeMethod, NewEntryKind, PrCheck, PrMergeCapability,
    PrStatus, PrTimelineItem, ProjectNode, QueuedPrompt, RewindMode, RewindModes, StashEntry,
    TurnBehavior, WorkspaceNode, WorkspaceScript, parse_provider_subagent_key,
    provider_subagent_key,
};
use bachuc_protocol::generated::ServerInfoPermissionsItem;
use common::{FakeServer, Harness, agent_snapshot, session};
use serde_json::{Value, json};
use std::time::Duration;
use tungstenite::Message;

const WAIT: Duration = Duration::from_secs(5);

fn connect(server: &FakeServer) -> Harness {
    let mut harness = Harness::connect(&server.url, None);
    assert!(harness.connected(), "connects");
    harness
}

/// Pumps until the server logged one `kind` request, and returns it.
fn sent(server: &FakeServer, harness: &mut Harness, kind: &str) -> Value {
    assert!(
        harness.pump_until(WAIT, |_| !server.requests_of(kind).is_empty()),
        "{kind} is sent"
    );
    server.requests_of(kind).pop().unwrap()
}

fn workspace(id: &str, project: &str, branch: Option<&str>, pinned_at: Value) -> Value {
    json!({
        "id": id, "projectId": project, "projectDisplayName": format!("Project {project}"),
        "projectRootPath": format!("/repo/{project}"), "workspaceDirectory": format!("/repo/{id}"),
        "projectKind": "git", "workspaceKind": "worktree", "name": format!("ws {id}"),
        "pinnedAt": pinned_at, "status": "done", "activityAt": null,
        "gitRuntime": {"currentBranch": branch}, "githubRuntime": null,
    })
}

fn project(id: &str) -> Value {
    json!({"projectId": id, "projectDisplayName": format!("Project {id}"),
           "projectRootPath": format!("/repo/{id}"), "projectKind": "git"})
}

fn page_info() -> Value {
    json!({"nextCursor": null, "prevCursor": null, "hasMore": false})
}

#[test]
fn server_info_is_typed_and_gates_the_queue() {
    let server = FakeServer::start();
    server.add_agent("a1", "idle");
    let harness = connect(&server);
    let info = harness.core.typed_server_info().expect("typed server_info");
    assert_eq!(info.server_id, "fake");
    // The unknown permission is skipped; the known ones stay.
    assert_eq!(
        info.permissions,
        [
            ServerInfoPermissionsItem::DaemonRead,
            ServerInfoPermissionsItem::WorkspaceWrite
        ]
    );
    assert!(harness.core.supports_queue());
}

#[test]
fn project_tree_joins_projects_workspaces_and_agents() {
    let server = FakeServer::start();
    server.add_agent("a1", "idle");
    server.with(|s| {
        let mut second = agent_snapshot("a2", "running");
        second["workspaceId"] = json!("w1");
        let mut first = agent_snapshot("a1", "idle");
        first["workspaceId"] = json!("w1");
        s.agents = vec![first, second];
    });
    server.reply(
        "project.list.request",
        "project.list.response",
        json!({"projects": [project("p1"), project("p2")]}),
    );
    server.reply(
        "fetch_workspaces_request",
        "fetch_workspaces_response",
        json!({"subscriptionId": "sub-w", "entries": [
            workspace("w1", "p1", Some("main"), json!("2026-10-16T00:00:00.000Z")),
            workspace("w2", "p1", None, Value::Null),
        ], "emptyProjects": [project("p3")], "pageInfo": page_info()}),
    );
    let mut harness = connect(&server);
    let loaded = harness.pump_until(WAIT, |h| h.core.projects().len() == 3);
    assert!(
        loaded,
        "tree {:?} from {:?}",
        harness.core.projects(),
        server.messages()
    );
    let request = sent(&server, &mut harness, "project.list.request");
    assert!(request["requestId"].is_string());
    let subscription = sent(
        &server,
        &mut harness,
        "session.events.set_subscription.request",
    );
    assert!(
        subscription["events"]
            .as_array()
            .unwrap()
            .contains(&json!("project.update"))
    );

    let expected = vec![
        ProjectNode {
            id: "p1".into(),
            name: "Project p1".into(),
            root: "/repo/p1".into(),
            workspaces: vec![
                WorkspaceNode {
                    id: "w1".into(),
                    name: "ws w1".into(),
                    cwd: "/repo/w1".into(),
                    branch: Some("main".into()),
                    pinned: true,
                    agent_ids: vec!["a1".into(), "a2".into()],
                    ..Default::default()
                },
                WorkspaceNode {
                    id: "w2".into(),
                    name: "ws w2".into(),
                    cwd: "/repo/w2".into(),
                    branch: None,
                    pinned: false,
                    agent_ids: vec![],
                    ..Default::default()
                },
            ],
        },
        ProjectNode {
            id: "p2".into(),
            name: "Project p2".into(),
            root: "/repo/p2".into(),
            workspaces: vec![],
        },
        ProjectNode {
            id: "p3".into(),
            name: "Project p3".into(),
            root: "/repo/p3".into(),
            workspaces: vec![],
        },
    ];
    assert_eq!(harness.core.projects(), expected.as_slice());
    assert!(
        harness
            .directory_events
            .contains(&DirectoryEvent::ProjectsChanged)
    );

    // A pin update and a removed project change the tree.
    harness.directory_events.clear();
    let mut pinned = workspace("w2", "p1", Some("feat"), json!("2026-10-16T01:00:00.000Z"));
    pinned["name"] = json!("renamed");
    server.broadcast(
        json!({"type": "workspace_update", "payload": {"kind": "upsert", "workspace": pinned}}),
    );
    server.broadcast(
        json!({"type": "project.update", "payload": {"kind": "remove", "projectId": "p2"}}),
    );
    assert!(harness.pump_until(WAIT, |h| h.core.projects().len() == 2
        && h.core.projects()[0].workspaces[1].pinned));
    let w2 = &harness.core.projects()[0].workspaces[1];
    assert_eq!(w2.name, "renamed");
    assert_eq!(w2.branch.as_deref(), Some("feat"));
    assert_eq!(harness.core.projects()[1].id, "p3");
    assert!(
        harness
            .directory_events
            .contains(&DirectoryEvent::ProjectsChanged)
    );

    // An agent status change alone leaves the tree as it is and emits nothing.
    harness.directory_events.clear();
    let mut running = agent_snapshot("a1", "running");
    running["workspaceId"] = json!("w1");
    server.broadcast(
        json!({"type": "agent_update", "payload": {"kind": "upsert", "agent": running}}),
    );
    harness.pump_until(Duration::from_millis(300), |_| false);
    assert!(harness.directory_events.is_empty());
}

#[test]
fn an_older_agent_snapshot_does_not_replace_a_newer_one() {
    let server = FakeServer::start();
    server.add_agent("a1", "idle");
    let mut harness = connect(&server);
    let snapshot = |status: &str, updated_at: &str| {
        let mut agent = agent_snapshot("a1", status);
        agent["updatedAt"] = json!(updated_at);
        json!({"type": "agent_update", "payload": {"kind": "upsert", "agent": agent}})
    };
    // As live sends them at agent start: running, then the idle snapshot from before it.
    server.broadcast(snapshot("idle", "2026-10-16T03:48:44.759Z"));
    server.broadcast(snapshot("running", "2026-10-16T03:48:45.016Z"));
    server.broadcast(snapshot("idle", "2026-10-16T03:48:44.759Z"));
    server.broadcast(snapshot("running", "2026-10-16T03:48:45.016Z"));
    let running = |h: &mut Harness| h.core.agents()[0].status == "running";
    assert!(harness.pump_until(WAIT, running));
    harness.pump_until(Duration::from_millis(300), |_| false);
    assert_eq!(harness.core.agents()[0].status, "running");
    server.broadcast(snapshot("idle", "2026-10-16T03:48:58.363Z"));
    assert!(harness.pump_until(WAIT, |h| h.core.agents()[0].status == "idle"));
}

#[test]
fn workspace_and_agent_actions_send_their_requests() {
    let server = FakeServer::start();
    server.add_agent("a1", "idle");
    let mut harness = connect(&server);
    harness.core.rename_workspace("w1", "New title");
    harness.core.pin_workspace("w1", true);
    harness.core.archive_workspace("w1");
    harness.core.mark_workspace_unread("w1");
    harness.core.archive_agent("a1");
    harness.core.rename_agent("a1", "Agent title");

    let title = sent(&server, &mut harness, "workspace.title.set.request");
    assert_eq!(
        (title["workspaceId"].as_str(), title["title"].as_str()),
        (Some("w1"), Some("New title"))
    );
    let pin = sent(&server, &mut harness, "workspace.pin.set.request");
    assert_eq!(pin["pinned"], true);
    assert_eq!(
        sent(&server, &mut harness, "archive_workspace_request")["workspaceId"],
        "w1"
    );
    assert_eq!(
        sent(&server, &mut harness, "workspace.mark_unread.request")["workspaceId"],
        "w1"
    );
    assert_eq!(
        sent(&server, &mut harness, "archive_agent_request")["agentId"],
        "a1"
    );
    let rename = sent(&server, &mut harness, "update_agent_request");
    assert_eq!(
        (rename["agentId"].as_str(), rename["name"].as_str()),
        (Some("a1"), Some("Agent title"))
    );
}

#[test]
fn thinking_options_come_from_the_agent_model() {
    let server = FakeServer::start();
    server.with(|s| {
        let mut agent = agent_snapshot("a1", "idle");
        agent["thinkingOptionId"] = json!("high");
        let mut plain = agent_snapshot("a2", "idle");
        plain["model"] = json!("m2");
        s.agents = vec![agent, plain];
    });
    server.reply(
        "list_provider_models_request",
        "list_provider_models_response",
        json!({"fetchedAt": "now", "provider": "mock", "models": [
            {"provider": "mock", "id": "m1", "label": "Model 1", "isDefault": true,
             "thinkingOptions": [{"id": "low", "label": "Low"}, {"id": "high", "label": "High"}],
             "defaultThinkingOptionId": "low"},
            {"provider": "mock", "id": "m2", "label": "Model 2"}
        ]}),
    );
    let mut harness = connect(&server);
    assert!(harness.pump_until(WAIT, |h| h.core.agent_thinking("a1").is_some()));
    let thinking = harness.core.agent_thinking("a1").unwrap();
    assert_eq!(
        thinking.options,
        vec![
            ("low".to_string(), "Low".to_string()),
            ("high".to_string(), "High".to_string())
        ]
    );
    assert_eq!(thinking.current.as_deref(), Some("high"));
    assert_eq!(
        harness.core.agent_thinking("a2"),
        None,
        "a model without options"
    );

    harness.core.set_thinking("a1", "low");
    let request = sent(&server, &mut harness, "set_agent_thinking_request");
    assert_eq!(
        (
            request["agentId"].as_str(),
            request["thinkingOptionId"].as_str()
        ),
        (Some("a1"), Some("low"))
    );
}

#[test]
fn list_commands_parses_the_commands() {
    let server = FakeServer::start();
    server.add_agent("a1", "idle");
    server.reply(
        "list_commands_request",
        "list_commands_response",
        json!({"agentId": "a1", "error": null, "commands": [
            {"name": "review", "description": "Review the diff", "argumentHint": ""},
            {"name": "plan", "description": "Plan", "argumentHint": "<goal>", "kind": "skill"}
        ]}),
    );
    let mut harness = connect(&server);
    let reply = harness.core.list_commands("a1");
    let commands = harness.wait(reply, WAIT).unwrap();
    assert_eq!(commands.len(), 2);
    assert_eq!(commands[0].name, "review");
    assert_eq!(commands[0].description, "Review the diff");
    assert!(!commands[0].skill);
    assert!(commands[1].skill);
    let request = &server.requests_of("list_commands_request")[0];
    assert_eq!(request["agentId"], "a1");
    assert!(request.get("draftConfig").is_none());
}

#[test]
fn list_draft_commands_sends_the_draft_config() {
    let server = FakeServer::start();
    server.add_agent("a1", "idle");
    server.reply(
        "list_commands_request",
        "list_commands_response",
        json!({"agentId": "draft", "error": null, "commands": [
            {"name": "deploy", "description": "Deploy", "argumentHint": "", "kind": "skill"}
        ]}),
    );
    let mut harness = connect(&server);
    let config = bachuc_client::DraftConfig {
        provider: "claude".to_string(),
        cwd: "/repo".to_string(),
        model: Some("opus".to_string()),
        ..Default::default()
    };
    let reply = harness.core.list_draft_commands("draft", &config);
    let commands = harness.wait(reply, WAIT).unwrap();
    assert_eq!(commands.len(), 1);
    assert!(commands[0].skill);
    let request = &server.requests_of("list_commands_request")[0];
    assert_eq!(request["agentId"], "draft");
    assert_eq!(
        request["draftConfig"],
        json!({"provider": "claude", "cwd": "/repo", "model": "opus"})
    );
}

fn file_frame(opcode: u8, request_id: &str, body: &[u8]) -> Message {
    let mut bytes = vec![opcode, request_id.len() as u8];
    bytes.extend_from_slice(request_id.as_bytes());
    bytes.extend_from_slice(body);
    Message::binary(bytes)
}

#[test]
fn files_list_and_read_in_every_form() {
    let server = FakeServer::start();
    server.add_agent("a1", "idle");
    server.on("file_explorer_request", |request| {
        let request_id = request["requestId"].as_str().unwrap().to_string();
        let path = request["path"].as_str().unwrap();
        let response = |file: Value, directory: Value| {
            session("file_explorer_response", json!({"cwd": request["cwd"], "path": path,
                "mode": request["mode"], "directory": directory, "file": file, "error": null,
                "requestId": request_id}))
        };
        match path {
            "." => vec![response(Value::Null, json!({"path": ".", "entries": [
                {"name": "src", "path": "src", "kind": "directory", "size": 0, "modifiedAt": "x"},
                {"name": "a.txt", "path": "a.txt", "kind": "file", "size": 5, "modifiedAt": "x"}
            ]}))],
            "a.txt" => vec![response(json!({"path": "a.txt", "kind": "text", "encoding": "utf-8",
                "content": "hello", "mimeType": "text/plain", "size": 5, "modifiedAt": "x"}), Value::Null)],
            "b.png" => vec![response(json!({"path": "b.png", "kind": "image", "encoding": "base64",
                "content": "iVBORw==", "mimeType": "image/png", "size": 4, "modifiedAt": "x"}), Value::Null)],
            "c.png" => {
                let metadata = br#"{"mime":"image/png","size":6,"encoding":"binary","modifiedAt":"x"}"#;
                let mut begin = (metadata.len() as u16).to_be_bytes().to_vec();
                begin.extend_from_slice(metadata);
                vec![
                    file_frame(0x10, &request_id, &begin),
                    file_frame(0x11, &request_id, &[0x89, b'P', b'N']),
                    file_frame(0x11, &request_id, b"G\r\n"),
                    file_frame(0x12, &request_id, &[]),
                ]
            }
            _ => vec![session("file_explorer_response", json!({"cwd": request["cwd"], "path": path,
                "mode": "file", "directory": null, "file": null, "error": "ENOENT", "requestId": request_id}))],
        }
    });
    let mut harness = connect(&server);

    let reply = harness.core.list_directory("/repo", ".");
    let entries = harness.wait(reply, WAIT).unwrap();
    assert_eq!(entries.len(), 2);
    assert!(entries[0].is_dir && entries[0].name == "src");
    assert_eq!(
        (entries[1].path.as_str(), entries[1].size, entries[1].is_dir),
        ("a.txt", 5, false)
    );
    let list = &server.requests_of("file_explorer_request")[0];
    assert_eq!(
        (list["mode"].as_str(), list["cwd"].as_str()),
        (Some("list"), Some("/repo"))
    );

    let reply = harness.core.read_file("/repo", "a.txt");
    let text = harness.wait(reply, WAIT).unwrap();
    assert_eq!(
        (text.kind, text.text.as_deref(), text.bytes.as_ref()),
        (FileKind::Text, Some("hello"), None)
    );
    let read = &server.requests_of("file_explorer_request")[1];
    assert_eq!(
        (read["mode"].as_str(), read["acceptBinary"].as_bool()),
        (Some("file"), Some(true))
    );

    let reply = harness.core.read_file("/repo", "b.png");
    let base64 = harness.wait(reply, WAIT).unwrap();
    assert_eq!(base64.kind, FileKind::Image);
    assert_eq!(base64.bytes.as_deref(), Some(&[0x89, b'P', b'N', b'G'][..]));

    let reply = harness.core.read_file("/repo", "c.png");
    let streamed = harness.wait(reply, WAIT).unwrap();
    assert_eq!(streamed.kind, FileKind::Image);
    assert_eq!(streamed.path, "c.png");
    assert_eq!(streamed.mime_type.as_deref(), Some("image/png"));
    assert_eq!(streamed.size, 6);
    assert_eq!(streamed.bytes.as_deref(), Some(&b"\x89PNG\r\n"[..]));

    let reply = harness.core.read_file("/repo", "missing");
    assert_eq!(harness.wait(reply, WAIT).unwrap_err().to_string(), "ENOENT");
}

fn diff_files() -> Value {
    json!([
        {"path": "src/a.rs", "isNew": false, "isDeleted": false, "additions": 2, "deletions": 1,
         "status": "ok", "hunks": [{"oldStart": 1, "oldCount": 2, "newStart": 1, "newCount": 3, "lines": [
            {"type": "header", "content": "@@ -1,2 +1,3 @@"},
            {"type": "context", "content": "fn a() {}"},
            {"type": "remove", "content": "let x = 1;"},
            {"type": "add", "content": "let x = 2;"},
            {"type": "add", "content": "let y = 3;"}]}]},
        {"path": "big.json", "isNew": true, "isDeleted": false, "additions": 90000, "deletions": 0,
         "status": "too_large", "hunks": []}
    ])
}

fn git_server() -> FakeServer {
    let server = FakeServer::start();
    server.add_agent("a1", "idle");
    server.on("checkout_status_request", |request| {
        let cwd = request["cwd"].as_str().unwrap();
        let common = json!({"cwd": cwd, "error": null, "requestId": request["requestId"],
            "isPaseoOwnedWorktree": false, "hasRemote": false});
        let mut payload = common;
        if cwd == "/plain" {
            for key in [
                "repoRoot",
                "currentBranch",
                "isDirty",
                "baseRef",
                "aheadBehind",
                "aheadOfOrigin",
                "behindOfOrigin",
                "remoteUrl",
            ] {
                payload[key] = Value::Null;
            }
            payload["isGit"] = json!(false);
        } else {
            payload["isGit"] = json!(true);
            payload["repoRoot"] = json!(cwd);
            payload["currentBranch"] = json!("main");
            payload["isDirty"] = json!(true);
            payload["baseRef"] = Value::Null;
            payload["aheadBehind"] = Value::Null;
            payload["aheadOfOrigin"] = Value::Null;
            payload["behindOfOrigin"] = Value::Null;
            payload["remoteUrl"] = Value::Null;
        }
        vec![session("checkout_status_response", payload)]
    });
    server.reply(
        "checkout.diff.get.request",
        "checkout.diff.get.response",
        json!({"cwd": "/repo", "files": diff_files(), "error": null}),
    );
    server.reply(
        "subscribe_checkout_diff_request",
        "subscribe_checkout_diff_response",
        json!({"subscriptionId": "diff-1", "cwd": "/repo", "files": diff_files(), "error": null}),
    );
    server
}

#[test]
fn checkout_status_lists_changed_files_with_counts_and_status() {
    let server = git_server();
    let mut harness = connect(&server);
    let reply = harness.core.checkout_status("/repo");
    let status = harness.wait(reply, WAIT).unwrap();
    assert_eq!(
        status,
        CheckoutStatus {
            branch: Some("main".into()),
            files: vec![
                ChangedFile {
                    path: "src/a.rs".into(),
                    status: "ok".into(),
                    added: 2,
                    removed: 1
                },
                ChangedFile {
                    path: "big.json".into(),
                    status: "too_large".into(),
                    added: 90000,
                    removed: 0
                },
            ],
        }
    );
    let diff = &server.requests_of("checkout.diff.get.request")[0];
    assert_eq!(
        (diff["cwd"].as_str(), diff["compare"]["mode"].as_str()),
        (Some("/repo"), Some("uncommitted"))
    );
}

#[test]
fn checkout_status_of_a_folder_outside_git_is_empty() {
    let server = git_server();
    let mut harness = connect(&server);
    let reply = harness.core.checkout_status("/plain");
    let status = harness.wait(reply, WAIT).unwrap();
    assert_eq!(
        status,
        CheckoutStatus {
            branch: None,
            files: vec![]
        }
    );
    assert!(server.requests_of("checkout.diff.get.request").is_empty());
}

#[test]
fn file_diff_renders_one_file_and_refuses_a_too_large_one() {
    let server = git_server();
    let mut harness = connect(&server);
    let reply = harness.core.file_diff("/repo", "src/a.rs");
    assert_eq!(
        harness.wait(reply, WAIT).unwrap(),
        "--- a/src/a.rs\n+++ b/src/a.rs\n@@ -1,2 +1,3 @@\n fn a() {}\n-let x = 1;\n+let x = 2;\n+let y = 3;\n"
    );
    let reply = harness.core.file_diff("/repo", "big.json");
    assert!(
        harness
            .wait(reply, WAIT)
            .unwrap_err()
            .to_string()
            .contains("too_large")
    );
    let reply = harness.core.file_diff("/repo", "unchanged.rs");
    assert_eq!(harness.wait(reply, WAIT).unwrap(), "");
}

#[test]
fn watched_checkout_emits_changes_until_unwatched() {
    let server = git_server();
    let mut harness = connect(&server);
    harness.core.watch_checkout("/repo");
    let subscribe = sent(&server, &mut harness, "subscribe_checkout_diff_request");
    assert_eq!(subscribe["compare"]["mode"], "uncommitted");
    let changed = DirectoryEvent::CheckoutChanged {
        cwd: "/repo".into(),
    };
    assert!(harness.pump_until(WAIT, |h| h.directory_events.contains(&changed)));

    harness.directory_events.clear();
    server.broadcast(json!({"type": "checkout_diff_update", "payload": {
        "subscriptionId": "diff-1", "cwd": "/repo", "files": [], "error": null}}));
    assert!(harness.pump_until(WAIT, |h| h.directory_events.contains(&changed)));

    harness.core.unwatch_checkout("/repo");
    let unsubscribe = sent(&server, &mut harness, "unsubscribe_checkout_diff_request");
    assert_eq!(unsubscribe["subscriptionId"], "diff-1");
    harness.directory_events.clear();
    server.broadcast(json!({"type": "checkout_diff_update", "payload": {
        "subscriptionId": "diff-1", "cwd": "/repo", "files": [], "error": null}}));
    harness.pump_until(Duration::from_millis(300), |_| false);
    assert!(harness.directory_events.is_empty());
}

#[test]
fn steer_and_queue_requests() {
    let server = FakeServer::start();
    server.with(|s| {
        let mut agent = agent_snapshot("a1", "running");
        agent["queue"] = json!({"held": false, "items": [
            {"id": "q0", "text": "earlier", "attachmentKinds": [], "createdAt": "x"}]});
        s.agents = vec![agent];
    });
    server.on("agent.queue.enqueue.request", |request| {
        vec![session(
            "agent.queue.enqueue.response",
            json!({
            "requestId": request["requestId"], "agentId": request["agentId"], "itemId": "q1",
            "error": null, "queue": {"held": false, "items": [
                {"id": "q0", "text": "earlier", "attachmentKinds": [], "createdAt": "x"},
                {"id": "q1", "text": request["text"], "attachmentKinds": [], "createdAt": "y"}]}}),
        )]
    });
    server.on("agent.queue.cancel.request", |request| {
        vec![session(
            "agent.queue.cancel.response",
            json!({
            "requestId": request["requestId"], "agentId": request["agentId"], "error": null,
            "item": null, "queue": {"held": false, "items": [
                {"id": "q0", "text": "earlier", "attachmentKinds": [], "createdAt": "x"}]}}),
        )]
    });
    let mut harness = connect(&server);
    let queued = |id: &str, text: &str| QueuedPrompt {
        id: id.into(),
        text: text.into(),
    };
    assert_eq!(
        harness.core.queued_prompts("a1"),
        vec![queued("q0", "earlier")]
    );

    harness
        .core
        .send_prompt_with("a1", "steer me".into(), vec![], TurnBehavior::Steer);
    harness
        .core
        .send_prompt_with("a1", "stop".into(), vec![], TurnBehavior::Interrupt);
    assert!(harness.pump_until(
        WAIT,
        |_| server.requests_of("send_agent_message_request").len() == 2
    ));
    let sends = server.requests_of("send_agent_message_request");
    assert_eq!(
        (
            sends[0]["text"].as_str(),
            sends[0]["activeTurnBehavior"].as_str()
        ),
        (Some("steer me"), Some("steer"))
    );
    assert_eq!(sends[1]["activeTurnBehavior"], "interrupt");
    harness.core.send_prompt("a1", "plain".into(), vec![]);
    assert!(harness.pump_until(
        WAIT,
        |_| server.requests_of("send_agent_message_request").len() == 3
    ));
    assert!(
        server.requests_of("send_agent_message_request")[2]
            .get("activeTurnBehavior")
            .is_none()
    );

    let image = json!({"data": "aGk=", "mimeType": "image/png"});
    harness
        .core
        .enqueue_prompt("a1", "later".into(), vec![image.clone()]);
    let enqueue = sent(&server, &mut harness, "agent.queue.enqueue.request");
    assert_eq!(
        (enqueue["agentId"].as_str(), enqueue["text"].as_str()),
        (Some("a1"), Some("later"))
    );
    assert_eq!(enqueue["images"], json!([image]));
    assert!(harness.pump_until(WAIT, |h| h.core.queued_prompts("a1").len() == 2));
    assert_eq!(harness.core.queued_prompts("a1")[1], queued("q1", "later"));

    harness.core.cancel_queued("a1", "q1");
    let cancel = sent(&server, &mut harness, "agent.queue.cancel.request");
    assert_eq!(cancel["itemId"], "q1");
    assert!(harness.pump_until(WAIT, |h| h.core.queued_prompts("a1").len() == 1));
}

#[test]
fn permission_response_carries_the_full_payload() {
    let server = FakeServer::start();
    server.add_agent("a1", "idle");
    let mut harness = connect(&server);
    let response = json!({"behavior": "allow", "selectedActionId": "implement",
        "updatedInput": {"answers": {"Which?": "B"}}});
    harness
        .core
        .respond_permission_with("a1", "perm-1", response.clone());
    let sent = sent(&server, &mut harness, "agent_permission_response");
    assert_eq!(
        (sent["agentId"].as_str(), sent["requestId"].as_str()),
        (Some("a1"), Some("perm-1"))
    );
    assert_eq!(sent["response"], response);
}

#[test]
fn project_config_reads_and_writes_with_the_revision() {
    let server = FakeServer::start();
    server.add_agent("a1", "idle");
    server.on("read_project_config_request", |request| {
        vec![session(
            "read_project_config_response",
            json!({
            "requestId": request["requestId"], "repoRoot": request["repoRoot"], "ok": true,
            "config": {"worktree": {"setup": ["npm ci"]}}, "revision": {"mtimeMs": 1, "size": 10}}),
        )]
    });
    server.on("write_project_config_request", |request| {
        // Numbers pass the generated types as f64, so compare them as f64.
        let payload = if request["expectedRevision"]["mtimeMs"].as_f64() == Some(1.0) {
            json!({"requestId": request["requestId"], "repoRoot": request["repoRoot"], "ok": true,
                   "config": request["config"], "revision": {"mtimeMs": 2, "size": 12}})
        } else {
            json!({"requestId": request["requestId"], "repoRoot": request["repoRoot"], "ok": false,
                   "error": {"code": "stale_project_config", "currentRevision": null}})
        };
        vec![session("write_project_config_response", payload)]
    });
    let mut harness = connect(&server);

    // Without a read the write sends no revision, and the daemon refuses it as stale.
    let config = json!({"worktree": {"setup": ["npm ci", "npm run build"]}});
    let reply = harness.core.write_project_config("/repo", config.clone());
    let error = harness.wait(reply, WAIT).unwrap_err();
    assert_eq!(error.to_string(), "stale_project_config");
    assert!(server.requests_of("write_project_config_request")[0]["expectedRevision"].is_null());

    let reply = harness.core.read_project_config("/repo");
    assert_eq!(
        harness.wait(reply, WAIT).unwrap(),
        json!({"worktree": {"setup": ["npm ci"]}})
    );
    assert_eq!(
        server.requests_of("read_project_config_request")[0]["repoRoot"],
        "/repo"
    );
    let reply = harness.core.write_project_config("/repo", config.clone());
    harness.wait(reply, WAIT).unwrap();
    let write = &server.requests_of("write_project_config_request")[1];
    assert_eq!(write["config"], config);
    assert_eq!(write["expectedRevision"]["mtimeMs"].as_f64(), Some(1.0));
    assert_eq!(write["expectedRevision"]["size"].as_f64(), Some(10.0));
}

// ---- follow-up fields and requests (TASK-157) ----

#[test]
fn agent_attention_reads_the_flag_and_the_reason() {
    let server = FakeServer::start();
    server.with(|s| {
        let mut flagged = agent_snapshot("a1", "idle");
        flagged["requiresAttention"] = json!(true);
        flagged["attentionReason"] = json!("permission");
        let mut cleared = agent_snapshot("a2", "idle");
        cleared["requiresAttention"] = json!(false);
        cleared["attentionReason"] = Value::Null;
        s.agents = vec![flagged, cleared, agent_snapshot("a3", "idle")];
    });
    let mut harness = connect(&server);
    assert!(harness.pump_until(WAIT, |h| h.core.agents().len() == 3));
    assert_eq!(
        harness.core.agent_attention("a1"),
        Some((true, Some(AttentionReason::Permission)))
    );
    assert_eq!(harness.core.agent_attention("a2"), Some((false, None)));
    assert_eq!(
        harness.core.agent_attention("a3"),
        Some((false, None)),
        "absent fields"
    );
    assert_eq!(harness.core.agent_attention("missing"), None);

    let mut finished = agent_snapshot("a3", "idle");
    finished["updatedAt"] = json!("2026-10-16T05:00:00.000Z");
    finished["requiresAttention"] = json!(true);
    finished["attentionReason"] = json!("finished");
    server.broadcast(
        json!({"type": "agent_update", "payload": {"kind": "upsert", "agent": finished}}),
    );
    assert!(harness.pump_until(WAIT, |h| h.core.agent_attention("a3")
        == Some((true, Some(AttentionReason::Finished)))));
}

#[test]
fn project_view_key_uses_the_project_key_or_the_placement_key() {
    let server = FakeServer::start();
    server.add_agent("a1", "idle");
    let mut keyed = project("p1");
    keyed["projectKey"] = json!("github.com/acme/app");
    server.reply(
        "project.list.request",
        "project.list.response",
        json!({"projects": [keyed, project("p2")]}),
    );
    let mut harness = connect(&server);
    assert!(harness.pump_until(WAIT, |h| h.core.projects().len() == 2));
    assert_eq!(
        harness.core.project_view_key("p1").as_deref(),
        Some("github.com/acme/app")
    );
    assert_eq!(
        harness.core.project_view_key("p2").as_deref(),
        Some(r#"["fake","p2"]"#),
        "the JSON.stringify form of createProjectViewKey"
    );
    assert_eq!(harness.core.project_view_key("p9"), None);
}

#[test]
fn workspace_diff_stat_and_pinned_at() {
    let server = FakeServer::start();
    server.add_agent("a1", "idle");
    let mut changed = workspace("w1", "p1", Some("main"), json!("2026-10-16T00:00:00.000Z"));
    changed["diffStat"] = json!({"additions": 12, "deletions": 3});
    let mut clean = workspace("w2", "p1", None, Value::Null);
    clean["diffStat"] = Value::Null;
    server.reply(
        "fetch_workspaces_request",
        "fetch_workspaces_response",
        json!({"subscriptionId": "sub-w", "entries": [
            changed, clean, workspace("w3", "p1", None, Value::Null),
        ], "pageInfo": page_info()}),
    );
    let mut harness = connect(&server);
    assert!(harness.pump_until(WAIT, |h| h.core.workspace_diff_stat("w1").is_some()));
    assert_eq!(
        harness.core.workspace_diff_stat("w1"),
        Some(DiffStat {
            additions: 12,
            deletions: 3
        })
    );
    assert_eq!(harness.core.workspace_diff_stat("w2"), None, "null");
    assert_eq!(harness.core.workspace_diff_stat("w3"), None, "absent");
    assert_eq!(
        harness.core.workspace_pinned_at("w1").as_deref(),
        Some("2026-10-16T00:00:00.000Z")
    );
    assert_eq!(harness.core.workspace_pinned_at("w2"), None);
    assert_eq!(harness.core.workspace_pinned_at("missing"), None);
}

#[test]
fn workspace_is_dirty_reads_the_git_runtime_flag() {
    let server = FakeServer::start();
    server.add_agent("a1", "idle");
    let mut dirty = workspace("w1", "p1", Some("main"), Value::Null);
    dirty["gitRuntime"]["isDirty"] = json!(true);
    let mut clean = workspace("w2", "p1", Some("main"), Value::Null);
    clean["gitRuntime"]["isDirty"] = json!(false);
    let mut null = workspace("w3", "p1", None, Value::Null);
    null["gitRuntime"]["isDirty"] = Value::Null;
    server.reply(
        "fetch_workspaces_request",
        "fetch_workspaces_response",
        json!({"subscriptionId": "sub-w", "entries": [
            dirty, clean, null, workspace("w4", "p1", None, Value::Null),
        ], "pageInfo": page_info()}),
    );
    let mut harness = connect(&server);
    assert!(harness.pump_until(WAIT, |h| h.core.workspace_is_dirty("w1").is_some()));
    assert_eq!(harness.core.workspace_is_dirty("w1"), Some(true));
    assert_eq!(harness.core.workspace_is_dirty("w2"), Some(false));
    assert_eq!(harness.core.workspace_is_dirty("w3"), None, "null");
    assert_eq!(harness.core.workspace_is_dirty("w4"), None, "absent");
    assert_eq!(harness.core.workspace_is_dirty("missing"), None, "unknown");

    let mut cleaned = workspace("w1", "p1", Some("main"), Value::Null);
    cleaned["gitRuntime"]["isDirty"] = json!(false);
    server.broadcast(json!({"type": "workspace_update", "payload": {
        "kind": "upsert", "workspace": cleaned,
    }}));
    assert!(harness.pump_until(WAIT, |h| h.core.workspace_is_dirty("w1") == Some(false)));
}

#[test]
fn checkout_details_carry_the_git_flag_and_the_file_fields() {
    let server = git_server();
    server.reply(
        "checkout.diff.get.request",
        "checkout.diff.get.response",
        json!({"cwd": "/repo", "error": null, "files": [
            {"path": "src/new.rs", "oldPath": "src/old.rs", "isNew": false, "isDeleted": false,
             "additions": 1, "deletions": 1, "status": "ok", "hunks": []},
            {"path": "gone.rs", "isNew": false, "isDeleted": true, "additions": 0,
             "deletions": 4, "hunks": []},
            {"path": "added.rs", "isNew": true, "isDeleted": false, "additions": 7,
             "deletions": 0, "status": "ok", "hunks": []},
        ]}),
    );
    let mut harness = connect(&server);
    let file = |path: &str, old_path: Option<&str>, flags: (bool, bool), counts: (u32, u32)| {
        ChangedFileDetails {
            path: path.into(),
            old_path: old_path.map(str::to_string),
            is_new: flags.0,
            is_deleted: flags.1,
            status: "ok".into(),
            additions: counts.0,
            deletions: counts.1,
        }
    };
    let reply = harness.core.checkout_details("/repo");
    assert_eq!(
        harness.wait(reply, WAIT).unwrap(),
        CheckoutDetails {
            is_git: true,
            branch: Some("main".into()),
            files: vec![
                file("src/new.rs", Some("src/old.rs"), (false, false), (1, 1)),
                file("gone.rs", None, (false, true), (0, 4)),
                file("added.rs", None, (true, false), (7, 0)),
            ],
        }
    );

    let reply = harness.core.checkout_details("/plain");
    assert_eq!(
        harness.wait(reply, WAIT).unwrap(),
        CheckoutDetails {
            is_git: false,
            branch: None,
            files: vec![]
        }
    );
    assert_eq!(server.requests_of("checkout.diff.get.request").len(), 1);
}

#[test]
fn agent_features_decode_toggles_and_selects_and_skip_unknown_types() {
    let server = FakeServer::start();
    server.with(|s| {
        let mut agent = agent_snapshot("a1", "idle");
        agent["features"] = json!([
            {"type": "toggle", "id": "fast", "label": "Fast mode", "description": "Faster",
             "tooltip": "Use the fast model", "icon": "zap", "value": true},
            {"type": "slider", "id": "temp", "label": "Temperature", "min": 0, "max": 1},
            {"type": "select", "id": "effort", "label": "Effort", "value": null, "options": [
                {"id": "low", "label": "Low", "isDefault": true},
                {"id": "high", "label": "High", "description": "Slow", "metadata": {"tier": 2}}]},
        ]);
        s.agents = vec![agent, agent_snapshot("a2", "idle")];
    });
    let mut harness = connect(&server);
    assert!(harness.pump_until(WAIT, |h| h.core.agents().len() == 2));
    assert_eq!(
        harness.core.agents()[0].title,
        "agent a1",
        "the snapshot decodes"
    );
    let mut metadata = serde_json::Map::new();
    metadata.insert("tier".into(), json!(2));
    assert_eq!(
        harness.core.agent_features("a1"),
        vec![
            AgentFeature {
                id: "fast".into(),
                label: "Fast mode".into(),
                description: Some("Faster".into()),
                tooltip: Some("Use the fast model".into()),
                icon: Some("zap".into()),
                kind: AgentFeatureKind::Toggle { value: true },
            },
            AgentFeature {
                id: "effort".into(),
                label: "Effort".into(),
                description: None,
                tooltip: None,
                icon: None,
                kind: AgentFeatureKind::Select {
                    value: None,
                    options: vec![
                        AgentFeatureOption {
                            id: "low".into(),
                            label: "Low".into(),
                            description: None,
                            is_default: Some(true),
                            metadata: serde_json::Map::new(),
                        },
                        AgentFeatureOption {
                            id: "high".into(),
                            label: "High".into(),
                            description: Some("Slow".into()),
                            is_default: None,
                            metadata,
                        },
                    ],
                },
            },
        ]
    );
    assert_eq!(harness.core.agent_features("a2"), vec![]);
}

#[test]
fn set_agent_feature_sends_the_value_and_reports_a_refusal() {
    let server = FakeServer::start();
    server.add_agent("a1", "idle");
    server.on("set_agent_feature_request", |request| {
        let known = request["featureId"] == "fast";
        vec![session(
            "set_agent_feature_response",
            json!({"requestId": request["requestId"], "agentId": request["agentId"],
                   "accepted": known, "error": if known { Value::Null } else { json!("unknown feature") }}),
        )]
    });
    let mut harness = connect(&server);
    let reply = harness.core.set_agent_feature("a1", "fast", json!(true));
    harness.wait(reply, WAIT).unwrap();
    let requests = server.requests_of("set_agent_feature_request");
    assert_eq!(requests.len(), 1);
    assert_eq!(
        (
            requests[0]["agentId"].as_str(),
            requests[0]["featureId"].as_str(),
            &requests[0]["value"]
        ),
        (Some("a1"), Some("fast"), &json!(true))
    );

    let reply = harness
        .core
        .set_agent_feature("a1", "effort", json!("high"));
    assert_eq!(
        harness.wait(reply, WAIT).unwrap_err().to_string(),
        "unknown feature"
    );
    assert_eq!(
        server.requests_of("set_agent_feature_request")[1]["value"],
        "high"
    );
}

#[test]
fn checkout_diff_returns_every_file_from_one_request() {
    let server = git_server();
    let mut files = diff_files();
    files.as_array_mut().unwrap().push(json!(
        {"path": "logo.png", "oldPath": "img/logo.png", "isNew": false, "isDeleted": false,
         "additions": 0, "deletions": 0, "status": "binary", "hunks": []}));
    server.reply(
        "checkout.diff.get.request",
        "checkout.diff.get.response",
        json!({"cwd": "/repo", "files": files, "error": null}),
    );
    let mut harness = connect(&server);
    let reply = harness.core.checkout_diff("/repo");
    let diff = harness.wait(reply, WAIT).unwrap();
    assert_eq!(
        diff,
        vec![
            FileDiff {
                path: "src/a.rs".into(),
                old_path: None,
                is_new: false,
                is_deleted: false,
                status: FileDiffStatus::Ok,
                additions: 2,
                deletions: 1,
                text: "--- a/src/a.rs\n+++ b/src/a.rs\n@@ -1,2 +1,3 @@\n fn a() {}\n-let x = 1;\n+let x = 2;\n+let y = 3;\n".into(),
            },
            FileDiff {
                path: "big.json".into(),
                old_path: None,
                is_new: true,
                is_deleted: false,
                status: FileDiffStatus::TooLarge,
                additions: 90000,
                deletions: 0,
                text: String::new(),
            },
            FileDiff {
                path: "logo.png".into(),
                old_path: Some("img/logo.png".into()),
                is_new: false,
                is_deleted: false,
                status: FileDiffStatus::Binary,
                additions: 0,
                deletions: 0,
                text: String::new(),
            },
        ]
    );
    let requests = server.requests_of("checkout.diff.get.request");
    assert_eq!(requests.len(), 1);
    assert_eq!(
        (
            requests[0]["cwd"].as_str(),
            requests[0]["compare"]["mode"].as_str()
        ),
        (Some("/repo"), Some("uncommitted"))
    );
    assert!(server.requests_of("checkout_status_request").is_empty());
}

#[test]
fn commits_list_the_commits_or_fail_with_the_checkout_error() {
    let server = FakeServer::start();
    server.add_agent("a1", "idle");
    server.on("checkout.commits.list.request", |request| {
        let payload = if request["cwd"] == "/repo" {
            json!({"cwd": "/repo", "baseRef": "origin/main", "error": null, "commits": [
                {"sha": "abc123", "shortSha": "abc", "subject": "feat: one", "authorName": "Hiep",
                 "authorDate": "2026-10-16T01:00:00Z", "isOnRemote": false, "isOnBase": false,
                 "files": [{"path": "a.rs", "additions": 3, "deletions": 1, "status": "modified"},
                           {"path": "b.rs", "additions": 5, "deletions": 0}]},
                {"sha": "def456", "shortSha": "def", "subject": "fix: two", "authorName": "Hiep",
                 "authorDate": "2026-10-15T01:00:00Z", "isOnRemote": true, "files": []}]})
        } else {
            json!({"cwd": request["cwd"], "baseRef": null, "commits": [],
                   "error": {"code": "UNKNOWN", "message": "git log failed"}})
        };
        let mut payload = payload;
        payload["requestId"] = request["requestId"].clone();
        vec![session("checkout.commits.list.response", payload)]
    });
    let mut harness = connect(&server);
    let reply = harness.core.commits("/repo");
    assert_eq!(
        harness.wait(reply, WAIT).unwrap(),
        CommitList {
            base_ref: Some("origin/main".into()),
            commits: vec![
                CommitSummary {
                    sha: "abc123".into(),
                    short_sha: "abc".into(),
                    subject: "feat: one".into(),
                    author_name: "Hiep".into(),
                    author_date: "2026-10-16T01:00:00Z".into(),
                    is_on_remote: false,
                    is_on_base: Some(false),
                    files: vec![
                        CommitFile {
                            path: "a.rs".into(),
                            additions: 3,
                            deletions: 1,
                            status: Some("modified".into()),
                        },
                        CommitFile {
                            path: "b.rs".into(),
                            additions: 5,
                            deletions: 0,
                            status: None,
                        },
                    ],
                },
                CommitSummary {
                    sha: "def456".into(),
                    short_sha: "def".into(),
                    subject: "fix: two".into(),
                    author_name: "Hiep".into(),
                    author_date: "2026-10-15T01:00:00Z".into(),
                    is_on_remote: true,
                    is_on_base: None,
                    files: vec![],
                },
            ],
        }
    );
    assert_eq!(
        server.requests_of("checkout.commits.list.request")[0]["cwd"],
        "/repo"
    );

    let reply = harness.core.commits("/broken");
    assert_eq!(
        harness.wait(reply, WAIT).unwrap_err().to_string(),
        "git log failed"
    );
}

#[test]
fn commit_file_diff_renders_a_file_or_gives_none_for_null() {
    let server = FakeServer::start();
    server.add_agent("a1", "idle");
    server.on("checkout.commits.file_diff.request", |request| {
        let file = if request["path"] == "a.rs" {
            json!({"path": "a.rs", "isNew": false, "isDeleted": false, "additions": 1,
                   "deletions": 0, "hunks": [{"oldStart": 1, "oldCount": 1, "newStart": 1,
                   "newCount": 2, "lines": [{"type": "header", "content": "@@ -1 +1,2 @@"},
                   {"type": "context", "content": "a"}, {"type": "add", "content": "b"}]}]})
        } else {
            Value::Null
        };
        vec![session(
            "checkout.commits.file_diff.response",
            json!({"requestId": request["requestId"], "cwd": request["cwd"],
                   "sha": request["sha"], "path": request["path"], "file": file, "error": null}),
        )]
    });
    let mut harness = connect(&server);
    let reply = harness.core.commit_file_diff("/repo", "abc123", "a.rs");
    let diff = harness.wait(reply, WAIT).unwrap().expect("a file");
    assert_eq!(
        (diff.status, diff.additions, diff.text.as_str()),
        (
            FileDiffStatus::Ok,
            1,
            "--- a/a.rs\n+++ b/a.rs\n@@ -1 +1,2 @@\n a\n+b\n"
        )
    );
    let request = &server.requests_of("checkout.commits.file_diff.request")[0];
    assert_eq!(
        (
            request["cwd"].as_str(),
            request["sha"].as_str(),
            request["path"].as_str()
        ),
        (Some("/repo"), Some("abc123"), Some("a.rs"))
    );

    let reply = harness.core.commit_file_diff("/repo", "abc123", "logo.png");
    assert_eq!(harness.wait(reply, WAIT).unwrap(), None);
}

// ---- git, PR, stash, and script calls (TASK-204) ----

/// A fake daemon whose checkout requests succeed; `reply_kinds` maps request to response.
fn checkout_server(reply_kinds: &[(&str, &str)]) -> FakeServer {
    let server = FakeServer::start();
    server.add_agent("a1", "idle");
    for (request, response) in reply_kinds {
        server.reply(
            request,
            response,
            json!({"cwd": "/repo", "success": true, "error": null}),
        );
    }
    server
}

fn checkout_failure(message: &str) -> Value {
    json!({"cwd": "/repo", "success": false, "error": {"code": "UNKNOWN", "message": message}})
}

#[test]
fn git_action_sends_each_request_with_its_fields() {
    let cases = [
        (
            GitAction::Commit,
            "checkout_commit_request",
            "checkout_commit_response",
        ),
        (
            GitAction::Pull,
            "checkout_pull_request",
            "checkout_pull_response",
        ),
        (
            GitAction::Push,
            "checkout_push_request",
            "checkout_push_response",
        ),
        (
            GitAction::MergePr(MergeMethod::Squash),
            "checkout_pr_merge_request",
            "checkout_pr_merge_response",
        ),
        (
            GitAction::MergeBranch,
            "checkout_merge_request",
            "checkout_merge_response",
        ),
        (
            GitAction::MergeFromBase,
            "checkout_merge_from_base_request",
            "checkout_merge_from_base_response",
        ),
        (
            GitAction::Refresh,
            "checkout.refresh.request",
            "checkout.refresh.response",
        ),
    ];
    let kinds: Vec<(&str, &str)> = cases.iter().map(|(_, req, res)| (*req, *res)).collect();
    let server = checkout_server(&kinds);
    let mut harness = connect(&server);
    for (action, request, _) in cases {
        let reply = harness.core.git_action("/repo", action);
        assert_eq!(
            harness.wait(reply, WAIT).unwrap(),
            GitActionReply::default(),
            "{request}"
        );
    }
    let field = |kind: &str, key: &str| server.requests_of(kind)[0][key].clone();
    assert_eq!(field("checkout_commit_request", "addAll"), json!(true));
    assert_eq!(field("checkout_commit_request", "message"), Value::Null);
    assert_eq!(
        field("checkout_pr_merge_request", "mergeMethod"),
        json!("squash")
    );
    assert_eq!(field("checkout_merge_request", "strategy"), json!("merge"));
    assert_eq!(
        field("checkout_merge_request", "requireCleanTarget"),
        json!(true)
    );
    assert_eq!(
        field("checkout_merge_from_base_request", "requireCleanTarget"),
        json!(true)
    );
    for (_, request, _) in cases {
        assert_eq!(field(request, "cwd"), json!("/repo"), "{request}");
    }
}

#[test]
fn git_action_fails_with_the_daemon_message() {
    let server = FakeServer::start();
    server.add_agent("a1", "idle");
    server.reply(
        "checkout_push_request",
        "checkout_push_response",
        checkout_failure("rejected: non-fast-forward"),
    );
    let mut harness = connect(&server);
    let reply = harness.core.git_action("/repo", GitAction::Push);
    assert_eq!(
        harness.wait(reply, WAIT).unwrap_err().to_string(),
        "rejected: non-fast-forward"
    );
}

#[test]
fn pull_and_push_pushes_only_after_the_pull_succeeds() {
    let server = checkout_server(&[
        ("checkout_pull_request", "checkout_pull_response"),
        ("checkout_push_request", "checkout_push_response"),
    ]);
    let mut harness = connect(&server);
    let reply = harness.core.git_action("/repo", GitAction::PullAndPush);
    harness.wait(reply, WAIT).unwrap();
    let order: Vec<Value> = server
        .messages()
        .into_iter()
        .map(|m| m["type"].clone())
        .filter(|kind| kind == "checkout_pull_request" || kind == "checkout_push_request")
        .collect();
    assert_eq!(
        order,
        [
            json!("checkout_pull_request"),
            json!("checkout_push_request")
        ]
    );

    server.reply(
        "checkout_pull_request",
        "checkout_pull_response",
        checkout_failure("merge conflict"),
    );
    let reply = harness.core.git_action("/repo", GitAction::PullAndPush);
    assert_eq!(
        harness.wait(reply, WAIT).unwrap_err().to_string(),
        "merge conflict"
    );
    harness.pump_for(Duration::from_millis(100));
    assert_eq!(
        server.requests_of("checkout_push_request").len(),
        1,
        "no second push"
    );
}

#[test]
fn create_pr_sends_no_title_and_returns_the_url_and_number() {
    let server = FakeServer::start();
    server.add_agent("a1", "idle");
    server.reply(
        "checkout_pr_create_request",
        "checkout_pr_create_response",
        json!({"cwd": "/repo", "url": "https://github.com/o/r/pull/7", "number": 7,
               "error": null}),
    );
    let mut harness = connect(&server);
    let reply = harness.core.git_action("/repo", GitAction::CreatePr);
    assert_eq!(
        harness.wait(reply, WAIT).unwrap(),
        GitActionReply {
            url: Some("https://github.com/o/r/pull/7".into()),
            number: Some(7),
        }
    );
    let request = &server.requests_of("checkout_pr_create_request")[0];
    assert!(request.get("title").is_none() && request.get("body").is_none());
}

#[test]
fn auto_merge_uses_the_forge_request_and_the_github_one_only_without_it() {
    let reply_kinds = [
        (
            "checkout.forge.set_auto_merge.request",
            "checkout.forge.set_auto_merge.response",
        ),
        (
            "checkout.github.set_auto_merge.request",
            "checkout.github.set_auto_merge.response",
        ),
    ];
    let enable = GitAction::SetAutoMerge {
        enabled: true,
        method: Some(MergeMethod::Rebase),
    };
    let disable = GitAction::SetAutoMerge {
        enabled: false,
        method: None,
    };

    let server = checkout_server(&reply_kinds);
    server.with(|s| {
        s.features
            .insert("checkoutForgeSetAutoMerge".into(), json!(true));
        s.features
            .insert("checkoutGithubSetAutoMerge".into(), json!(true));
    });
    let mut harness = connect(&server);
    let reply = harness.core.git_action("/repo", enable);
    harness.wait(reply, WAIT).unwrap();
    let reply = harness.core.git_action("/repo", disable);
    harness.wait(reply, WAIT).unwrap();
    let sent = server.requests_of("checkout.forge.set_auto_merge.request");
    assert_eq!(
        (sent[0]["enabled"].clone(), sent[0]["mergeMethod"].clone()),
        (json!(true), json!("rebase"))
    );
    assert_eq!(sent[1]["enabled"], json!(false));
    assert!(sent[1].get("mergeMethod").is_none());
    assert!(
        server
            .requests_of("checkout.github.set_auto_merge.request")
            .is_empty()
    );

    let legacy = checkout_server(&reply_kinds);
    legacy.with(|s| {
        s.features
            .insert("checkoutGithubSetAutoMerge".into(), json!(true));
    });
    let mut harness = connect(&legacy);
    let reply = harness.core.git_action("/repo", enable);
    harness.wait(reply, WAIT).unwrap();
    assert_eq!(
        legacy
            .requests_of("checkout.github.set_auto_merge.request")
            .len(),
        1
    );

    let old = checkout_server(&reply_kinds);
    let mut harness = connect(&old);
    let reply = harness.core.git_action("/repo", enable);
    assert_eq!(
        harness.wait(reply, WAIT).unwrap_err().to_string(),
        "Update the host to use auto-merge actions."
    );
}

#[test]
fn git_status_of_a_folder_outside_git_is_not_git() {
    let server = git_server();
    let mut harness = connect(&server);
    let reply = harness.core.git_status("/plain");
    assert_eq!(harness.wait(reply, WAIT).unwrap(), GitStatus::default());

    server.reply(
        "checkout_status_request",
        "checkout_status_response",
        json!({"cwd": "/gone", "isGit": false, "isPaseoOwnedWorktree": false, "hasRemote": false,
               "error": {"code": "NOT_GIT_REPO", "message": "not a git repository"}}),
    );
    let reply = harness.core.git_status("/gone");
    assert!(
        !harness.wait(reply, WAIT).unwrap().is_git,
        "NOT_GIT_REPO is no error"
    );
}

#[test]
fn git_status_maps_a_null_ahead_behind_to_zero() {
    let server = git_server();
    let mut harness = connect(&server);
    let reply = harness.core.git_status("/repo");
    let status = harness.wait(reply, WAIT).unwrap();
    assert_eq!(
        status,
        GitStatus {
            is_git: true,
            branch: Some("main".into()),
            is_dirty: true,
            ..GitStatus::default()
        }
    );
    assert_eq!(
        server.requests_of("checkout_status_request")[0]["cwd"],
        json!("/repo")
    );
}

#[test]
fn git_status_reads_counts_remote_and_upstream() {
    let server = FakeServer::start();
    server.add_agent("a1", "idle");
    server.reply(
        "checkout_status_request",
        "checkout_status_response",
        json!({"cwd": "/wt", "isGit": true, "isPaseoOwnedWorktree": true, "repoRoot": "/wt",
               "mainRepoRoot": "/repo", "currentBranch": "feat/x", "isDirty": false,
               "baseRef": "main", "aheadBehind": {"ahead": 3, "behind": 1},
               "aheadOfOrigin": 2, "behindOfOrigin": 0, "hasRemote": true,
               "remoteUrl": "git@github.com:o/r.git", "error": null}),
    );
    let mut harness = connect(&server);
    let reply = harness.core.git_status("/wt");
    assert_eq!(
        harness.wait(reply, WAIT).unwrap(),
        GitStatus {
            is_git: true,
            branch: Some("feat/x".into()),
            is_dirty: false,
            base_ref: Some("main".into()),
            ahead: 3,
            behind: 1,
            ahead_of_origin: 2,
            behind_of_origin: 0,
            has_remote: true,
            remote_url: Some("git@github.com:o/r.git".into()),
            is_paseo_owned_worktree: true,
            has_upstream: true,
        }
    );

    server.reply(
        "checkout_status_request",
        "checkout_status_response",
        checkout_failure("git failed"),
    );
    let reply = harness.core.git_status("/wt");
    assert_eq!(
        harness.wait(reply, WAIT).unwrap_err().to_string(),
        "git failed"
    );
}

fn pr_status_payload() -> Value {
    json!({"cwd": "/repo", "githubFeaturesEnabled": true, "forge": "github", "error": null,
        "status": {"url": "https://github.com/o/r/pull/9", "title": "Add git", "state": "open",
            "baseRefName": "main", "headRefName": "feat/git", "isMerged": false,
            "isDraft": true, "mergeable": "MERGEABLE", "reviewDecision": "APPROVED",
            "repoOwner": "o", "repoName": "r",
            "checks": [
                {"name": "build", "status": "success", "url": "https://ci/1",
                 "workflow": "CI"},
                {"name": "lint", "status": "pending", "url": null}
            ],
            "github": {"mergeStateStatus": "BLOCKED",
                "autoMergeRequest": {"mergeMethod": "SQUASH", "enabledAt": null,
                                     "enabledBy": null},
                "viewerCanEnableAutoMerge": true, "viewerCanDisableAutoMerge": true,
                "repository": {"autoMergeAllowed": true, "mergeCommitAllowed": false,
                    "squashMergeAllowed": true, "rebaseMergeAllowed": true}}}})
}

#[test]
fn pr_status_parses_the_pr_checks_and_merge_capability() {
    let server = FakeServer::start();
    server.add_agent("a1", "idle");
    server.reply(
        "checkout_pr_status_request",
        "checkout_pr_status_response",
        pr_status_payload(),
    );
    let mut harness = connect(&server);
    let reply = harness.core.pr_status("/repo");
    let answer = harness.wait(reply, WAIT).unwrap();
    assert!(answer.github_features_enabled);
    assert_eq!(
        answer.status,
        Some(PrStatus {
            number: None,
            url: "https://github.com/o/r/pull/9".into(),
            title: "Add git".into(),
            state: "open".into(),
            is_merged: false,
            is_draft: true,
            mergeable: Some("MERGEABLE".into()),
            checks: vec![
                PrCheck {
                    name: "build".into(),
                    status: "success".into(),
                    url: Some("https://ci/1".into()),
                    workflow: Some("CI".into()),
                    duration: None,
                    traits: vec![],
                },
                PrCheck {
                    name: "lint".into(),
                    status: "pending".into(),
                    url: None,
                    workflow: None,
                    duration: None,
                    traits: vec![],
                },
            ],
            review_decision: Some("APPROVED".into()),
            repo_owner: "o".into(),
            repo_name: "r".into(),
            merge: PrMergeCapability {
                auto_merge_enabled: true,
                auto_merge_method: Some(MergeMethod::Squash),
                can_enable_auto_merge: true,
                can_disable_auto_merge: true,
                allowed_methods: vec![MergeMethod::Squash, MergeMethod::Rebase],
                merge_state_status: Some("BLOCKED".into()),
            },
        })
    );
    assert_eq!(
        server.requests_of("checkout_pr_status_request")[0]["cwd"],
        json!("/repo")
    );
}

#[test]
fn pr_status_without_a_pr_or_github_facts() {
    let server = FakeServer::start();
    server.add_agent("a1", "idle");
    let mut payload = pr_status_payload();
    payload["status"]["number"] = json!(9);
    payload["status"]["github"] = Value::Null;
    payload["status"]["mergeable"] = Value::Null;
    server.reply(
        "checkout_pr_status_request",
        "checkout_pr_status_response",
        payload,
    );
    let mut harness = connect(&server);
    let reply = harness.core.pr_status("/repo");
    let status = harness.wait(reply, WAIT).unwrap().status.expect("a PR");
    assert_eq!(status.number, Some(9));
    assert_eq!(status.mergeable, None);
    assert_eq!(status.merge, PrMergeCapability::default());

    server.reply(
        "checkout_pr_status_request",
        "checkout_pr_status_response",
        json!({"cwd": "/repo", "status": null, "githubFeaturesEnabled": false, "error": null}),
    );
    let reply = harness.core.pr_status("/repo");
    let answer = harness.wait(reply, WAIT).unwrap();
    assert_eq!(
        (answer.status, answer.github_features_enabled),
        (None, false)
    );
}

#[test]
fn pr_timeline_sends_the_pr_and_parses_reviews_and_comments() {
    let server = FakeServer::start();
    server.add_agent("a1", "idle");
    server.reply(
        "pull_request_timeline_request",
        "pull_request_timeline_response",
        json!({"cwd": "/repo", "prNumber": 9, "truncated": false, "error": null, "items": [
            {"id": "r1", "kind": "review", "author": "ann", "body": "LGTM",
             "createdAt": 1760000000000_i64, "url": "https://github.com/o/r/pull/9#r1",
             "reviewState": "approved"},
            {"id": "c1", "kind": "comment", "author": "bob", "body": "nit",
             "createdAt": 1760000001000_i64, "url": "",
             "location": {"path": "src/a.rs", "line": 12}},
            {"id": "x1", "kind": "event", "body": "?"}
        ]}),
    );
    let mut harness = connect(&server);
    let reply = harness.core.pr_timeline("/repo", 9, "o", "r");
    let items = harness.wait(reply, WAIT).unwrap();
    assert_eq!(
        items,
        [
            PrTimelineItem::Review {
                author: "ann".into(),
                body: "LGTM".into(),
                created_at_ms: 1_760_000_000_000,
                url: Some("https://github.com/o/r/pull/9#r1".into()),
                state: "approved".into(),
            },
            PrTimelineItem::Comment {
                author: "bob".into(),
                body: "nit".into(),
                created_at_ms: 1_760_000_001_000,
                url: None,
                path: Some("src/a.rs".into()),
                line: Some(12),
            },
            PrTimelineItem::Comment {
                author: "unknown".into(),
                body: "?".into(),
                created_at_ms: 0,
                url: None,
                path: None,
                line: None,
            },
        ]
    );
    let request = &server.requests_of("pull_request_timeline_request")[0];
    // The generated schema types `prNumber` as f64, so it goes out as `9.0`.
    assert_eq!(
        (
            &request["cwd"],
            request["prNumber"].as_f64(),
            &request["repoOwner"],
            &request["repoName"]
        ),
        (&json!("/repo"), Some(9.0), &json!("o"), &json!("r"))
    );

    server.reply(
        "pull_request_timeline_request",
        "pull_request_timeline_response",
        json!({"items": [], "error": {"kind": "forbidden", "message": "no access"}}),
    );
    let reply = harness.core.pr_timeline("/repo", 9, "o", "r");
    assert_eq!(
        harness.wait(reply, WAIT).unwrap_err().to_string(),
        "no access"
    );
}

#[test]
fn switch_branch_sends_the_branch_and_reports_errors() {
    let server = FakeServer::start();
    server.add_agent("a1", "idle");
    server.reply(
        "checkout_switch_branch_request",
        "checkout_switch_branch_response",
        json!({"cwd": "/repo", "success": true, "branch": "dev", "error": null}),
    );
    let mut harness = connect(&server);
    let reply = harness.core.switch_branch("/repo", "dev");
    harness.wait(reply, WAIT).unwrap();
    let request = &server.requests_of("checkout_switch_branch_request")[0];
    assert_eq!(
        (&request["cwd"], &request["branch"]),
        (&json!("/repo"), &json!("dev"))
    );

    let mut failure = checkout_failure("local changes would be overwritten");
    failure["branch"] = json!("dev");
    server.reply(
        "checkout_switch_branch_request",
        "checkout_switch_branch_response",
        failure,
    );
    let reply = harness.core.switch_branch("/repo", "dev");
    assert_eq!(
        harness.wait(reply, WAIT).unwrap_err().to_string(),
        "local changes would be overwritten"
    );
}

#[test]
fn stash_save_pop_and_list_send_their_fields() {
    let server = checkout_server(&[
        ("stash_save_request", "stash_save_response"),
        ("stash_pop_request", "stash_pop_response"),
    ]);
    server.reply(
        "stash_list_request",
        "stash_list_response",
        json!({"cwd": "/repo", "error": null, "entries": [
            {"index": 0, "message": "paseo: main", "branch": "main", "isPaseo": true},
            {"index": 1, "message": "WIP", "branch": null, "isPaseo": false}
        ]}),
    );
    let mut harness = connect(&server);
    let reply = harness.core.stash_save("/repo", Some("main"));
    harness.wait(reply, WAIT).unwrap();
    let reply = harness.core.stash_save("/repo", None);
    harness.wait(reply, WAIT).unwrap();
    let reply = harness.core.stash_pop("/repo", 1);
    harness.wait(reply, WAIT).unwrap();
    let reply = harness.core.stash_list("/repo", false);
    assert_eq!(
        harness.wait(reply, WAIT).unwrap(),
        [
            StashEntry {
                index: 0,
                message: "paseo: main".into(),
                branch: Some("main".into()),
                is_paseo: true,
            },
            StashEntry {
                index: 1,
                message: "WIP".into(),
                branch: None,
                is_paseo: false,
            },
        ]
    );
    let saves = server.requests_of("stash_save_request");
    assert_eq!(saves[0]["branch"], json!("main"));
    assert!(saves[1].get("branch").is_none());
    assert_eq!(
        server.requests_of("stash_pop_request")[0]["stashIndex"],
        json!(1)
    );
    assert_eq!(
        server.requests_of("stash_list_request")[0]["paseoOnly"],
        json!(false)
    );

    server.reply(
        "stash_pop_request",
        "stash_pop_response",
        checkout_failure("conflict"),
    );
    let reply = harness.core.stash_pop("/repo", 0);
    assert_eq!(
        harness.wait(reply, WAIT).unwrap_err().to_string(),
        "conflict"
    );
}

#[test]
fn start_workspace_script_returns_the_terminal_id() {
    let server = FakeServer::start();
    server.add_agent("a1", "idle");
    server.on("start_workspace_script_request", |request| {
        let ok = request["scriptName"] == "web";
        vec![session(
            "start_workspace_script_response",
            json!({"requestId": request["requestId"], "workspaceId": request["workspaceId"],
                   "scriptName": request["scriptName"],
                   "terminalId": if ok { json!("t1") } else { Value::Null },
                   "error": if ok { Value::Null } else { json!("unknown script") }}),
        )]
    });
    let mut harness = connect(&server);
    let reply = harness.core.start_workspace_script("w1", "web");
    assert_eq!(harness.wait(reply, WAIT).unwrap(), "t1");
    let request = &server.requests_of("start_workspace_script_request")[0];
    assert_eq!(
        (&request["workspaceId"], &request["scriptName"]),
        (&json!("w1"), &json!("web"))
    );
    let reply = harness.core.start_workspace_script("w1", "nope");
    assert_eq!(
        harness.wait(reply, WAIT).unwrap_err().to_string(),
        "unknown script"
    );
}

#[test]
fn pr_checks_parse_duration_and_traits() {
    let server = FakeServer::start();
    server.add_agent("a1", "idle");
    let mut payload = pr_status_payload();
    payload["status"]["checks"] = json!([
        {"name": "build", "status": "success", "url": null, "duration": "2m 5s",
         "traits": ["required", "future-trait"]}
    ]);
    server.reply(
        "checkout_pr_status_request",
        "checkout_pr_status_response",
        payload,
    );
    let mut harness = connect(&server);
    let reply = harness.core.pr_status("/repo");
    let checks = harness.wait(reply, WAIT).unwrap().status.unwrap().checks;
    assert_eq!(
        checks,
        [PrCheck {
            name: "build".into(),
            status: "success".into(),
            url: None,
            workflow: None,
            duration: Some("2m 5s".into()),
            traits: vec!["required".into(), "future-trait".into()],
        }]
    );
}

#[test]
fn workspace_scripts_parse_their_proxy_urls() {
    let server = FakeServer::start();
    server.add_agent("a1", "idle");
    let mut described = workspace("w1", "p1", Some("main"), Value::Null);
    described["scripts"] = json!([{"scriptName": "web", "type": "service", "hostname": "web",
        "port": 3000, "lifecycle": "running", "health": "healthy",
        "proxyUrl": "https://web.example.test", "localProxyUrl": "http://web.localhost:6767",
        "publicProxyUrl": "https://web.example.test", "exitCode": null, "terminalId": "t1"}]);
    server.reply(
        "fetch_workspaces_request",
        "fetch_workspaces_response",
        json!({"subscriptionId": "sub-w", "entries": [described], "pageInfo": page_info()}),
    );
    let mut harness = connect(&server);
    assert!(harness.pump_until(WAIT, |h| !h.core.workspace_scripts("w1").is_empty()));
    assert_eq!(
        harness.core.workspace_scripts("w1"),
        [WorkspaceScript {
            name: "web".into(),
            kind: "service".into(),
            lifecycle: "running".into(),
            health: Some("healthy".into()),
            terminal_id: Some("t1".into()),
            exit_code: None,
            proxy_url: Some("https://web.example.test".into()),
            local_proxy_url: Some("http://web.localhost:6767".into()),
            public_proxy_url: Some("https://web.example.test".into()),
        }]
    );
}

#[test]
fn status_updates_are_subscribed_and_emit_git_status_events() {
    let server = FakeServer::start();
    server.add_agent("a1", "idle");
    let mut described = workspace("w1", "p1", Some("main"), Value::Null);
    described["scripts"] = json!([{"scriptName": "web", "type": "service", "hostname": "web",
        "port": 3000, "lifecycle": "stopped", "health": null}]);
    server.reply(
        "fetch_workspaces_request",
        "fetch_workspaces_response",
        json!({"subscriptionId": "sub-w", "entries": [described], "pageInfo": page_info()}),
    );
    let mut harness = connect(&server);
    let events = sent(
        &server,
        &mut harness,
        "session.events.set_subscription.request",
    )["events"]
        .clone();
    let events = events.as_array().unwrap();
    assert!(events.contains(&json!("checkout_status_update")));
    assert!(events.contains(&json!("script_status_update")));

    let mut seen = Vec::new();
    assert!(harness.pump_until(WAIT, |h| {
        seen.extend(h.core.take_git_status_events());
        !h.core.workspace_scripts("w1").is_empty()
    }));
    assert_eq!(
        harness.core.workspace_scripts("w1"),
        [WorkspaceScript {
            name: "web".into(),
            kind: "service".into(),
            lifecycle: "stopped".into(),
            health: None,
            terminal_id: None,
            exit_code: None,
            proxy_url: None,
            local_proxy_url: None,
            public_proxy_url: None,
        }]
    );
    assert!(seen.contains(&GitStatusEvent::ScriptsChanged {
        workspace_id: "w1".into()
    }));

    seen.clear();
    server.broadcast(json!({"type": "checkout_status_update", "payload": {
        "cwd": "/repo", "isGit": true, "isPaseoOwnedWorktree": false, "repoRoot": "/repo",
        "currentBranch": "main", "isDirty": false, "baseRef": null, "aheadBehind": null,
        "aheadOfOrigin": null, "behindOfOrigin": null, "hasRemote": false, "remoteUrl": null,
        "error": null, "requestId": "daemon-1"}}));
    server.broadcast(json!({"type": "script_status_update", "payload": {
        "workspaceId": "w1", "scripts": [{"scriptName": "web", "type": "service",
        "hostname": "web", "port": 3000, "lifecycle": "running", "health": "healthy",
        "exitCode": null, "terminalId": "t9"}]}}));
    assert!(harness.pump_until(WAIT, |h| {
        seen.extend(h.core.take_git_status_events());
        seen.len() >= 2
    }));
    assert_eq!(
        seen,
        [
            GitStatusEvent::Changed {
                cwd: "/repo".into()
            },
            GitStatusEvent::ScriptsChanged {
                workspace_id: "w1".into()
            },
        ]
    );
    let script = &harness.core.workspace_scripts("w1")[0];
    assert_eq!(
        (
            script.lifecycle.as_str(),
            script.health.as_deref(),
            script.terminal_id.as_deref()
        ),
        ("running", Some("healthy"), Some("t9"))
    );
}

fn with_features(server: &FakeServer, features: &[&str]) {
    server.with(|s| {
        for feature in features {
            s.features.insert((*feature).into(), json!(true));
        }
    });
}

fn history_row(id: &str, archived_at: Value) -> Value {
    let mut agent = agent_snapshot(id, "idle");
    agent["archivedAt"] = archived_at;
    agent["updatedAt"] = json!("2026-10-17T00:00:00.000Z");
    json!({"agent": agent, "project": {"projectKey": "p1", "projectName": "Project p1",
        "workspaceName": "ws one", "checkout": {"cwd": "/tmp/fake", "isGit": true,
        "currentBranch": "main", "remoteUrl": null, "isPaseoOwnedWorktree": false}}})
}

#[test]
fn agent_history_sends_the_search_page_and_sort_and_parses_the_page() {
    let server = FakeServer::start();
    server.add_agent("a1", "idle");
    with_features(&server, &["agentHistorySearch"]);
    server.reply(
        "fetch_agent_history_request",
        "fetch_agent_history_response",
        json!({"entries": [history_row("a1", Value::Null),
                           history_row("old", json!("2026-10-01T00:00:00.000Z"))],
               "pageInfo": {"nextCursor": "c2", "prevCursor": null, "hasMore": true},
               "searchTruncated": true}),
    );
    let mut harness = connect(&server);
    assert!(harness.core.history_search_supported());
    let reply = harness.core.fetch_agent_history(Some("bug"), Some("c1"));
    let page = harness.wait(reply, WAIT).unwrap();
    let request = &server.requests_of("fetch_agent_history_request")[0];
    assert_eq!(request["search"], json!("bug"));
    assert_eq!(
        request["sort"],
        json!([{"key": "updated_at", "direction": "desc"}])
    );
    assert_eq!(request["page"], json!({"limit": 200, "cursor": "c1"}));
    assert!(request.get("filter").is_none());

    let entry = |id: &str, archived: bool| HistoryEntry {
        agent_id: id.into(),
        title: format!("agent {id}"),
        provider: "mock".into(),
        workspace_id: Some("ws1".into()),
        workspace_name: Some("ws one".into()),
        project_name: "Project p1".into(),
        branch: Some("main".into()),
        cwd: "/tmp/fake".into(),
        archived,
        last_activity_at: Some("2026-10-17T00:00:00.000Z".into()),
        pending_permissions: 0,
    };
    assert_eq!(
        page,
        HistoryPage {
            entries: vec![entry("a1", false), entry("old", true)],
            next_cursor: Some("c2".into()),
            has_more: true,
            search_truncated: true,
        }
    );
    // The archived row fills the archived map; the live directory keeps only a1.
    assert!(harness.core.is_archived("old"));
    assert!(!harness.core.is_archived("a1"));
    let ids: Vec<&str> = harness
        .core
        .agents()
        .iter()
        .map(|a| a.id.as_str())
        .collect();
    assert_eq!(ids, ["a1"]);
}

#[test]
fn agent_history_sends_no_search_when_empty_or_unsupported() {
    let server = FakeServer::start();
    server.add_agent("a1", "idle");
    server.reply(
        "fetch_agent_history_request",
        "fetch_agent_history_response",
        json!({"entries": [], "pageInfo": page_info()}),
    );
    let mut harness = connect(&server);
    assert!(!harness.core.history_search_supported());
    let reply = harness.core.fetch_agent_history(Some("bug"), None);
    let page = harness.wait(reply, WAIT).unwrap();
    assert_eq!(page, HistoryPage::default());
    let request = &server.requests_of("fetch_agent_history_request")[0];
    assert!(request.get("search").is_none());
    assert_eq!(request["page"], json!({"limit": 200}));

    with_features(&server, &["agentHistorySearch"]);
    let mut harness = connect(&server);
    let reply = harness.core.fetch_agent_history(Some(""), None);
    harness.wait(reply, WAIT).unwrap();
    let request = server
        .requests_of("fetch_agent_history_request")
        .pop()
        .unwrap();
    assert!(request.get("search").is_none());
}

#[test]
fn archived_upserts_move_to_the_archived_map_and_back() {
    let server = FakeServer::start();
    server.add_agent("a1", "idle");
    server.add_agent("a2", "idle");
    let mut harness = connect(&server);
    assert!(harness.pump_until(WAIT, |h| h.core.agents().len() == 2));
    let live_a2 = harness.core.agents()[1].clone();

    let mut archived = agent_snapshot("a1", "idle");
    archived["archivedAt"] = json!("2026-10-17T00:00:00.000Z");
    server.broadcast(json!({"type": "agent_update",
                            "payload": {"kind": "upsert", "agent": archived}}));
    assert!(harness.pump_until(WAIT, |h| h.core.is_archived("a1")));
    let ids: Vec<&str> = harness
        .core
        .agents()
        .iter()
        .map(|a| a.id.as_str())
        .collect();
    assert_eq!(ids, ["a2"]);
    assert_eq!(harness.core.agents()[0], live_a2);
    assert_eq!(
        harness.core.archived_agent("a1").map(|a| a.title.as_str()),
        Some("agent a1")
    );

    server.broadcast(json!({"type": "agent_update",
                            "payload": {"kind": "upsert", "agent": agent_snapshot("a1", "idle")}}));
    assert!(harness.pump_until(WAIT, |h| !h.core.is_archived("a1")));
    assert!(harness.core.archived_agent("a1").is_none());
    assert!(harness.core.agents().iter().any(|a| a.id == "a1"));
}

/// The daemon sends `remove`, not an archived upsert, to a subscription without
/// `includeArchived`, so only a fetch fills the archived map (a stale Board card).
#[test]
fn an_archived_remove_needs_a_fetch_to_fill_the_archived_map() {
    let server = FakeServer::start();
    server.add_agent("a1", "idle");
    let mut archived = agent_snapshot("a1", "idle");
    archived["archivedAt"] = json!("2026-10-17T00:00:00.000Z");
    archived["workspaceId"] = json!("ws1");
    server.reply(
        "fetch_agent_request",
        "fetch_agent_response",
        json!({"agent": archived, "project": null, "error": null}),
    );
    let mut harness = connect(&server);
    assert!(harness.pump_until(WAIT, |h| h.core.agents().len() == 1));

    server.broadcast(json!({"type": "agent_update",
                            "payload": {"kind": "remove", "agentId": "a1"}}));
    assert!(harness.pump_until(WAIT, |h| h.core.agents().is_empty()));
    assert!(!harness.core.is_archived("a1"));

    let reply = harness.core.fetch_agent("a1");
    harness.wait(reply, WAIT).unwrap();
    assert_eq!(
        server.requests_of("fetch_agent_request")[0]["agentId"],
        json!("a1")
    );
    assert_eq!(
        harness
            .core
            .archived_agent("a1")
            .and_then(|a| a.workspace_id.as_deref()),
        Some("ws1")
    );
    assert!(harness.core.agents().is_empty());
}

#[test]
fn a_fetch_of_a_missing_agent_fails() {
    let server = FakeServer::start();
    server.add_agent("a1", "idle");
    server.reply(
        "fetch_agent_request",
        "fetch_agent_response",
        json!({"agent": null, "project": null, "error": "Agent not found: gone"}),
    );
    let mut harness = connect(&server);
    let reply = harness.core.fetch_agent("gone");
    let error = harness.wait(reply, WAIT).unwrap_err();
    assert_eq!(error.to_string(), "Agent not found: gone");
    assert!(!harness.core.is_archived("gone"));
}

#[test]
fn unarchive_sends_a_refresh() {
    let server = FakeServer::start();
    server.add_agent("a1", "idle");
    server.reply(
        "refresh_agent_request",
        "status",
        json!({"status": "agent_refreshed", "agentId": "old", "timelineSize": 0}),
    );
    let mut harness = connect(&server);
    let reply = harness.core.unarchive_agent("old");
    harness.wait(reply, WAIT).unwrap();
    assert_eq!(
        server.requests_of("refresh_agent_request")[0]["agentId"],
        json!("old")
    );
}

fn fork_reply(attachment: Value, error: Value) -> Value {
    json!({"agentId": "a1", "attachment": attachment, "itemCount": 4,
           "boundaryMessageId": null, "error": error})
}

#[test]
fn fork_context_keeps_the_raw_attachment_and_gates_the_cursor() {
    let server = FakeServer::start();
    server.add_agent("a1", "idle");
    with_features(&server, &["agentForkContext"]);
    let attachment = json!({"type": "text", "mimeType": "text/plain",
                            "contextKind": "chat_history", "title": "Chat", "text": "hi"});
    server.reply(
        "agent.fork_context.request",
        "agent.fork_context.response",
        fork_reply(attachment.clone(), Value::Null),
    );
    let mut harness = connect(&server);
    assert!(harness.core.fork_supported());
    assert!(!harness.core.fork_cursor_supported());
    let boundary = ForkBoundary {
        cursor: Some(ForkCursor {
            epoch: "e1".into(),
            seq: 5,
        }),
        message_id: Some("m1".into()),
    };
    let reply = harness.core.fork_context("a1", boundary.clone());
    let context = harness.wait(reply, WAIT).unwrap();
    assert_eq!(context.attachment, attachment);
    assert_eq!(context.title.as_deref(), Some("Chat"));
    assert_eq!(context.item_count, 4);
    let request = &server.requests_of("agent.fork_context.request")[0];
    assert_eq!(request["agentId"], json!("a1"));
    assert_eq!(request["boundaryMessageId"], json!("m1"));
    assert!(request.get("boundaryCursor").is_none());

    server.reply(
        "agent.fork_context.request",
        "agent.fork_context.response",
        fork_reply(Value::Null, json!("Agent not found")),
    );
    let failed = harness.core.fork_context("a1", ForkBoundary::default());
    assert_eq!(
        harness.wait(failed, WAIT).unwrap_err().to_string(),
        "Agent not found"
    );
    server.reply(
        "agent.fork_context.request",
        "agent.fork_context.response",
        fork_reply(Value::Null, Value::Null),
    );
    let empty = harness.core.fork_context("a1", ForkBoundary::default());
    assert_eq!(
        harness.wait(empty, WAIT).unwrap_err().to_string(),
        "Nothing to fork"
    );

    with_features(&server, &["agentForkContextCursor"]);
    server.reply(
        "agent.fork_context.request",
        "agent.fork_context.response",
        fork_reply(attachment, Value::Null),
    );
    let mut harness = connect(&server);
    let reply = harness.core.fork_context("a1", boundary);
    harness.wait(reply, WAIT).unwrap();
    let request = server
        .requests_of("agent.fork_context.request")
        .pop()
        .unwrap();
    assert_eq!(request["boundaryCursor"], json!({"epoch": "e1", "seq": 5}));
}

fn prompts_reply(error: Value) -> Value {
    json!({"agentId": "a1", "epoch": "e1", "error": error, "prompts": [
        {"seq": 2, "timestamp": "2026-10-19T00:00:00.000Z", "preview": "first"},
        {"seq": 9, "timestamp": "2026-10-19T00:01:00.000Z", "preview": "second"}]})
}

#[test]
fn list_prompts_is_gated_and_parses_the_index() {
    let server = FakeServer::start();
    server.add_agent("a1", "idle");
    let mut harness = connect(&server);
    assert!(!harness.core.prompt_index_supported());
    let reply = harness.core.list_prompts("a1");
    assert!(harness.wait(reply, WAIT).is_err());
    assert!(
        server
            .requests_of("agent.timeline.list_prompts.request")
            .is_empty()
    );

    with_features(&server, &["agentTimelinePromptIndex"]);
    server.reply(
        "agent.timeline.list_prompts.request",
        "agent.timeline.list_prompts.response",
        prompts_reply(Value::Null),
    );
    let mut harness = connect(&server);
    assert!(harness.core.prompt_index_supported());
    let reply = harness.core.list_prompts("a1");
    let index = harness.wait(reply, WAIT).unwrap();
    assert_eq!(index.epoch, "e1");
    let seqs: Vec<(u64, &str)> = index
        .prompts
        .iter()
        .map(|prompt| (prompt.seq, prompt.preview.as_str()))
        .collect();
    assert_eq!(seqs, vec![(2, "first"), (9, "second")]);
    let request = &server.requests_of("agent.timeline.list_prompts.request")[0];
    assert_eq!(request["agentId"], json!("a1"));

    server.reply(
        "agent.timeline.list_prompts.request",
        "agent.timeline.list_prompts.response",
        prompts_reply(json!("Agent not found")),
    );
    let failed = harness.core.list_prompts("a1");
    assert_eq!(
        harness.wait(failed, WAIT).unwrap_err().to_string(),
        "Agent not found"
    );
}

#[test]
fn timeline_epoch_follows_the_page_and_a_reset_page() {
    let server = FakeServer::start();
    server.add_agent("a1", "idle");
    server.append(
        "a1",
        json!({"type": "user_message", "text": "hi", "messageId": "u1"}),
        "t1",
        false,
    );
    let mut harness = connect(&server);
    assert_eq!(harness.core.timeline_epoch("a1"), None);
    assert_eq!(harness.core.timeline_epoch("nobody"), None);
    harness.core.view_agent("a1");
    assert!(harness.pump_until(WAIT, |h| h.core.timeline_epoch("a1") == Some("e1")));

    // A new epoch makes the resume cursor stale, so the daemon answers with a reset page.
    server.with(|s| s.timelines.get_mut("a1").unwrap().epoch = "e2".into());
    server.drop_connections();
    assert!(harness.pump_until(WAIT, |h| !h.core.is_connected()));
    assert!(harness.pump_until(WAIT, |h| h.core.timeline_epoch("a1") == Some("e2")));
    let resume = server
        .requests_of("fetch_agent_timeline_request")
        .pop()
        .unwrap();
    assert_eq!(resume["cursor"]["epoch"], json!("e1"));
}

#[test]
fn discard_changes_supported_follows_the_host_feature() {
    let server = FakeServer::start();
    server.add_agent("a1", "idle");
    let harness = connect(&server);
    assert!(!harness.core.discard_changes_supported());

    with_features(&server, &["checkoutDiscardChanges"]);
    let harness = connect(&server);
    assert!(harness.core.discard_changes_supported());
}

#[test]
fn fork_context_without_the_feature_fails_without_a_request() {
    let server = FakeServer::start();
    server.add_agent("a1", "idle");
    let mut harness = connect(&server);
    assert!(!harness.core.fork_supported());
    let reply = harness.core.fork_context("a1", ForkBoundary::default());
    assert_eq!(
        harness.wait(reply, WAIT).unwrap_err().to_string(),
        "Update the host to use this."
    );
    assert!(server.requests_of("agent.fork_context.request").is_empty());
}

// ---- file suggestion, entry, and discard calls (TASK-206) ----

#[test]
fn directory_suggestions_send_their_fields_and_read_entries_or_directories() {
    let server = FakeServer::start();
    server.add_agent("a1", "idle");
    server.reply(
        "directory_suggestions_request",
        "directory_suggestions_response",
        json!({"directories": ["src"], "error": null, "entries": [
            {"path": "src", "kind": "directory"},
            {"path": "src/main.rs", "kind": "file"},
            {"path": "odd", "kind": "socket"}
        ]}),
    );
    let mut harness = connect(&server);
    let reply = harness
        .core
        .directory_suggestions("/repo", "ma", true, false, 50);
    assert_eq!(
        harness.wait(reply, WAIT).unwrap(),
        [
            DirectorySuggestion {
                path: "src".into(),
                is_directory: true,
            },
            DirectorySuggestion {
                path: "src/main.rs".into(),
                is_directory: false,
            },
        ]
    );
    let request = &server.requests_of("directory_suggestions_request")[0];
    assert_eq!(request["cwd"], json!("/repo"));
    assert_eq!(request["query"], json!("ma"));
    assert_eq!(request["includeFiles"], json!(true));
    assert_eq!(request["includeDirectories"], json!(false));
    assert_eq!(request["limit"].as_f64(), Some(50.0));
    assert!(request.get("matchMode").is_none());

    // A host without `entries` gives only folders.
    server.reply(
        "directory_suggestions_request",
        "directory_suggestions_response",
        json!({"directories": ["docs"], "error": null}),
    );
    let reply = harness
        .core
        .directory_suggestions("/repo", "do", true, true, 500);
    assert_eq!(
        harness.wait(reply, WAIT).unwrap(),
        [DirectorySuggestion {
            path: "docs".into(),
            is_directory: true,
        }]
    );
    let request = server
        .requests_of("directory_suggestions_request")
        .pop()
        .unwrap();
    assert_eq!(request["limit"].as_f64(), Some(100.0));

    server.reply(
        "directory_suggestions_request",
        "directory_suggestions_response",
        json!({"directories": [], "entries": [], "error": "cwd not found"}),
    );
    let reply = harness
        .core
        .directory_suggestions("/gone", "", true, true, 10);
    assert_eq!(
        harness.wait(reply, WAIT).unwrap_err().to_string(),
        "cwd not found"
    );
}

#[test]
fn directory_suggestions_matching_sends_the_match_mode() {
    let server = FakeServer::start();
    server.add_agent("a1", "idle");
    server.reply(
        "directory_suggestions_request",
        "directory_suggestions_response",
        json!({"directories": [], "error": null, "entries": [
            {"path": "src/main.rs", "kind": "file"}
        ]}),
    );
    let mut harness = connect(&server);
    let reply = harness.core.directory_suggestions_matching(
        "/repo",
        "main.rs",
        true,
        false,
        5,
        MatchMode::Suffix,
    );
    assert_eq!(
        harness.wait(reply, WAIT).unwrap(),
        [DirectorySuggestion {
            path: "src/main.rs".into(),
            is_directory: false,
        }]
    );
    let request = &server.requests_of("directory_suggestions_request")[0];
    assert_eq!(request["matchMode"], json!("suffix"));
    assert_eq!(request["query"], json!("main.rs"));
    assert_eq!(request["includeFiles"], json!(true));
    assert_eq!(request["includeDirectories"], json!(false));
    assert_eq!(request["limit"].as_f64(), Some(5.0));
}

fn entry_reply(success: bool, error: Value) -> Value {
    json!({"cwd": "/repo", "path": "src/a.rs", "parentPath": "src", "renamedPath": null,
           "duplicatedPath": null, "success": success, "error": error})
}

#[test]
fn create_entry_sends_its_fields_and_reports_the_error() {
    let server = FakeServer::start();
    server.add_agent("a1", "idle");
    with_features(&server, &["fsEntryOps"]);
    server.reply(
        "fs.entry.create.request",
        "fs.entry.create.response",
        entry_reply(true, Value::Null),
    );
    let mut harness = connect(&server);
    assert!(harness.core.fs_entry_ops_supported());
    let reply = harness
        .core
        .create_entry("/repo", "src", "a.rs", NewEntryKind::File);
    harness.wait(reply, WAIT).unwrap();
    let reply = harness
        .core
        .create_entry("/repo", "", "lib", NewEntryKind::Directory);
    harness.wait(reply, WAIT).unwrap();
    let requests = server.requests_of("fs.entry.create.request");
    assert_eq!(requests[0]["cwd"], json!("/repo"));
    assert_eq!(requests[0]["parentPath"], json!("src"));
    assert_eq!(requests[0]["name"], json!("a.rs"));
    assert_eq!(requests[0]["kind"], json!("file"));
    assert_eq!(requests[1]["parentPath"], json!(""));
    assert_eq!(requests[1]["kind"], json!("directory"));

    server.reply(
        "fs.entry.create.request",
        "fs.entry.create.response",
        entry_reply(false, json!("An entry named a.rs already exists")),
    );
    let reply = harness
        .core
        .create_entry("/repo", "src", "a.rs", NewEntryKind::File);
    assert_eq!(
        harness.wait(reply, WAIT).unwrap_err().to_string(),
        "An entry named a.rs already exists"
    );
}

#[test]
fn rename_entry_sends_its_fields_and_reports_the_error() {
    let server = FakeServer::start();
    server.add_agent("a1", "idle");
    with_features(&server, &["fsEntryOps"]);
    server.reply(
        "fs.entry.rename.request",
        "fs.entry.rename.response",
        entry_reply(true, Value::Null),
    );
    let mut harness = connect(&server);
    let reply = harness.core.rename_entry("/repo", "src/a.rs", "b.rs");
    harness.wait(reply, WAIT).unwrap();
    let request = &server.requests_of("fs.entry.rename.request")[0];
    assert_eq!(request["cwd"], json!("/repo"));
    assert_eq!(request["path"], json!("src/a.rs"));
    assert_eq!(request["name"], json!("b.rs"));

    server.reply(
        "fs.entry.rename.request",
        "fs.entry.rename.response",
        entry_reply(false, json!("Invalid name")),
    );
    let reply = harness.core.rename_entry("/repo", "src/a.rs", "../b");
    assert_eq!(
        harness.wait(reply, WAIT).unwrap_err().to_string(),
        "Invalid name"
    );
}

#[test]
fn duplicate_entry_needs_its_own_feature() {
    let server = FakeServer::start();
    server.add_agent("a1", "idle");
    with_features(&server, &["fsEntryOps"]);
    let mut harness = connect(&server);
    assert!(!harness.core.fs_entry_duplicate_supported());
    let reply = harness.core.duplicate_entry("/repo", "src/a.rs");
    assert_eq!(
        harness.wait(reply, WAIT).unwrap_err().to_string(),
        "Update the host to use this."
    );
    assert!(server.requests_of("fs.entry.duplicate.request").is_empty());

    with_features(&server, &["fsEntryDuplicate"]);
    server.reply(
        "fs.entry.duplicate.request",
        "fs.entry.duplicate.response",
        entry_reply(true, Value::Null),
    );
    let mut harness = connect(&server);
    assert!(harness.core.fs_entry_duplicate_supported());
    let reply = harness.core.duplicate_entry("/repo", "src/a.rs");
    harness.wait(reply, WAIT).unwrap();
    let request = &server.requests_of("fs.entry.duplicate.request")[0];
    assert_eq!(request["cwd"], json!("/repo"));
    assert_eq!(request["path"], json!("src/a.rs"));

    server.reply(
        "fs.entry.duplicate.request",
        "fs.entry.duplicate.response",
        entry_reply(false, json!("Not found")),
    );
    let reply = harness.core.duplicate_entry("/repo", "src/gone.rs");
    assert_eq!(
        harness.wait(reply, WAIT).unwrap_err().to_string(),
        "Not found"
    );
}

#[test]
fn delete_entry_sends_its_fields_and_reports_a_failure() {
    let server = FakeServer::start();
    server.add_agent("a1", "idle");
    with_features(&server, &["fsEntryOps"]);
    server.reply(
        "fs.entry.delete.request",
        "fs.entry.delete.response",
        entry_reply(true, Value::Null),
    );
    let mut harness = connect(&server);
    let reply = harness.core.delete_entry("/repo", "src/a.rs");
    harness.wait(reply, WAIT).unwrap();
    let request = &server.requests_of("fs.entry.delete.request")[0];
    assert_eq!(request["cwd"], json!("/repo"));
    assert_eq!(request["path"], json!("src/a.rs"));

    server.reply(
        "fs.entry.delete.request",
        "fs.entry.delete.response",
        entry_reply(false, Value::Null),
    );
    let reply = harness.core.delete_entry("/repo", "src/a.rs");
    assert_eq!(
        harness.wait(reply, WAIT).unwrap_err().to_string(),
        "the file operation failed"
    );
}

#[test]
fn entry_calls_without_fs_entry_ops_fail_without_a_request() {
    let server = FakeServer::start();
    server.add_agent("a1", "idle");
    let mut harness = connect(&server);
    assert!(!harness.core.fs_entry_ops_supported());
    let replies = [
        harness
            .core
            .create_entry("/repo", "", "a", NewEntryKind::File),
        harness.core.rename_entry("/repo", "a", "b"),
        harness.core.delete_entry("/repo", "a"),
    ];
    for reply in replies {
        assert_eq!(
            harness.wait(reply, WAIT).unwrap_err().to_string(),
            "Update the host to use this."
        );
    }
    for kind in [
        "fs.entry.create.request",
        "fs.entry.rename.request",
        "fs.entry.delete.request",
    ] {
        assert!(server.requests_of(kind).is_empty(), "{kind} is not sent");
    }
}

#[test]
fn discard_changes_sends_the_paths_and_needs_one() {
    let server = checkout_server(&[(
        "checkout.discard_changes.request",
        "checkout.discard_changes.response",
    )]);
    let mut harness = connect(&server);
    let reply = harness.core.discard_changes("/repo", Vec::new());
    assert_eq!(
        harness.wait(reply, WAIT).unwrap_err().to_string(),
        "No files to discard"
    );
    assert!(
        server
            .requests_of("checkout.discard_changes.request")
            .is_empty()
    );

    let reply = harness
        .core
        .discard_changes("/repo", vec!["a.rs".into(), "src/b.rs".into()]);
    harness.wait(reply, WAIT).unwrap();
    let request = &server.requests_of("checkout.discard_changes.request")[0];
    assert_eq!(request["cwd"], json!("/repo"));
    assert_eq!(request["paths"], json!(["a.rs", "src/b.rs"]));

    server.reply(
        "checkout.discard_changes.request",
        "checkout.discard_changes.response",
        checkout_failure("pathspec did not match"),
    );
    let reply = harness
        .core
        .discard_changes("/repo", vec!["gone.rs".into()]);
    assert_eq!(
        harness.wait(reply, WAIT).unwrap_err().to_string(),
        "pathspec did not match"
    );
}

fn rewind_reply(ok: bool, error: Value) -> Value {
    json!({"agentId": "a1", "ok": ok, "error": error})
}

/// A host with the `rewind` feature, an agent with a short timeline, and a viewed timeline.
fn rewind_harness() -> (FakeServer, Harness) {
    let server = FakeServer::start();
    server.add_agent("a1", "idle");
    with_features(&server, &["rewind"]);
    for (id, text) in [("u1", "first"), ("u2", "second")] {
        server.append(
            "a1",
            json!({"type": "user_message", "text": text, "messageId": id}),
            id,
            false,
        );
    }
    let mut harness = connect(&server);
    harness.core.view_agent("a1");
    assert!(harness.pump_until(WAIT, |h| h.entries("a1").len() == 2));
    (server, harness)
}

fn tail_fetches(server: &FakeServer) -> usize {
    server.requests_of("fetch_agent_timeline_request").len()
}

#[test]
fn rewind_modes_come_from_the_agent_capabilities_of_a_host_with_rewind() {
    let (server, mut harness) = rewind_harness();
    assert_eq!(harness.core.rewind_modes("a1"), RewindModes::default());
    let mut agent = agent_snapshot("a1", "idle");
    agent["capabilities"] = json!({"supportsRewindConversation": true, "supportsRewindBoth": true});
    server
        .broadcast(json!({"type": "agent_update", "payload": {"kind": "upsert", "agent": agent}}));
    let expected = RewindModes {
        conversation: true,
        files: false,
        both: true,
    };
    assert!(harness.pump_until(WAIT, |h| h.core.rewind_modes("a1") == expected));
    assert_eq!(harness.core.rewind_modes("missing"), RewindModes::default());
}

#[test]
fn a_host_without_rewind_offers_no_modes_and_refuses_the_call() {
    let server = FakeServer::start();
    let mut agent = agent_snapshot("a1", "idle");
    agent["capabilities"] = json!({"supportsRewindConversation": true,
                                   "supportsRewindFiles": true, "supportsRewindBoth": true});
    server.with(|s| s.agents.push(agent));
    let mut harness = connect(&server);
    assert_eq!(harness.core.rewind_modes("a1"), RewindModes::default());
    let reply = harness.core.rewind("a1", "u1", RewindMode::Both);
    assert_eq!(
        harness.wait(reply, WAIT).unwrap_err().to_string(),
        "Update the host to use this."
    );
    assert!(server.requests_of("agent.rewind.request").is_empty());
}

#[test]
fn rewind_sends_the_wire_request_and_resolves_on_the_reply() {
    let (server, mut harness) = rewind_harness();
    server.reply(
        "agent.rewind.request",
        "agent.rewind.response",
        rewind_reply(true, Value::Null),
    );
    let before = tail_fetches(&server);
    let reply = harness.core.rewind("a1", "u2", RewindMode::Conversation);
    harness.wait(reply, WAIT).unwrap();
    let request = &server.requests_of("agent.rewind.request")[0];
    assert_eq!(request["type"], json!("agent.rewind.request"));
    assert_eq!(request["agentId"], json!("a1"));
    assert_eq!(request["messageId"], json!("u2"));
    assert_eq!(request["mode"], json!("conversation"));
    assert!(request["requestId"].is_string());
    assert!(
        harness.pump_until(WAIT, |_| tail_fetches(&server) > before),
        "a rewound conversation reloads the timeline tail"
    );

    // The files alone leave the timeline as it is.
    let before = tail_fetches(&server);
    let reply = harness.core.rewind("a1", "u1", RewindMode::Files);
    harness.wait(reply, WAIT).unwrap();
    assert_eq!(
        server.requests_of("agent.rewind.request")[1]["mode"],
        json!("files")
    );
    harness.pump_until(Duration::from_millis(300), |_| false);
    assert_eq!(tail_fetches(&server), before);
}

#[test]
fn a_failed_rewind_resolves_with_the_error_and_changes_no_client_state() {
    let (server, mut harness) = rewind_harness();
    server.reply(
        "agent.rewind.request",
        "agent.rewind.response",
        rewind_reply(false, json!("Cannot rewind a running agent")),
    );
    let entries = harness.entries("a1");
    let before = tail_fetches(&server);
    let reply = harness.core.rewind("a1", "u1", RewindMode::Both);
    assert_eq!(
        harness.wait(reply, WAIT).unwrap_err().to_string(),
        "Cannot rewind a running agent"
    );
    harness.pump_until(Duration::from_millis(300), |_| false);
    assert_eq!(harness.entries("a1").len(), 2, "the timeline item count");
    assert_eq!(harness.entries("a1"), entries);
    assert_eq!(tail_fetches(&server), before, "no tail reload");

    // `ok: false` without an error text still fails.
    server.reply(
        "agent.rewind.request",
        "agent.rewind.response",
        rewind_reply(false, Value::Null),
    );
    let reply = harness.core.rewind("a1", "u1", RewindMode::Both);
    assert_eq!(
        harness.wait(reply, WAIT).unwrap_err().to_string(),
        "Rewind failed"
    );
    assert_eq!(tail_fetches(&server), before);
}

// ---- background processes (TASK-264) ----

fn background_process(id: &str) -> Value {
    json!({"id": id, "kind": "bash", "description": format!("run {id}"),
           "command": "sleep 1", "status": "running", "toolCallId": "tc1",
           "startedAt": "2026-10-19T10:00:00.000Z", "endedAt": null, "exitCode": null,
           "outputFile": "/tmp/out", "tail": ["a", "b"]})
}

fn agent_with_processes(id: &str, processes: Value) -> Value {
    let mut agent = agent_snapshot(id, "running");
    agent["backgroundProcesses"] = processes;
    json!({"type": "agent_update", "payload": {"kind": "upsert", "agent": agent}})
}

#[test]
fn an_agent_snapshot_keeps_its_background_processes() {
    let server = FakeServer::start();
    server.add_agent("a1", "idle");
    let mut harness = connect(&server);
    let mut completed = background_process("p1");
    completed["command"] = Value::Null;
    completed["status"] = json!("completed");
    completed["endedAt"] = json!("2026-10-19T10:01:00.000Z");
    completed["exitCode"] = json!(0);
    let running = background_process("p2");
    server.broadcast(agent_with_processes("a1", json!([completed, running])));
    assert!(
        harness.pump_until(WAIT, |h| h.core.background_processes("a1").len() == 2),
        "both processes arrive"
    );
    let processes = harness.core.background_processes("a1");
    assert_eq!(
        processes.iter().map(|p| p.id.as_str()).collect::<Vec<_>>(),
        ["p1", "p2"],
        "daemon order"
    );
    assert_eq!(processes[0].command, None);
    assert_eq!(processes[0].exit_code, Some(0));
    assert_eq!(processes[0].status, "completed");
    assert_eq!(processes[0].tail, ["a", "b"]);
    assert_eq!(
        processes[0].ended_at_ms.unwrap() - processes[0].started_at_ms,
        60_000
    );
    assert_eq!(processes[1].command.as_deref(), Some("sleep 1"));
    assert_eq!(processes[1].ended_at_ms, None);
    assert_eq!(processes[1].exit_code, None);
    assert_eq!(processes[1].tool_call_id.as_deref(), Some("tc1"));
    assert_eq!(processes[1].output_file.as_deref(), Some("/tmp/out"));
}

#[test]
fn an_upsert_without_processes_clears_the_list() {
    let server = FakeServer::start();
    server.add_agent("a1", "idle");
    let mut harness = connect(&server);
    server.broadcast(agent_with_processes(
        "a1",
        json!([background_process("p1")]),
    ));
    assert!(harness.pump_until(WAIT, |h| { h.core.background_processes("a1").len() == 1 }));
    let agent = agent_snapshot("a1", "running");
    server
        .broadcast(json!({"type": "agent_update", "payload": {"kind": "upsert", "agent": agent}}));
    assert!(harness.pump_until(WAIT, |h| { h.core.background_processes("a1").is_empty() }));
    server.broadcast(agent_with_processes("a1", json!([])));
    harness.pump_until(Duration::from_millis(300), |_| false);
    assert!(harness.core.background_processes("a1").is_empty());
}

#[test]
fn a_process_with_an_unreadable_start_or_end_is_dropped_or_kept_open() {
    let server = FakeServer::start();
    server.add_agent("a1", "idle");
    let mut harness = connect(&server);
    let mut unreadable_start = background_process("bad");
    unreadable_start["startedAt"] = json!("not a date");
    let mut unreadable_end = background_process("p2");
    unreadable_end["endedAt"] = json!("not a date");
    server.broadcast(agent_with_processes(
        "a1",
        json!([unreadable_start, unreadable_end]),
    ));
    assert!(harness.pump_until(WAIT, |h| { h.core.background_processes("a1").len() == 1 }));
    let processes = harness.core.background_processes("a1");
    assert_eq!(processes[0].id, "p2");
    assert_eq!(processes[0].ended_at_ms, None);
}

#[test]
fn background_stop_follows_the_host_feature() {
    let server = FakeServer::start();
    server.add_agent("a1", "idle");
    with_features(&server, &["agentBackgroundProcesses"]);
    let harness = connect(&server);
    assert!(harness.core.supports_background_stop());

    let plain = FakeServer::start();
    plain.add_agent("a1", "idle");
    let harness = connect(&plain);
    assert!(!harness.core.supports_background_stop());
}

#[test]
fn stop_background_process_sends_the_request_and_survives_a_refusal() {
    let server = FakeServer::start();
    server.add_agent("a1", "idle");
    server.on("agent.background_process.stop.request", |request| {
        vec![session(
            "agent.background_process.stop.response",
            json!({"requestId": request["requestId"], "agentId": request["agentId"],
                   "processId": request["processId"], "error": "gone"}),
        )]
    });
    let mut harness = connect(&server);
    harness.core.stop_background_process("a1", "p1");
    let request = sent(
        &server,
        &mut harness,
        "agent.background_process.stop.request",
    );
    assert_eq!(request["agentId"], "a1");
    assert_eq!(request["processId"], "p1");
    assert!(request["requestId"].is_string());
    // The refusal logs a warning and leaves the core serving requests.
    harness.pump_until(Duration::from_millis(300), |_| false);
    assert!(harness.core.is_connected());
    harness.core.cancel_agent("a1");
    let cancel = sent(&server, &mut harness, "cancel_agent_request");
    assert_eq!(cancel["agentId"], "a1");
}

// ---- provider subagent timelines (TASK-270) ----

fn provider_rows(seqs: &[u64]) -> Value {
    Value::Array(
        seqs.iter()
            .map(|seq| {
                json!({
                    "item": {"type": "user_message", "text": format!("row {seq}"),
                             "messageId": format!("m{seq}")},
                    "timestamp": format!("2026-10-19T10:00:{seq:02}.000Z"),
                    "seq": seq,
                })
            })
            .collect(),
    )
}

fn provider_page(
    request: &Value,
    rows: Value,
    start: u64,
    end: u64,
    has_older: bool,
    reset: bool,
) -> Value {
    json!({
        "requestId": request["requestId"],
        "parentAgentId": "a1",
        "subagentId": "s1",
        "provider": "claude",
        "direction": request["direction"],
        "epoch": "e1",
        "projection": "projected",
        "startCursor": {"epoch": "e1", "seq": start},
        "endCursor": {"epoch": "e1", "seq": end},
        "reset": reset,
        "staleCursor": false,
        "gap": false,
        "window": {"minSeq": start, "maxSeq": end, "nextSeq": end + 1},
        "hasOlder": has_older,
        "hasNewer": false,
        "rows": rows,
        "error": null,
    })
}

/// A host with the projected subagent timeline feature, whose child tail holds rows 3 and
/// 4 and whose older page holds rows 1 and 2.
fn provider_timeline_harness() -> (FakeServer, Harness, String) {
    let server = FakeServer::start();
    server.add_agent("a1", "idle");
    with_features(&server, &["projectedSubagentTimeline"]);
    server.on("agent.provider_subagents.timeline.get.request", |request| {
        let payload = if request["direction"] == "before" {
            provider_page(request, provider_rows(&[1, 2]), 1, 2, false, false)
        } else {
            provider_page(request, provider_rows(&[3, 4]), 3, 4, true, true)
        };
        vec![session(
            "agent.provider_subagents.timeline.get.response",
            payload,
        )]
    });
    let harness = connect(&server);
    let key = provider_subagent_key("a1", "s1");
    (server, harness, key)
}

fn child_tail_fetches(server: &FakeServer) -> usize {
    server
        .requests_of("agent.provider_subagents.timeline.get.request")
        .into_iter()
        .filter(|request| request["direction"] == "tail")
        .count()
}

#[test]
fn viewing_a_provider_subagent_key_fetches_and_stores_its_tail() {
    let (server, mut harness, key) = provider_timeline_harness();
    assert_eq!(
        parse_provider_subagent_key(&key),
        Some(("a1".into(), "s1".into()))
    );
    harness.core.view_agent(&key);
    let request = sent(
        &server,
        &mut harness,
        "agent.provider_subagents.timeline.get.request",
    );
    assert_eq!(request["parentAgentId"], "a1");
    assert_eq!(request["subagentId"], "s1");
    assert_eq!(request["direction"], "tail");
    assert_eq!(request["limit"], 40);
    assert!(
        harness.pump_until(WAIT, |h| h.entries(&key).len() == 2),
        "the reply rows are stored"
    );
    let entries = harness.entries(&key);
    assert_eq!(entries[0]["item"]["text"], "row 3");
    assert_eq!(entries[1]["item"]["text"], "row 4");
    assert_eq!(entries[0]["provider"], "claude");
    assert_eq!(entries[0]["seqStart"], 3);
    assert_eq!(
        harness.core.timeline_epoch(&key),
        Some("e1"),
        "the child epoch"
    );
}

#[test]
fn provider_subagent_live_items_ignore_an_old_seq_or_another_epoch() {
    let (server, mut harness, key) = provider_timeline_harness();
    harness.core.view_agent(&key);
    assert!(harness.pump_until(WAIT, |h| h.entries(&key).len() == 2));
    let live = |seq: u64, epoch: &str| {
        json!({"type": "agent.provider_subagents.update", "payload": {
            "kind": "timeline", "parentAgentId": "a1", "subagentId": "s1",
            "provider": "claude",
            "item": {"type": "user_message", "text": format!("live {seq}"),
                "messageId": format!("live{seq}")},
            "timestamp": "2026-10-19T10:01:00.000Z", "seq": seq, "epoch": epoch}})
    };
    server.broadcast(live(5, "e1"));
    assert!(harness.pump_until(WAIT, |h| h.entries(&key).len() == 3));
    server.broadcast(live(4, "e1"));
    server.broadcast(live(6, "e2"));
    harness.pump_until(Duration::from_millis(300), |_| false);
    let entries = harness.entries(&key);
    assert_eq!(entries.len(), 3, "an old seq and another epoch are dropped");
    assert_eq!(entries[2]["item"]["text"], "live 5");
}

#[test]
fn loading_older_provider_rows_sends_a_before_request_and_prepends_them() {
    let (server, mut harness, key) = provider_timeline_harness();
    harness.core.view_agent(&key);
    assert!(harness.pump_until(WAIT, |h| h.entries(&key).len() == 2));
    harness.core.load_older(&key);
    assert!(harness.pump_until(WAIT, |h| h.entries(&key).len() == 4));
    let requests = server.requests_of("agent.provider_subagents.timeline.get.request");
    let before = requests
        .iter()
        .find(|request| request["direction"] == "before")
        .expect("a before request");
    assert_eq!(before["cursor"]["epoch"], "e1");
    assert_eq!(before["cursor"]["seq"], 3, "the first seq is the cursor");
    let texts: Vec<String> = harness
        .entries(&key)
        .iter()
        .map(|entry| {
            entry["item"]["text"]
                .as_str()
                .unwrap_or_default()
                .to_string()
        })
        .collect();
    assert_eq!(texts, ["row 1", "row 2", "row 3", "row 4"]);
}

#[test]
fn a_host_without_projected_subagent_timeline_sends_no_child_request() {
    let server = FakeServer::start();
    server.add_agent("a1", "idle");
    let mut harness = connect(&server);
    let key = provider_subagent_key("a1", "s1");
    harness.core.view_agent(&key);
    harness.pump_until(Duration::from_millis(300), |_| false);
    assert!(
        server
            .requests_of("agent.provider_subagents.timeline.get.request")
            .is_empty()
    );
}

#[test]
fn a_reconnect_refetches_the_child_tail_and_never_sends_its_key_as_an_agent() {
    let (server, mut harness, key) = provider_timeline_harness();
    harness.core.view_agent(&key);
    assert!(harness.pump_until(WAIT, |_| child_tail_fetches(&server) == 1));
    server.drop_connections();
    assert!(harness.pump_until(WAIT, |h| !h.core.is_connected()));
    assert!(
        harness.pump_until(WAIT, |_| child_tail_fetches(&server) == 2),
        "a new tail goes out after the reconnect"
    );
    for request in server.requests_of("agent.timeline.set_subscription.request") {
        let ids = request["agentIds"].as_array().cloned().unwrap_or_default();
        assert!(!ids.contains(&json!(key)), "the key is not an agent id");
    }
    for request in server.requests_of("fetch_agent_timeline_request") {
        assert_ne!(request["agentId"], json!(key));
    }
}

#[test]
fn a_remove_update_drops_a_viewed_child_timeline() {
    let (server, mut harness, key) = provider_timeline_harness();
    harness.core.view_agent(&key);
    assert!(harness.pump_until(WAIT, |h| h.entries(&key).len() == 2));
    server.broadcast(
        json!({"type": "agent.provider_subagents.update", "payload": {
        "kind": "remove", "parentAgentId": "a1", "subagentId": "s1"}}),
    );
    assert!(harness.pump_until(WAIT, |h| h.core.timeline(&key).is_none()));
}

#[test]
fn provider_subagent_keys_round_trip_and_plain_agent_ids_do_not() {
    let key = provider_subagent_key("a:1", "s/2");
    assert_eq!(key, "provider:a%3A1:s%2F2");
    assert_eq!(
        parse_provider_subagent_key(&key),
        Some(("a:1".into(), "s/2".into()))
    );
    assert_eq!(parse_provider_subagent_key("a1"), None);
    assert_eq!(parse_provider_subagent_key("provider:a1"), None);
}

#[test]
fn remove_project_sends_the_project_remove_request() {
    let server = FakeServer::start();
    server.add_agent("a1", "idle");
    let mut harness = connect(&server);
    harness.core.remove_project("p1");
    let request = sent(&server, &mut harness, "project.remove.request");
    assert_eq!(request["projectId"], "p1");
    assert!(
        request["requestId"]
            .as_str()
            .is_some_and(|id| !id.is_empty())
    );
}

#[test]
fn a_refused_project_remove_is_reported() {
    let server = FakeServer::start();
    server.add_agent("a1", "idle");
    server.reply(
        "project.remove.request",
        "project.remove.response",
        json!({"projectId": "p1", "accepted": false, "removedWorkspaceIds": [], "error": "busy"}),
    );
    let mut harness = connect(&server);
    harness.core.remove_project("p1");
    assert!(harness.pump_until(WAIT, |harness| {
        harness.events.contains(&ClientEvent::ProjectRemoveFailed {
            project_id: "p1".to_string(),
        })
    }));
}

#[test]
fn an_accepted_project_remove_drops_the_project_and_its_workspaces() {
    let server = FakeServer::start();
    server.add_agent("a1", "idle");
    server.reply(
        "project.list.request",
        "project.list.response",
        json!({"projects": [project("p1"), project("p2")]}),
    );
    server.reply(
        "fetch_workspaces_request",
        "fetch_workspaces_response",
        json!({"subscriptionId": "sub-w", "entries": [
            workspace("w1", "p1", Some("main"), Value::Null),
        ], "emptyProjects": [], "pageInfo": page_info()}),
    );
    server.reply(
        "project.remove.request",
        "project.remove.response",
        json!({"projectId": "p1", "accepted": true, "removedWorkspaceIds": ["w1"], "error": null}),
    );
    let mut harness = connect(&server);
    assert!(harness.pump_until(WAIT, |h| h.core.projects().len() == 2
        && h.core.projects()[0].workspaces.len() == 1));
    harness.core.remove_project("p1");
    server.broadcast(json!({"type": "workspace_update", "payload": {
            "kind": "remove", "id": "w1", "removedProjectId": "p1"}}));
    assert!(harness.pump_until(WAIT, |h| h.core.projects().len() == 1));
    assert_eq!(harness.core.projects()[0].id, "p2");
    assert!(
        !harness
            .events
            .iter()
            .any(|event| matches!(event, ClientEvent::ProjectRemoveFailed { .. }))
    );
}

#[test]
fn a_project_remove_with_an_error_keeps_the_project() {
    let server = FakeServer::start();
    server.add_agent("a1", "idle");
    server.reply(
        "project.list.request",
        "project.list.response",
        json!({"projects": [project("p1")]}),
    );
    server.reply(
        "project.remove.request",
        "project.remove.response",
        json!({"projectId": "p1", "accepted": true, "removedWorkspaceIds": [], "error": "busy"}),
    );
    let mut harness = connect(&server);
    assert!(harness.pump_until(WAIT, |h| h.core.projects().len() == 1));
    harness.core.remove_project("p1");
    assert!(harness.pump_until(WAIT, |h| {
        h.events.contains(&ClientEvent::ProjectRemoveFailed {
            project_id: "p1".to_string(),
        })
    }));
    assert_eq!(harness.core.projects().len(), 1);
}
