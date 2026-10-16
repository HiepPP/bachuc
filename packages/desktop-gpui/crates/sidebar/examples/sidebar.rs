//! Shows the sidebar alone, connected to a daemon host. The events it emits print to stdout.
//!
//! Usage: sidebar [ws://host:port/ws]
//! The default host is live, `ws://127.0.0.1:6768/ws`. Set BACHUC_DARK for the dark theme.

use bachuc_client::{Client, HostConfig};
use bachuc_sidebar::{Sidebar, SidebarEvent};
use bachuc_theme::{ThemeChoice, set_theme, tokens};
use gpui_kit::component::h_flex;
use gpui_kit::*;

struct Example {
    sidebar: Entity<Sidebar>,
    _subscription: Subscription,
}

impl Example {
    fn new(client: Entity<Client>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let sidebar = cx.new(|cx| Sidebar::new(client, window, cx));
        let subscription = cx.subscribe(
            &sidebar,
            |this: &mut Example, _, event: &SidebarEvent, cx| {
                println!("{event:?}");
                if let SidebarEvent::OpenWorkspace { workspace_id } = event {
                    let id = Some(workspace_id.clone());
                    this.sidebar
                        .update(cx, |sidebar, cx| sidebar.set_selected_workspace(id, cx));
                }
            },
        );
        Example {
            sidebar,
            _subscription: subscription,
        }
    }
}

impl Render for Example {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = tokens(cx);
        h_flex()
            .size_full()
            .bg(colors.surface_workspace)
            .text_color(colors.foreground)
            .child(self.sidebar.clone())
    }
}

fn main() {
    let url = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "ws://127.0.0.1:6768/ws".to_string());
    gpui_kit::application()
        .with_assets(bachuc_icons::Assets)
        .run(move |cx| {
            gpui_kit::init(cx);
            bachuc_theme::init(cx);
            if std::env::var_os("BACHUC_DARK").is_some() {
                set_theme(ThemeChoice::ClaudeDark, cx);
            }
            cx.activate(true);
            let client = Client::connect(
                HostConfig {
                    name: "sidebar".into(),
                    url: url.clone(),
                    password: None,
                },
                cx,
            );
            let options = WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                    None,
                    // The size of the reference Electron window, for side-by-side checks.
                    size(px(1200.), px(800.)),
                    cx,
                ))),
                titlebar: Some(TitlebarOptions {
                    title: Some("Bachuc sidebar".into()),
                    ..Default::default()
                }),
                ..Default::default()
            };
            gpui_kit::open_window(options, cx, |window, cx| {
                cx.new(|cx| Example::new(client, window, cx))
            })
            .expect("open window");
        });
}
