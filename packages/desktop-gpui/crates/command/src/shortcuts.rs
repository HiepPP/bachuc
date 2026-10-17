//! The shortcut table of the Electron app (`DEFAULT_BINDINGS` in `keyboard-shortcuts.ts`,
//! macOS bindings only) and the stored overrides of it.
//!
//! The table keeps the actions whose feature is in the GPUI plan, one chord per action, in one
//! global scope. Electron's focus and command-center conditions are not ported.

use crate::chord::is_valid_chord;
use anyhow::{Context as _, Result};
use serde_json::{Map, Value};
use std::path::{Path, PathBuf};

/// One action with its chord.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Shortcut {
    /// The action id: the Electron action name, with `.prev` and `.next` on the two relative
    /// navigation actions and `message-input.focus` or `message-input.mode-cycle` for the
    /// message input actions, so every id is unique.
    pub action: String,
    /// The label of the help row.
    pub label: String,
    /// The Electron combo string, for example `Cmd+Shift+P`. A multi-key chord is the combos
    /// joined by one space. None means unassigned.
    pub chord: Option<String>,
}

/// Action, label, and default chord, in the order of the Electron help sections.
const DEFAULTS: &[(&str, &str, &str)] = &[
    // General
    ("command-center.toggle", "Toggle command center", "Cmd+K"),
    ("settings.toggle", "Toggle settings", "Cmd+,"),
    ("theme.cycle", "Cycle theme", "Cmd+Alt+T"),
    ("host.cycle", "Switch to next host", "Cmd+`"),
    // Projects & Workspaces
    ("workspace.navigate.prev", "Previous workspace", "Cmd+["),
    ("workspace.navigate.next", "Next workspace", "Cmd+]"),
    (
        "workspace.archive",
        "Archive workspace",
        "Cmd+Shift+Backspace",
    ),
    // Tabs & Panes
    ("workspace.tab.menu.open", "New tab", "Cmd+T"),
    ("workspace.tab.target.agent", "New agent", "Cmd+Shift+A"),
    ("workspace.terminal.new", "New terminal", "Cmd+Shift+T"),
    ("workspace.tab.target.changes", "Changes", "Cmd+Shift+G"),
    ("workspace.tab.target.files", "Files", "Cmd+Shift+E"),
    ("workspace.tab.close.current", "Close current tab", "Cmd+W"),
    (
        "workspace.tab.navigate.index",
        "Jump to tab",
        "Cmd+Alt+Digit",
    ),
    ("workspace.tab.navigate.prev", "Previous tab", "Alt+Shift+["),
    ("workspace.tab.navigate.next", "Next tab", "Alt+Shift+]"),
    ("workspace.pane.split.right", "Split pane right", "Cmd+\\"),
    (
        "workspace.pane.split.down",
        "Split pane down",
        "Cmd+Shift+\\",
    ),
    (
        "workspace.pane.focus.left",
        "Focus pane left",
        "Cmd+Shift+ArrowLeft",
    ),
    (
        "workspace.pane.focus.right",
        "Focus pane right",
        "Cmd+Shift+ArrowRight",
    ),
    (
        "workspace.pane.focus.up",
        "Focus pane up",
        "Cmd+Shift+ArrowUp",
    ),
    (
        "workspace.pane.focus.down",
        "Focus pane down",
        "Cmd+Shift+ArrowDown",
    ),
    ("workspace.pane.close", "Close pane", "Cmd+Shift+W"),
    // Layout
    ("sidebar.toggle.left", "Toggle left sidebar", "Cmd+B"),
    ("sidebar.toggle.right", "Toggle Explorer sidebar", "Cmd+E"),
    ("sidebar.toggle.both", "Toggle both sidebars", "Cmd+."),
    // Agent Input
    ("message-input.focus", "Focus message input", "Cmd+L"),
    ("message-input.mode-cycle", "Cycle agent mode", "Shift+Tab"),
    ("agent.interrupt", "Interrupt agent", "Escape"),
];

/// The Electron defaults.
pub fn default_shortcuts() -> Vec<Shortcut> {
    DEFAULTS
        .iter()
        .map(|(action, label, chord)| Shortcut {
            action: (*action).to_string(),
            label: (*label).to_string(),
            chord: Some((*chord).to_string()),
        })
        .collect()
}

/// The folder of `shortcuts.json`: `~/Library/Application Support/BachucGpui`.
pub fn data_dir() -> PathBuf {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default();
    home.join("Library/Application Support/BachucGpui")
}

fn overrides_path(dir: &Path) -> PathBuf {
    dir.join("shortcuts.json")
}

/// The defaults with the stored overrides applied.
pub fn load_shortcuts() -> Vec<Shortcut> {
    load_shortcuts_from(&data_dir())
}

/// Stores the chords that differ from the defaults and drops the keys of the ones that do not.
pub fn save_overrides(shortcuts: &[Shortcut]) -> Result<()> {
    save_overrides_to(&data_dir(), shortcuts)
}

/// `load_shortcuts` for a given folder.
///
/// A missing key keeps the default. `null`, `""`, and whitespace unassign the action. A value
/// that is not a string, or a string that is not a chord, keeps the default, as Electron does for
/// stored values it cannot use. A missing, unreadable, or corrupt file gives the defaults, and
/// this function never writes, so a corrupt file stays for the user to fix.
pub fn load_shortcuts_from(dir: &Path) -> Vec<Shortcut> {
    let mut shortcuts = default_shortcuts();
    let path = overrides_path(dir);
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) => {
            if error.kind() != std::io::ErrorKind::NotFound {
                log::warn!("cannot read {}: {error}", path.display());
            }
            return shortcuts;
        }
    };
    let Ok(Value::Object(stored)) = serde_json::from_str::<Value>(&text) else {
        log::warn!(
            "{} is not a JSON object; using the defaults",
            path.display()
        );
        return shortcuts;
    };
    for shortcut in &mut shortcuts {
        match stored.get(&shortcut.action) {
            Some(Value::Null) => shortcut.chord = None,
            Some(Value::String(text)) => {
                let chord = text.trim();
                if chord.is_empty() {
                    shortcut.chord = None;
                } else if is_valid_chord(chord) {
                    shortcut.chord = Some(chord.to_string());
                } else {
                    log::warn!("ignoring the chord {chord:?} of {}", shortcut.action);
                }
            }
            _ => {}
        }
    }
    shortcuts
}

/// `save_overrides` for a given folder. The write goes to a temporary file and then renames, so
/// a crash never leaves half a file.
pub fn save_overrides_to(dir: &Path, shortcuts: &[Shortcut]) -> Result<()> {
    let defaults = default_shortcuts();
    let mut stored = Map::new();
    for shortcut in shortcuts {
        let Some(default) = defaults.iter().find(|d| d.action == shortcut.action) else {
            continue;
        };
        if shortcut.chord == default.chord {
            continue;
        }
        stored.insert(
            shortcut.action.clone(),
            match &shortcut.chord {
                Some(chord) => Value::String(chord.clone()),
                None => Value::Null,
            },
        );
    }
    std::fs::create_dir_all(dir).with_context(|| format!("create {}", dir.display()))?;
    let path = overrides_path(dir);
    let temp = dir.join("shortcuts.json.tmp");
    let text = serde_json::to_string_pretty(&Value::Object(stored))?;
    std::fs::write(&temp, format!("{text}\n"))
        .with_context(|| format!("write {}", temp.display()))?;
    std::fs::rename(&temp, &path).with_context(|| format!("replace {}", path.display()))?;
    Ok(())
}
