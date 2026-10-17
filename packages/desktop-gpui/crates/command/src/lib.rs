//! `bachuc-command`: the Electron command center and the Electron shortcut table.
//!
//! `Shortcut`, `default_shortcuts`, `load_shortcuts`, and `save_overrides` hold the macOS key
//! bindings of `keyboard-shortcuts.ts` and the stored overrides of them. `CommandCenter` is the
//! palette view: the app passes `CommandItem`s and runs the id of `CommandEvent::Run`. The view
//! adds the active plugins' items itself and runs them; the app ignores a `plugin:` id.
//! `gpui_keystrokes` turns a chord into the strings that `KeyBinding::new` takes.

mod center;
mod chord;
mod plugin_rows;
pub mod search;
mod shortcuts;

pub use center::{CommandCenter, CommandEvent, CommandItem};
pub use chord::{display_combos, gpui_keystrokes, is_valid_chord};
pub use shortcuts::{
    Shortcut, data_dir, default_shortcuts, load_shortcuts, load_shortcuts_from, save_overrides,
    save_overrides_to,
};
