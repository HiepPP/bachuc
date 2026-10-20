//! `bachuc-settings`: the settings screen of the GPUI client, after the Electron settings page.
//!
//! Sections: hosts, appearance, keyboard shortcuts, providers, new agent defaults, and project
//! settings.
//! `Settings` is the view; it emits `SettingsEvent` so the app can reconnect, rebind its
//! shortcuts, or close the screen. `load_appearance` applies the stored theme, fonts, content
//! size, and syntax theme at start. The sidebar also lists the settings screens of the active
//! plugins (`Settings::open_plugin_screen` opens one), and `watch_plugin_themes` hands the plugin
//! themes to `bachuc_theme` for the Appearance picker. The pure models (`appearance`, `hosts`, `bindings`, `config`) hold the rules and carry
//! the tests.

pub mod agent_defaults;
pub mod appearance;
pub mod bindings;
pub mod config;
pub mod hosts;

mod appearance_preview;
mod pane_agent_defaults;
mod pane_appearance;
mod pane_hosts;
mod pane_projects;
mod pane_providers;
mod pane_shortcuts;
mod plugins;
mod settings;
mod ui;

pub use appearance::load_appearance;
pub use plugins::watch_plugin_themes;
pub use settings::{Settings, SettingsEvent};
