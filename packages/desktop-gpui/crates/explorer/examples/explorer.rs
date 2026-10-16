//! The Explorer sidebar beside a pane that shows the picked file or diff, on a live workspace.
//!
//! Usage: explorer [folder] [--url <ws url>] [--changes] [--file <path> [--line <n>]]
//!                 [--diff <path>]
//!
//! The folder is a directory the daemon can list (default `/tmp/gpui-bench`). The host is the
//! live daemon, `ws://127.0.0.1:6768/ws`, unless `--url` says otherwise. `--changes` starts on
//! the Changes tab; `--file` and `--diff` open a viewer at start. A click on a file or a changed
//! file in the sidebar opens it in the right pane.

use bachuc_client::{Client, HostConfig};
use bachuc_explorer::{DiffView, Explorer, ExplorerEvent, ExplorerTab, FileView};
use bachuc_theme::{ThemeChoice, set_theme, tokens};
use gpui_kit::component::h_flex;
use gpui_kit::*;

enum Open {
    File { path: String, line: Option<u32> },
    Diff { path: String },
}

struct Shell {
    client: Entity<Client>,
    cwd: String,
    explorer: Entity<Explorer>,
    pane: Option<AnyView>,
}

impl Shell {
    fn new(
        client: Entity<Client>,
        cwd: String,
        changes: bool,
        open: Option<Open>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let explorer = cx.new(|cx| Explorer::new(client.clone(), cwd.clone(), window, cx));
        if changes {
            explorer.update(cx, |explorer, cx| {
                explorer.select_tab(ExplorerTab::Changes, cx)
            });
        }
        cx.subscribe_in(
            &explorer,
            window,
            |this: &mut Self, _, event: &ExplorerEvent, window, cx| match event {
                ExplorerEvent::OpenFile { path, .. } => {
                    println!("open file {path}");
                    this.open_file(path.clone(), None, window, cx);
                }
                ExplorerEvent::OpenDiff { path, .. } => {
                    println!("open diff {path}");
                    this.open_diff(path.clone(), window, cx);
                }
            },
        )
        .detach();
        let mut shell = Self {
            client,
            cwd,
            explorer,
            pane: None,
        };
        match open {
            Some(Open::File { path, line }) => shell.open_file(path, line, window, cx),
            Some(Open::Diff { path }) => shell.open_diff(path, window, cx),
            None => {}
        }
        shell
    }

    fn open_file(
        &mut self,
        path: String,
        line: Option<u32>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let client = self.client.clone();
        let cwd = self.cwd.clone();
        let view = cx.new(|cx| FileView::new(client, cwd, path, line, window, cx));
        self.pane = Some(view.into());
        cx.notify();
    }

    fn open_diff(&mut self, path: String, window: &mut Window, cx: &mut Context<Self>) {
        let client = self.client.clone();
        let cwd = self.cwd.clone();
        let view = cx.new(|cx| DiffView::new(client, cwd, path, window, cx));
        self.pane = Some(view.into());
        cx.notify();
    }
}

impl Render for Shell {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = tokens(cx);
        h_flex()
            .size_full()
            .bg(colors.surface0)
            .child(
                div()
                    .w(px(bachuc_theme::metrics::SIDEBAR_WIDTH))
                    .h_full()
                    .flex_shrink_0()
                    .border_r_1()
                    .border_color(colors.border)
                    .child(self.explorer.clone()),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .children(self.pane.clone()),
            )
    }
}

fn main() {
    let mut folder = "/tmp/gpui-bench".to_string();
    let mut url = "ws://127.0.0.1:6768/ws".to_string();
    let mut changes = false;
    let mut file = None;
    let mut line = None;
    let mut diff = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--url" => url = args.next().unwrap_or(url),
            "--changes" => changes = true,
            "--file" => file = args.next(),
            "--line" => line = args.next().and_then(|text| text.parse().ok()),
            "--diff" => diff = args.next(),
            _ => folder = arg,
        }
    }
    let open = match (file, diff) {
        (Some(path), _) => Some(Open::File { path, line }),
        (None, Some(path)) => Some(Open::Diff { path }),
        (None, None) => None,
    };

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
                    name: "explorer".into(),
                    url,
                    password: std::env::var("BACHUC_PASSWORD").ok(),
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
                    title: Some("Bachuc explorer".into()),
                    ..Default::default()
                }),
                ..Default::default()
            };
            gpui_kit::open_window(options, cx, move |window, cx| {
                cx.new(|cx| Shell::new(client, folder, changes, open, window, cx))
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
