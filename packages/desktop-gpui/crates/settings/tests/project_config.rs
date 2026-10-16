//! The project config form against the fake daemon of the client tests: read, edit the draft,
//! write, and the stale and empty cases.

#[path = "../../client/tests/common/mod.rs"]
mod common;

use bachuc_settings::config::{ConfigDraft, apply_draft, config_to_draft};
use common::{FakeServer, Harness, session};
use serde_json::{Value, json};
use std::time::Duration;

const WAIT: Duration = Duration::from_secs(5);

fn connect(server: &FakeServer) -> Harness {
    let mut harness = Harness::connect(&server.url, None);
    assert!(harness.connected(), "connects");
    harness
}

/// A daemon that keeps one `paseo.json` and bumps its revision on each accepted write.
fn serve_config(server: &FakeServer, config: Value) {
    let stored = std::sync::Arc::new(std::sync::Mutex::new((config, 1.0_f64)));
    let reader = stored.clone();
    server.on("read_project_config_request", move |request| {
        let (config, mtime) = reader.lock().unwrap().clone();
        vec![session(
            "read_project_config_response",
            json!({
                "requestId": request["requestId"], "repoRoot": request["repoRoot"], "ok": true,
                "config": config, "revision": {"mtimeMs": mtime, "size": 10}}),
        )]
    });
    server.on("write_project_config_request", move |request| {
        let mut stored = stored.lock().unwrap();
        let payload = if request["expectedRevision"]["mtimeMs"].as_f64() == Some(stored.1) {
            stored.0 = request["config"].clone();
            stored.1 += 1.0;
            json!({"requestId": request["requestId"], "repoRoot": request["repoRoot"], "ok": true,
                   "config": request["config"], "revision": {"mtimeMs": stored.1, "size": 12}})
        } else {
            json!({"requestId": request["requestId"], "repoRoot": request["repoRoot"], "ok": false,
                   "error": {"code": "stale_project_config", "currentRevision": null}})
        };
        vec![session("write_project_config_response", payload)]
    });
}

fn start(config: Value) -> (FakeServer, Harness) {
    let server = FakeServer::start();
    server.add_agent("a1", "idle");
    serve_config(&server, config);
    let harness = connect(&server);
    (server, harness)
}

#[test]
fn an_edited_form_round_trips_through_the_daemon() {
    let original = json!({
        "worktree": {"setup": ["npm ci"]},
        "scripts": {"dev": {"command": "npm run dev", "type": "service", "port": 3000}},
        "unrelated": {"keep": true},
    });
    let (server, mut harness) = start(original.clone());

    let reply = harness.core.read_project_config("/repo");
    let read = harness.wait(reply, WAIT).unwrap();
    assert_eq!(read, original);

    let mut draft = config_to_draft(&read);
    draft.setup_text = "npm ci\nnpm run build".into();
    draft.prompts[0] = "Prefix branches with feat/".into();
    let written = apply_draft(&draft, &read);
    let reply = harness.core.write_project_config("/repo", written.clone());
    harness.wait(reply, WAIT).unwrap();

    let sent = server
        .requests_of("write_project_config_request")
        .pop()
        .unwrap();
    assert_eq!(sent["config"], written);
    assert_eq!(sent["config"]["unrelated"], json!({"keep": true}));
    assert_eq!(
        sent["config"]["worktree"]["setup"],
        json!(["npm ci", "npm run build"])
    );

    // A second read shows what the daemon now stores.
    let reply = harness.core.read_project_config("/repo");
    let again = harness.wait(reply, WAIT).unwrap();
    assert_eq!(again, written);
    assert_eq!(
        config_to_draft(&again).prompts[0],
        "Prefix branches with feat/"
    );
}

#[test]
fn a_project_with_no_config_shows_the_empty_form_and_saves_a_new_file() {
    let (server, mut harness) = start(Value::Null);

    let reply = harness.core.read_project_config("/repo");
    let read = harness.wait(reply, WAIT).unwrap();
    assert_eq!(read, Value::Null);

    let mut draft = config_to_draft(&read);
    assert_eq!(draft.setup_text, "");
    assert!(draft.scripts.is_empty());
    let mut script = draft.new_script();
    script.name = "dev".into();
    script.command_text = "npm run dev".into();
    draft.scripts.push(script);

    let reply = harness
        .core
        .write_project_config("/repo", apply_draft(&draft, &read));
    harness.wait(reply, WAIT).unwrap();
    let sent = server
        .requests_of("write_project_config_request")
        .pop()
        .unwrap();
    assert_eq!(
        sent["config"],
        json!({"scripts": {"dev": {"command": "npm run dev"}}})
    );
}

#[test]
fn a_write_after_the_file_changed_is_refused_as_stale() {
    let (server, mut harness) = start(json!({"worktree": {"setup": "npm ci"}}));

    let reply = harness.core.read_project_config("/repo");
    let read = harness.wait(reply, WAIT).unwrap();

    // Another client writes first: the revision moves on.
    let mut other = Harness::connect(&server.url, None);
    assert!(other.connected());
    let reply = other.core.read_project_config("/repo");
    other.wait(reply, WAIT).unwrap();
    let reply = other
        .core
        .write_project_config("/repo", json!({"worktree": {"setup": "pnpm i"}}));
    other.wait(reply, WAIT).unwrap();

    let mut draft = config_to_draft(&read);
    draft.setup_text = "npm install".into();
    let reply = harness
        .core
        .write_project_config("/repo", apply_draft(&draft, &read));
    let error = harness.wait(reply, WAIT).unwrap_err();
    assert_eq!(error.to_string(), "stale_project_config");
}

#[test]
fn the_empty_draft_matches_a_null_read() {
    assert_eq!(config_to_draft(&Value::Null), ConfigDraft::empty());
}
