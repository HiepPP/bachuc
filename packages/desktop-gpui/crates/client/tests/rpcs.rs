//! The parity requests (TASK-146) against the local fake daemon: request shapes and parsed
//! answers.

mod common;

use bachuc_client::{
    ChangedFile, CheckoutStatus, DirectoryEvent, FileKind, ProjectNode, QueuedPrompt, TurnBehavior,
    WorkspaceNode,
};
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
                },
                WorkspaceNode {
                    id: "w2".into(),
                    name: "ws w2".into(),
                    cwd: "/repo/w2".into(),
                    branch: None,
                    pinned: false,
                    agent_ids: vec![],
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
    assert_eq!(
        server.requests_of("list_commands_request")[0]["agentId"],
        "a1"
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
