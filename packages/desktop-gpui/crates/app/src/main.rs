//! `bachuc-desktop`: the GPUI desktop client. One window shows one host at a time.
//!
//! Bundle: `packages/desktop-gpui/bundle.sh bin bachuc-desktop "Bachuc GPUI" io.github.hieppp.bachuc-desktop`
//! The host list is `~/Library/Application Support/BachucGpui/hosts.json`. The app opens on the
//! host named `live`, or on the host named by `BACHUC_HOST`.

mod actions;
mod chrome;
mod dock;
mod new_agent;
mod shell;
mod state;

use gpui_kit::*;

/// Prints log records of level info and above to stderr. GPUI reports a refused notification
/// permission this way.
struct StderrLogger;

impl log::Log for StderrLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= log::Level::Info
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            eprintln!(
                "[{}] {}: {}",
                record.level(),
                record.target(),
                record.args()
            );
        }
    }

    fn flush(&self) {}
}

fn main() {
    if log::set_logger(&StderrLogger).is_ok() {
        log::set_max_level(log::LevelFilter::Info);
    }
    gpui_kit::application().run(|cx| {
        // The theme reads the registry that `init` builds, so it comes second.
        gpui_kit::init(cx);
        bachuc_theme::init(cx);
        cx.bind_keys(actions::key_bindings());
        cx.activate(true);
        let (width, height) = state::window_size(std::env::var("BACHUC_WINDOW").ok().as_deref());
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                None,
                size(px(width), px(height)),
                cx,
            ))),
            titlebar: Some(TitlebarOptions {
                title: Some("Bachuc GPUI".into()),
                ..Default::default()
            }),
            ..Default::default()
        };
        gpui_kit::open_window(options, cx, |window, cx| {
            cx.new(|cx| shell::Shell::new(window, cx))
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
