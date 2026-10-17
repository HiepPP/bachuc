use bachuc_command::{
    Shortcut, default_shortcuts, gpui_keystrokes, load_shortcuts_from, save_overrides_to,
};
use gpui_kit::Keystroke;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// A fixed folder per test, with the stored file removed first. The folder stays for the next run.
fn folder(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join("bachuc-command-tests").join(name);
    std::fs::create_dir_all(&dir).unwrap();
    for file in ["shortcuts.json", "shortcuts.json.tmp"] {
        let _ = std::fs::remove_file(dir.join(file));
    }
    dir
}

fn write(dir: &Path, text: &str) {
    std::fs::write(dir.join("shortcuts.json"), text).unwrap();
}

fn chord(shortcuts: &[Shortcut], action: &str) -> Option<String> {
    shortcuts
        .iter()
        .find(|s| s.action == action)
        .unwrap_or_else(|| panic!("no action {action}"))
        .chord
        .clone()
}

#[test]
fn default_table_matches_electron() {
    let defaults = default_shortcuts();
    let actions: HashSet<_> = defaults.iter().map(|s| &s.action).collect();
    assert_eq!(actions.len(), defaults.len(), "action ids are unique");
    assert_eq!(
        chord(&defaults, "command-center.toggle").as_deref(),
        Some("Cmd+K")
    );
    assert_eq!(
        chord(&defaults, "workspace.pane.split.down").as_deref(),
        Some("Cmd+Shift+\\")
    );
    assert_eq!(
        chord(&defaults, "workspace.tab.navigate.index").as_deref(),
        Some("Cmd+Alt+Digit")
    );
    assert_eq!(
        defaults
            .iter()
            .find(|s| s.action == "sidebar.toggle.right")
            .unwrap()
            .label,
        "Toggle Explorer sidebar"
    );
    assert!(defaults.iter().all(|s| s.chord.is_some()));
}

#[test]
fn every_default_chord_parses_as_a_gpui_keystroke() {
    for shortcut in default_shortcuts() {
        let chord = shortcut.chord.unwrap();
        let strokes = gpui_keystrokes(&chord);
        assert!(!strokes.is_empty(), "{} {chord}", shortcut.action);
        for stroke in strokes {
            for part in stroke.split(' ') {
                Keystroke::parse(part)
                    .unwrap_or_else(|e| panic!("{} {stroke:?}: {e}", shortcut.action));
            }
        }
    }
}

#[test]
fn no_file_gives_the_defaults() {
    let dir = folder("no-file");
    assert_eq!(load_shortcuts_from(&dir), default_shortcuts());
}

#[test]
fn missing_key_keeps_the_default() {
    let dir = folder("missing-key");
    write(&dir, r#"{ "settings.toggle": "Cmd+Shift+," }"#);
    let loaded = load_shortcuts_from(&dir);
    assert_eq!(
        chord(&loaded, "settings.toggle").as_deref(),
        Some("Cmd+Shift+,")
    );
    assert_eq!(chord(&loaded, "theme.cycle").as_deref(), Some("Cmd+Alt+T"));
}

#[test]
fn null_and_empty_overrides_load_as_unassigned() {
    let dir = folder("unassigned");
    write(
        &dir,
        r#"{ "theme.cycle": null, "host.cycle": "", "settings.toggle": "   " }"#,
    );
    let loaded = load_shortcuts_from(&dir);
    assert_eq!(chord(&loaded, "theme.cycle"), None);
    assert_eq!(chord(&loaded, "host.cycle"), None);
    assert_eq!(chord(&loaded, "settings.toggle"), None);
    // The action stays in the table with its label, so the settings screen can list it.
    assert!(
        loaded
            .iter()
            .any(|s| s.action == "theme.cycle" && !s.label.is_empty())
    );
    // Nothing binds an unassigned action.
    let binds: Vec<String> = loaded
        .iter()
        .filter_map(|s| s.chord.as_deref())
        .flat_map(gpui_keystrokes)
        .collect();
    assert!(!binds.contains(&"alt-cmd-t".to_string()));
}

#[test]
fn multi_key_chord_survives_a_restart() {
    let dir = folder("multi-key");
    let mut shortcuts = default_shortcuts();
    for shortcut in &mut shortcuts {
        if shortcut.action == "sidebar.toggle.left" {
            shortcut.chord = Some("Cmd+K Cmd+B".to_string());
        }
        if shortcut.action == "agent.interrupt" {
            shortcut.chord = None;
        }
    }
    save_overrides_to(&dir, &shortcuts).unwrap();
    let loaded = load_shortcuts_from(&dir);
    assert_eq!(loaded, shortcuts);
    assert_eq!(
        gpui_keystrokes("Cmd+K Cmd+B"),
        vec!["cmd-k cmd-b".to_string()]
    );
}

#[test]
fn save_writes_only_the_differences() {
    let dir = folder("differences");
    let mut shortcuts = default_shortcuts();
    save_overrides_to(&dir, &shortcuts).unwrap();
    let text = std::fs::read_to_string(dir.join("shortcuts.json")).unwrap();
    assert_eq!(text.trim(), "{}");
    shortcuts[0].chord = None;
    save_overrides_to(&dir, &shortcuts).unwrap();
    let text = std::fs::read_to_string(dir.join("shortcuts.json")).unwrap();
    assert!(
        text.contains(&format!("\"{}\": null", shortcuts[0].action)),
        "{text}"
    );
    assert!(!dir.join("shortcuts.json.tmp").exists());
}

#[test]
fn corrupt_file_loads_the_defaults_and_stays() {
    for (name, text) in [
        ("corrupt-json", "{ not json"),
        ("corrupt-array", "[1, 2]"),
        ("corrupt-empty", ""),
    ] {
        let dir = folder(name);
        write(&dir, text);
        assert_eq!(load_shortcuts_from(&dir), default_shortcuts(), "{name}");
        let kept = std::fs::read_to_string(dir.join("shortcuts.json")).unwrap();
        assert_eq!(kept, text, "{name}: the file is untouched");
    }
}

#[test]
fn unusable_values_keep_the_default() {
    let dir = folder("unusable");
    write(
        &dir,
        r#"{ "theme.cycle": 7, "host.cycle": "Cmd+Nope", "settings.toggle": true, "gone.action": "Cmd+Q" }"#,
    );
    assert_eq!(load_shortcuts_from(&dir), default_shortcuts());
}
