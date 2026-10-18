//! A window with one daemon terminal.
//!
//! Usage: terminal [folder] [--attach <terminal id>] [--url <ws url>]
//!
//! Without `--attach` it opens a new terminal in the folder, which must be a workspace the daemon
//! knows (default `/tmp/gpui-bench`). The terminal id is printed when it opens. Closing the
//! window leaves the terminal running, so `--attach <id>` can show it again. The host is the live
//! daemon, `ws://127.0.0.1:6768/ws`, unless `--url` says otherwise.

use bachuc_client::{Client, ClientEvent, HostConfig};
use bachuc_terminal::{TerminalEvent, TerminalView};
use gpui_kit::*;

enum Target {
    Create(String),
    Attach(String),
}

struct Shell {
    client: Entity<Client>,
    target: Target,
    terminal: Option<Entity<TerminalView>>,
}

impl Shell {
    fn new(
        client: Entity<Client>,
        target: Target,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        // A subscription holds its emitter weakly; the field above keeps the client alive.
        cx.subscribe_in(
            &client,
            window,
            |this: &mut Self, _, event: &ClientEvent, window, cx| {
                if matches!(event, ClientEvent::Connected | ClientEvent::AgentsChanged) {
                    this.open(window, cx);
                }
            },
        )
        .detach();
        let mut shell = Self {
            client,
            target,
            terminal: None,
        };
        shell.open(window, cx);
        shell
    }

    /// Opens the terminal once the client is connected. A new terminal also needs the workspace
    /// list, which arrives right after the connection.
    fn open(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.terminal.is_some() {
            return;
        }
        let ready = {
            let client = self.client.read(cx);
            client.is_connected()
                && (matches!(self.target, Target::Attach(_)) || !client.workspaces().is_empty())
        };
        if !ready {
            return;
        }
        let client = self.client.clone();
        let view = cx.new(|cx| match &self.target {
            Target::Create(cwd) => TerminalView::new(client, cwd.clone(), window, cx),
            Target::Attach(id) => TerminalView::attach(client, id.clone(), window, cx),
        });
        cx.subscribe(&view, |_, _, event: &TerminalEvent, cx| match event {
            TerminalEvent::Opened { id } => println!("terminal id: {id}"),
            TerminalEvent::Exited => {
                println!("terminal exited");
                cx.notify();
            }
            TerminalEvent::OpenFile { cwd, path, line } => {
                println!("open file: {cwd} {path} {line:?}")
            }
        })
        .detach();
        self.terminal = Some(view);
        cx.notify();
    }
}

impl Render for Shell {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let pane = div().size_full().bg(rgb(0x0f1115));
        match &self.terminal {
            Some(terminal) => pane.child(terminal.clone()),
            None => pane,
        }
    }
}

fn main() {
    let mut folder = "/tmp/gpui-bench".to_string();
    let mut attach = None;
    let mut url = "ws://127.0.0.1:6768/ws".to_string();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--attach" => attach = args.next(),
            "--url" => url = args.next().unwrap_or(url),
            _ => folder = arg,
        }
    }
    let target = match attach {
        Some(id) => Target::Attach(id),
        None => Target::Create(folder),
    };

    gpui_kit::application().run(move |cx| {
        gpui_kit::init(cx);
        cx.activate(true);
        let client = Client::connect(
            HostConfig {
                name: "terminal".into(),
                url,
                password: std::env::var("BACHUC_PASSWORD").ok(),
            },
            cx,
        );
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                None,
                size(px(900.), px(560.)),
                cx,
            ))),
            titlebar: Some(TitlebarOptions {
                title: Some("Bachuc Terminal".into()),
                ..Default::default()
            }),
            ..Default::default()
        };
        gpui_kit::open_window(options, cx, move |window, cx| {
            cx.new(|cx| Shell::new(client, target, window, cx))
        })
        .expect("open window");
        cx.on_window_closed(|cx, _| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();
    });
}
