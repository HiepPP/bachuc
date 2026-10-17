//! `bachuc-desktop`: the GPUI desktop client. One window shows one host at a time.
//!
//! Bundle: `packages/desktop-gpui/bundle.sh bin bachuc-desktop "Bachuc GPUI" io.github.hieppp.bachuc-desktop`
//! The host list is `~/Library/Application Support/BachucGpui/hosts.json`. The app opens on the
//! host named `live`, or on the host named by `BACHUC_HOST`. `BACHUC_WINDOW=900x800` opens the
//! window at that size (default 1200x800, minimum 640x480), for checks at a set width.

mod actions;
mod chrome;
mod dock;
mod new_workspace;
mod panes;
mod plugin_views;
mod plugins;
mod shell;
mod state;
mod workspace_draft;

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
    // The views draw lucide icons from the Bachuc asset source.
    gpui_kit::application()
        .with_assets(bachuc_icons::Assets)
        .run(|cx| {
            // The theme reads the registry that `init` builds, so it comes second; the stored
            // appearance applies over the theme defaults.
            gpui_kit::init(cx);
            bachuc_theme::init(cx);
            // The views read the host's contributions; no plugin is active before the shell
            // gives it a client.
            bachuc_plugin_host::init(plugins::registered_plugins(), cx);
            bachuc_settings::load_appearance(cx);
            // The shell binds the shortcut table; these never change.
            cx.bind_keys(actions::fixed_bindings());
            cx.activate(true);
            open_main_window(cx);
            cx.on_window_closed(|cx, _| {
                if cx.windows().is_empty() {
                    cx.quit();
                }
            })
            .detach();
        });
}

/// Electron's window chrome: `titleBarStyle: "hidden"` with the traffic lights at (16, 14)
/// moved by the app's -4 offset (`window-manager.ts`), so at (16, 10). The app draws its own
/// title-bar rows and moves the window from them.
fn open_main_window(cx: &mut App) {
    let (width, height) = state::window_size(std::env::var("BACHUC_WINDOW").ok().as_deref());
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
            None,
            size(px(width), px(height)),
            cx,
        ))),
        titlebar: Some(TitlebarOptions {
            title: Some("Bachuc GPUI".into()),
            appears_transparent: true,
            traffic_light_position: Some(point(px(16.), px(10.))),
        }),
        app_owns_titlebar_drag: true,
        window_min_size: Some(size(px(640.), px(480.))),
        ..Default::default()
    };
    gpui_kit::open_window(options, cx, |window, cx| {
        cx.new(|cx| shell::Shell::new(window, cx))
    })
    .expect("open window");
}
