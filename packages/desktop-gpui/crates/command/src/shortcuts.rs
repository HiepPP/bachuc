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

/// The help sections of the Electron shortcuts page (`ShortcutSectionId`), in its order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShortcutSection {
    General,
    Workspaces,
    TabsPanes,
    Layout,
    AgentInput,
}

impl ShortcutSection {
    pub const ALL: [ShortcutSection; 5] = [
        ShortcutSection::General,
        ShortcutSection::Workspaces,
        ShortcutSection::TabsPanes,
        ShortcutSection::Layout,
        ShortcutSection::AgentInput,
    ];

    /// `settings.shortcuts.sections.*` of the Electron app.
    pub fn title(self) -> &'static str {
        match self {
            ShortcutSection::General => "General",
            ShortcutSection::Workspaces => "Projects & Workspaces",
            ShortcutSection::TabsPanes => "Tabs & Panes",
            ShortcutSection::Layout => "Layout",
            ShortcutSection::AgentInput => "Agent Input",
        }
    }
}

use ShortcutSection::{AgentInput, General, Layout, TabsPanes, Workspaces};

/// Section, action, label, and default chord. Rows follow `SHORTCUT_HELP_ROW_ORDER`, so a
/// section lists its rows in this order.
const DEFAULTS: &[(ShortcutSection, &str, &str, &str)] = &[
    (
        General,
        "command-center.toggle",
        "Toggle command center",
        "Cmd+K",
    ),
    (General, "command-center.files", "Search files", "Cmd+P"),
    // Electron binds it only while focus is not in a text field or terminal; the app gives
    // it a key context that excludes those.
    (
        General,
        "shortcuts.dialog.toggle",
        "Show keyboard shortcuts",
        "Shift+?",
    ),
    (General, "settings.toggle", "Toggle settings", "Cmd+,"),
    (General, "theme.cycle", "Cycle theme", "Cmd+Alt+T"),
    (General, "host.cycle", "Switch to next host", "Cmd+`"),
    // The View menu zoom of the Electron desktop app (`features/menu.ts`), not in
    // `DEFAULT_BINDINGS`. The app also binds Cmd++ to zoom in while it has Cmd+=.
    (General, "view.zoom.in", "Zoom in", "Cmd+="),
    (General, "view.zoom.out", "Zoom out", "Cmd+-"),
    (General, "view.zoom.reset", "Actual size", "Cmd+0"),
    (Workspaces, "agent.new", "Open project", "Cmd+O"),
    (Workspaces, "workspace.new", "New workspace", "Cmd+N"),
    (
        Workspaces,
        "workspace.navigate.prev",
        "Previous workspace",
        "Cmd+[",
    ),
    (
        Workspaces,
        "workspace.navigate.next",
        "Next workspace",
        "Cmd+]",
    ),
    (Workspaces, "workspace.pin", "Pin chat", "Cmd+Shift+P"),
    (
        Workspaces,
        "workspace.archive",
        "Archive workspace",
        "Cmd+Shift+Backspace",
    ),
    (
        Workspaces,
        "agent.unread.switcher",
        "Open unread thread",
        "Cmd+J",
    ),
    (
        Workspaces,
        "agent.previous",
        "Back to previous thread",
        "Cmd+Shift+J",
    ),
    (TabsPanes, "workspace.tab.menu.open", "New tab", "Cmd+T"),
    (
        TabsPanes,
        "workspace.tab.target.agent",
        "New agent",
        "Cmd+Shift+A",
    ),
    (
        TabsPanes,
        "workspace.terminal.new",
        "New terminal",
        "Cmd+Shift+T",
    ),
    (
        TabsPanes,
        "workspace.tab.target.changes",
        "Changes",
        "Cmd+Shift+G",
    ),
    (
        TabsPanes,
        "workspace.tab.target.files",
        "Files",
        "Cmd+Shift+E",
    ),
    (
        TabsPanes,
        "workspace.tab.close.current",
        "Close current tab",
        "Cmd+W",
    ),
    (
        TabsPanes,
        "workspace.tab.navigate.index",
        "Jump to tab",
        "Cmd+Alt+Digit",
    ),
    (
        TabsPanes,
        "workspace.tab.navigate.prev",
        "Previous tab",
        "Alt+Shift+[",
    ),
    (
        TabsPanes,
        "workspace.tab.navigate.next",
        "Next tab",
        "Alt+Shift+]",
    ),
    (
        TabsPanes,
        "workspace.pane.split.right",
        "Split pane right",
        "Cmd+\\",
    ),
    (
        TabsPanes,
        "workspace.pane.split.down",
        "Split pane down",
        "Cmd+Shift+\\",
    ),
    (
        TabsPanes,
        "workspace.pane.focus.left",
        "Focus pane left",
        "Cmd+Shift+ArrowLeft",
    ),
    (
        TabsPanes,
        "workspace.pane.focus.right",
        "Focus pane right",
        "Cmd+Shift+ArrowRight",
    ),
    (
        TabsPanes,
        "workspace.pane.focus.up",
        "Focus pane up",
        "Cmd+Shift+ArrowUp",
    ),
    (
        TabsPanes,
        "workspace.pane.focus.down",
        "Focus pane down",
        "Cmd+Shift+ArrowDown",
    ),
    (
        TabsPanes,
        "workspace.pane.close",
        "Close pane",
        "Cmd+Shift+W",
    ),
    (
        Layout,
        "sidebar.toggle.left",
        "Toggle left sidebar",
        "Cmd+B",
    ),
    (
        Layout,
        "sidebar.toggle.right",
        "Toggle Explorer sidebar",
        "Cmd+E",
    ),
    (
        Layout,
        "sidebar.toggle.both",
        "Toggle both sidebars",
        "Cmd+.",
    ),
    (
        AgentInput,
        "message-input.focus",
        "Focus message input",
        "Cmd+L",
    ),
    (
        AgentInput,
        "message-input.mode-cycle",
        "Cycle agent mode",
        "Shift+Tab",
    ),
    (AgentInput, "agent.interrupt", "Interrupt agent", "Escape"),
];

/// The Electron defaults.
pub fn default_shortcuts() -> Vec<Shortcut> {
    DEFAULTS
        .iter()
        .map(|(_, action, label, chord)| Shortcut {
            action: (*action).to_string(),
            label: (*label).to_string(),
            chord: Some((*chord).to_string()),
        })
        .collect()
}

/// The help section of an action. An unknown action is General.
pub fn shortcut_section(action: &str) -> ShortcutSection {
    DEFAULTS
        .iter()
        .find(|(_, id, _, _)| *id == action)
        .map_or(General, |(section, ..)| *section)
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
