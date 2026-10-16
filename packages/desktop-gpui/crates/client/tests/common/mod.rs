//! A local fake daemon and a harness that drives `ClientCore` without GPUI.

#![allow(dead_code)]

use bachuc_client::timeline::{LiveRow, merge_entry};
use bachuc_client::{ClientCore, ClientEvent, ConnectionConfig, DirectoryEvent, WorkerEvent};
use futures::channel::oneshot;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::io::ErrorKind;
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc::{Receiver, RecvTimeoutError, channel};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tungstenite::{Message, WebSocket};

// ---- harness ----

pub struct Harness {
    pub core: ClientCore,
    receiver: Receiver<WorkerEvent>,
    pub events: Vec<ClientEvent>,
    pub directory_events: Vec<DirectoryEvent>,
}

impl Harness {
    /// Pumps until the core answers `reply`, and returns the answer.
    pub fn wait<T>(
        &mut self,
        mut reply: oneshot::Receiver<anyhow::Result<T>>,
        timeout: Duration,
    ) -> anyhow::Result<T> {
        let mut result = None;
        self.pump_until(timeout, |_| {
            if let Ok(Some(value)) = reply.try_recv() {
                result = Some(value);
            }
            result.is_some()
        });
        result.unwrap_or_else(|| Err(anyhow::anyhow!("no answer in {timeout:?}")))
    }

    pub fn connect(url: &str, password: Option<&str>) -> Self {
        let client_id = format!("bachuc-client-test-{}", uuid::Uuid::new_v4());
        let config = ConnectionConfig::new(url, client_id, password.map(str::to_string));
        let (sender, receiver) = channel();
        let core = ClientCore::new(config, move |event| sender.send(event).is_ok());
        Self {
            core,
            receiver,
            events: Vec::new(),
            directory_events: Vec::new(),
        }
    }

    fn collect(&mut self) {
        self.events.extend(self.core.take_events());
        self.directory_events
            .extend(self.core.take_directory_events());
        for agent_id in self.core.take_dirty_timelines() {
            self.events.push(ClientEvent::TimelineChanged { agent_id });
        }
    }

    /// Feeds worker events to the core until `done` holds or the timeout passes.
    pub fn pump_until(
        &mut self,
        timeout: Duration,
        mut done: impl FnMut(&mut Harness) -> bool,
    ) -> bool {
        let deadline = Instant::now() + timeout;
        loop {
            self.collect();
            if done(self) {
                return true;
            }
            let now = Instant::now();
            if now >= deadline {
                return false;
            }
            let wait = (deadline - now).min(Duration::from_millis(20));
            match self.receiver.recv_timeout(wait) {
                Ok(event) => {
                    self.core.handle(event);
                    while let Ok(event) = self.receiver.try_recv() {
                        self.core.handle(event);
                    }
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => return false,
            }
        }
    }

    pub fn pump_for(&mut self, duration: Duration) {
        self.pump_until(duration, |_| false);
    }

    pub fn connected(&mut self) -> bool {
        self.pump_until(Duration::from_secs(5), |h| {
            h.core.is_connected() && !h.core.agents().is_empty()
        })
    }

    pub fn entries(&self, agent_id: &str) -> Vec<Value> {
        self.core
            .timeline(agent_id)
            .map(|t| t.entries.clone())
            .unwrap_or_default()
    }
}

// ---- fake daemon ----

#[derive(Clone, Debug)]
pub enum Log {
    Connect(Instant),
    Hello(Value),
    ServerInfoSent,
    Message(Value),
    Binary(Vec<u8>),
    /// A session message before server_info; the daemon closes the socket for it.
    Violation(String),
}

pub struct FakeTimeline {
    pub epoch: String,
    /// Source rows as live entries (one source seq each).
    pub rows: Vec<Value>,
    /// Rows below this seq count as trimmed for the `after` gap check.
    pub min_seq: u64,
}

#[derive(Default)]
struct Conn {
    ready: bool,
    outbox: Vec<Message>,
    kill: bool,
}

/// Answers one request type in place of the built-in reply.
pub type Handler = Box<dyn Fn(&Value) -> Vec<Message> + Send>;

#[derive(Default)]
pub struct State {
    pub handlers: HashMap<String, Handler>,
    pub password: Option<String>,
    pub server_info_delay: Duration,
    pub page_cap: Option<usize>,
    pub log: Vec<Log>,
    pub agents: Vec<Value>,
    pub timelines: HashMap<String, FakeTimeline>,
    conns: Vec<Arc<Mutex<Conn>>>,
    next_subscription: u64,
}

#[derive(Clone)]
pub struct FakeServer {
    pub url: String,
    pub state: Arc<Mutex<State>>,
}

impl FakeServer {
    pub fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("ws://{}/ws", listener.local_addr().unwrap());
        let state = Arc::new(Mutex::new(State::default()));
        let shared = state.clone();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(stream) = stream else { continue };
                let state = shared.clone();
                std::thread::spawn(move || serve(stream, state));
            }
        });
        Self { url, state }
    }

    pub fn with<R>(&self, f: impl FnOnce(&mut State) -> R) -> R {
        f(&mut self.state.lock().unwrap())
    }

    /// Answers every `kind` request with `handler`, which sees the request.
    pub fn on(&self, kind: &str, handler: impl Fn(&Value) -> Vec<Message> + Send + 'static) {
        self.with(|s| s.handlers.insert(kind.to_string(), Box::new(handler)));
    }

    /// Answers every `kind` request with one `reply_kind` message; the request id is added.
    pub fn reply(&self, kind: &str, reply_kind: &str, payload: Value) {
        let reply_kind = reply_kind.to_string();
        self.on(kind, move |request| {
            let mut payload = payload.clone();
            payload["requestId"] = request["requestId"].clone();
            vec![session(&reply_kind, payload)]
        });
    }

    pub fn log(&self) -> Vec<Log> {
        self.with(|s| s.log.clone())
    }

    pub fn messages(&self) -> Vec<Value> {
        self.log()
            .into_iter()
            .filter_map(|entry| match entry {
                Log::Message(message) => Some(message),
                _ => None,
            })
            .collect()
    }

    pub fn requests_of(&self, kind: &str) -> Vec<Value> {
        self.messages()
            .into_iter()
            .filter(|m| m["type"] == kind)
            .collect()
    }

    pub fn connect_times(&self) -> Vec<Instant> {
        self.log()
            .into_iter()
            .filter_map(|entry| match entry {
                Log::Connect(at) => Some(at),
                _ => None,
            })
            .collect()
    }

    pub fn add_agent(&self, id: &str, status: &str) {
        let agent = agent_snapshot(id, status);
        self.with(|s| {
            s.agents.push(agent);
            s.timelines.insert(
                id.to_string(),
                FakeTimeline {
                    epoch: "e1".into(),
                    rows: Vec::new(),
                    min_seq: 1,
                },
            );
        });
    }

    /// Appends a source row; with `push` it is also streamed to connected clients.
    pub fn append(&self, agent_id: &str, item: Value, turn: &str, push: bool) -> u64 {
        let message = self.with(|s| {
            let timeline = s.timelines.get_mut(agent_id).unwrap();
            let seq = timeline.rows.len() as u64 + 1;
            let payload = json!({
                "agentId": agent_id,
                "event": {"type": "timeline", "provider": "mock", "item": item, "turnId": turn},
                "timestamp": format!("2026-10-15T00:00:{seq:02}.000Z"),
                "seq": seq,
                "epoch": timeline.epoch,
            });
            timeline
                .rows
                .push(LiveRow::from_stream(&payload).unwrap().entry);
            json!({"type": "agent_stream", "payload": payload})
        });
        if push {
            self.broadcast(message.clone());
        }
        message["payload"]["seq"].as_u64().unwrap()
    }

    pub fn broadcast(&self, message: Value) {
        let text = json!({"type": "session", "message": message}).to_string();
        self.with(|s| {
            for conn in &s.conns {
                let mut conn = conn.lock().unwrap();
                if conn.ready {
                    conn.outbox.push(Message::text(text.clone()));
                }
            }
        });
    }

    /// Closes every socket without a close frame.
    pub fn drop_connections(&self) {
        self.with(|s| {
            for conn in &s.conns {
                conn.lock().unwrap().kill = true;
            }
        });
    }

    /// The projected entries a fresh fetch would return.
    pub fn projected(&self, agent_id: &str) -> Vec<Value> {
        self.with(|s| project(&s.timelines[agent_id].rows))
    }
}

pub fn agent_snapshot(id: &str, status: &str) -> Value {
    json!({
        "id": id,
        "provider": "mock",
        "cwd": "/tmp/fake",
        "workspaceId": "ws1",
        "model": "m1",
        "status": status,
        "title": format!("agent {id}"),
        "currentModeId": "load-test",
        "availableModes": [{"id": "load-test", "label": "Load Test"}, {"id": "approval-test", "label": "Approval Test"}],
        "pendingPermissions": [],
        "archivedAt": null,
    })
}

pub fn project(rows: &[Value]) -> Vec<Value> {
    let mut entries = Vec::new();
    for row in rows {
        merge_entry(&mut entries, row.clone());
    }
    entries
}

fn serve(stream: TcpStream, state: Arc<Mutex<State>>) {
    state.lock().unwrap().log.push(Log::Connect(Instant::now()));
    let Ok(mut socket) = tungstenite::accept(stream) else {
        return;
    };
    let _ = socket
        .get_ref()
        .set_read_timeout(Some(Duration::from_millis(5)));
    let conn = Arc::new(Mutex::new(Conn::default()));
    state.lock().unwrap().conns.push(conn.clone());
    let mut hello_at: Option<Instant> = None;
    loop {
        if conn.lock().unwrap().kill {
            return; // Dropping the socket closes TCP without a close frame.
        }
        match socket.read() {
            Ok(Message::Text(text)) => {
                let value: Value = serde_json::from_str(&text).unwrap();
                if value["type"] == "hello" {
                    state.lock().unwrap().log.push(Log::Hello(value.clone()));
                    let password = state.lock().unwrap().password.clone();
                    if let Some(password) = password
                        && value["auth"]["password"].as_str() != Some(password.as_str())
                    {
                        let rejected = json!({"type": "hello.rejected", "reason": "incorrect_password", "accepts": ["password"]});
                        let _ = socket.send(Message::text(rejected.to_string()));
                        let _ = socket.close(None);
                        let _ = socket.flush();
                        return;
                    }
                    hello_at = Some(Instant::now());
                } else if value["type"] == "session" {
                    let message = value["message"].clone();
                    if !conn.lock().unwrap().ready {
                        state
                            .lock()
                            .unwrap()
                            .log
                            .push(Log::Violation(message["type"].to_string()));
                        let _ = socket.close(None);
                        let _ = socket.flush();
                        return;
                    }
                    state
                        .lock()
                        .unwrap()
                        .log
                        .push(Log::Message(message.clone()));
                    let replies = respond(&mut state.lock().unwrap(), &message);
                    for reply in replies {
                        let _ = socket.write(reply);
                    }
                }
            }
            Ok(Message::Binary(bytes)) => {
                state.lock().unwrap().log.push(Log::Binary(bytes));
            }
            Ok(Message::Close(_)) => return,
            Ok(_) => {}
            Err(tungstenite::Error::Io(error))
                if matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {}
            Err(_) => return,
        }
        if let Some(at) = hello_at {
            let delay = state.lock().unwrap().server_info_delay;
            if !conn.lock().unwrap().ready && at.elapsed() >= delay {
                let info = json!({"type": "session", "message": {"type": "status", "payload": {
                    "status": "server_info", "serverId": "fake", "version": "test",
                    "features": {"ownedSubscriptions": true, "selectiveAgentTimeline": true,
                        "agentMessageQueue": true}}}});
                let _ = socket.write(Message::text(info.to_string()));
                state.lock().unwrap().log.push(Log::ServerInfoSent);
                conn.lock().unwrap().ready = true;
            }
        }
        let outbox = std::mem::take(&mut conn.lock().unwrap().outbox);
        for message in outbox {
            let _ = socket.write(message);
        }
        if flush(&mut socket).is_err() {
            return;
        }
    }
}

fn flush(socket: &mut WebSocket<TcpStream>) -> Result<(), ()> {
    match socket.flush() {
        Ok(()) => Ok(()),
        Err(tungstenite::Error::Io(error))
            if matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) =>
        {
            Ok(())
        }
        Err(_) => Err(()),
    }
}

pub fn session(kind: &str, payload: Value) -> Message {
    Message::text(
        json!({"type": "session", "message": {"type": kind, "payload": payload}}).to_string(),
    )
}

fn respond(state: &mut State, message: &Value) -> Vec<Message> {
    let request_id = message["requestId"].clone();
    let page_info = json!({"nextCursor": null, "prevCursor": null, "hasMore": false});
    state.next_subscription += 1;
    let subscription_id = format!("sub-{}", state.next_subscription);
    if let Some(handler) = state
        .handlers
        .get(message["type"].as_str().unwrap_or_default())
    {
        return handler(message);
    }
    match message["type"].as_str().unwrap_or_default() {
        "fetch_agents_request" => {
            let entries: Vec<Value> = state.agents.iter().map(|a| json!({"agent": a})).collect();
            vec![session(
                "fetch_agents_response",
                json!({"requestId": request_id, "subscriptionId": subscription_id, "entries": entries, "pageInfo": page_info}),
            )]
        }
        "fetch_workspaces_request" => vec![session(
            "fetch_workspaces_response",
            json!({"requestId": request_id, "subscriptionId": subscription_id, "entries": [
                {"id": "ws1", "name": "fake", "projectRootPath": "/tmp/fake", "workspaceDirectory": "/tmp/fake"}
            ], "pageInfo": page_info}),
        )],
        "list_available_providers_request" => vec![session(
            "list_available_providers_response",
            json!({"requestId": request_id, "fetchedAt": "now", "providers": [
                {"provider": "mock", "available": true}, {"provider": "pi", "available": false}
            ]}),
        )],
        "list_provider_models_request" => vec![session(
            "list_provider_models_response",
            json!({"requestId": request_id, "fetchedAt": "now", "provider": message["provider"], "models": [
                {"provider": "mock", "id": "m1", "label": "Model 1"},
                {"provider": "mock", "id": "hidden", "label": "Hidden", "isSelectable": false}
            ]}),
        )],
        "session.events.set_subscription.request" => vec![session(
            "session.events.set_subscription.response",
            json!({"requestId": request_id, "subscriptionId": subscription_id}),
        )],
        "agent.timeline.set_subscription.request" => vec![session(
            "agent.timeline.set_subscription.response",
            json!({"requestId": request_id, "agentIds": message["agentIds"], "subscriptionId": subscription_id}),
        )],
        "subscription.release.request" => vec![session(
            "subscription.release.response",
            json!({"requestId": request_id, "subscriptionId": message["subscriptionId"]}),
        )],
        "fetch_agent_timeline_request" => {
            let agent_id = message["agentId"].as_str().unwrap_or_default();
            let Some(timeline) = state.timelines.get(agent_id) else {
                return vec![];
            };
            let mut payload = page(timeline, message, state.page_cap);
            payload["requestId"] = request_id;
            payload["agentId"] = json!(agent_id);
            vec![session("fetch_agent_timeline_response", payload)]
        }
        "create_terminal_request" => vec![session(
            "create_terminal_response",
            json!({"requestId": request_id, "error": null, "terminal": {"id": "term-1", "name": "t", "cwd": message["cwd"]}}),
        )],
        "subscribe_terminal_request" => {
            let snapshot = json!({"rows": 2, "cols": 4, "grid": [], "scrollback": [], "cursor": {"row": 0, "col": 0}});
            let mut snapshot_frame = vec![0x04, 7];
            snapshot_frame.extend_from_slice(snapshot.to_string().as_bytes());
            vec![
                session(
                    "subscribe_terminal_response",
                    json!({"requestId": request_id, "terminalId": message["terminalId"], "slot": 7, "error": null}),
                ),
                Message::binary(snapshot_frame),
                Message::binary(vec![0x01, 7, b'o', b'k']),
            ]
        }
        _ => vec![],
    }
}

/// A projected page, following `selectProjectedTimelinePage` in the daemon.
fn page(timeline: &FakeTimeline, request: &Value, cap: Option<usize>) -> Value {
    let all = project(&timeline.rows);
    let max_seq = timeline.rows.len() as u64;
    let min_seq = timeline.min_seq;
    let limit = request["limit"].as_u64().unwrap_or(0) as usize;
    let limit = cap.map_or(limit, |cap| if limit == 0 { cap } else { limit.min(cap) });
    let direction = request["direction"].as_str().unwrap_or("tail");
    let cursor = &request["cursor"];
    let stale = !cursor.is_null() && cursor["epoch"].as_str() != Some(timeline.epoch.as_str());
    let gap = !stale
        && direction == "after"
        && cursor["seq"].as_u64().is_some_and(|seq| seq + 1 < min_seq)
        && !all.is_empty();
    let reset = stale || gap;
    let cursor_of = |seq: Option<u64>| match seq {
        Some(seq) => json!({"epoch": timeline.epoch, "seq": seq}),
        None => Value::Null,
    };
    let start = |e: &Value| e["seqStart"].as_u64().unwrap();
    let (entries, start_seq, end_seq, has_older, has_newer): (
        Vec<Value>,
        Option<u64>,
        Option<u64>,
        bool,
        bool,
    ) = if reset || direction == "tail" {
        let mut first = if limit == 0 {
            0
        } else {
            all.len().saturating_sub(limit)
        };
        for i in (0..first).rev() {
            if all[i]["seqEnd"].as_u64().unwrap() >= start(&all[first]) {
                first = i;
            }
        }
        let selected = all[first..].to_vec();
        let start_seq = selected.first().map(start);
        let end_seq = if selected.is_empty() {
            None
        } else {
            Some(max_seq)
        };
        (selected, start_seq, end_seq, first > 0, false)
    } else if direction == "after" {
        let from = cursor["seq"].as_u64().unwrap_or(0) + 1;
        let mut eligible: Vec<(u64, usize)> = all
            .iter()
            .enumerate()
            .filter_map(|(index, entry)| {
                entry["sourceSeqRanges"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter_map(|r| {
                        let (s, e) = (
                            r["startSeq"].as_u64().unwrap(),
                            r["endSeq"].as_u64().unwrap(),
                        );
                        (e >= from && s <= max_seq).then(|| s.max(from))
                    })
                    .min()
                    .map(|first| (first, index))
            })
            .collect();
        eligible.sort();
        if limit > 0 {
            eligible.truncate(limit);
        }
        eligible.sort_by_key(|(_, index)| *index);
        let selected: Vec<Value> = eligible.iter().map(|(_, i)| all[*i].clone()).collect();
        let mut ranges: Vec<(u64, u64)> = selected
            .iter()
            .flat_map(|e| e["sourceSeqRanges"].as_array().unwrap().clone())
            .map(|r| {
                (
                    r["startSeq"].as_u64().unwrap(),
                    r["endSeq"].as_u64().unwrap(),
                )
            })
            .collect();
        ranges.sort();
        let mut end = from - 1;
        for (s, e) in ranges {
            if e <= end {
                continue;
            }
            if s > end + 1 {
                break;
            }
            end = e.min(max_seq);
        }
        let end_seq = (end >= from && !selected.is_empty()).then_some(end);
        let has_newer = end_seq.is_some_and(|end| end < max_seq);
        (
            selected,
            end_seq.map(|_| from),
            end_seq,
            from > min_seq,
            has_newer,
        )
    } else {
        let before = cursor["seq"].as_u64().unwrap_or(max_seq + 1);
        let eligible: Vec<Value> = all.iter().filter(|e| start(e) < before).cloned().collect();
        let skip = if limit == 0 {
            0
        } else {
            eligible.len().saturating_sub(limit)
        };
        let selected = eligible[skip..].to_vec();
        let start_seq = selected.first().map(start);
        (
            selected,
            start_seq,
            start_seq.map(|_| before - 1),
            skip > 0,
            true,
        )
    };
    json!({
        "direction": direction,
        "projection": "projected",
        "epoch": timeline.epoch,
        "reset": reset,
        "staleCursor": stale,
        "gap": gap,
        "window": {"minSeq": min_seq, "maxSeq": max_seq, "nextSeq": max_seq + 1},
        "startCursor": cursor_of(start_seq),
        "endCursor": cursor_of(end_seq),
        "hasOlder": has_older,
        "hasNewer": has_newer,
        "entries": entries,
        "error": null,
    })
}
