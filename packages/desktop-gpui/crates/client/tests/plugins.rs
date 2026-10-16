//! The plugin channel (TASK-168) against the local fake daemon: plugin list, catalog, RPCs,
//! settings, plugin events, and the agent label and runtime calls.

mod common;

use bachuc_client::{
    PluginEvent, PluginListItem, PluginSettingsRead, PluginSettingsWrite, PluginStatus,
};
use common::{FakeServer, Harness, agent_snapshot, session};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::time::Duration;

const WAIT: Duration = Duration::from_secs(5);

fn connect(server: &FakeServer) -> Harness {
    let mut harness = Harness::connect(&server.url, None);
    assert!(harness.connected(), "connects");
    harness
}

fn rpc_error(request: &Value, error: &str) -> Vec<tungstenite::Message> {
    vec![session(
        "rpc_error",
        json!({"requestId": request["requestId"], "requestType": request["type"],
               "error": error, "code": "plugin_failed"}),
    )]
}

#[test]
fn list_plugins_keeps_the_daemon_order_and_reads_an_unknown_status() {
    let server = FakeServer::start();
    server.add_agent("a1", "idle");
    server.reply(
        "plugin.list.request",
        "plugin.list.response",
        json!({"plugins": [
            {"id": "skill-pins", "path": "/p/a", "enabled": true, "status": "running",
             "description": "Pins"},
            {"id": "stop-orb", "path": "/p/b", "enabled": true, "status": "starting"},
            {"path": "/p/broken", "enabled": true, "status": "running"},
            {"id": "edit-diffs", "path": "/p/c", "enabled": false, "status": "failed",
             "error": "boom"},
            {"id": "pr-watch", "path": "/p/d", "enabled": false, "status": "disabled"},
        ]}),
    );
    let mut harness = connect(&server);
    let reply = harness.core.list_plugins();
    let plugins = harness.wait(reply, WAIT).expect("plugin list");
    let item = |id: &str, enabled, status, description: Option<&str>, error: Option<&str>| {
        PluginListItem {
            id: id.into(),
            description: description.map(str::to_string),
            enabled,
            status,
            error: error.map(str::to_string),
        }
    };
    assert_eq!(
        plugins,
        vec![
            item(
                "skill-pins",
                true,
                PluginStatus::Running,
                Some("Pins"),
                None
            ),
            item("stop-orb", true, PluginStatus::Unknown, None, None),
            item(
                "edit-diffs",
                false,
                PluginStatus::Failed,
                None,
                Some("boom")
            ),
            item("pr-watch", false, PluginStatus::Disabled, None, None),
        ]
    );
}

#[test]
fn catalog_ids_read_only_the_ids() {
    let server = FakeServer::start();
    server.add_agent("a1", "idle");
    server.reply(
        "plugin.catalog.get.request",
        "plugin.catalog.get.response",
        json!({"plugins": [
            {"id": "skill-pins", "clientBundle": "export default 1"},
            {"id": "stop-orb", "clientBundle": "x", "requirements": {"paseo": ">=1"}},
        ]}),
    );
    let mut harness = connect(&server);
    let reply = harness.core.plugin_catalog_ids();
    assert_eq!(
        harness.wait(reply, WAIT).expect("catalog"),
        vec!["skill-pins".to_string(), "stop-orb".to_string()]
    );
}

#[test]
fn rpc_sends_one_request_and_resolves_to_its_output() {
    let server = FakeServer::start();
    server.add_agent("a1", "idle");
    server.on("plugin.rpc.invoke.request", |request| {
        let output = match request["method"].as_str() {
            Some("pins.list") => json!({"pins": ["a", "b"], "echo": request["input"]}),
            Some("nothing") => Value::Null,
            Some("decode") => json!("not a list"),
            _ => return rpc_error(request, "Unknown plugin RPC method"),
        };
        vec![session(
            "plugin.rpc.invoke.response",
            json!({"requestId": request["requestId"], "output": output}),
        )]
    });
    let mut harness = connect(&server);

    let reply = harness
        .core
        .plugin_rpc::<Value>("skill-pins", "pins.list", json!({"cwd": "/r"}));
    let output = harness.wait(reply, WAIT).expect("output");
    assert_eq!(output, json!({"pins": ["a", "b"], "echo": {"cwd": "/r"}}));
    let requests = server.requests_of("plugin.rpc.invoke.request");
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0]["pluginId"], "skill-pins");
    assert_eq!(requests[0]["method"], "pins.list");
    assert_eq!(requests[0]["input"], json!({"cwd": "/r"}));
    assert!(requests[0]["requestId"].is_string());

    // `output: null` is a value, not an error.
    let reply = harness
        .core
        .plugin_rpc::<Value>("skill-pins", "nothing", json!({}));
    assert_eq!(harness.wait(reply, WAIT).expect("null output"), Value::Null);

    // `rpc_error` carries the daemon text.
    let reply = harness
        .core
        .plugin_rpc::<Value>("skill-pins", "missing", json!({}));
    let error = harness.wait(reply, WAIT).expect_err("rpc error");
    assert_eq!(error.to_string(), "Unknown plugin RPC method");

    // An output that does not fit the type is an error, not a panic.
    let reply = harness
        .core
        .plugin_rpc::<Vec<u32>>("skill-pins", "decode", json!({}));
    let error = harness.wait(reply, WAIT).expect_err("decode error");
    assert!(error.to_string().contains("does not decode"), "{error}");
}

#[test]
fn status_messages_emit_plugin_events_and_the_subscription_survives_a_reconnect() {
    let server = FakeServer::start();
    server.add_agent("a1", "idle");
    let mut harness = connect(&server);
    let names = |request: &Value| -> Vec<String> {
        request["events"]
            .as_array()
            .unwrap()
            .iter()
            .map(|name| name.as_str().unwrap().to_string())
            .collect()
    };
    assert!(harness.pump_until(WAIT, |_| {
        !server
            .requests_of("session.events.set_subscription.request")
            .is_empty()
    }));
    let subscription = server
        .requests_of("session.events.set_subscription.request")
        .remove(0);
    for name in [
        "status.plugin_catalog_changed",
        "status.plugin_settings_changed",
    ] {
        assert!(names(&subscription).contains(&name.to_string()), "{name}");
    }

    let status = |payload: Value| json!({"type": "status", "payload": payload});
    // An unrelated status without a request id emits nothing.
    server.broadcast(status(
        json!({"status": "daemon_config_changed", "config": {}}),
    ));
    server.broadcast(status(
        json!({"status": "plugin_catalog_changed", "pluginId": "skill-pins"}),
    ));
    server.broadcast(status(json!({"status": "plugin_settings_changed",
        "pluginId": "skill-pins", "settingsId": "general"})));
    assert!(harness.pump_until(WAIT, |h| h.plugin_events.len() >= 2));
    harness.pump_for(Duration::from_millis(200));
    assert_eq!(
        harness.plugin_events,
        vec![
            PluginEvent::CatalogChanged {
                plugin_id: "skill-pins".into()
            },
            PluginEvent::SettingsChanged {
                plugin_id: "skill-pins".into(),
                settings_id: "general".into()
            },
        ]
    );

    // After a reconnect the plugin events are asked for again.
    server.drop_connections();
    assert!(
        harness.pump_until(Duration::from_secs(10), |_| server
            .requests_of("session.events.set_subscription.request")
            .len()
            >= 2),
        "subscribes again after the reconnect"
    );
    let again = server
        .requests_of("session.events.set_subscription.request")
        .pop()
        .unwrap();
    assert_eq!(names(&again), names(&subscription));
    harness.plugin_events.clear();
    assert!(harness.pump_until(WAIT, |h| h.core.is_connected()));
    server.broadcast(status(
        json!({"status": "plugin_catalog_changed", "pluginId": "stop-orb"}),
    ));
    assert!(harness.pump_until(WAIT, |h| h.plugin_events
        == vec![PluginEvent::CatalogChanged {
            plugin_id: "stop-orb".into()
        }]));
}

#[test]
fn settings_helpers_call_the_settings_rpcs_and_decode_by_status() {
    let server = FakeServer::start();
    server.add_agent("a1", "idle");
    server.on("plugin.rpc.invoke.request", |request| {
        let input = &request["input"];
        let output = match request["method"].as_str().unwrap_or_default() {
            "settings.ready.read" => {
                json!({"status": "ready", "revision": "r1", "values": {"on": true}})
            }
            "settings.broken.read" => {
                json!({"status": "invalid", "revision": "r2", "error": "bad file"})
            }
            "settings.odd.read" => json!({"status": "later", "revision": "r3"}),
            "settings.general.write" => match input["revision"].as_str() {
                Some("r1") => {
                    json!({"status": "saved", "revision": "r2", "values": input["values"]})
                }
                Some("old") => json!({"status": "conflict", "error": "changed"}),
                _ => json!({"status": "invalid", "error": "values do not match"}),
            },
            "settings.general.reset" => {
                json!({"status": "saved", "revision": "r9", "values": {}})
            }
            _ => json!({"status": "unknown"}),
        };
        vec![session(
            "plugin.rpc.invoke.response",
            json!({"requestId": request["requestId"], "output": output}),
        )]
    });
    let mut harness = connect(&server);

    let reply = harness.core.read_plugin_settings("p", "ready");
    assert_eq!(
        harness.wait(reply, WAIT).unwrap(),
        PluginSettingsRead::Ready {
            revision: "r1".into(),
            values: json!({"on": true})
        }
    );
    let reply = harness.core.read_plugin_settings("p", "broken");
    assert_eq!(
        harness.wait(reply, WAIT).unwrap(),
        PluginSettingsRead::Invalid {
            revision: "r2".into(),
            error: "bad file".into()
        }
    );
    let reply = harness.core.read_plugin_settings("p", "odd");
    assert!(harness.wait(reply, WAIT).is_err(), "unknown read status");

    let reply = harness
        .core
        .write_plugin_settings("p", "general", "r1", json!({"on": false}));
    assert_eq!(
        harness.wait(reply, WAIT).unwrap(),
        PluginSettingsWrite::Saved {
            revision: "r2".into(),
            values: json!({"on": false})
        }
    );
    let reply = harness
        .core
        .write_plugin_settings("p", "general", "old", json!({}));
    assert_eq!(
        harness.wait(reply, WAIT).unwrap(),
        PluginSettingsWrite::Conflict {
            error: "changed".into()
        }
    );
    let reply = harness
        .core
        .write_plugin_settings("p", "general", "r5", json!({}));
    assert_eq!(
        harness.wait(reply, WAIT).unwrap(),
        PluginSettingsWrite::Invalid {
            error: "values do not match".into()
        }
    );
    let reply = harness.core.reset_plugin_settings("p", "general", "r2");
    assert_eq!(
        harness.wait(reply, WAIT).unwrap(),
        PluginSettingsWrite::Saved {
            revision: "r9".into(),
            values: json!({})
        }
    );
    let reply = harness
        .core
        .write_plugin_settings("p", "unknown", "r1", json!({}));
    assert!(harness.wait(reply, WAIT).is_err(), "unknown write status");

    let requests = server.requests_of("plugin.rpc.invoke.request");
    let find = |method: &str| {
        requests
            .iter()
            .find(|r| r["method"] == method)
            .unwrap_or_else(|| panic!("{method} sent"))
            .clone()
    };
    assert_eq!(find("settings.ready.read")["input"], json!({}));
    assert_eq!(find("settings.ready.read")["pluginId"], "p");
    assert_eq!(
        requests
            .iter()
            .find(|r| r["method"] == "settings.general.write")
            .unwrap()["input"],
        json!({"revision": "r1", "values": {"on": false}})
    );
    assert_eq!(
        find("settings.general.reset")["input"],
        json!({"revision": "r2"})
    );
}

#[test]
fn every_plugin_request_fails_while_disconnected() {
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let mut harness = Harness::connect(&format!("ws://127.0.0.1:{port}/ws"), None);
    harness.pump_for(Duration::from_millis(100));
    let not_connected = |result: anyhow::Result<()>| {
        assert_eq!(result.unwrap_err().to_string(), "not connected");
    };
    let reply = harness.core.list_plugins();
    not_connected(harness.wait(reply, WAIT).map(|_| ()));
    let reply = harness.core.plugin_catalog_ids();
    not_connected(harness.wait(reply, WAIT).map(|_| ()));
    let reply = harness.core.plugin_rpc::<Value>("p", "m", json!({}));
    not_connected(harness.wait(reply, WAIT).map(|_| ()));
    let reply = harness.core.read_plugin_settings("p", "s");
    not_connected(harness.wait(reply, WAIT).map(|_| ()));
    let reply = harness.core.write_plugin_settings("p", "s", "r", json!({}));
    not_connected(harness.wait(reply, WAIT).map(|_| ()));
    let reply = harness.core.reset_plugin_settings("p", "s", "r");
    not_connected(harness.wait(reply, WAIT).map(|_| ()));
    let reply = harness.core.set_agent_labels("a1", BTreeMap::new());
    not_connected(harness.wait(reply, WAIT));
    let reply = harness.core.close_agent_runtime("a1");
    not_connected(harness.wait(reply, WAIT));
}

#[test]
fn agent_labels_come_from_the_snapshot() {
    let server = FakeServer::start();
    server.with(|s| {
        let mut labeled = agent_snapshot("a1", "idle");
        labeled["labels"] = json!({"paseo.parent-agent-id": "a0", "team": "ui"});
        s.agents = vec![labeled, agent_snapshot("a2", "idle")];
    });
    let mut harness = connect(&server);
    assert!(harness.pump_until(WAIT, |h| h.core.agents().len() == 2));
    let expected: BTreeMap<String, String> = [
        ("paseo.parent-agent-id".to_string(), "a0".to_string()),
        ("team".to_string(), "ui".to_string()),
    ]
    .into();
    assert_eq!(harness.core.agent_labels("a1"), Some(expected));
    assert_eq!(harness.core.agent_labels("a2"), Some(BTreeMap::new()));
    assert_eq!(harness.core.agent_labels("missing"), None);
}

#[test]
fn labels_and_runtime_close_fail_when_not_accepted() {
    let server = FakeServer::start();
    server.add_agent("a1", "idle");
    server.on("update_agent_request", |request| {
        let accepted = request["agentId"] == "a1";
        vec![session(
            "update_agent_response",
            json!({"requestId": request["requestId"], "agentId": request["agentId"],
                   "accepted": accepted,
                   "error": if accepted { Value::Null } else { json!("Agent not found") }}),
        )]
    });
    server.on("agent.runtime.close.request", |request| {
        vec![session(
            "agent.runtime.close.response",
            json!({"requestId": request["requestId"], "agentId": request["agentId"],
                   "accepted": false, "error": "Agent is running"}),
        )]
    });
    let mut harness = connect(&server);

    let labels: BTreeMap<String, String> = [("idle".to_string(), "1".to_string())].into();
    let reply = harness.core.set_agent_labels("a1", labels.clone());
    harness.wait(reply, WAIT).expect("labels accepted");
    let request = server.requests_of("update_agent_request").remove(0);
    assert_eq!(
        request,
        json!({"type": "update_agent_request", "agentId": "a1", "labels": {"idle": "1"},
               "requestId": request["requestId"]})
    );

    let reply = harness.core.set_agent_labels("gone", labels);
    let error = harness.wait(reply, WAIT).expect_err("refused");
    assert_eq!(error.to_string(), "Agent not found");

    let reply = harness.core.close_agent_runtime("a1");
    let error = harness.wait(reply, WAIT).expect_err("refused");
    assert_eq!(error.to_string(), "Agent is running");
    let request = server.requests_of("agent.runtime.close.request").remove(0);
    assert_eq!(request["agentId"], "a1");
}
