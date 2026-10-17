//! Key press to terminal bytes.

use gpui_kit::Keystroke;

/// The bytes a terminal expects for `keystroke`, or `None` when the key sends nothing. Cmd
/// shortcuts send nothing; the view handles copy and paste before it calls this.
/// `app_cursor` is the DECCKM mode: arrows and Home/End use `ESC O` instead of `ESC [`.
pub fn key_bytes(keystroke: &Keystroke, app_cursor: bool) -> Option<Vec<u8>> {
    let modifiers = &keystroke.modifiers;
    if modifiers.platform || modifiers.function {
        return None;
    }
    let key = keystroke.key.as_str();

    let named: Option<Vec<u8>> = match key {
        "enter" => Some(b"\r".to_vec()),
        "backspace" => Some(b"\x7f".to_vec()),
        "tab" if modifiers.shift => Some(b"\x1b[Z".to_vec()),
        "tab" => Some(b"\t".to_vec()),
        "escape" => Some(b"\x1b".to_vec()),
        "up" => Some(cursor_key(b'A', app_cursor)),
        "down" => Some(cursor_key(b'B', app_cursor)),
        "right" => Some(cursor_key(b'C', app_cursor)),
        "left" => Some(cursor_key(b'D', app_cursor)),
        "home" => Some(cursor_key(b'H', app_cursor)),
        "end" => Some(cursor_key(b'F', app_cursor)),
        "pageup" => Some(b"\x1b[5~".to_vec()),
        "pagedown" => Some(b"\x1b[6~".to_vec()),
        "delete" => Some(b"\x1b[3~".to_vec()),
        _ => None,
    };
    if let Some(bytes) = named {
        return Some(with_option(bytes, modifiers.alt));
    }

    if modifiers.control {
        let byte = control_byte(key)?;
        return Some(with_option(vec![byte], modifiers.alt));
    }

    if modifiers.alt {
        // Option changes the typed character on macOS, so send the key itself.
        let mut text = if key == "space" {
            " ".to_string()
        } else {
            key.to_string()
        };
        if text.chars().count() != 1 {
            return None;
        }
        if modifiers.shift {
            text = text.to_uppercase();
        }
        return Some(with_option(text.into_bytes(), true));
    }

    let text = keystroke
        .key_char
        .as_deref()
        .filter(|text| !text.is_empty())?;
    Some(text.as_bytes().to_vec())
}

/// The bytes for pasted text. With bracketed paste on, the app gets the text between markers and
/// can tell it from typing; Escape is stripped so the text cannot close the markers early.
/// Without it, line breaks become Enter presses.
pub fn paste_bytes(text: &str, bracketed: bool) -> Vec<u8> {
    if bracketed {
        let mut bytes = b"\x1b[200~".to_vec();
        bytes.extend(text.replace('\x1b', "").into_bytes());
        bytes.extend_from_slice(b"\x1b[201~");
        bytes
    } else {
        text.replace("\r\n", "\r").replace('\n', "\r").into_bytes()
    }
}

fn cursor_key(final_byte: u8, app_cursor: bool) -> Vec<u8> {
    vec![0x1b, if app_cursor { b'O' } else { b'[' }, final_byte]
}

/// Option sends Escape plus the key.
fn with_option(mut bytes: Vec<u8>, alt: bool) -> Vec<u8> {
    if alt {
        bytes.insert(0, 0x1b);
    }
    bytes
}

fn control_byte(key: &str) -> Option<u8> {
    if key == "space" {
        return Some(0);
    }
    let mut chars = key.chars();
    let c = chars
        .next()
        .filter(|_| chars.next().is_none())?
        .to_ascii_lowercase();
    match c {
        'a'..='z' => Some(c as u8 - b'a' + 1),
        '@' | '2' => Some(0x00),
        '[' | '3' => Some(0x1b),
        '\\' | '4' => Some(0x1c),
        ']' | '5' => Some(0x1d),
        '^' | '6' => Some(0x1e),
        '_' | '-' | '/' | '7' => Some(0x1f),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn press(source: &str, key_char: Option<&str>) -> Keystroke {
        let mut keystroke = Keystroke::parse(source).unwrap();
        keystroke.key_char = key_char.map(str::to_string);
        keystroke
    }

    fn bytes(source: &str, key_char: Option<&str>) -> Option<Vec<u8>> {
        key_bytes(&press(source, key_char), false)
    }

    #[test]
    fn printable_keys_send_their_character() {
        assert_eq!(bytes("a", Some("a")), Some(b"a".to_vec()));
        assert_eq!(bytes("shift-a", Some("A")), Some(b"A".to_vec()));
        assert_eq!(bytes("space", Some(" ")), Some(b" ".to_vec()));
        assert_eq!(bytes("1", Some("1")), Some(b"1".to_vec()));
        assert_eq!(bytes("é", Some("é")), Some("é".as_bytes().to_vec()));
    }

    #[test]
    fn editing_keys_send_their_control_codes() {
        assert_eq!(bytes("enter", None), Some(b"\r".to_vec()));
        assert_eq!(bytes("backspace", None), Some(b"\x7f".to_vec()));
        assert_eq!(bytes("tab", None), Some(b"\t".to_vec()));
        assert_eq!(bytes("shift-tab", None), Some(b"\x1b[Z".to_vec()));
        assert_eq!(bytes("escape", None), Some(b"\x1b".to_vec()));
        assert_eq!(bytes("delete", None), Some(b"\x1b[3~".to_vec()));
    }

    #[test]
    fn navigation_keys_follow_the_cursor_mode() {
        assert_eq!(bytes("up", None), Some(b"\x1b[A".to_vec()));
        assert_eq!(bytes("left", None), Some(b"\x1b[D".to_vec()));
        assert_eq!(bytes("home", None), Some(b"\x1b[H".to_vec()));
        assert_eq!(bytes("end", None), Some(b"\x1b[F".to_vec()));
        assert_eq!(bytes("pageup", None), Some(b"\x1b[5~".to_vec()));
        assert_eq!(bytes("pagedown", None), Some(b"\x1b[6~".to_vec()));
        let up = press("up", None);
        assert_eq!(key_bytes(&up, true), Some(b"\x1bOA".to_vec()));
        let home = press("home", None);
        assert_eq!(key_bytes(&home, true), Some(b"\x1bOH".to_vec()));
    }

    #[test]
    fn control_letters_send_codes_1_to_26() {
        assert_eq!(bytes("ctrl-a", None), Some(vec![1]));
        assert_eq!(bytes("ctrl-c", Some("c")), Some(vec![3]));
        assert_eq!(bytes("ctrl-z", None), Some(vec![26]));
        assert_eq!(bytes("ctrl-[", None), Some(vec![0x1b]));
        assert_eq!(bytes("ctrl-space", None), Some(vec![0]));
        assert_eq!(bytes("ctrl-1", None), None);
    }

    #[test]
    fn option_sends_escape_and_the_base_key() {
        assert_eq!(bytes("alt-b", Some("∫")), Some(b"\x1bb".to_vec()));
        assert_eq!(bytes("alt-shift-f", Some("Ï")), Some(b"\x1bF".to_vec()));
        assert_eq!(bytes("alt-left", None), Some(b"\x1b\x1b[D".to_vec()));
        assert_eq!(bytes("alt-backspace", None), Some(b"\x1b\x7f".to_vec()));
        assert_eq!(bytes("ctrl-alt-a", None), Some(vec![0x1b, 1]));
    }

    #[test]
    fn paste_is_bracketed_only_when_the_app_asks() {
        assert_eq!(
            paste_bytes("a\nb", true),
            b"\x1b[200~a\nb\x1b[201~".to_vec()
        );
        assert_eq!(
            paste_bytes("x\x1b[201~y", true),
            b"\x1b[200~x[201~y\x1b[201~".to_vec()
        );
        assert_eq!(paste_bytes("a\r\nb\nc", false), b"a\rb\rc".to_vec());
    }

    #[test]
    fn command_shortcuts_send_nothing() {
        assert_eq!(bytes("cmd-c", None), None);
        assert_eq!(bytes("cmd-v", None), None);
        assert_eq!(bytes("cmd-left", None), None);
    }
}
