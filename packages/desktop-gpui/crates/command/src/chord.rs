//! Electron combo strings and their GPUI forms.
//!
//! A combo is the Electron string of `keyboard-shortcuts.ts`: modifiers and one key joined by
//! `+`, for example `Cmd+Shift+P`. A chord is one or more combos joined by one space. Parsing
//! accepts the modifier names in any case, and `Mod` and `Meta` mean Cmd (macOS only client).

use gpui_kit::Keystroke;
use gpui_kit::component::kbd::Kbd;

/// A parsed combo: the modifiers and the GPUI key name.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Combo {
    ctrl: bool,
    alt: bool,
    shift: bool,
    cmd: bool,
    /// The GPUI key name, or `digit` for the Electron `Digit` wildcard (any of 1 to 9).
    key: String,
}

const DIGIT: &str = "digit";

fn key_name(part: &str) -> Option<String> {
    let lower = part.to_ascii_lowercase();
    let named = match lower.as_str() {
        "digit" => DIGIT,
        "space" => "space",
        "enter" | "return" => "enter",
        "backspace" => "backspace",
        "escape" | "esc" => "escape",
        "arrowleft" | "left" => "left",
        "arrowright" | "right" => "right",
        "arrowup" | "up" => "up",
        "arrowdown" | "down" => "down",
        "tab" => "tab",
        "delete" => "delete",
        "home" => "home",
        "end" => "end",
        "pageup" => "pageup",
        "pagedown" => "pagedown",
        "insert" => "insert",
        _ => "",
    };
    if !named.is_empty() {
        return Some(named.to_string());
    }
    let mut chars = lower.chars();
    let first = chars.next()?;
    if chars.next().is_none() {
        let known = first.is_ascii_alphanumeric() || "-=\\[];',.`/?".contains(first);
        return known.then(|| first.to_string());
    }
    // F1 to F12.
    let number = lower.strip_prefix('f')?.parse::<u8>().ok()?;
    (1..=12).contains(&number).then_some(lower)
}

fn parse_combo(text: &str) -> Option<Combo> {
    let mut combo = Combo {
        ctrl: false,
        alt: false,
        shift: false,
        cmd: false,
        key: String::new(),
    };
    let mut key: Option<String> = None;
    for part in text.split('+') {
        match part.to_ascii_lowercase().as_str() {
            "" => return None,
            "cmd" | "meta" | "mod" | "command" => combo.cmd = true,
            "ctrl" | "control" => combo.ctrl = true,
            "alt" | "option" => combo.alt = true,
            "shift" => combo.shift = true,
            _ => {
                if key.is_some() {
                    return None;
                }
                key = Some(key_name(part)?);
            }
        }
    }
    combo.key = key?;
    Some(combo)
}

fn parse_chord(chord: &str) -> Option<Vec<Combo>> {
    let combos: Option<Vec<Combo>> = chord.split_whitespace().map(parse_combo).collect();
    combos.filter(|combos| !combos.is_empty())
}

/// Whether the chord parses. An empty string is not a chord.
pub fn is_valid_chord(chord: &str) -> bool {
    parse_chord(chord).is_some()
}

fn keystroke_for(combo: &Combo, key: &str) -> String {
    let mut text = String::new();
    for (on, name) in [
        (combo.ctrl, "ctrl"),
        (combo.alt, "alt"),
        (combo.shift, "shift"),
        (combo.cmd, "cmd"),
    ] {
        if on {
            text.push_str(name);
            text.push('-');
        }
    }
    text.push_str(key);
    text
}

/// The GPUI binding strings for a chord, for `KeyBinding::new`, for example `cmd-shift-p`. A
/// multi-key chord gives `cmd-k cmd-b`. The Electron `Digit` wildcard gives nine strings, for
/// 1 to 9, in that order, so the index is the position plus one. An invalid chord gives none.
pub fn gpui_keystrokes(chord: &str) -> Vec<String> {
    let Some(combos) = parse_chord(chord) else {
        return Vec::new();
    };
    let mut results = vec![String::new()];
    for combo in &combos {
        let keys: Vec<String> = if combo.key == DIGIT {
            (1..=9).map(|digit| digit.to_string()).collect()
        } else {
            vec![combo.key.clone()]
        };
        let mut next = Vec::with_capacity(results.len() * keys.len());
        for prefix in &results {
            for key in &keys {
                let stroke = keystroke_for(combo, key);
                next.push(if prefix.is_empty() {
                    stroke
                } else {
                    format!("{prefix} {stroke}")
                });
            }
        }
        results = next;
    }
    results
}

/// The macOS display text of each combo of a chord, for example `["⇧⌘P"]`. The `Digit`
/// wildcard shows as `1-9`. An invalid chord gives none.
pub fn display_combos(chord: &str) -> Vec<String> {
    let Some(combos) = parse_chord(chord) else {
        return Vec::new();
    };
    combos
        .iter()
        .filter_map(|combo| {
            let wildcard = combo.key == DIGIT;
            let key = if wildcard { "1" } else { combo.key.as_str() };
            let stroke = Keystroke::parse(&keystroke_for(combo, key)).ok()?;
            let text = Kbd::format(&stroke);
            Some(if wildcard {
                format!("{}1-9", text.strip_suffix('1').unwrap_or(&text))
            } else {
                text
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_combo() {
        assert_eq!(gpui_keystrokes("Cmd+Shift+P"), ["shift-cmd-p"]);
        assert_eq!(gpui_keystrokes("mod+shift+p"), ["shift-cmd-p"]);
        assert_eq!(display_combos("Cmd+Shift+P"), ["⇧⌘P"]);
    }

    #[test]
    fn multi_key_chord() {
        assert_eq!(gpui_keystrokes("Cmd+K Cmd+B"), ["cmd-k cmd-b"]);
        assert_eq!(display_combos("Cmd+K Cmd+B"), ["⌘K", "⌘B"]);
    }

    #[test]
    fn digit_wildcard_expands() {
        let strokes = gpui_keystrokes("Cmd+Alt+Digit");
        assert_eq!(strokes.len(), 9);
        assert_eq!(strokes[0], "alt-cmd-1");
        assert_eq!(strokes[8], "alt-cmd-9");
        assert_eq!(display_combos("Cmd+Alt+Digit"), ["⌥⌘1-9"]);
    }

    #[test]
    fn invalid_chords_give_nothing() {
        for chord in [
            "",
            "  ",
            "Cmd+",
            "Cmd++P",
            "Cmd+Shift",
            "Cmd+P+Q",
            "Cmd+Nope",
            "F13",
        ] {
            assert!(!is_valid_chord(chord), "{chord:?}");
            assert!(gpui_keystrokes(chord).is_empty(), "{chord:?}");
            assert!(display_combos(chord).is_empty(), "{chord:?}");
        }
    }

    #[test]
    fn punctuation_and_named_keys() {
        assert_eq!(gpui_keystrokes("Cmd+\\"), ["cmd-\\"]);
        assert_eq!(gpui_keystrokes("Cmd+,"), ["cmd-,"]);
        assert_eq!(gpui_keystrokes("Cmd+Shift+ArrowLeft"), ["shift-cmd-left"]);
        assert_eq!(gpui_keystrokes("Escape"), ["escape"]);
        assert_eq!(gpui_keystrokes("Shift+Tab"), ["shift-tab"]);
        assert_eq!(
            gpui_keystrokes("Cmd+Shift+Backspace"),
            ["shift-cmd-backspace"]
        );
    }
}
