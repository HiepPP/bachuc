//! A workspace with three plain text panes in one group. Split with the "More actions" menu or
//! cmd-\ and cmd-shift-\; each split and the new-tab button (cmd-t) open one more text pane.
//! Drag tabs to reorder them or to move them to another group, and drag a divider to resize.
//! cmd-w closes the active tab.
//!
//! Usage: workspace

use bachuc_icons::IconName;
use bachuc_theme::{metrics, tokens, ui_font_size};
use bachuc_workspace::actions::{CloseActiveTab, NewTab, SplitDown, SplitRight};
use bachuc_workspace::{KEY_CONTEXT, PaneEvent, PaneItem, Workspace};
use gpui_kit::*;

struct TextPane {
    text: SharedString,
}

impl Render for TextPane {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = tokens(cx);
        div()
            .size_full()
            .p(px(metrics::SPACE_4))
            .text_size(ui_font_size(metrics::FONT_CONTENT, cx))
            .text_color(t.foreground)
            .child(self.text.clone())
    }
}

struct Example {
    workspace: Entity<Workspace>,
    next: usize,
}

const ICONS: [IconName; 4] = [
    IconName::Bot,
    IconName::SquareTerminal,
    IconName::FileText,
    IconName::GitBranch,
];

impl Example {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let workspace = cx.new(|cx| Workspace::new(window, cx));
        cx.subscribe_in(&workspace, window, |this, _, event, window, cx| {
            if *event == PaneEvent::NewTab {
                this.open_pane(window, cx);
            }
        })
        .detach();
        let mut example = Example { workspace, next: 1 };
        for _ in 0..3 {
            example.open_pane(window, cx);
        }
        example
            .workspace
            .read(cx)
            .focus_handle(cx)
            .focus(window, cx);
        example
    }

    fn open_pane(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let number = self.next;
        self.next += 1;
        let text: SharedString = format!("Text pane {number}").into();
        let view = cx.new(|_| TextPane { text: text.clone() });
        let item = PaneItem {
            key: format!("pane-{number}"),
            title: text,
            icon: ICONS[number % ICONS.len()],
            modified: number.is_multiple_of(3),
            view: view.into(),
        };
        self.workspace
            .update(cx, |workspace, cx| workspace.open(item, window, cx));
    }
}

impl Render for Example {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child(self.workspace.clone())
    }
}

fn main() {
    gpui_kit::application()
        .with_assets(bachuc_icons::Assets)
        .run(|cx| {
            gpui_kit::init(cx);
            bachuc_theme::init(cx);
            cx.bind_keys([
                KeyBinding::new("cmd-\\", SplitRight, Some(KEY_CONTEXT)),
                KeyBinding::new("cmd-shift-\\", SplitDown, Some(KEY_CONTEXT)),
                KeyBinding::new("cmd-|", SplitDown, Some(KEY_CONTEXT)),
                KeyBinding::new("cmd-w", CloseActiveTab, Some(KEY_CONTEXT)),
                KeyBinding::new("cmd-t", NewTab, Some(KEY_CONTEXT)),
            ]);
            cx.activate(true);
            let options = WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                    None,
                    size(px(1200.), px(800.)),
                    cx,
                ))),
                titlebar: Some(TitlebarOptions {
                    title: Some("Bachuc workspace".into()),
                    ..Default::default()
                }),
                ..Default::default()
            };
            gpui_kit::open_window(options, cx, |window, cx| {
                cx.new(|cx| Example::new(window, cx))
            })
            .expect("open window");
        });
}
