//! The socket worker: one thread per host that connects, sends `hello`, holds session messages
//! until `server_info`, and reconnects with backoff.
//!
//! The thread sleeps in `polling` until the socket is readable, writable after a partial write,
//! or a command arrives, so an idle connection costs no CPU.

use bachuc_protocol::{SessionOutbound, WsInbound, WsOutbound, decode_outbound, encode_inbound};
use polling::{Event, Events, Poller};
use serde_json::{Value, json};
use std::collections::VecDeque;
use std::io::ErrorKind;
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, TryRecvError, channel};
use std::time::{Duration, Instant};
use tungstenite::{Message, WebSocket};

/// What the worker reports, in order.
#[derive(Debug)]
pub enum WorkerEvent {
    /// The socket is open and `hello` is sent. Messages sent from now on with this generation
    /// are held until `Ready`.
    Opened { generation: u64 },
    /// The daemon answered `hello` with `server_info` (the status payload).
    Ready { generation: u64, server_info: Value },
    /// The session `message` of one text frame.
    Message(Value),
    /// One binary frame (`[opcode, slot, payload...]`).
    Binary(Vec<u8>),
    /// The connection ended or could not be made. A rejected `hello` gives its reason, for
    /// example `incorrect_password`.
    Closed { generation: u64, reason: String },
}

/// Settings for one host connection.
#[derive(Clone, Debug)]
pub struct ConnectionConfig {
    pub url: String,
    pub client_id: String,
    pub password: Option<String>,
    pub backoff_min: Duration,
    pub backoff_max: Duration,
}

impl ConnectionConfig {
    pub fn new(
        url: impl Into<String>,
        client_id: impl Into<String>,
        password: Option<String>,
    ) -> Self {
        Self {
            url: url.into(),
            client_id: client_id.into(),
            password,
            backoff_min: Duration::from_millis(500),
            backoff_max: Duration::from_secs(10),
        }
    }
}

/// The capabilities this client sends in `hello`. Names come from
/// `packages/protocol/src/client-capabilities.ts`.
pub const CAPABILITIES: [&str; 9] = [
    "hello_rejection",
    "all_providers",
    "selective_agent_timeline",
    "explicit_event_subscriptions",
    "owned_subscriptions",
    "timeline_replacement_invalidation",
    "reasoning_merge_enum",
    "custom_mode_icons",
    "plugin_timeline_items",
];

/// Builds the `hello` frame.
pub fn hello_frame(client_id: &str, password: Option<&str>) -> String {
    let capabilities: serde_json::Map<String, Value> = CAPABILITIES
        .iter()
        .map(|name| (name.to_string(), json!(true)))
        .collect();
    let mut hello = json!({
        "type": "hello",
        "clientId": client_id,
        "clientType": "cli",
        "protocolVersion": bachuc_protocol::PROTOCOL_VERSION,
        "capabilities": capabilities,
    });
    if let Some(password) = password {
        hello["auth"] = json!({"kind": "password", "password": password});
    }
    let message: WsInbound = serde_json::from_value(hello).expect("hello matches the protocol");
    encode_inbound(&message)
}

/// Wraps a session message in its envelope and checks it against the generated types.
pub fn session_frame(message: Value) -> anyhow::Result<String> {
    let kind = message["type"].as_str().unwrap_or_default().to_string();
    let envelope: WsInbound =
        serde_json::from_value(json!({"type": "session", "message": message}))
            .map_err(|error| anyhow::anyhow!("{kind} does not match the protocol: {error}"))?;
    if let WsInbound::Session { message } = &envelope
        && matches!(message, bachuc_protocol::SessionInbound::Unrecognized)
    {
        anyhow::bail!("{kind} is not a known session message");
    }
    Ok(encode_inbound(&envelope))
}

enum Command {
    Text { generation: u64, text: String },
    Binary { generation: u64, bytes: Vec<u8> },
    Shutdown,
}

/// A handle to the worker thread. Dropping it stops the worker.
pub struct Connection {
    commands: Sender<Command>,
    poller: Arc<Poller>,
}

impl Connection {
    /// Starts the worker. `sink` gets every event and returns false once nobody listens, which
    /// stops the worker.
    pub fn spawn(
        config: ConnectionConfig,
        sink: impl FnMut(WorkerEvent) -> bool + Send + 'static,
    ) -> Self {
        let (commands, receiver) = channel();
        let poller = Arc::new(Poller::new().expect("poller"));
        let worker = Worker {
            config,
            commands: receiver,
            poller: poller.clone(),
            sink: Box::new(sink),
            generation: 0,
        };
        std::thread::Builder::new()
            .name("bachuc-client-socket".into())
            .spawn(move || worker.run())
            .expect("socket thread");
        Self { commands, poller }
    }

    /// Sends a text frame on the connection of `generation`. The worker holds it until
    /// `server_info`, and drops it if that connection is gone.
    pub fn send_text(&self, generation: u64, text: String) {
        self.command(Command::Text { generation, text });
    }

    /// Sends a binary frame on the connection of `generation`.
    pub fn send_binary(&self, generation: u64, bytes: Vec<u8>) {
        self.command(Command::Binary { generation, bytes });
    }

    fn command(&self, command: Command) {
        if self.commands.send(command).is_ok() {
            let _ = self.poller.notify();
        }
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        self.command(Command::Shutdown);
    }
}

enum End {
    Shutdown,
    Closed { reason: String, was_ready: bool },
}

struct Worker {
    config: ConnectionConfig,
    commands: Receiver<Command>,
    poller: Arc<Poller>,
    sink: Box<dyn FnMut(WorkerEvent) -> bool + Send>,
    generation: u64,
}

const SOCKET_KEY: usize = 1;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

impl Worker {
    fn run(mut self) {
        let mut delay = self.config.backoff_min;
        loop {
            self.generation += 1;
            let generation = self.generation;
            let (reason, was_ready) = match self.session(generation) {
                End::Shutdown => return,
                End::Closed { reason, was_ready } => (reason, was_ready),
            };
            if !self.emit(WorkerEvent::Closed { generation, reason }) {
                return;
            }
            if was_ready {
                delay = self.config.backoff_min;
            }
            if !self.wait(delay) {
                return;
            }
            delay = (delay * 2).min(self.config.backoff_max);
        }
    }

    fn emit(&mut self, event: WorkerEvent) -> bool {
        (self.sink)(event)
    }

    /// Sleeps for `delay`, dropping commands for old connections. False on shutdown.
    fn wait(&mut self, delay: Duration) -> bool {
        let deadline = Instant::now() + delay;
        let mut events = Events::new();
        loop {
            loop {
                match self.commands.try_recv() {
                    Ok(Command::Shutdown) | Err(TryRecvError::Disconnected) => return false,
                    Ok(_) => {}
                    Err(TryRecvError::Empty) => break,
                }
            }
            let now = Instant::now();
            if now >= deadline {
                return true;
            }
            events.clear();
            let _ = self.poller.wait(&mut events, Some(deadline - now));
        }
    }

    fn connect(&self) -> Result<WebSocket<TcpStream>, String> {
        let uri: tungstenite::http::Uri = self
            .config
            .url
            .parse()
            .map_err(|error| format!("bad url: {error}"))?;
        let host = uri.host().ok_or("bad url: no host")?;
        let port = uri.port_u16().unwrap_or(80);
        let address = (host, port)
            .to_socket_addrs()
            .map_err(|error| format!("cannot resolve {host}: {error}"))?
            .next()
            .ok_or_else(|| format!("cannot resolve {host}"))?;
        let stream = TcpStream::connect_timeout(&address, CONNECT_TIMEOUT)
            .map_err(|error| format!("connect error: {error}"))?;
        let _ = stream.set_nodelay(true);
        let _ = stream.set_read_timeout(Some(CONNECT_TIMEOUT));
        let _ = stream.set_write_timeout(Some(CONNECT_TIMEOUT));
        let (socket, _) = tungstenite::client(self.config.url.as_str(), stream)
            .map_err(|error| format!("handshake error: {error}"))?;
        socket
            .get_ref()
            .set_nonblocking(true)
            .map_err(|error| format!("socket error: {error}"))?;
        Ok(socket)
    }

    fn session(&mut self, generation: u64) -> End {
        let mut socket = match self.connect() {
            Ok(socket) => socket,
            Err(reason) => {
                return End::Closed {
                    reason,
                    was_ready: false,
                };
            }
        };
        // SAFETY: the socket is removed from the poller before it is dropped, at the end of
        // this function.
        if let Err(error) = unsafe {
            self.poller
                .add(socket.get_ref(), Event::readable(SOCKET_KEY))
        } {
            return End::Closed {
                reason: format!("poll error: {error}"),
                was_ready: false,
            };
        }
        let end = self.drive(generation, &mut socket);
        let _ = self.poller.delete(socket.get_ref());
        if matches!(end, End::Shutdown) {
            let _ = socket.close(None);
            let _ = socket.flush();
        }
        end
    }

    fn drive(&mut self, generation: u64, socket: &mut WebSocket<TcpStream>) -> End {
        if !self.emit(WorkerEvent::Opened { generation }) {
            return End::Shutdown;
        }
        let hello = hello_frame(&self.config.client_id, self.config.password.as_deref());
        let closed = |reason: String, was_ready: bool| End::Closed { reason, was_ready };
        if let Err(reason) = write(socket, Message::text(hello)) {
            return closed(reason, false);
        }
        let mut ready = false;
        let mut held: VecDeque<Message> = VecDeque::new();
        let mut events = Events::new();
        loop {
            // Read everything the socket has.
            loop {
                match socket.read() {
                    Ok(Message::Text(text)) => {
                        if !ready {
                            match handshake_reply(&text) {
                                Handshake::Rejected(reason) => return closed(reason, false),
                                Handshake::Ready(server_info) => {
                                    ready = true;
                                    if !self.emit(WorkerEvent::Ready {
                                        generation,
                                        server_info,
                                    }) {
                                        return End::Shutdown;
                                    }
                                    for message in held.drain(..) {
                                        if let Err(reason) = write(socket, message) {
                                            return closed(reason, true);
                                        }
                                    }
                                    continue;
                                }
                                Handshake::Other => {}
                            }
                        }
                        let Ok(mut value) = serde_json::from_str::<Value>(&text) else {
                            log::warn!("dropping a frame that is not JSON");
                            continue;
                        };
                        if value["type"] == "session"
                            && !self.emit(WorkerEvent::Message(value["message"].take()))
                        {
                            return End::Shutdown;
                        }
                    }
                    Ok(Message::Binary(bytes)) => {
                        if !self.emit(WorkerEvent::Binary(bytes)) {
                            return End::Shutdown;
                        }
                    }
                    Ok(Message::Close(frame)) => {
                        let reason = match frame {
                            Some(frame) if !frame.reason.is_empty() => {
                                format!("closed by host: {}", frame.reason)
                            }
                            _ => "closed by host".to_string(),
                        };
                        return closed(reason, ready);
                    }
                    Ok(_) => {}
                    Err(tungstenite::Error::Io(error)) if error.kind() == ErrorKind::WouldBlock => {
                        break;
                    }
                    Err(error) => return closed(format!("disconnected: {error}"), ready),
                }
            }
            // Take commands.
            loop {
                let (command_generation, message) = match self.commands.try_recv() {
                    Ok(Command::Shutdown) | Err(TryRecvError::Disconnected) => {
                        return End::Shutdown;
                    }
                    Ok(Command::Text { generation, text }) => (generation, Message::text(text)),
                    Ok(Command::Binary { generation, bytes }) => {
                        (generation, Message::binary(bytes))
                    }
                    Err(TryRecvError::Empty) => break,
                };
                if command_generation != generation {
                    continue;
                }
                // The daemon closes the socket on a session message sent before server_info.
                if !ready {
                    held.push_back(message);
                } else if let Err(reason) = write(socket, message) {
                    return closed(reason, ready);
                }
            }
            let writable = match socket.flush() {
                Ok(()) => false,
                Err(tungstenite::Error::Io(error)) if error.kind() == ErrorKind::WouldBlock => true,
                Err(error) => return closed(format!("send error: {error}"), ready),
            };
            let interest = Event::new(SOCKET_KEY, true, writable);
            if let Err(error) = self.poller.modify(socket.get_ref(), interest) {
                return closed(format!("poll error: {error}"), ready);
            }
            events.clear();
            if let Err(error) = self.poller.wait(&mut events, None)
                && error.kind() != ErrorKind::Interrupted
            {
                return closed(format!("poll error: {error}"), ready);
            }
        }
    }
}

/// Queues a frame. A write that would block stays buffered and goes out on the next flush.
fn write(socket: &mut WebSocket<TcpStream>, message: Message) -> Result<(), String> {
    match socket.write(message) {
        Ok(()) => Ok(()),
        Err(tungstenite::Error::Io(error)) if error.kind() == ErrorKind::WouldBlock => Ok(()),
        Err(error) => Err(format!("send error: {error}")),
    }
}

enum Handshake {
    Ready(Value),
    Rejected(String),
    Other,
}

/// Reads the daemon's answer to `hello` through the generated types.
fn handshake_reply(text: &str) -> Handshake {
    match decode_outbound(text) {
        Ok(WsOutbound::HelloRejected { reason, .. }) => Handshake::Rejected(reason.to_string()),
        Ok(WsOutbound::Session {
            message: SessionOutbound::Status { payload },
        }) if payload.status == "server_info" => {
            let mut server_info = Value::Object(payload.extra);
            server_info["status"] = json!("server_info");
            Handshake::Ready(server_info)
        }
        _ => Handshake::Other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hello_carries_the_capabilities_and_password() {
        let hello: Value = serde_json::from_str(&hello_frame("id-1", Some("pw"))).unwrap();
        assert_eq!(hello["type"], "hello");
        assert_eq!(hello["clientId"], "id-1");
        assert_eq!(hello["clientType"], "cli");
        assert_eq!(hello["protocolVersion"], 1);
        assert_eq!(hello["auth"], json!({"kind": "password", "password": "pw"}));
        for name in CAPABILITIES {
            assert_eq!(hello["capabilities"][name], true, "{name}");
        }
        // Without it the daemon withholds plugin items, such as the turn diff card.
        assert!(CAPABILITIES.contains(&"plugin_timeline_items"));
        let without: Value = serde_json::from_str(&hello_frame("id-1", None)).unwrap();
        assert!(without.get("auth").is_none());
    }

    #[test]
    fn unknown_session_messages_are_refused() {
        assert!(session_frame(json!({"type": "no_such_request", "requestId": "1"})).is_err());
        let frame = session_frame(json!({
            "type": "fetch_agents_request",
            "requestId": "r1",
            "subscribe": {},
        }))
        .unwrap();
        let value: Value = serde_json::from_str(&frame).unwrap();
        assert_eq!(value["message"]["type"], "fetch_agents_request");
    }
}
