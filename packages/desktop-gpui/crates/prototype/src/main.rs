mod daemon;
mod terminal;

use daemon::{Daemon, Incoming, Outgoing};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::text::{TextView, TextViewState};
use gpui_kit::component::{ActiveTheme as _, h_flex, v_flex};
use gpui_kit::*;
use gpui_wry::WebView;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use terminal::{COLS, ROWS, TerminalState, key_bytes};

const SAMPLE_MERMAID: &str = "flowchart LR\n  App[GPUI client] -->|WebSocket /ws| Daemon\n  Daemon --> Agents\n  Daemon -->|binary frames| Terminal[alacritty_terminal]";

#[derive(Clone)]
struct AgentSummary {
    id: String,
    title: String,
    status: String,
    cwd: String,
}

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    User,
    Assistant,
    Reasoning,
    Tool,
    Other,
}

struct Row {
    kind: Kind,
    /// messageId for assistant text, callId for tool calls.
    key: Option<String>,
    text: String,
    markdown: Option<Entity<TextViewState>>,
}

enum Pending {
    Agents,
    Timeline(String),
    CreateTerminal,
    SubscribeTerminal,
    Ignore,
}

struct BachucApp {
    daemon: Daemon,
    connection_state: String,
    agents: Vec<AgentSummary>,
    selected: Option<String>,
    auto_select: Option<String>,
    rows: Vec<Row>,
    list_state: ListState,
    /// Stream items wait here and reach the UI in one update per 50 ms.
    buffer: Vec<Value>,
    pending: HashMap<String, Pending>,
    next_request: u64,
    input: Entity<InputState>,
    terminal: TerminalState,
    terminal_focus: FocusHandle,
    webview: Entity<WebView>,
    mermaid: String,
    last_flush: Instant,
    stats: String,
    stat_events: u32,
    stat_flushes: u32,
    stat_started: Instant,
    _tick: Task<()>,
}

impl BachucApp {
    fn view(
        window: &mut Window,
        cx: &mut App,
        url: String,
        auto_select: Option<String>,
    ) -> Entity<Self> {
        let input = cx.new(|cx| InputState::new(window, cx).placeholder("Message"));
        let webview = cx.new(|cx| {
            use raw_window_handle::HasWindowHandle;
            let handle = window.window_handle().expect("window handle");
            let view = wry::WebViewBuilder::new()
                .with_initialization_script(mermaid_init_script())
                .with_html(format!(
                    "<html><body style='background:#1e1e1e;margin:12px'><div id='d'></div>\
                     <script>renderMermaid({})</script></body></html>",
                    json!(SAMPLE_MERMAID)
                ))
                .build_as_child(&handle)
                .expect("webview");
            WebView::new(view, window, cx)
        });

        cx.new(|cx| {
            cx.subscribe_in(
                &input,
                window,
                |this: &mut Self, input, event: &InputEvent, window, cx| {
                    if let InputEvent::PressEnter { .. } = event {
                        let text = input.read(cx).value().to_string();
                        this.send_prompt(text);
                        input.update(cx, |state, cx| state.set_value("", window, cx));
                    }
                },
            )
            .detach();

            // One 16 ms tick drains the socket: terminal bytes render on the next
            // frame, chat items are flushed at most every 50 ms.
            let tick = cx.spawn(async move |this, cx| {
                loop {
                    cx.background_executor()
                        .timer(Duration::from_millis(16))
                        .await;
                    if this.update(cx, |this, cx| this.tick(cx)).is_err() {
                        break;
                    }
                }
            });

            let mut this = Self {
                daemon: daemon::spawn(url),
                connection_state: "connecting".into(),
                agents: Vec::new(),
                selected: None,
                auto_select,
                rows: Vec::new(),
                list_state: ListState::new(0, ListAlignment::Bottom, px(400.)),
                buffer: Vec::new(),
                pending: HashMap::new(),
                next_request: 0,
                input,
                terminal: TerminalState::new(),
                terminal_focus: cx.focus_handle(),
                webview,
                mermaid: SAMPLE_MERMAID.into(),
                last_flush: Instant::now(),
                stats: String::new(),
                stat_events: 0,
                stat_flushes: 0,
                stat_started: Instant::now(),
                _tick: tick,
            };
            this.request(
                json!({
                    "type": "fetch_agents_request",
                    "sort": [{"key": "updated_at", "direction": "desc"}],
                    "page": {"limit": 50},
                }),
                Pending::Agents,
            );
            this
        })
    }

    fn request(&mut self, mut message: Value, kind: Pending) {
        self.next_request += 1;
        let id = format!("gpui-{}", self.next_request);
        message["requestId"] = json!(id);
        self.pending.insert(id, kind);
        let _ = self.daemon.tx.send(Outgoing::Session(message));
    }

    fn send_terminal_frame(&self, opcode: u8, payload: &[u8]) {
        let Some(slot) = self.terminal.slot else {
            return;
        };
        let mut frame = vec![opcode, slot];
        frame.extend_from_slice(payload);
        let _ = self.daemon.tx.send(Outgoing::Binary(frame));
    }

    fn send_prompt(&mut self, text: String) {
        let Some(agent_id) = self.selected.clone() else {
            return;
        };
        if text.is_empty() {
            return;
        }
        self.request(
            json!({"type": "send_agent_message_request", "agentId": agent_id, "text": text}),
            Pending::Ignore,
        );
    }

    fn select(&mut self, agent: AgentSummary, cx: &mut Context<Self>) {
        self.selected = Some(agent.id.clone());
        self.rows.clear();
        self.buffer.clear();
        self.list_state.reset(0);
        self.request(
            json!({
                "type": "fetch_agent_timeline_request",
                "agentId": agent.id,
                "direction": "tail",
                "limit": 200,
                "projection": "projected",
            }),
            Pending::Timeline(agent.id.clone()),
        );
        if self.terminal.id.is_none() {
            self.request(
                json!({
                    "type": "create_terminal_request",
                    "cwd": agent.cwd,
                    "name": "bachuc-gpui-proto",
                    "size": {"rows": ROWS, "cols": COLS},
                }),
                Pending::CreateTerminal,
            );
        }
        cx.notify();
    }

    fn tick(&mut self, cx: &mut Context<Self>) {
        let mut dirty = false;
        while let Ok(incoming) = self.daemon.rx.try_recv() {
            match incoming {
                Incoming::State(state) => {
                    self.connection_state = state;
                    dirty = true;
                }
                Incoming::Binary(bytes) => {
                    if bytes.len() >= 2 && bytes[0] == 0x01 && Some(bytes[1]) == self.terminal.slot
                    {
                        self.terminal.feed(&bytes[2..]);
                        dirty = true;
                    }
                }
                Incoming::Session(message) => self.handle_session(message, cx),
            }
        }
        let replies = self.terminal.take_replies();
        if !replies.is_empty() {
            self.send_terminal_frame(0x02, &replies);
        }
        if !self.buffer.is_empty() && self.last_flush.elapsed() >= Duration::from_millis(50) {
            self.flush(cx);
            dirty = true;
        }
        let elapsed = self.stat_started.elapsed().as_secs_f64();
        if elapsed >= 1.0 {
            self.stats = format!(
                "{:.0} events/s · {:.0} UI updates/s · {} rows",
                self.stat_events as f64 / elapsed,
                self.stat_flushes as f64 / elapsed,
                self.rows.len()
            );
            self.stat_events = 0;
            self.stat_flushes = 0;
            self.stat_started = Instant::now();
            dirty = true;
        }
        if dirty {
            cx.notify();
        }
    }

    fn handle_session(&mut self, message: Value, cx: &mut Context<Self>) {
        let payload = &message["payload"];
        if let Some(kind) = payload["requestId"]
            .as_str()
            .and_then(|id| self.pending.remove(id))
        {
            self.handle_response(kind, payload, cx);
            return;
        }
        if message["type"] == "agent_stream"
            && payload["agentId"].as_str() == self.selected.as_deref()
            && payload["event"]["type"] == "timeline"
        {
            self.buffer.push(payload["event"]["item"].clone());
            self.stat_events += 1;
        }
    }

    fn handle_response(&mut self, kind: Pending, payload: &Value, cx: &mut Context<Self>) {
        match kind {
            Pending::Agents => {
                self.agents = payload["entries"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|entry| {
                        let agent = &entry["agent"];
                        let id = agent["id"].as_str()?.to_string();
                        Some(AgentSummary {
                            title: agent["title"]
                                .as_str()
                                .map(str::to_string)
                                .unwrap_or_else(|| id[..8].to_string()),
                            status: agent["status"].as_str().unwrap_or("?").to_string(),
                            cwd: agent["cwd"].as_str().unwrap_or("/tmp").to_string(),
                            id,
                        })
                    })
                    .collect();
                if let Some(wanted) = self.auto_select.take() {
                    if let Some(agent) = self
                        .agents
                        .iter()
                        .find(|a| a.id.starts_with(&wanted) || a.title == wanted)
                    {
                        self.select(agent.clone(), cx);
                    }
                }
            }
            Pending::Timeline(agent_id) => {
                if self.selected.as_deref() != Some(&agent_id) {
                    return;
                }
                for entry in payload["entries"].as_array().into_iter().flatten() {
                    self.apply(&entry["item"], cx);
                }
                self.list_state.reset(self.rows.len());
                self.list_state.scroll_to_end();
            }
            Pending::CreateTerminal => {
                if let Some(id) = payload["terminal"]["id"].as_str() {
                    self.terminal.id = Some(id.to_string());
                    self.request(
                        json!({
                            "type": "subscribe_terminal_request",
                            "terminalId": id,
                            "restore": {"mode": "live", "size": {"rows": ROWS, "cols": COLS}},
                        }),
                        Pending::SubscribeTerminal,
                    );
                }
            }
            Pending::SubscribeTerminal => {
                if let Some(slot) = payload["slot"].as_u64() {
                    self.terminal.slot = Some(slot as u8);
                    let size = json!({"rows": ROWS, "cols": COLS, "intent": "claim"}).to_string();
                    self.send_terminal_frame(0x03, size.as_bytes());
                }
            }
            Pending::Ignore => {}
        }
    }

    fn flush(&mut self, cx: &mut Context<Self>) {
        let old_count = self.rows.len();
        let mut first_dirty = old_count;
        for item in std::mem::take(&mut self.buffer) {
            if let Some(index) = self.apply(&item, cx) {
                first_dirty = first_dirty.min(index);
            }
        }
        self.list_state
            .splice(first_dirty..old_count, self.rows.len() - first_dirty);
        self.list_state.scroll_to_end();
        self.last_flush = Instant::now();
        self.stat_flushes += 1;
        self.update_mermaid(cx);
    }

    /// Applies one timeline item; returns the index of the row it changed or added.
    fn apply(&mut self, item: &Value, cx: &mut Context<Self>) -> Option<usize> {
        let text = item["text"].as_str().unwrap_or("").to_string();
        match item["type"].as_str().unwrap_or("") {
            "user_message" => Some(self.push(Kind::User, item["messageId"].as_str(), text, cx)),
            "assistant_message" => {
                let key = item["messageId"].as_str();
                if let Some(last) = self.rows.last_mut() {
                    if last.kind == Kind::Assistant && last.key.as_deref() == key {
                        last.text.push_str(&text);
                        if let Some(markdown) = &last.markdown {
                            markdown.update(cx, |state, cx| state.push_str(&text, cx));
                        }
                        return Some(self.rows.len() - 1);
                    }
                }
                Some(self.push(Kind::Assistant, key, text, cx))
            }
            "reasoning" => {
                if let Some(last) = self.rows.last_mut() {
                    if last.kind == Kind::Reasoning {
                        last.text.push_str(&text);
                        return Some(self.rows.len() - 1);
                    }
                }
                Some(self.push(Kind::Reasoning, None, text, cx))
            }
            "tool_call" => {
                let key = item["callId"].as_str();
                let detail = &item["detail"];
                let target = ["filePath", "command", "query"]
                    .iter()
                    .find_map(|field| detail[*field].as_str())
                    .unwrap_or("");
                let summary = format!(
                    "{} {} · {}",
                    item["name"].as_str().unwrap_or("tool"),
                    target,
                    item["status"].as_str().unwrap_or("")
                );
                if let Some(index) = self
                    .rows
                    .iter()
                    .rposition(|r| r.kind == Kind::Tool && r.key.as_deref() == key)
                {
                    self.rows[index].text = summary;
                    return Some(index);
                }
                Some(self.push(Kind::Tool, key, summary, cx))
            }
            other => Some(self.push(Kind::Other, None, other.to_string(), cx)),
        }
    }

    fn push(
        &mut self,
        kind: Kind,
        key: Option<&str>,
        text: String,
        cx: &mut Context<Self>,
    ) -> usize {
        let markdown =
            (kind == Kind::Assistant).then(|| cx.new(|cx| TextViewState::markdown(&text, cx)));
        self.rows.push(Row {
            kind,
            key: key.map(str::to_string),
            text,
            markdown,
        });
        self.rows.len() - 1
    }

    fn update_mermaid(&mut self, cx: &App) {
        let Some(row) = self.rows.iter().rev().find(|r| r.kind == Kind::Assistant) else {
            return;
        };
        let Some(start) = row.text.rfind("```mermaid\n") else {
            return;
        };
        let body = &row.text[start + 11..];
        let Some(end) = body.find("\n```") else {
            return;
        };
        let source = &body[..end];
        if source != self.mermaid {
            self.mermaid = source.to_string();
            let script = format!("renderMermaid({})", json!(source));
            let _ = self.webview.read(cx).raw().evaluate_script(&script);
        }
    }

    fn render_row(&mut self, index: usize, _: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let Some(row) = self.rows.get(index) else {
            return div().into_any_element();
        };
        let theme = cx.theme();
        let content = match row.kind {
            Kind::User => h_flex()
                .justify_end()
                .child(
                    div()
                        .p_2()
                        .rounded_md()
                        .bg(theme.secondary)
                        .child(row.text.clone()),
                )
                .into_any_element(),
            Kind::Assistant => match &row.markdown {
                Some(state) => TextView::new(state).selectable(true).into_any_element(),
                None => div().child(row.text.clone()).into_any_element(),
            },
            Kind::Reasoning => div()
                .italic()
                .text_color(theme.muted_foreground)
                .child(row.text.chars().take(400).collect::<String>())
                .into_any_element(),
            Kind::Tool | Kind::Other => div()
                .text_xs()
                .font_family("Menlo")
                .text_color(theme.muted_foreground)
                .child(row.text.clone())
                .into_any_element(),
        };
        div().px_3().py_1().child(content).into_any_element()
    }
}

impl Render for BachucApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let selected = self.selected.clone();
        let sidebar = v_flex()
            .w(px(220.))
            .h_full()
            .bg(theme.sidebar)
            .border_r_1()
            .border_color(theme.border)
            .children(self.agents.iter().enumerate().map(|(index, agent)| {
                let agent_for_click = agent.clone();
                let active = selected.as_deref() == Some(agent.id.as_str());
                v_flex()
                    .id(("agent", index))
                    .px_3()
                    .py_1()
                    .bg(if active {
                        theme.accent
                    } else {
                        transparent_black()
                    })
                    .child(agent.title.clone())
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .child(agent.status.clone()),
                    )
                    .on_click(
                        cx.listener(move |this, _, _, cx| this.select(agent_for_click.clone(), cx)),
                    )
            }));

        let chat = v_flex()
            .flex_1()
            .h_full()
            .min_w(px(420.))
            .child(
                list(self.list_state.clone(), cx.processor(Self::render_row))
                    .flex_1()
                    .w_full(),
            )
            .child(
                h_flex()
                    .p_2()
                    .gap_2()
                    .border_t_1()
                    .border_color(theme.border)
                    .child(div().flex_1().child(Input::new(&self.input)))
                    .child(
                        div()
                            .flex_none()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .child(self.stats.clone()),
                    ),
            );

        let terminal = div()
            .id("terminal")
            .track_focus(&self.terminal_focus)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, window, cx| this.terminal_focus.focus(window, cx)),
            )
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                if let Some(bytes) = key_bytes(&event.keystroke) {
                    this.send_terminal_frame(0x02, &bytes);
                    cx.stop_propagation();
                }
            }))
            .h(px(16. * ROWS as f32 + 8.))
            .p_1()
            .bg(rgb(0x000000))
            .text_color(rgb(0xd0d0d0))
            .font_family("Menlo")
            .text_size(px(12.))
            .overflow_hidden()
            .children(self.terminal.render_lines());

        let _ = window;
        h_flex()
            .size_full()
            .bg(theme.background)
            .text_color(theme.foreground)
            .child(sidebar)
            .child(chat)
            .child(
                v_flex()
                    .w(px(460.))
                    .h_full()
                    .border_l_1()
                    .border_color(theme.border)
                    .child(terminal)
                    .child(div().flex_1().w_full().child(self.webview.clone()))
                    .child(
                        div()
                            .px_2()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .child(self.connection_state.clone()),
                    ),
            )
    }
}

fn mermaid_init_script() -> String {
    let path = std::env::var("BACHUC_MERMAID_JS").unwrap_or_else(|_| {
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../../node_modules/mermaid/dist/mermaid.min.js"
        )
        .to_string()
    });
    let mermaid = std::fs::read_to_string(&path)
        .unwrap_or_else(|_| "console.error('mermaid.min.js not found')".into());
    format!(
        "{mermaid}\n;mermaid.initialize({{startOnLoad:false,theme:'dark'}});\
         window.renderMermaid = async (src) => {{ const d = document.getElementById('d'); \
         try {{ const r = await mermaid.render('m' + Date.now(), src); d.innerHTML = r.svg; }} \
         catch (e) {{ d.textContent = String(e); }} }};"
    )
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let auto_select = args
        .iter()
        .position(|a| a == "--agent")
        .and_then(|i| args.get(i + 1))
        .cloned();
    let url = std::env::var("BACHUC_URL").unwrap_or_else(|_| "ws://127.0.0.1:6768/ws".into());

    gpui_kit::application().run(move |cx| {
        gpui_kit::init(cx);
        cx.activate(true);
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                None,
                size(px(1100.), px(752.)),
                cx,
            ))),
            titlebar: Some(TitlebarOptions {
                title: Some("Bachuc GPUI proto".into()),
                ..Default::default()
            }),
            ..Default::default()
        };
        gpui_kit::open_window(options, cx, |window, cx| {
            BachucApp::view(window, cx, url.clone(), auto_select.clone())
        })
        .expect("open window");
    });
}
