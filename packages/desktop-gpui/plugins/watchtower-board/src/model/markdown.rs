//! A small reader for ADR files (`client/markdown.ts`): headings, bullets, paragraphs, and fenced
//! code. Inline marks such as backticks and links stay text, and HTML is never rendered.

use regex::Regex;
use std::sync::LazyLock;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Block {
    Heading { level: usize, text: String },
    Bullet { depth: usize, text: String },
    Paragraph { text: String },
    Code { text: String },
}

static FENCE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s{0,3}(```|~~~)").unwrap());
static HEADING: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s{0,3}(#{1,6})\s+(.*?)\s*$").unwrap());
static BULLET: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(\s*)[-*]\s+(.*)$").unwrap());

pub(crate) fn parse_markdown(markdown: &str) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut paragraph: Vec<String> = Vec::new();
    let mut code: Option<Vec<String>> = None;
    let flush = |paragraph: &mut Vec<String>, blocks: &mut Vec<Block>| {
        if !paragraph.is_empty() {
            blocks.push(Block::Paragraph {
                text: paragraph.join("\n"),
            });
            paragraph.clear();
        }
    };
    for line in markdown.split('\n') {
        let line = line.strip_suffix('\r').unwrap_or(line);
        if FENCE.is_match(line) {
            match code.take() {
                Some(lines) => blocks.push(Block::Code {
                    text: lines.join("\n"),
                }),
                None => {
                    flush(&mut paragraph, &mut blocks);
                    code = Some(Vec::new());
                }
            }
            continue;
        }
        if let Some(lines) = code.as_mut() {
            lines.push(line.to_string());
            continue;
        }
        if let Some(heading) = HEADING.captures(line) {
            flush(&mut paragraph, &mut blocks);
            blocks.push(Block::Heading {
                level: heading[1].len(),
                text: heading[2].to_string(),
            });
        } else if let Some(bullet) = BULLET.captures(line) {
            flush(&mut paragraph, &mut blocks);
            blocks.push(Block::Bullet {
                depth: bullet[1].chars().count() / 2,
                text: bullet[2].to_string(),
            });
        } else if line.trim().is_empty() {
            flush(&mut paragraph, &mut blocks);
        } else {
            paragraph.push(line.trim().to_string());
        }
    }
    flush(&mut paragraph, &mut blocks);
    // An unclosed fence still shows what it held.
    if let Some(lines) = code {
        blocks.push(Block::Code {
            text: lines.join("\n"),
        });
    }
    blocks
}

#[cfg(test)]
mod tests {
    use super::*;

    fn heading(level: usize, text: &str) -> Block {
        Block::Heading {
            level,
            text: text.into(),
        }
    }
    fn bullet(depth: usize, text: &str) -> Block {
        Block::Bullet {
            depth,
            text: text.into(),
        }
    }
    fn paragraph(text: &str) -> Block {
        Block::Paragraph { text: text.into() }
    }
    fn code(text: &str) -> Block {
        Block::Code { text: text.into() }
    }

    // markdown.test.ts
    #[test]
    fn headings_bullets_and_blank_line_paragraphs_become_blocks() {
        let blocks = parse_markdown(
            &[
                "# ADR-0003 Chip-only sources",
                "",
                "Status: proposed",
                "Date: 2026-10-07",
                "",
                "## Context",
                "",
                "- First `code` point",
                "  - Nested point",
                "* Star bullet",
                "",
                "Closing words.",
            ]
            .join("\n"),
        );
        assert_eq!(
            blocks,
            [
                heading(1, "ADR-0003 Chip-only sources"),
                paragraph("Status: proposed\nDate: 2026-10-07"),
                heading(2, "Context"),
                bullet(0, "First `code` point"),
                bullet(1, "Nested point"),
                bullet(0, "Star bullet"),
                paragraph("Closing words."),
            ]
        );
    }

    #[test]
    fn links_and_html_stay_as_text_and_a_hash_without_a_space_is_not_a_heading() {
        assert_eq!(
            parse_markdown("See [ADR-0004](a.md) and <b>bold</b>.\n#tag\nC#"),
            [paragraph("See [ADR-0004](a.md) and <b>bold</b>.\n#tag\nC#")]
        );
        assert_eq!(
            parse_markdown("## Heading with C#"),
            [heading(2, "Heading with C#")]
        );
    }

    #[test]
    fn a_fenced_block_keeps_its_lines_even_an_unclosed_one() {
        assert_eq!(
            parse_markdown("Before\n```ts\n# not a heading\n- not a bullet\n```\nAfter"),
            [
                paragraph("Before"),
                code("# not a heading\n- not a bullet"),
                paragraph("After"),
            ]
        );
        assert_eq!(parse_markdown("```\nopen"), [code("open")]);
        assert_eq!(parse_markdown(""), []);
    }

    #[test]
    fn a_tilde_fence_and_crlf_lines_work() {
        assert_eq!(
            parse_markdown("Intro\r\n~~~\r\nx = 1\r\n~~~\r\n- a\r\n    - deep"),
            [
                paragraph("Intro"),
                code("x = 1"),
                bullet(0, "a"),
                bullet(2, "deep")
            ]
        );
    }
}
