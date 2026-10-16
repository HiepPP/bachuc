//! The chip item of a quote and its source (`shared/item.ts`). A chip made here must open in
//! Electron and the reverse, so the `id` hash, the `url` pairs, and the title cut follow the JS
//! code unit by unit.

use crate::format::{
    QuoteFailure, format_quote, highlight_text, is_js_space, quote_from_text, utf16_len,
};
use bachuc_plugin_host::AttachmentItem;

pub(crate) const QUOTE_SOURCE_ID: &str = "quote";
const SOURCE_URL: &str = "paseo-cite://quote";
const TITLE_WORDS: usize = 8;
const TITLE_LENGTH: usize = 60;

/// Where a quote came from. `message_id` is `None` for a message the server did not name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct QuoteSource<'a> {
    pub(crate) server_id: &'a str,
    pub(crate) agent_id: &'a str,
    pub(crate) message_id: Option<&'a str>,
}

/// The passage a chip jumps back to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct QuotePassage {
    pub(crate) server_id: String,
    pub(crate) agent_id: String,
    pub(crate) message_id: String,
    pub(crate) text: Option<String>,
}

/// FNV-1a over the UTF-16 code units, in base 36: a short stable id, so citing the same passage
/// again replaces its chip.
pub(crate) fn hash(value: &str) -> String {
    let mut result: u32 = 0x811c_9dc5;
    for unit in value.encode_utf16() {
        result ^= u32::from(unit);
        result = result.wrapping_mul(0x0100_0193);
    }
    to_base36(result)
}

fn to_base36(mut value: u32) -> String {
    const DIGITS: &[u8; 36] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    if value == 0 {
        return "0".into();
    }
    let mut digits = Vec::new();
    while value > 0 {
        digits.push(DIGITS[(value % 36) as usize]);
        value /= 36;
    }
    digits.reverse();
    String::from_utf8(digits).unwrap()
}

/// The first 8 words, cut at 60 UTF-16 units, with an ellipsis when words were left out.
fn first_words(quote: &str) -> String {
    let words: Vec<&str> = quote
        .split(is_js_space)
        .filter(|word| !word.is_empty())
        .collect();
    let mut title = words
        .iter()
        .take(TITLE_WORDS)
        .copied()
        .collect::<Vec<_>>()
        .join(" ");
    if utf16_len(&title) > TITLE_LENGTH {
        title = cut_utf16(&title, TITLE_LENGTH)
            .trim_end_matches(is_js_space)
            .to_string();
    }
    let all = words.join(" ");
    if utf16_len(&title) < utf16_len(&all) {
        title.push('…');
    }
    title
}

/// The first `units` UTF-16 code units. JS `slice` can end inside a surrogate pair and keep a
/// lone surrogate, which a Rust string cannot hold; this drops it.
fn cut_utf16(text: &str, units: usize) -> String {
    let mut count = 0;
    let mut end = 0;
    for (index, c) in text.char_indices() {
        count += c.len_utf16();
        if count > units {
            break;
        }
        end = index + c.len_utf8();
    }
    text[..end].to_string()
}

/// The chip item keeps its source in `url` and the quote in `text`, so a chip opens its source
/// after a reload and on another client.
pub(crate) fn build_quote_item(
    source: QuoteSource<'_>,
    selection: &str,
) -> Result<AttachmentItem, QuoteFailure> {
    let formatted = format_quote(selection)?;
    let mut params = form_urlencoded::Serializer::new(String::new());
    params.append_pair("server", source.server_id);
    params.append_pair("agent", source.agent_id);
    if let Some(message) = source.message_id.filter(|message| !message.is_empty()) {
        params.append_pair("message", message);
    }
    Ok(AttachmentItem {
        id: format!(
            "{}:{}",
            source.message_id.unwrap_or(source.agent_id),
            hash(&formatted.quote)
        ),
        identifier: "reply".into(),
        title: first_words(&formatted.quote),
        subtitle: None,
        url: format!("{SOURCE_URL}?{}", params.finish()),
        text: formatted.text,
        resource_type: "quote".into(),
    })
}

/// The passage a chip jumps back to, or `None` when the chip has no source message.
pub(crate) fn parse_quote_item(item: &AttachmentItem) -> Option<QuotePassage> {
    let query = item.url.strip_prefix(SOURCE_URL)?.strip_prefix('?')?;
    let param = |key: &str| {
        form_urlencoded::parse(query.as_bytes())
            .find(|(name, _)| name == key)
            .map(|(_, value)| value.into_owned())
            .filter(|value| !value.is_empty())
    };
    let (server_id, agent_id, message_id) = (param("server")?, param("agent")?, param("message")?);
    let text = quote_from_text(&item.text).and_then(|quote| highlight_text(&quote));
    Some(QuotePassage {
        server_id,
        agent_id,
        message_id,
        text,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::{MAX_QUOTE_LENGTH, js_trim};

    const SOURCE: QuoteSource<'static> = QuoteSource {
        server_id: "host-1",
        agent_id: "agent-1",
        message_id: Some("msg-1"),
    };

    fn item(selection: &str) -> AttachmentItem {
        build_quote_item(SOURCE, selection).unwrap()
    }

    #[test]
    fn builds_a_chip_item_titled_by_the_first_words() {
        let quote =
            item("Keep the queue in the daemon so every client sees the same order of messages");
        assert_eq!(quote.title, "Keep the queue in the daemon so every…");
        assert_eq!(quote.resource_type, "quote");
        assert_eq!(quote.identifier, "reply");
        assert_eq!(quote.subtitle, None);
        assert!(
            quote
                .text
                .starts_with("Quoted from your earlier reply:\n\n> Keep")
        );
    }

    #[test]
    fn same_passage_same_id_so_citing_again_replaces_the_chip() {
        assert_eq!(item("Same words").id, item("  Same words ").id);
        assert_ne!(item("Same words").id, item("Other words").id);
    }

    #[test]
    fn jumps_back_to_the_source_message_with_the_first_line_as_the_highlight() {
        assert_eq!(
            parse_quote_item(&item("**Use** the queue.\nSecond line")),
            Some(QuotePassage {
                server_id: "host-1".into(),
                agent_id: "agent-1".into(),
                message_id: "msg-1".into(),
                text: Some("Use the queue.".into()),
            })
        );
    }

    #[test]
    fn does_not_jump_back_without_a_source_message_or_for_another_plugins_item() {
        let no_message = QuoteSource {
            message_id: None,
            ..SOURCE
        };
        let chip = build_quote_item(no_message, "Quote").unwrap();
        assert_eq!(parse_quote_item(&chip), None);
        let foreign = AttachmentItem {
            url: "https://example.com/q".into(),
            ..item("Quote")
        };
        assert_eq!(parse_quote_item(&foreign), None);
    }

    #[test]
    fn adds_no_chip_for_an_empty_or_over_long_selection() {
        assert_eq!(build_quote_item(SOURCE, ""), Err(QuoteFailure::Empty));
        assert_eq!(
            build_quote_item(SOURCE, &"x".repeat(MAX_QUOTE_LENGTH + 1)),
            Err(QuoteFailure::TooLong)
        );
    }

    /// The strings below come from the JS code (`shared/item.ts` run in Node) for the same input.
    #[test]
    fn item_id_and_url_match_js() {
        let chip = item("Same words");
        assert_eq!(chip.id, "msg-1:6n05hy");
        assert_eq!(
            chip.url,
            "paseo-cite://quote?server=host-1&agent=agent-1&message=msg-1"
        );
        assert_eq!(hash("Other words"), "ovu1u8");
        assert_eq!(hash(""), "ztntfp");
        // Without a message the id starts with the agent id, and the url has no message pair.
        let no_message = build_quote_item(
            QuoteSource {
                message_id: None,
                ..SOURCE
            },
            "Same words",
        )
        .unwrap();
        assert_eq!(no_message.id, "agent-1:6n05hy");
        assert_eq!(
            no_message.url,
            "paseo-cite://quote?server=host-1&agent=agent-1"
        );
    }

    #[test]
    fn url_pairs_encode_like_url_search_params() {
        let source = QuoteSource {
            server_id: "ho st/é&=+*~'!()",
            agent_id: "a",
            message_id: None,
        };
        let chip = build_quote_item(source, "x").unwrap();
        assert_eq!(
            chip.url,
            "paseo-cite://quote?server=ho+st%2F%C3%A9%26%3D%2B*%7E%27%21%28%29&agent=a"
        );
    }

    #[test]
    fn length_counts_utf16_units() {
        // JS hashes 16-bit code units, so an astral character is two of them.
        assert_eq!(hash("😀 astral"), "1rnqbgf");
        assert_eq!(hash("😀😀"), "1e58txp");
        assert_eq!(utf16_len("😀😀"), 4);
        // The 60-unit title cut counts the same units and drops a half-cut pair.
        assert_eq!(
            first_words(&format!("{} b", "a".repeat(70))),
            format!("{}…", "a".repeat(60))
        );
        assert_eq!(cut_utf16("a😀b", 2), "a");
        assert_eq!(cut_utf16("a😀b", 3), "a😀");
    }

    #[test]
    fn first_words_trims_and_marks_what_it_drops() {
        assert_eq!(first_words("one two"), "one two");
        assert_eq!(first_words("1 2 3 4 5 6 7 8 9"), "1 2 3 4 5 6 7 8…");
        assert_eq!(js_trim(&first_words("a\u{a0}b\tc")), "a b c");
    }
}
