//! Shows one agent's timeline from a host.
//!
//! Usage: timeline <agent id or id prefix> [ws url]
//! The url defaults to the live daemon, `ws://127.0.0.1:6768/ws`. A password comes from
//! `BACHUC_PASSWORD`. With `BACHUC_TIMELINE_TRACE` set, it prints each client event with the
//! viewed timeline's version to stderr.
//! Bundle: `packages/desktop-gpui/bundle.sh example timeline BachucTimeline io.github.hieppp.bachuc-timeline`

use bachuc_client::{Client, ClientEvent, HostConfig};
use bachuc_diagram::DiagramRenderer;
use bachuc_timeline::TimelineView;
use gpui_kit::component::ActiveTheme as _;
use gpui_kit::*;

struct Shell {
    client: Entity<Client>,
    diagrams: Entity<DiagramRenderer>,
    wanted: String,
    timeline: Option<Entity<TimelineView>>,
    status: String,
    _subscription: Subscription,
}

impl Shell {
    fn new(url: String, wanted: String, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let host = HostConfig {
            name: "timeline".into(),
            url,
            password: std::env::var("BACHUC_PASSWORD").ok(),
        };
        let client = Client::connect(host, cx);
        let diagrams = cx.new(|cx| DiagramRenderer::new(window, cx));
        let subscription = cx.subscribe_in(&client, window, |this, client, event, window, cx| {
            if std::env::var_os("BACHUC_TIMELINE_TRACE").is_some() {
                let state = client.read(cx);
                let timeline = state
                    .agents()
                    .iter()
                    .find(|agent| agent.id.starts_with(&this.wanted))
                    .and_then(|agent| state.timeline(&agent.id).map(|t| (agent, t)));
                if let Some((agent, timeline)) = timeline {
                    eprintln!(
                        "{event:?} status={} version={} entries={} permissions={}",
                        agent.status,
                        timeline.version,
                        timeline.entries.len(),
                        timeline.pending_permissions.len()
                    );
                }
            }
            match event {
                ClientEvent::Connected => this.status = "Connected".into(),
                ClientEvent::Disconnected { reason } => {
                    this.status = format!("Disconnected: {reason}")
                }
                ClientEvent::AgentsChanged if this.timeline.is_none() => {
                    let agent = client
                        .read(cx)
                        .agents()
                        .iter()
                        .find(|agent| agent.id.starts_with(&this.wanted))
                        .map(|agent| agent.id.clone());
                    match agent {
                        Some(agent_id) => {
                            let client = client.clone();
                            let diagrams = this.diagrams.clone();
                            this.timeline = Some(cx.new(|cx| {
                                TimelineView::new(client, agent_id, diagrams, window, cx)
                            }));
                        }
                        None => this.status = format!("Agent {} not found", this.wanted),
                    }
                }
                _ => return,
            }
            cx.notify();
        });
        Self {
            client,
            diagrams,
            wanted,
            timeline: None,
            status: "Connecting...".into(),
            _subscription: subscription,
        }
    }
}

impl Render for Shell {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let _ = &self.client;
        let theme = cx.theme().clone();
        match &self.timeline {
            Some(timeline) => div().size_full().child(timeline.clone()),
            None => div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .bg(theme.background)
                .text_color(theme.muted_foreground)
                .child(self.status.clone()),
        }
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let wanted = args.next().unwrap_or_default();
    let url = args
        .next()
        .unwrap_or_else(|| "ws://127.0.0.1:6768/ws".into());
    gpui_kit::application().run(move |cx| {
        gpui_kit::init(cx);
        bachuc_theme::init(cx);
        cx.activate(true);
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                None,
                size(px(900.), px(760.)),
                cx,
            ))),
            titlebar: Some(TitlebarOptions {
                title: Some("Bachuc timeline".into()),
                ..Default::default()
            }),
            ..Default::default()
        };
        gpui_kit::open_window(options, cx, |window, cx| {
            cx.new(|cx| Shell::new(url.clone(), wanted.clone(), window, cx))
        })
        .expect("open window");
    });
}
