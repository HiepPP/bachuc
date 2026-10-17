//! Shows 3 mermaid diagrams, 3 math blocks, and 1 broken diagram as images in a scroll list.
//! Run: `packages/desktop-gpui/bundle.sh example diagrams BachucDiagrams io.github.hieppp.bachuc-diagrams`

use bachuc_diagram::{DiagramKind, DiagramReady, DiagramRenderer, DiagramState, image_size};
use gpui_kit::component::{ActiveTheme as _, v_flex};
use gpui_kit::*;

const BLOCKS: &[(DiagramKind, &str)] = &[
    (
        DiagramKind::Mermaid,
        "flowchart LR\n  App[GPUI client] -->|WebSocket /ws| Daemon\n  Daemon --> Agents\n  Daemon -->|binary frames| Terminal[alacritty_terminal]",
    ),
    (
        DiagramKind::Math,
        r"\int_0^\infty e^{-x^2}\,dx = \frac{\sqrt{\pi}}{2}",
    ),
    (
        DiagramKind::Mermaid,
        "sequenceDiagram\n  participant C as Client\n  participant D as Daemon\n  C->>D: hello\n  D-->>C: server_info\n  C->>D: fetch_agent_timeline\n  D-->>C: timeline page",
    ),
    (
        DiagramKind::Math,
        r"\sum_{n=1}^{\infty} \frac{1}{n^2} = \frac{\pi^2}{6}",
    ),
    (
        DiagramKind::Mermaid,
        "stateDiagram-v2\n  [*] --> Idle\n  Idle --> Running: prompt\n  Running --> Idle: finished\n  Running --> Error: failed\n  Error --> [*]",
    ),
    (
        DiagramKind::Math,
        r"A = \begin{pmatrix} \alpha & \beta \\ \gamma & \delta \end{pmatrix},\quad \det A = \alpha\delta - \beta\gamma",
    ),
    (DiagramKind::Mermaid, "flowchart LR\n  A --> ("),
];

struct Diagrams {
    renderer: Entity<DiagramRenderer>,
    scroll: ScrollHandle,
    _auto_scroll: Option<Task<()>>,
}

impl Diagrams {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let renderer = cx.new(|cx| DiagramRenderer::new(window, cx));
        cx.subscribe(&renderer, |_, _, _: &DiagramReady, cx| cx.notify())
            .detach();
        let scroll = ScrollHandle::new();
        // `--auto-scroll` scrolls the list up and down without input, for the CPU and memory check.
        let auto_scroll = std::env::args()
            .any(|arg| arg == "--auto-scroll")
            .then(|| auto_scroll(scroll.clone(), cx));
        Self {
            renderer,
            scroll,
            _auto_scroll: auto_scroll,
        }
    }
}

fn auto_scroll(scroll: ScrollHandle, cx: &mut Context<Diagrams>) -> Task<()> {
    cx.spawn(async move |this, cx| {
        let mut step = px(-4.);
        loop {
            cx.background_executor()
                .timer(std::time::Duration::from_millis(16))
                .await;
            let max = scroll.max_offset().y;
            let y = scroll.offset().y + step;
            if y <= -max || y >= px(0.) {
                step = -step;
            }
            scroll.set_offset(point(px(0.), y.clamp(-max, px(0.))));
            if this.update(cx, |_, cx| cx.notify()).is_err() {
                break;
            }
        }
    })
}

impl Render for Diagrams {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let rows = BLOCKS.iter().enumerate().map(|(index, (kind, source))| {
            let state = self
                .renderer
                .update(cx, |renderer, cx| renderer.render(*kind, source, cx));
            let label = format!("{} {}", kind.label(), index + 1);
            let content = match state {
                DiagramState::Ready(image) => {
                    let size = image_size(&image);
                    div()
                        .id(("diagram", index))
                        .role(Role::Image)
                        .aria_label(label)
                        .max_w_full()
                        // A diagram wider than the list scales down and keeps its aspect ratio.
                        .child(img(image).w(size.width).max_w_full())
                        .into_any_element()
                }
                DiagramState::Pending => div()
                    .text_color(theme.muted_foreground)
                    .child("Rendering...")
                    .into_any_element(),
                DiagramState::Failed(error) => v_flex()
                    .id(("diagram", index))
                    .role(Role::Group)
                    .aria_label(format!("{label}, failed"))
                    .gap_1()
                    .p_2()
                    .rounded_md()
                    .bg(theme.muted)
                    .font_family("Menlo")
                    .text_xs()
                    .child(source.to_string())
                    .child(div().text_color(theme.danger).child(error))
                    .into_any_element(),
            };
            v_flex()
                .gap_1()
                .child(
                    div()
                        .text_sm()
                        .text_color(theme.muted_foreground)
                        .child(format!("{}. {}", index + 1, kind.label())),
                )
                .child(content)
        });

        div()
            .id("list")
            .size_full()
            .overflow_y_scroll()
            .track_scroll(&self.scroll)
            .bg(theme.background)
            .text_color(theme.foreground)
            .child(v_flex().p_4().gap_6().children(rows))
    }
}

fn main() {
    gpui_kit::application().run(|cx| {
        gpui_kit::init(cx);
        cx.activate(true);
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                None,
                size(px(760.), px(560.)),
                cx,
            ))),
            titlebar: Some(TitlebarOptions {
                title: Some("Bachuc diagrams".into()),
                ..Default::default()
            }),
            ..Default::default()
        };
        gpui_kit::open_window(options, cx, |window, cx| {
            cx.new(|cx| Diagrams::new(window, cx))
        })
        .expect("open window");
    });
}
