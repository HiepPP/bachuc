//! Shows the settings screen alone, connected to a daemon host. The events it emits print to
//! stdout.
//!
//! Usage: settings [ws://host:port/ws]
//! The default host is live, `ws://127.0.0.1:6768/ws`. The screen reads and writes `hosts.json`,
//! `appearance.json`, and `shortcuts.json` in `~/Library/Application Support/BachucGpui`.
//! Set BACHUC_DARK for the dark theme without storing it.

use bachuc_client::{Client, HostConfig};
use bachuc_settings::{Settings, SettingsEvent, load_appearance};
use bachuc_theme::{ThemeChoice, set_theme};
use gpui_kit::*;

fn main() {
    let url = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "ws://127.0.0.1:6768/ws".to_string());
    gpui_kit::application()
        .with_assets(bachuc_icons::Assets)
        .run(move |cx| {
            gpui_kit::init(cx);
            bachuc_theme::init(cx);
            load_appearance(cx);
            if std::env::var_os("BACHUC_DARK").is_some() {
                set_theme(ThemeChoice::ClaudeDark, cx);
            }
            cx.activate(true);
            let client = Client::connect(
                HostConfig {
                    name: "settings".into(),
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
                    title: Some("Bachuc settings".into()),
                    ..Default::default()
                }),
                ..Default::default()
            };
            gpui_kit::open_window(options, cx, |window, cx| {
                let settings = cx.new(|cx| Settings::new(client, window, cx));
                cx.subscribe(&settings, |_, event: &SettingsEvent, cx| {
                    println!("{event:?}");
                    if matches!(event, SettingsEvent::Closed) {
                        cx.quit();
                    }
                })
                .detach();
                settings
            })
            .expect("open window");
        });
}
