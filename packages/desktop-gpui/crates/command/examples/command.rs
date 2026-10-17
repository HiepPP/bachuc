//! Shows the command center over a plain window with sample items: the app actions from the
//! shortcut table (with their stored chords as hints), then workspaces and agents. The picked
//! id appears under the button. Escape or a click outside closes the palette.
//!
//! Usage: command [item count]
//! A count above the sample size adds numbered filler rows, to try the search on a long list.

use bachuc_command::{CommandCenter, CommandEvent, CommandItem, load_shortcuts};
use bachuc_icons::IconName;
use bachuc_theme::{ThemeChoice, set_theme, tokens};
use gpui_kit::component::button::Button;
use gpui_kit::component::{h_flex, v_flex};
use gpui_kit::*;

struct Example {
    center: Option<Entity<CommandCenter>>,
    status: String,
    filler: usize,
    _subscription: Option<Subscription>,
}

fn icon_for(action: &str) -> Option<IconName> {
    Some(match action {
        "command-center.toggle" => IconName::Search,
        "settings.toggle" => IconName::Settings,
        "theme.cycle" => IconName::Palette,
        "host.cycle" => IconName::Server,
        "workspace.archive" => IconName::Archive,
        "workspace.terminal.new" => IconName::Terminal,
        "workspace.tab.target.agent" => IconName::Bot,
        "workspace.tab.target.changes" => IconName::GitCompareArrows,
        "workspace.tab.target.files" => IconName::Files,
        "workspace.pane.split.right" | "workspace.pane.split.down" => IconName::Split,
        "sidebar.toggle.left" => IconName::PanelLeft,
        "sidebar.toggle.right" => IconName::PanelRight,
        "message-input.focus" => IconName::MessageSquare,
        _ => return None,
    })
}

fn sample_items(filler: usize) -> Vec<CommandItem> {
    let mut items: Vec<CommandItem> = load_shortcuts()
        .into_iter()
        .map(|shortcut| CommandItem {
            icon: icon_for(&shortcut.action),
            id: format!("action:{}", shortcut.action),
            title: shortcut.label.into(),
            group: "Actions".into(),
            shortcut: shortcut.chord,
        })
        .collect();
    for (name, project) in [
        ("main", "bachuc"),
        ("feat/bachuc-gpui", "bachuc"),
        ("fix/timeline", "paseo"),
    ] {
        items.push(CommandItem {
            id: format!("workspace:{name}"),
            title: format!("{project} / {name}").into(),
            group: "Workspaces".into(),
            icon: Some(IconName::Folder),
            shortcut: None,
        });
    }
    for title in [
        "Fix flaky terminal test",
        "Port the settings screen",
        "gpui-age",
    ] {
        items.push(CommandItem {
            id: format!("agent:{title}"),
            title: title.into(),
            group: "Agents".into(),
            icon: Some(IconName::Circle),
            shortcut: None,
        });
    }
    for number in 0..filler.saturating_sub(items.len()) {
        items.push(CommandItem {
            id: format!("filler:{number}"),
            title: format!("Filler row {number}").into(),
            group: "Filler".into(),
            icon: None,
            shortcut: None,
        });
    }
    items
}

impl Example {
    fn open(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let items = sample_items(self.filler);
        let center = cx.new(|cx| CommandCenter::new(items, window, cx));
        self._subscription = Some(cx.subscribe(
            &center,
            |this: &mut Example, _, event: &CommandEvent, cx| {
                this.status = match event {
                    CommandEvent::Run { id } => format!("run {id}"),
                    CommandEvent::Dismissed => "dismissed".to_string(),
                };
                println!("{}", this.status);
                this.center = None;
                cx.notify();
            },
        ));
        self.center = Some(center);
        cx.notify();
    }
}

impl Render for Example {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = tokens(cx);
        div()
            .relative()
            .size_full()
            .bg(colors.surface0)
            .text_color(colors.foreground)
            .child(
                v_flex()
                    .size_full()
                    .items_center()
                    .justify_center()
                    .gap_3()
                    .child(
                        h_flex().child(
                            Button::new("open")
                                .outline()
                                .label("Open command center")
                                .accessibility_label("Open command center")
                                .on_click(cx.listener(|this, _, window, cx| this.open(window, cx))),
                        ),
                    )
                    .child(
                        div()
                            .text_color(colors.foreground_muted)
                            .child(self.status.clone()),
                    ),
            )
            .children(self.center.clone())
    }
}

fn main() {
    let filler = std::env::args()
        .nth(1)
        .and_then(|text| text.parse().ok())
        .unwrap_or(0);
    gpui_kit::application()
        .with_assets(bachuc_icons::Assets)
        .run(move |cx| {
            gpui_kit::init(cx);
            bachuc_theme::init(cx);
            if std::env::var_os("BACHUC_DARK").is_some() {
                set_theme(ThemeChoice::ClaudeDark, cx);
            }
            cx.activate(true);
            let options = WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                    None,
                    // The size of the reference Electron window, for side-by-side checks.
                    size(px(1200.), px(800.)),
                    cx,
                ))),
                titlebar: Some(TitlebarOptions {
                    title: Some("Bachuc command center".into()),
                    ..Default::default()
                }),
                ..Default::default()
            };
            gpui_kit::open_window(options, cx, |window, cx| {
                cx.new(|cx| {
                    let mut example = Example {
                        center: None,
                        status: "closed".to_string(),
                        filler,
                        _subscription: None,
                    };
                    example.open(window, cx);
                    example
                })
            })
            .expect("open window");
        });
}
