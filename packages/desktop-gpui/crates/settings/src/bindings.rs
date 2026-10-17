//! Editing the shortcut table: recording a chord, conflicts, clear, and reset.
//!
//! The table and its storage live in `bachuc_command`; this module changes a `Vec<Shortcut>` in
//! memory and the settings view stores it with `save_overrides`.

use bachuc_command::{Shortcut, default_shortcuts, gpui_keystrokes, is_valid_chord};
use gpui_kit::Keystroke;

/// The help sections of the Electron shortcuts page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Section {
    General,
    Workspaces,
    TabsPanes,
    Layout,
    AgentInput,
}

impl Section {
    pub const ALL: [Section; 5] = [
        Section::General,
        Section::Workspaces,
        Section::TabsPanes,
        Section::Layout,
        Section::AgentInput,
    ];

    /// `settings.shortcuts.sections.*` of the Electron app.
    pub fn title(self) -> &'static str {
        match self {
            Section::General => "General",
            Section::Workspaces => "Projects & Workspaces",
            Section::TabsPanes => "Tabs & Panes",
            Section::Layout => "Layout",
            Section::AgentInput => "Agent Input",
        }
    }
}

/// The help section of an action.
pub fn section_of(action: &str) -> Section {
    if action.starts_with("workspace.navigate") || action == "workspace.archive" {
        Section::Workspaces
    } else if action.starts_with("workspace.") {
        Section::TabsPanes
    } else if action.starts_with("sidebar.") {
        Section::Layout
    } else if action.starts_with("message-input.") || action.starts_with("agent.") {
        Section::AgentInput
    } else {
        Section::General
    }
}

/// The Electron combo string for a key press, for example `Cmd+Shift+P`, or None for a key that
/// is not part of the Electron set.
pub fn combo_from_keystroke(keystroke: &Keystroke) -> Option<String> {
    let key = electron_key(&keystroke.key)?;
    let modifiers = &keystroke.modifiers;
    let mut combo = String::new();
    for (on, name) in [
        (modifiers.platform, "Cmd"),
        (modifiers.control, "Ctrl"),
        (modifiers.alt, "Alt"),
        (modifiers.shift, "Shift"),
    ] {
        if on {
            combo.push_str(name);
            combo.push('+');
        }
    }
    combo.push_str(&key);
    is_valid_chord(&combo).then_some(combo)
}

fn electron_key(key: &str) -> Option<String> {
    Some(match key {
        "left" => "ArrowLeft".into(),
        "right" => "ArrowRight".into(),
        "up" => "ArrowUp".into(),
        "down" => "ArrowDown".into(),
        "escape" => "Escape".into(),
        "enter" => "Enter".into(),
        "space" => "Space".into(),
        "tab" => "Tab".into(),
        "backspace" => "Backspace".into(),
        "delete" => "Delete".into(),
        "home" => "Home".into(),
        "end" => "End".into(),
        "pageup" => "PageUp".into(),
        "pagedown" => "PageDown".into(),
        "insert" => "Insert".into(),
        other => {
            let mut chars = other.chars();
            let first = chars.next()?;
            if chars.next().is_none() {
                first.to_ascii_uppercase().to_string()
            } else if other.starts_with('f') && other[1..].chars().all(|c| c.is_ascii_digit()) {
                other.to_ascii_uppercase()
            } else {
                return None;
            }
        }
    })
}

/// Whether two chords can fire on the same key press. The `Digit` wildcard overlaps each digit.
pub fn overlaps(a: &str, b: &str) -> bool {
    let a = gpui_keystrokes(a);
    gpui_keystrokes(b).iter().any(|stroke| a.contains(stroke))
}

/// The other action that already uses `chord`, if any.
pub fn conflict<'a>(shortcuts: &'a [Shortcut], action: &str, chord: &str) -> Option<&'a Shortcut> {
    shortcuts.iter().find(|other| {
        other.action != action
            && other
                .chord
                .as_deref()
                .is_some_and(|existing| overlaps(existing, chord))
    })
}

/// Sets the chord of an action; None unassigns it. Returns false for an unknown action.
pub fn set_chord(shortcuts: &mut [Shortcut], action: &str, chord: Option<String>) -> bool {
    match shortcuts.iter_mut().find(|s| s.action == action) {
        Some(shortcut) => {
            shortcut.chord = chord;
            true
        }
        None => false,
    }
}

/// Restores the default chord of one action.
pub fn reset(shortcuts: &mut [Shortcut], action: &str) -> bool {
    let default = default_shortcuts().into_iter().find(|d| d.action == action);
    match default {
        Some(default) => set_chord(shortcuts, action, default.chord),
        None => false,
    }
}

/// Restores every default chord.
pub fn reset_all(shortcuts: &mut [Shortcut]) {
    for default in default_shortcuts() {
        set_chord(shortcuts, &default.action, default.chord);
    }
}

/// Whether the action differs from its default, which is when Reset applies.
pub fn is_overridden(shortcuts: &[Shortcut], action: &str) -> bool {
    let default = default_shortcuts().into_iter().find(|d| d.action == action);
    let current = shortcuts.iter().find(|s| s.action == action);
    match (default, current) {
        (Some(default), Some(current)) => default.chord != current.chord,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bachuc_command::{load_shortcuts_from, save_overrides_to};

    fn keystroke(text: &str) -> Keystroke {
        Keystroke::parse(text).unwrap()
    }

    fn chord_of(shortcuts: &[Shortcut], action: &str) -> Option<String> {
        shortcuts
            .iter()
            .find(|s| s.action == action)
            .and_then(|s| s.chord.clone())
    }

    #[test]
    fn key_presses_become_electron_combos() {
        for (stroke, combo) in [
            ("cmd-shift-p", "Cmd+Shift+P"),
            ("ctrl-alt-t", "Ctrl+Alt+T"),
            ("cmd-left", "Cmd+ArrowLeft"),
            ("shift-tab", "Shift+Tab"),
            ("escape", "Escape"),
            ("cmd-\\", "Cmd+\\"),
            ("cmd-,", "Cmd+,"),
            ("f5", "F5"),
            ("cmd-1", "Cmd+1"),
        ] {
            assert_eq!(
                combo_from_keystroke(&keystroke(stroke)).as_deref(),
                Some(combo),
                "{stroke}"
            );
        }
    }

    #[test]
    fn recorded_combos_parse_back_to_the_same_keystroke() {
        for stroke in ["cmd-shift-p", "alt-cmd-t", "cmd-down", "cmd-]"] {
            let combo = combo_from_keystroke(&keystroke(stroke)).unwrap();
            let parsed = gpui_keystrokes(&combo);
            assert_eq!(parsed.len(), 1);
            let round = Keystroke::parse(&parsed[0]).unwrap();
            assert_eq!(round, keystroke(stroke), "{stroke} via {combo}");
        }
    }

    #[test]
    fn sections_follow_the_electron_help() {
        assert_eq!(section_of("command-center.toggle"), Section::General);
        assert_eq!(section_of("host.cycle"), Section::General);
        assert_eq!(section_of("workspace.navigate.prev"), Section::Workspaces);
        assert_eq!(section_of("workspace.archive"), Section::Workspaces);
        assert_eq!(section_of("workspace.tab.menu.open"), Section::TabsPanes);
        assert_eq!(section_of("workspace.pane.close"), Section::TabsPanes);
        assert_eq!(section_of("sidebar.toggle.both"), Section::Layout);
        assert_eq!(section_of("agent.interrupt"), Section::AgentInput);
        for shortcut in default_shortcuts() {
            // Every default lands in a known section; the page shows them all.
            assert!(Section::ALL.contains(&section_of(&shortcut.action)));
        }
    }

    #[test]
    fn a_clashing_chord_names_the_other_action() {
        let shortcuts = default_shortcuts();
        let clash = conflict(&shortcuts, "settings.toggle", "Cmd+K").unwrap();
        assert_eq!(clash.action, "command-center.toggle");
        // Another spelling of the same keys clashes too.
        assert!(conflict(&shortcuts, "settings.toggle", "mod+k").is_some());
        // An action does not clash with itself.
        assert!(conflict(&shortcuts, "command-center.toggle", "Cmd+K").is_none());
        assert!(conflict(&shortcuts, "settings.toggle", "Cmd+Alt+Shift+Q").is_none());
    }

    #[test]
    fn the_digit_wildcard_clashes_with_a_digit() {
        let shortcuts = default_shortcuts();
        let clash = conflict(&shortcuts, "settings.toggle", "Cmd+Alt+3").unwrap();
        assert_eq!(clash.action, "workspace.tab.navigate.index");
    }

    #[test]
    fn a_cleared_chord_no_longer_clashes() {
        let mut shortcuts = default_shortcuts();
        assert!(set_chord(&mut shortcuts, "command-center.toggle", None));
        assert_eq!(chord_of(&shortcuts, "command-center.toggle"), None);
        assert!(conflict(&shortcuts, "settings.toggle", "Cmd+K").is_none());
        assert!(is_overridden(&shortcuts, "command-center.toggle"));
    }

    #[test]
    fn reset_restores_one_action_or_all() {
        let mut shortcuts = default_shortcuts();
        set_chord(&mut shortcuts, "theme.cycle", Some("Cmd+Alt+Y".into()));
        set_chord(&mut shortcuts, "host.cycle", None);
        assert!(is_overridden(&shortcuts, "theme.cycle"));
        assert!(reset(&mut shortcuts, "theme.cycle"));
        assert_eq!(
            chord_of(&shortcuts, "theme.cycle").as_deref(),
            Some("Cmd+Alt+T")
        );
        assert!(is_overridden(&shortcuts, "host.cycle"));
        reset_all(&mut shortcuts);
        assert_eq!(shortcuts, default_shortcuts());
        assert!(!reset(&mut shortcuts, "no.such.action"));
    }

    #[test]
    fn rebind_and_clear_survive_a_reload() {
        let dir =
            std::env::temp_dir().join(format!("bachuc-settings-bindings-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut shortcuts = load_shortcuts_from(&dir);
        set_chord(&mut shortcuts, "theme.cycle", Some("Cmd+Alt+Y".into()));
        set_chord(&mut shortcuts, "host.cycle", None);
        save_overrides_to(&dir, &shortcuts).unwrap();
        let reloaded = load_shortcuts_from(&dir);
        assert_eq!(
            chord_of(&reloaded, "theme.cycle").as_deref(),
            Some("Cmd+Alt+Y")
        );
        assert_eq!(chord_of(&reloaded, "host.cycle"), None);
        // Reset removes the keys, so the file holds only what still differs.
        let mut reloaded = reloaded;
        reset_all(&mut reloaded);
        save_overrides_to(&dir, &reloaded).unwrap();
        let text = std::fs::read_to_string(dir.join("shortcuts.json")).unwrap();
        assert_eq!(text.trim(), "{}");
    }
}
