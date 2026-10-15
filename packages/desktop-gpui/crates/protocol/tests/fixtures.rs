//! Decodes frames recorded from the live daemon on 2026-10-15 (`tests/fixtures/*.json`).

use bachuc_protocol::generated::{
    AgentStreamPayloadEvent, WsInboundHelloCapabilities, WsInboundHelloClientType,
};
use bachuc_protocol::{
    PROTOCOL_VERSION, SessionOutbound, WsInbound, WsOutbound, decode_outbound, encode_inbound,
};
use serde_json::{Value, json};

/// Decodes a frame, checks that no level fell back to `Unrecognized`, and checks that encoding
/// the decoded value again gives the same JSON, so every recorded field has a typed home.
fn decode_fixture(text: &str) -> SessionOutbound {
    let decoded = decode_outbound(text).expect("fixture decodes");
    let encoded = serde_json::to_value(&decoded).expect("decoded value encodes");
    let original: Value = serde_json::from_str(text).unwrap();
    if let Some(difference) = first_difference(&normalize(original), &normalize(encoded), "") {
        panic!("round trip changed the frame at {difference}");
    }
    match decoded {
        WsOutbound::Session { message } => {
            assert!(!matches!(message, SessionOutbound::Unrecognized));
            message
        }
        other => panic!("expected a session frame, got {other:?}"),
    }
}

/// Drops nulls, empty arrays, and empty maps, which decode to `None` or an empty default and are
/// not written back, and compares numbers as f64, because a schema `number` decodes to f64.
fn normalize(value: Value) -> Value {
    let empty = |v: &Value| match v {
        Value::Null => true,
        Value::Array(a) => a.is_empty(),
        Value::Object(o) => o.is_empty(),
        _ => false,
    };
    match value {
        Value::Object(map) => Value::Object(
            map.into_iter()
                .filter(|(_, v)| !empty(v))
                .map(|(k, v)| (k, normalize(v)))
                .collect(),
        ),
        Value::Array(items) => Value::Array(items.into_iter().map(normalize).collect()),
        Value::Number(n) => json!(n.as_f64().unwrap()),
        other => other,
    }
}

fn first_difference(original: &Value, encoded: &Value, path: &str) -> Option<String> {
    match (original, encoded) {
        (Value::Object(a), Value::Object(b)) => {
            a.keys()
                .chain(b.keys())
                .find_map(|key| match (a.get(key), b.get(key)) {
                    (Some(x), Some(y)) => first_difference(x, y, &format!("{path}/{key}")),
                    (x, y) => Some(format!("{path}/{key}: {x:?} became {y:?}")),
                })
        }
        (Value::Array(a), Value::Array(b)) if a.len() == b.len() => a
            .iter()
            .zip(b)
            .enumerate()
            .find_map(|(i, (x, y))| first_difference(x, y, &format!("{path}/{i}"))),
        (a, b) if a == b => None,
        (a, b) => Some(format!("{path}: {a} became {b}")),
    }
}

macro_rules! fixture {
    ($file:literal) => {
        include_str!(concat!("fixtures/", $file, ".json"))
    };
}

#[test]
fn server_info() {
    let message = decode_fixture(fixture!("server_info"));
    let SessionOutbound::Status { payload } = message else {
        panic!("expected status, got {message:?}");
    };
    assert_eq!(payload.status, "server_info");
}

#[test]
fn fetch_agents_response() {
    let message = decode_fixture(fixture!("fetch_agents_response"));
    let SessionOutbound::FetchAgentsResponse { payload } = message else {
        panic!("expected fetch_agents_response, got {message:?}");
    };
    assert_eq!(payload.entries.len(), 1);
}

#[test]
fn fetch_agent_timeline_response() {
    let message = decode_fixture(fixture!("fetch_agent_timeline_response"));
    let SessionOutbound::FetchAgentTimelineResponse { payload } = message else {
        panic!("expected fetch_agent_timeline_response, got {message:?}");
    };
    assert!(!payload.entries.is_empty());
}

fn decode_stream(text: &str) -> AgentStreamPayloadEvent {
    let message = decode_fixture(text);
    let SessionOutbound::AgentStream { payload } = message else {
        panic!("expected agent_stream, got {message:?}");
    };
    assert!(!matches!(
        payload.event,
        AgentStreamPayloadEvent::Unrecognized
    ));
    payload.event
}

#[test]
fn agent_stream_timeline_user_message() {
    decode_stream(fixture!("agent_stream_timeline_user_message"));
}

#[test]
fn agent_stream_timeline_assistant_message() {
    decode_stream(fixture!("agent_stream_timeline_assistant_message"));
}

#[test]
fn agent_stream_timeline_reasoning() {
    decode_stream(fixture!("agent_stream_timeline_reasoning"));
}

#[test]
fn agent_stream_timeline_tool_call() {
    decode_stream(fixture!("agent_stream_timeline_tool_call"));
}

#[test]
fn agent_stream_timeline_todo() {
    decode_stream(fixture!("agent_stream_timeline_todo"));
}

#[test]
fn agent_stream_turn_started() {
    decode_stream(fixture!("agent_stream_turn_started"));
}

#[test]
fn agent_stream_turn_completed() {
    decode_stream(fixture!("agent_stream_turn_completed"));
}

#[test]
fn agent_stream_permission_requested() {
    decode_stream(fixture!("agent_stream_permission_requested"));
}

#[test]
fn agent_permission_request() {
    let message = decode_fixture(fixture!("agent_permission_request"));
    assert!(matches!(
        message,
        SessionOutbound::AgentPermissionRequest { .. }
    ));
}

#[test]
fn create_terminal_response() {
    let message = decode_fixture(fixture!("create_terminal_response"));
    let SessionOutbound::CreateTerminalResponse { payload } = message else {
        panic!("expected create_terminal_response, got {message:?}");
    };
    assert!(payload.terminal.is_some());
}

#[test]
fn unknown_fields_are_ignored() {
    let mut frame: Value =
        serde_json::from_str(fixture!("agent_stream_timeline_tool_call")).unwrap();
    frame["futureEnvelopeField"] = json!(1);
    frame["message"]["futureMessageField"] = json!("x");
    frame["message"]["payload"]["futurePayloadField"] = json!({"nested": true});
    frame["message"]["payload"]["event"]["item"]["futureItemField"] = json!([1, 2]);
    let decoded = decode_outbound(&frame.to_string()).expect("frame with new fields decodes");
    let WsOutbound::Session {
        message: SessionOutbound::AgentStream { payload },
    } = decoded
    else {
        panic!("expected agent_stream, got {decoded:?}");
    };
    assert!(matches!(
        payload.event,
        AgentStreamPayloadEvent::Timeline { .. }
    ));
}

#[test]
fn unknown_message_type_decodes_to_the_fallback() {
    let frame = json!({
        "type": "session",
        "message": {"type": "future.message.response", "payload": {"requestId": "r1"}},
    });
    let decoded = decode_outbound(&frame.to_string()).expect("unknown session message decodes");
    assert!(matches!(
        decoded,
        WsOutbound::Session {
            message: SessionOutbound::Unrecognized
        }
    ));

    let decoded = decode_outbound(r#"{"type":"future_envelope","data":1}"#)
        .expect("unknown envelope decodes");
    assert!(matches!(decoded, WsOutbound::Unrecognized));
}

#[test]
fn unknown_stream_event_decodes_to_the_fallback() {
    let mut frame: Value = serde_json::from_str(fixture!("agent_stream_turn_started")).unwrap();
    frame["message"]["payload"]["event"] = json!({"type": "future_event", "x": 1});
    let decoded = decode_outbound(&frame.to_string()).expect("unknown stream event decodes");
    let WsOutbound::Session {
        message: SessionOutbound::AgentStream { payload },
    } = decoded
    else {
        panic!("expected agent_stream, got {decoded:?}");
    };
    assert!(matches!(
        payload.event,
        AgentStreamPayloadEvent::Unrecognized
    ));
}

fn hello() -> WsInbound {
    WsInbound::Hello {
        app_version: None,
        auth: None,
        capabilities: Some(WsInboundHelloCapabilities {
            hello_rejection: Some(true),
            ..serde_json::from_value(json!({})).unwrap()
        }),
        client_id: "bachuc-protocol-test".into(),
        client_type: WsInboundHelloClientType::Cli,
        protocol_version: PROTOCOL_VERSION.into(),
    }
}

#[test]
fn hello_encodes_the_required_fields() {
    let encoded: Value = serde_json::from_str(&encode_inbound(&hello())).unwrap();
    assert_eq!(encoded["type"], "hello");
    assert_eq!(encoded["clientId"], "bachuc-protocol-test");
    assert_eq!(encoded["clientType"], "cli");
    assert_eq!(encoded["protocolVersion"], 1);
    assert_eq!(encoded["capabilities"], json!({"hello_rejection": true}));
}

/// Needs the live daemon: `cargo.sh test -p bachuc-protocol --test fixtures -- --ignored live_hello`.
#[test]
#[ignore]
fn live_hello() {
    use tungstenite::Message;
    let (mut socket, _) = tungstenite::connect("ws://127.0.0.1:6768/ws").expect("live daemon");
    socket
        .send(Message::text(encode_inbound(&hello())))
        .unwrap();
    loop {
        let Message::Text(text) = socket.read().expect("frame from live") else {
            continue;
        };
        let decoded = decode_outbound(&text).expect("live frame decodes");
        if let WsOutbound::Session {
            message: SessionOutbound::Status { payload },
        } = &decoded
        {
            if payload.status == "server_info" {
                break;
            }
        }
        assert!(
            !matches!(decoded, WsOutbound::HelloRejected { .. }),
            "hello rejected: {decoded:?}"
        );
    }
    let _ = socket.close(None);
}
