//! Shows the composer for one agent above a plain text log of its stream.
//!
//! Usage: composer [ws url] [agent id prefix]
//! The url defaults to `BACHUC_URL`, then the live daemon. Without an agent id it uses the
//! newest agent in the list. A password comes from `BACHUC_PASSWORD`.

use bachuc_client::{Client, ClientEvent, HostConfig};
use bachuc_composer::Composer;
use gpui_kit::component::{ActiveTheme as _, v_flex};
use gpui_kit::*;
use serde_json::Value;

/// The log keeps this many of the newest entries.
const LOG_LINES: usize = 40;

struct Example {
    client: Entity<Client>,
    wanted: Option<String>,
    agent_id: Option<String>,
    composer: Option<Entity<Composer>>,
    status: String,
}

impl Example {
    fn new(
        client: Entity<Client>,
        wanted: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        cx.subscribe_in(
            &client,
            window,
            |this: &mut Example, client, event: &ClientEvent, window, cx| match event {
                ClientEvent::AgentsChanged => this.pick_agent(client, window, cx),
                ClientEvent::TimelineChanged { agent_id }
                    if this.agent_id.as_deref() == Some(agent_id.as_str()) =>
                {
                    cx.notify()
                }
                ClientEvent::Connected => {
                    this.status = "connected".into();
                    cx.notify();
                }
                ClientEvent::Disconnected { reason } => {
                    this.status = format!("disconnected: {reason}");
                    cx.notify();
                }
                _ => {}
            },
        )
        .detach();
        Example {
            client,
            wanted,
            agent_id: None,
            composer: None,
            status: "connecting".into(),
        }
    }

    fn pick_agent(&mut self, client: &Entity<Client>, window: &mut Window, cx: &mut Context<Self>) {
        if self.agent_id.is_some() {
            return;
        }
        let agents = client.read(cx).agents();
        let found = match &self.wanted {
            Some(prefix) => agents.iter().find(|a| a.id.starts_with(prefix.as_str())),
            None => agents.first(),
        };
        let Some(agent) = found else {
            return;
        };
        let id = agent.id.clone();
        client.update(cx, |client, cx| client.view_agent(&id, cx));
        let composer_client = client.clone();
        let composer_id = id.clone();
        let composer = cx.new(|cx| Composer::new(composer_client, composer_id, window, cx));
        composer.update(cx, |composer, cx| composer.focus(window, cx));
        self.composer = Some(composer);
        self.agent_id = Some(id);
        cx.notify();
    }
}

fn summary(entry: &Value) -> String {
    let item = &entry["item"];
    let kind = item["type"].as_str().unwrap_or("?");
    let text = match kind {
        "tool_call" => format!(
            "{} {}",
            item["name"].as_str().unwrap_or("?"),
            item["status"].as_str().unwrap_or("")
        ),
        _ => item["text"].as_str().unwrap_or("").replace('\n', " "),
    };
    let text: String = text.chars().take(200).collect();
    format!("[{kind}] {text}")
}

impl Render for Example {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let lines: Vec<String> = self
            .agent_id
            .as_deref()
            .and_then(|id| self.client.read(cx).timeline(id))
            .map(|timeline| {
                let skip = timeline.entries.len().saturating_sub(LOG_LINES);
                timeline.entries[skip..].iter().map(summary).collect()
            })
            .unwrap_or_default();
        let theme = cx.theme();
        v_flex()
            .size_full()
            .bg(theme.background)
            .text_color(theme.foreground)
            .child(
                v_flex()
                    .id("log")
                    .flex_1()
                    .min_h_0()
                    .p_2()
                    .overflow_hidden()
                    .justify_end()
                    .font_family("Menlo")
                    .text_xs()
                    .children(lines.into_iter().map(|line| div().child(line))),
            )
            .child(
                div()
                    .px_2()
                    .text_xs()
                    .text_color(theme.muted_foreground)
                    .child(self.status.clone()),
            )
            .children(self.composer.clone())
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let url = args
        .next()
        .or_else(|| std::env::var("BACHUC_URL").ok())
        .unwrap_or_else(|| "ws://127.0.0.1:6768/ws".into());
    let wanted = args.next();
    gpui_kit::application().run(move |cx: &mut App| {
        gpui_kit::init(cx);
        bachuc_theme::init(cx);
        cx.activate(true);
        let host = HostConfig {
            name: "composer".into(),
            url,
            password: std::env::var("BACHUC_PASSWORD").ok(),
        };
        let client = Client::connect(host, cx);
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                None,
                size(px(720.), px(560.)),
                cx,
            ))),
            titlebar: Some(TitlebarOptions {
                title: Some("Bachuc composer".into()),
                ..Default::default()
            }),
            ..Default::default()
        };
        // `Example` holds the client, so the socket stays open while the window lives.
        gpui_kit::open_window(options, cx, move |window, cx| {
            cx.new(|cx| Example::new(client, wanted, window, cx))
        })
        .expect("open window");
    });
}
