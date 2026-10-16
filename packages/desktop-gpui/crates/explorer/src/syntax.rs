//! Syntax colors for the file viewer: the gpui-kit tree-sitter highlighter, with its capture
//! names mapped onto the roles and colors of `@getpaseo/highlight` (`colors.ts`).
//!
//! Only the grammars that the crate enables resolve (see `Cargo.toml`); any other file shows in
//! the plain text color. The highlighter parses the whole text once, off the UI thread, and the
//! view then reads the runs of the lines it draws.

use bachuc_theme::Scheme;
use gpui_kit::base::input::HighlightStyleResolver;
use gpui_kit::component::Rope;
use gpui_kit::component::highlighter::SyntaxHighlighter;
use gpui_kit::{HighlightStyle, Hsla, rgb};
use std::ops::Range;

/// Tab stops of the viewer, as the CodeMirror `tabSize` default.
pub const TAB_SIZE: usize = 4;

/// The tree-sitter language for a file path, when the crate has its grammar.
pub fn language_for(path: &str) -> Option<&'static str> {
    let extension = path.rsplit_once('.')?.1.to_lowercase();
    Some(match extension.as_str() {
        "rs" => "rust",
        "ts" | "mts" | "cts" => "typescript",
        "tsx" => "tsx",
        "js" | "jsx" | "mjs" | "cjs" => "javascript",
        "py" => "python",
        "sh" | "bash" | "zsh" => "bash",
        "json" | "jsonc" => "json",
        _ => return None,
    })
}

/// One color per role. Values are the `darkHighlightColors` and `lightHighlightColors` of
/// `packages/highlight/src/colors.ts`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SyntaxColors {
    pub keyword: Hsla,
    pub comment: Hsla,
    pub string: Hsla,
    pub number: Hsla,
    pub literal: Hsla,
    pub function: Hsla,
    pub class: Hsla,
    pub type_: Hsla,
    pub tag: Hsla,
    pub attribute: Hsla,
    pub property: Hsla,
    pub variable: Hsla,
    pub operator: Hsla,
    pub punctuation: Hsla,
    pub regexp: Hsla,
    pub escape: Hsla,
    pub meta: Hsla,
    pub heading: Hsla,
    pub link: Hsla,
}

impl SyntaxColors {
    pub fn for_scheme(scheme: Scheme) -> Self {
        match scheme {
            Scheme::Dark => Self::dark(),
            Scheme::Light => Self::light(),
        }
    }

    fn dark() -> Self {
        let c = |hex| Hsla::from(rgb(hex));
        Self {
            keyword: c(0xff7b72),
            comment: c(0x8b949e),
            string: c(0xa5d6ff),
            number: c(0x79c0ff),
            literal: c(0x79c0ff),
            function: c(0xd2a8ff),
            class: c(0xffa657),
            type_: c(0xff7b72),
            tag: c(0x7ee787),
            attribute: c(0x79c0ff),
            property: c(0x79c0ff),
            variable: c(0xc9d1d9),
            operator: c(0x79c0ff),
            punctuation: c(0xc9d1d9),
            regexp: c(0xa5d6ff),
            escape: c(0x79c0ff),
            meta: c(0x8b949e),
            heading: c(0x79c0ff),
            link: c(0xa5d6ff),
        }
    }

    fn light() -> Self {
        let c = |hex| Hsla::from(rgb(hex));
        Self {
            keyword: c(0xcf222e),
            comment: c(0x6e7781),
            string: c(0x0a3069),
            number: c(0x0550ae),
            literal: c(0x0550ae),
            function: c(0x8250df),
            class: c(0x953800),
            type_: c(0xcf222e),
            tag: c(0x116329),
            attribute: c(0x0550ae),
            property: c(0x0550ae),
            variable: c(0x24292f),
            operator: c(0x0550ae),
            punctuation: c(0x24292f),
            regexp: c(0x0a3069),
            escape: c(0x0550ae),
            meta: c(0x6e7781),
            heading: c(0x0550ae),
            link: c(0x0a3069),
        }
    }

    /// The color of a tree-sitter capture name such as `function.method` or `string.escape`.
    fn color_of(&self, capture: &str) -> Option<Hsla> {
        let starts = |prefix: &str| capture == prefix || capture.starts_with(&format!("{prefix}."));
        Some(if starts("comment") {
            self.comment
        } else if starts("string.escape") || starts("escape") {
            self.escape
        } else if starts("string.regex") || starts("string.regexp") {
            self.regexp
        } else if starts("string") || starts("character") {
            self.string
        } else if starts("number") {
            self.number
        } else if starts("boolean") || starts("constant") || starts("none") {
            self.literal
        } else if starts("keyword") || starts("variable.builtin") || starts("variable.special") {
            self.keyword
        } else if starts("function") || starts("constructor") {
            self.function
        } else if starts("class") || starts("namespace") || starts("module") {
            self.class
        } else if starts("type") {
            self.type_
        } else if starts("tag") {
            self.tag
        } else if starts("attribute") {
            self.attribute
        } else if starts("property") || starts("variable.member") {
            self.property
        } else if starts("variable") {
            self.variable
        } else if starts("operator") {
            self.operator
        } else if starts("punctuation") {
            self.punctuation
        } else if starts("label") || starts("preproc") {
            self.meta
        } else if starts("title") || starts("text.title") {
            self.heading
        } else if starts("link") || starts("text.uri") || starts("text.reference") {
            self.link
        } else {
            return None;
        })
    }
}

impl HighlightStyleResolver for SyntaxColors {
    fn style(&self, name: &str) -> Option<HighlightStyle> {
        self.color_of(name).map(|color| HighlightStyle {
            color: Some(color),
            ..Default::default()
        })
    }
}

/// A colored byte range of one line.
pub type Run = (Range<usize>, HighlightStyle);

/// Parses `text` as `language` and returns the colored ranges of each line, relative to the
/// start of the line. Runs on any thread.
pub fn highlight(language: &str, text: &str, colors: &SyntaxColors) -> Vec<Vec<Run>> {
    let mut highlighter = SyntaxHighlighter::new(language);
    let rope = Rope::from_str(text);
    highlighter.update(None, &rope, None);
    let styles = highlighter.styles(&(0..text.len()), colors);
    split_lines(text, &styles)
}

/// Cuts ranges that cover the whole text into per-line ranges relative to each line start. The
/// ranges must be sorted and must not overlap. A range that crosses a line break continues on
/// the next line; the break itself is in no range.
pub fn split_lines(text: &str, styles: &[Run]) -> Vec<Vec<Run>> {
    let mut starts = vec![0usize];
    starts.extend(text.match_indices('\n').map(|(at, _)| at + 1));
    let mut lines: Vec<Vec<Run>> = vec![Vec::new(); starts.len()];
    let mut line = 0;
    for (range, style) in styles {
        if style.color.is_none() {
            continue;
        }
        let mut start = range.start;
        // The ranges are sorted, so the line only moves forward.
        while line + 1 < starts.len() && starts[line + 1] <= start {
            line += 1;
        }
        loop {
            // The end of the line text, before its break.
            let line_end = starts
                .get(line + 1)
                .map_or(text.len(), |next| next.saturating_sub(1));
            let end = range.end.min(line_end);
            if end > start {
                lines[line].push((start - starts[line]..end - starts[line], *style));
            }
            if range.end <= line_end + 1 || line + 1 >= starts.len() {
                break;
            }
            line += 1;
            start = starts[line];
        }
    }
    lines
}

/// Replaces tabs with spaces up to the next multiple of `TAB_SIZE` columns and moves the runs
/// with the text.
pub fn expand_tabs(line: &str, runs: &[Run]) -> (String, Vec<Run>) {
    if !line.contains('\t') {
        return (line.to_string(), runs.to_vec());
    }
    let mut out = String::with_capacity(line.len() + 8);
    // New byte offset for each old byte offset; the bytes inside a character map to its start.
    let mut map = vec![0usize; line.len() + 1];
    let mut column = 0;
    for (at, ch) in line.char_indices() {
        map[at..at + ch.len_utf8()].fill(out.len());
        if ch == '\t' {
            let width = TAB_SIZE - column % TAB_SIZE;
            out.extend(std::iter::repeat_n(' ', width));
            column += width;
        } else {
            out.push(ch);
            column += 1;
        }
    }
    map[line.len()] = out.len();
    let runs = runs
        .iter()
        .map(|(range, style)| (map[range.start]..map[range.end], *style))
        .collect();
    (out, runs)
}
