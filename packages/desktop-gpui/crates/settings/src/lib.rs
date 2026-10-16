//! `bachuc-settings`: the settings screen of the GPUI client, after the Electron settings page.
//!
//! Sections: hosts, appearance, keyboard shortcuts, providers, and project settings.
//! `Settings` is the view; it emits `SettingsEvent` so the app can reconnect, rebind its
//! shortcuts, or close the screen. `load_appearance` applies the stored theme and font sizes at
//! start. The pure models (`appearance`, `hosts`, `bindings`, `config`) hold the rules and carry
//! the tests.

pub mod appearance;
pub mod bindings;
pub mod config;
pub mod hosts;

mod pane_appearance;
mod pane_hosts;
mod pane_projects;
mod pane_providers;
mod pane_shortcuts;
mod settings;
mod ui;

pub use appearance::load_appearance;
pub use settings::{Settings, SettingsEvent};
