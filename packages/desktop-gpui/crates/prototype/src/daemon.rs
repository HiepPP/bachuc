//! Minimal client for the daemon WebSocket at /ws, run on its own thread.
//! Text frames carry `{type:"session", message}` JSON; binary frames carry
//! terminal data as `[opcode, slot, payload...]`
//! (see packages/protocol/src/binary-frames/terminal.ts).

use serde_json::{Value, json};
use std::io::ErrorKind;
use std::sync::mpsc::{Receiver, Sender, TryRecvError, channel};
use std::time::Duration;
use tungstenite::{Message, stream::MaybeTlsStream};

pub enum Incoming {
    Session(Value),
    Binary(Vec<u8>),
    State(String),
}

pub enum Outgoing {
    Session(Value),
    Binary(Vec<u8>),
}

pub struct Daemon {
    pub tx: Sender<Outgoing>,
    pub rx: Receiver<Incoming>,
}

pub fn spawn(url: String) -> Daemon {
    let (out_tx, out_rx) = channel();
    let (in_tx, in_rx) = channel();
    std::thread::spawn(move || run(url, out_rx, in_tx));
    Daemon {
        tx: out_tx,
        rx: in_rx,
    }
}

fn run(url: String, out_rx: Receiver<Outgoing>, in_tx: Sender<Incoming>) {
    let state = |s: String| {
        let _ = in_tx.send(Incoming::State(s));
    };
    let (mut ws, _) = match tungstenite::connect(url.as_str()) {
        Ok(pair) => pair,
        Err(error) => return state(format!("connect error: {error}")),
    };
    // A short read timeout lets one thread both read and drain outgoing messages.
    if let MaybeTlsStream::Plain(stream) = ws.get_mut() {
        let _ = stream.set_read_timeout(Some(Duration::from_millis(10)));
    }
    let hello = json!({
        "type": "hello",
        "clientId": "bachuc-gpui-proto",
        "clientType": "cli",
        "protocolVersion": 1,
        // No selective-timeline capability: the daemon broadcasts every agent's
        // stream and the UI filters by agentId.
        "capabilities": {"hello_rejection": true, "all_providers": true},
    });
    if let Err(error) = ws.send(Message::text(hello.to_string())) {
        return state(format!("send error: {error}"));
    }
    state("handshaking".into());

    // The daemon closes the socket on session messages sent before it answers
    // hello with server_info, so outgoing messages wait until then.
    let mut ready = false;
    loop {
        match ws.read() {
            Ok(Message::Text(text)) => {
                let Ok(value) = serde_json::from_str::<Value>(&text) else {
                    continue;
                };
                if value["type"] != "session" {
                    continue;
                }
                let message = value["message"].clone();
                if !ready
                    && message["type"] == "status"
                    && message["payload"]["status"] == "server_info"
                {
                    ready = true;
                    state("connected".into());
                }
                let _ = in_tx.send(Incoming::Session(message));
            }
            Ok(Message::Binary(bytes)) => {
                let _ = in_tx.send(Incoming::Binary(bytes.to_vec()));
            }
            Ok(_) => {}
            Err(tungstenite::Error::Io(error))
                if matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {}
            Err(error) => return state(format!("disconnected: {error}")),
        }
        if !ready {
            continue;
        }
        loop {
            let message = match out_rx.try_recv() {
                Ok(Outgoing::Session(message)) => {
                    Message::text(json!({"type": "session", "message": message}).to_string())
                }
                Ok(Outgoing::Binary(bytes)) => Message::binary(bytes),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => return,
            };
            if let Err(error) = ws.send(message) {
                return state(format!("send error: {error}"));
            }
        }
    }
}
