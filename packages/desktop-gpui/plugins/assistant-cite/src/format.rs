//! The text the agent gets for a quote, and the reverse (`shared/format.ts`). JS counts and trims
//! strings by UTF-16 code units and its own whitespace set, so this port does the same: a chip
//! made here must read the same in Electron.

pub(crate) const QUOTE_LEAD: &str = "Quoted from your earlier reply:";
/// The longest quote, in UTF-16 code units (JS `length`).
pub(crate) const MAX_QUOTE_LENGTH: usize = 4000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum QuoteFailure {
    Empty,
    TooLong,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FormattedQuote {
    pub(crate) quote: String,
    pub(crate) text: String,
}

/// The JS whitespace set (`String.prototype.trim` and regex `\s`). It differs from Rust's
/// `char::is_whitespace` by U+FEFF, which JS adds, and U+0085, which JS leaves out.
pub(crate) fn is_js_space(c: char) -> bool {
    c == '\u{feff}' || (c.is_whitespace() && c != '\u{85}')
}

pub(crate) fn js_trim(text: &str) -> &str {
    text.trim_matches(is_js_space)
}

/// JS `length`: the number of UTF-16 code units.
pub(crate) fn utf16_len(text: &str) -> usize {
    text.encode_utf16().count()
}

/// `text.split(/\r?\n/)`: a `\r` right before a `\n` goes with it; a lone `\r` stays.
fn split_lines(text: &str) -> Vec<&str> {
    let mut lines: Vec<&str> = text.split('\n').collect();
    let last = lines.len() - 1;
    for line in &mut lines[..last] {
        *line = line.strip_suffix('\r').unwrap_or(line);
    }
    lines
}

/// The text the agent gets: the lead line, then the quote as a Markdown blockquote, so the agent
/// reads it as reference text. The host appends the chip comment after it.
pub(crate) fn format_quote(selection: &str) -> Result<FormattedQuote, QuoteFailure> {
    let quote = js_trim(selection);
    if quote.is_empty() {
        return Err(QuoteFailure::Empty);
    }
    if utf16_len(quote) > MAX_QUOTE_LENGTH {
        return Err(QuoteFailure::TooLong);
    }
    let blockquote = split_lines(quote)
        .into_iter()
        .map(|line| {
            if line.is_empty() {
                ">".to_string()
            } else {
                format!("> {line}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    Ok(FormattedQuote {
        quote: quote.to_string(),
        text: format!("{QUOTE_LEAD}\n\n{blockquote}"),
    })
}

/// Reverses `format_quote`, or returns `None` for text it did not make.
pub(crate) fn quote_from_text(text: &str) -> Option<String> {
    let body = text.strip_prefix(&format!("{QUOTE_LEAD}\n\n"))?;
    Some(
        body.split('\n')
            .map(|line| {
                line.strip_prefix('>')
                    .map(|rest| rest.strip_prefix(' ').unwrap_or(rest))
                    .unwrap_or(line)
            })
            .collect::<Vec<_>>()
            .join("\n"),
    )
}

/// The host highlights rendered text inside one block, so the jump back highlights the first
/// non-empty line of the quote with its Markdown syntax removed.
pub(crate) fn highlight_text(quote: &str) -> Option<String> {
    let line = split_lines(quote)
        .into_iter()
        .map(js_trim)
        .find(|line| !line.is_empty())?;
    let line = strip_quote_markers(line);
    let line = strip_heading(line);
    let line = strip_list_marker(line);
    let plain = strip_emphasis(&unwrap_links(line));
    let plain = js_trim(&plain);
    (!plain.is_empty()).then(|| plain.to_string())
}

/// `/^(?:>\s*)+/`
fn strip_quote_markers(mut line: &str) -> &str {
    while let Some(rest) = line.strip_prefix('>') {
        line = rest.trim_start_matches(is_js_space);
    }
    line
}

/// `/^#{1,6}\s+/`
fn strip_heading(line: &str) -> &str {
    let rest = line.trim_start_matches('#');
    let hashes = line.len() - rest.len();
    if (1..=6).contains(&hashes) && rest.starts_with(is_js_space) {
        rest.trim_start_matches(is_js_space)
    } else {
        line
    }
}

/// `/^(?:[-*+]|\d+[.)])\s+/`
fn strip_list_marker(line: &str) -> &str {
    let after_marker = if let Some(rest) = line.strip_prefix(['-', '*', '+']) {
        rest
    } else {
        let rest = line.trim_start_matches(|c: char| c.is_ascii_digit());
        if rest.len() == line.len() {
            return line;
        }
        match rest.strip_prefix(['.', ')']) {
            Some(rest) => rest,
            None => return line,
        }
    };
    if after_marker.starts_with(is_js_space) {
        after_marker.trim_start_matches(is_js_space)
    } else {
        line
    }
}

/// `/!?\[([^\]]*)\]\([^)]*\)/g` replaced by `$1`.
fn unwrap_links(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut rest = line;
    while !rest.is_empty() {
        match link_at(rest) {
            Some((label, consumed)) => {
                out.push_str(label);
                rest = &rest[consumed..];
            }
            None => {
                let mut chars = rest.chars();
                out.push(chars.next().unwrap());
                rest = chars.as_str();
            }
        }
    }
    out
}

/// The label and byte length of a Markdown link or image at the start of `text`.
fn link_at(text: &str) -> Option<(&str, usize)> {
    let start = if text.starts_with("![") {
        2
    } else if text.starts_with('[') {
        1
    } else {
        return None;
    };
    let label_end = start + text[start..].find(']')?;
    let target = text[label_end + 1..].strip_prefix('(')?;
    let close = target.find(')')?;
    Some((&text[start..label_end], label_end + 2 + close + 1))
}

/// `/(\*\*|__|~~|`)/g` removed, in one left-to-right pass.
fn strip_emphasis(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut rest = line;
    while !rest.is_empty() {
        if let Some(after) = ["**", "__", "~~", "`"]
            .iter()
            .find_map(|token| rest.strip_prefix(token))
        {
            rest = after;
        } else {
            let mut chars = rest.chars();
            out.push(chars.next().unwrap());
            rest = chars.as_str();
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(selection: &str) -> String {
        format_quote(selection).unwrap().text
    }

    #[test]
    fn formats_one_line_under_the_lead_line_as_a_blockquote() {
        assert_eq!(
            text("  Use the queue.  "),
            format!("{QUOTE_LEAD}\n\n> Use the queue.")
        );
    }

    #[test]
    fn quotes_every_line_and_keeps_blank_lines_inside_the_blockquote() {
        assert_eq!(
            text("First line\n\nSecond line"),
            format!("{QUOTE_LEAD}\n\n> First line\n>\n> Second line")
        );
    }

    #[test]
    fn nests_a_quote_that_already_holds_a_blockquote() {
        assert_eq!(
            text("> earlier\nnow"),
            format!("{QUOTE_LEAD}\n\n> > earlier\n> now")
        );
    }

    #[test]
    fn rejects_an_empty_selection() {
        assert_eq!(format_quote(" \n "), Err(QuoteFailure::Empty));
    }

    #[test]
    fn leaves_the_comment_to_the_host_which_appends_it_after_the_blockquote() {
        let sent = format!("{}\n\nComment: Why now?", text("Ship it"));
        assert_eq!(
            sent,
            format!("{QUOTE_LEAD}\n\n> Ship it\n\nComment: Why now?")
        );
        assert_eq!(
            quote_from_text(&text("Ship it")).as_deref(),
            Some("Ship it")
        );
    }

    #[test]
    fn caps_the_quote_body_at_the_limit() {
        let at_limit = format_quote(&"a".repeat(MAX_QUOTE_LENGTH)).unwrap();
        assert!(utf16_len(&at_limit.quote) <= MAX_QUOTE_LENGTH);
        assert_eq!(
            format_quote(&"a".repeat(MAX_QUOTE_LENGTH + 1)),
            Err(QuoteFailure::TooLong)
        );
    }

    #[test]
    fn round_trips_a_quote_through_the_agent_text() {
        let quote = "> nested\n\n  indented\nplain";
        assert_eq!(quote_from_text(&text(quote)).as_deref(), Some(quote.trim()));
        assert_eq!(quote_from_text("Something else"), None);
    }

    #[test]
    fn highlights_the_first_line_without_its_markdown_syntax() {
        assert_eq!(
            highlight_text("\n## **Bold** [link](https://x.dev) `code`\nmore").as_deref(),
            Some("Bold link code")
        );
        assert_eq!(
            highlight_text("- item one\n- item two").as_deref(),
            Some("item one")
        );
        assert_eq!(
            highlight_text("> quoted line").as_deref(),
            Some("quoted line")
        );
        assert_eq!(highlight_text("  \n "), None);
    }

    #[test]
    fn length_counts_utf16_units() {
        // One astral character is two code units in JS, so 2001 of them are 4002 long.
        assert_eq!(utf16_len("😀"), 2);
        assert!(format_quote(&"😀".repeat(2000)).is_ok());
        assert_eq!(format_quote(&"😀".repeat(2001)), Err(QuoteFailure::TooLong));
    }

    #[test]
    fn trims_like_js() {
        assert_eq!(js_trim("\u{feff} a \u{a0}"), "a");
        // JS keeps U+0085; Rust's own trim would drop it.
        assert_eq!(js_trim("\u{85}a"), "\u{85}a");
    }

    #[test]
    fn splits_lines_like_js() {
        assert_eq!(split_lines("a\r\nb\rc\r\r\nd"), ["a", "b\rc\r", "d"]);
        assert_eq!(text("a\r\nb"), format!("{QUOTE_LEAD}\n\n> a\n> b"));
    }

    #[test]
    fn strips_numbered_markers_headings_and_images() {
        assert_eq!(highlight_text("12) step").as_deref(), Some("step"));
        assert_eq!(highlight_text("1.step").as_deref(), Some("1.step"));
        assert_eq!(
            highlight_text("####### seven").as_deref(),
            Some("####### seven")
        );
        assert_eq!(highlight_text("### three").as_deref(), Some("three"));
        assert_eq!(
            highlight_text("see ![alt](a.png) and [b](c)").as_deref(),
            Some("see alt and b")
        );
        assert_eq!(highlight_text("_**_").as_deref(), Some("__"));
        assert_eq!(highlight_text("-no space").as_deref(), Some("-no space"));
        assert_eq!(highlight_text("> > deep").as_deref(), Some("deep"));
    }
}
