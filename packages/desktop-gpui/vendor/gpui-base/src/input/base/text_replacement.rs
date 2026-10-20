//! macOS Text Replacement (System Settings > Keyboard > Text Replacements).
//!
//! AppKit text views and Chromium swap a typed shortcut for its phrase when a
//! word boundary follows it. GPUI draws its own input, so the input asks for
//! the user's list itself (Bachuc patch, see `BACHUC-PATCH.md`).

/// Characters that end a word and trigger a replacement, as in AppKit.
pub(crate) fn is_boundary(c: char) -> bool {
    c.is_whitespace() || matches!(c, '.' | ',' | '!' | '?' | ';' | ':')
}

/// The edit for one typed `typed` after `before` (the text before the cursor):
/// the byte length of the shortcut to replace, and the text that replaces it
/// together with the typed boundary. `None` when nothing matches.
pub(crate) fn replacement_for(
    before: &str,
    typed: &str,
    items: &[(String, String)],
) -> Option<(usize, String)> {
    let mut chars = typed.chars();
    let boundary = chars.next()?;
    if chars.next().is_some() || !is_boundary(boundary) {
        return None;
    }
    let word_start = before
        .char_indices()
        .rev()
        .find(|(_, c)| c.is_whitespace())
        .map_or(0, |(i, c)| i + c.len_utf8());
    let word = &before[word_start..];
    let phrase = phrase_for(word, items)?;
    Some((word.len(), format!("{phrase}{typed}")))
}

/// The phrase of the whole-word shortcut `word`, when one exists and differs from it.
pub(crate) fn phrase_for<'a>(word: &str, items: &'a [(String, String)]) -> Option<&'a str> {
    if word.is_empty() {
        return None;
    }
    items
        .iter()
        .find(|(shortcut, phrase)| shortcut == word && phrase != word)
        .map(|(_, phrase)| phrase.as_str())
}

#[cfg(test)]
mod tests {
    use super::{phrase_for, replacement_for};

    fn items() -> Vec<(String, String)> {
        vec![
            ("omw".into(), "On my way!".into()),
            ("vn".into(), "Việt Nam".into()),
        ]
    }

    #[test]
    fn a_shortcut_before_a_space_becomes_its_phrase() {
        assert_eq!(
            replacement_for("I am omw", " ", &items()),
            Some((3, "On my way! ".into()))
        );
    }

    #[test]
    fn punctuation_also_ends_the_shortcut() {
        assert_eq!(
            replacement_for("vn", ",", &items()),
            Some((2, "Việt Nam,".into()))
        );
    }

    #[test]
    fn only_the_whole_word_matches() {
        assert_eq!(replacement_for("xomw", " ", &items()), None);
        assert_eq!(replacement_for("Omw", " ", &items()), None);
    }

    #[test]
    fn a_whole_shortcut_has_a_phrase_before_any_boundary() {
        assert_eq!(phrase_for("omw", &items()), Some("On my way!"));
        assert_eq!(phrase_for("om", &items()), None);
        assert_eq!(phrase_for("", &items()), None);
    }

    #[test]
    fn a_letter_or_a_paste_never_triggers() {
        assert_eq!(replacement_for("omw", "a", &items()), None);
        assert_eq!(replacement_for("omw", "  ", &items()), None);
        assert_eq!(replacement_for("omw ", " ", &items()), None);
        assert_eq!(replacement_for("", " ", &items()), None);
    }
}
