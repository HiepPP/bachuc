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
mod fire;
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

/// Makes gpui skip glyph dilation, which copies CoreGraphics font smoothing and thickens strokes
/// by text luminance. Electron turns smoothing off (`-webkit-font-smoothing: antialiased`).
/// gpui reads `AppleFontSmoothing` once, at the first glyph, and only an explicit `0` turns the
/// dilation off, so this runs first in `main` and on every launch: a user `defaults` change
/// cannot bring the dilation back.
#[cfg(target_os = "macos")]
fn disable_font_smoothing() {
    use core_foundation_sys::base::{CFRelease, kCFAllocatorDefault};
    use core_foundation_sys::number::{CFNumberCreate, kCFNumberSInt32Type};
    use core_foundation_sys::preferences::{
        CFPreferencesAppSynchronize, CFPreferencesSetAppValue, kCFPreferencesCurrentApplication,
    };
    use core_foundation_sys::string::{CFStringCreateWithCString, kCFStringEncodingUTF8};

    let zero: i32 = 0;
    // SAFETY: both Core Foundation objects are created here, checked for null, used by the
    // preference calls (which retain what they keep), and released once.
    unsafe {
        let key = CFStringCreateWithCString(
            kCFAllocatorDefault,
            c"AppleFontSmoothing".as_ptr(),
            kCFStringEncodingUTF8,
        );
        let value = CFNumberCreate(
            kCFAllocatorDefault,
            kCFNumberSInt32Type,
            (&raw const zero).cast(),
        );
        if !key.is_null() && !value.is_null() {
            CFPreferencesSetAppValue(key, value.cast(), kCFPreferencesCurrentApplication);
            CFPreferencesAppSynchronize(kCFPreferencesCurrentApplication);
        }
        if !key.is_null() {
            CFRelease(key.cast());
        }
        if !value.is_null() {
            CFRelease(value.cast());
        }
    }
}

#[cfg(not(target_os = "macos"))]
fn disable_font_smoothing() {}

/// Shows the standard About panel. GPUI has no API for it.
#[cfg(target_os = "macos")]
fn show_about_panel() {
    use objc2::MainThreadMarker;
    use objc2_app_kit::NSApplication;

    let Some(main_thread) = MainThreadMarker::new() else {
        log::warn!("about panel requested off the main thread");
        return;
    };
    NSApplication::sharedApplication(main_thread).orderFrontStandardAboutPanel(None);
}

#[cfg(not(target_os = "macos"))]
fn show_about_panel() {}

/// Handlers of the app and window menu items. Quit and Hide have no window; the window items
/// act on the active window, or the first one while the app is in the background.
fn register_app_actions(cx: &mut App) {
    cx.on_action(|_: &actions::Quit, cx| cx.quit());
    cx.on_action(|_: &actions::Hide, cx| cx.hide());
    cx.on_action(|_: &actions::HideOthers, cx| cx.hide_other_apps());
    cx.on_action(|_: &actions::ShowAll, cx| cx.unhide_other_apps());
    cx.on_action(|_: &actions::About, _| show_about_panel());
    cx.on_action(|_: &actions::Minimize, cx| on_main_window(cx, |window| window.minimize_window()));
    cx.on_action(|_: &actions::Zoom, cx| on_main_window(cx, |window| window.zoom_window()));
    cx.on_action(|_: &actions::ToggleFullScreen, cx| {
        on_main_window(cx, |window| window.toggle_fullscreen())
    });
}

/// Deferred, because a menu item or key on the active window dispatches inside that window's
/// update, where a nested `update` of the same window fails.
fn on_main_window(cx: &mut App, run: impl FnOnce(&mut Window) + 'static) {
    let Some(handle) = cx.active_window().or_else(|| cx.windows().first().copied()) else {
        return;
    };
    cx.defer(move |cx| {
        if let Err(error) = handle.update(cx, |_, window, _| run(window)) {
            log::warn!("window menu action failed: {error}");
        }
    });
}

fn main() {
    disable_font_smoothing();
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
            register_app_actions(cx);
            cx.set_menus(actions::app_menus());
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
