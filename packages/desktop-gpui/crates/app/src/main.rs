//! `bachuc-desktop`: the GPUI desktop client. One window shows one host at a time.
//!
//! Bundle: `packages/desktop-gpui/bundle.sh bin bachuc-desktop "Bachuc GPUI" io.github.hieppp.bachuc-desktop`
//! The host list is `~/Library/Application Support/BachucGpui/hosts.json`. The app opens on the
//! host named `live`, or on the host named by `BACHUC_HOST`. `BACHUC_WINDOW=900x800` opens the
//! window at that size (default 1200x800, minimum 640x480), for checks at a set width.
//! `BACHUC_NO_ACTIVATE=1` keeps the app in the background, for UI checks started with `open -g`.

mod actions;
mod chrome;
mod dock;
mod new_workspace;
mod panes;
mod plugin_views;
mod plugins;
mod shell;
mod state;
mod toast;
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
            let activate = activates_on_launch(std::env::var("BACHUC_NO_ACTIVATE").ok().as_deref());
            if activate {
                cx.activate(true);
            }
            open_main_window(activate, cx);
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
/// title-bar rows and moves the window from them. Without `activate` the window opens unfocused,
/// so AppKit orders it front without making it key.
fn open_main_window(activate: bool, cx: &mut App) {
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
        focus: activate,
        ..Default::default()
    };
    gpui_kit::open_window(options, cx, |window, cx| {
        cx.new(|cx| shell::Shell::new(window, cx))
    })
    .expect("open window");
}

/// The app comes to the front on launch unless `BACHUC_NO_ACTIVATE` is `1`.
fn activates_on_launch(no_activate: Option<&str>) -> bool {
    no_activate != Some("1")
}

#[cfg(test)]
mod tests {
    // Not `super::*`: the gpui glob brings a `test` macro that shadows the std one.
    use super::activates_on_launch;

    #[test]
    fn only_bachuc_no_activate_1_keeps_the_app_in_the_background() {
        assert!(activates_on_launch(None));
        assert!(activates_on_launch(Some("")));
        assert!(activates_on_launch(Some("0")));
        assert!(!activates_on_launch(Some("1")));
    }
}
