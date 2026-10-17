//! `client/highlight.ts`: colors the old and new sides as whole documents, so multi-line strings
//! and comments keep their color.
//!
//! The extension map, the capture-to-role map, and `split_lines` are copies of
//! `crates/explorer/src/syntax.rs` (lines 19-31, 166-209, and 286-318): `bachuc-explorer` is a
//! view crate, so this crate cannot depend on it. Keep the copies in step.

use crate::diff::{Hunk, RowKind};
use bachuc_theme::SyntaxColors;
use gpui_kit::base::input::HighlightStyleResolver;
use gpui_kit::component::Rope;
use gpui_kit::component::highlighter::SyntaxHighlighter;
use gpui_kit::{HighlightStyle, Hsla};
use std::ops::Range;

/// A colored byte range of one line.
pub(crate) type Run = (Range<usize>, HighlightStyle);

/// The tree-sitter language for a file path, when the workspace has its grammar.
pub(crate) fn language_for(path: &str) -> Option<&'static str> {
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Role {
    Comment,
    Escape,
    Regexp,
    String,
    Number,
    Literal,
    Keyword,
    Function,
    Class,
    Type,
    Tag,
    Attribute,
    Property,
    Variable,
    Operator,
    Punctuation,
    Meta,
    Heading,
    Link,
}

fn role_of(capture: &str) -> Option<Role> {
    let starts = |prefix: &str| capture == prefix || capture.starts_with(&format!("{prefix}."));
    Some(if starts("comment") {
        Role::Comment
    } else if starts("string.escape") || starts("escape") {
        Role::Escape
    } else if starts("string.regex") || starts("string.regexp") {
        Role::Regexp
    } else if starts("string") || starts("character") {
        Role::String
    } else if starts("number") {
        Role::Number
    } else if starts("boolean") || starts("constant") || starts("none") {
        Role::Literal
    } else if starts("keyword") || starts("variable.builtin") || starts("variable.special") {
        Role::Keyword
    } else if starts("function") || starts("constructor") {
        Role::Function
    } else if starts("class") || starts("namespace") || starts("module") {
        Role::Class
    } else if starts("type") {
        Role::Type
    } else if starts("tag") {
        Role::Tag
    } else if starts("attribute") {
        Role::Attribute
    } else if starts("property") || starts("variable.member") {
        Role::Property
    } else if starts("variable") {
        Role::Variable
    } else if starts("operator") {
        Role::Operator
    } else if starts("punctuation") {
        Role::Punctuation
    } else if starts("label") || starts("preproc") {
        Role::Meta
    } else if starts("title") || starts("text.title") {
        Role::Heading
    } else if starts("link") || starts("text.uri") || starts("text.reference") {
        Role::Link
    } else {
        return None;
    })
}

/// The colors of the syntax theme picked in Settings, as a resolver for the highlighter.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ThemeSyntax(pub SyntaxColors);

impl ThemeSyntax {
    fn color_of(&self, capture: &str) -> Option<Hsla> {
        let c = &self.0;
        Some(match role_of(capture)? {
            Role::Comment => c.comment,
            Role::Escape => c.escape,
            Role::Regexp => c.regexp,
            Role::String => c.string,
            Role::Number => c.number,
            Role::Literal => c.literal,
            Role::Keyword => c.keyword,
            Role::Function => c.function,
            Role::Class => c.class,
            Role::Type => c.r#type,
            Role::Tag => c.tag,
            Role::Attribute => c.attribute,
            Role::Property => c.property,
            Role::Variable => c.variable,
            Role::Operator => c.operator,
            Role::Punctuation => c.punctuation,
            Role::Meta => c.meta,
            Role::Heading => c.heading,
            Role::Link => c.link,
        })
    }
}

impl HighlightStyleResolver for ThemeSyntax {
    fn style(&self, name: &str) -> Option<HighlightStyle> {
        // The app's lezer grammars give a macro name no role, so `println!` keeps the text color.
        if name == "function.macro" || name.starts_with("function.macro.") {
            return None;
        }
        self.color_of(name).map(|color| HighlightStyle {
            color: Some(color),
            ..Default::default()
        })
    }
}

/// Cuts ranges that cover the whole text into per-line ranges relative to each line start. The
/// ranges must be sorted and must not overlap.
fn split_lines(text: &str, styles: &[Run]) -> Vec<Vec<Run>> {
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

fn highlight(language: &str, lines: &[&str], colors: SyntaxColors) -> Vec<Vec<Run>> {
    let text = lines.join("\n");
    let mut highlighter = SyntaxHighlighter::new(language);
    highlighter.update(None, &Rope::from_str(&text), None);
    let styles = highlighter.styles(&(0..text.len()), &ThemeSyntax(colors));
    split_lines(&text, &styles)
}

/// The runs of each row of each hunk: removed rows from the old document, other rows from the
/// new one. `None` when the path has no grammar. Runs on any thread.
pub(crate) fn highlight_hunks(
    hunks: &[Hunk],
    file_path: &str,
    colors: SyntaxColors,
) -> Option<Vec<Vec<Vec<Run>>>> {
    let language = language_for(file_path)?;
    let mut old_code: Vec<&str> = Vec::new();
    let mut new_code: Vec<&str> = Vec::new();
    let positions: Vec<Vec<(bool, usize)>> = hunks
        .iter()
        .map(|hunk| {
            hunk.rows
                .iter()
                .map(|row| {
                    if row.kind == RowKind::Remove {
                        old_code.push(&row.text);
                        return (true, old_code.len() - 1);
                    }
                    if row.kind == RowKind::Context {
                        old_code.push(&row.text);
                    }
                    new_code.push(&row.text);
                    (false, new_code.len() - 1)
                })
                .collect()
        })
        .collect();
    let side = |code: &[&str]| (!code.is_empty()).then(|| highlight(language, code, colors));
    let old_runs = side(&old_code);
    let new_runs = side(&new_code);
    Some(
        positions
            .into_iter()
            .map(|rows| {
                rows.into_iter()
                    .map(|(old, index)| {
                        let runs = if old { &old_runs } else { &new_runs };
                        runs.as_ref()
                            .and_then(|runs| runs.get(index))
                            .cloned()
                            .unwrap_or_default()
                    })
                    .collect()
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff::parse_unified_diff;
    use bachuc_theme::{SyntaxThemeId, syntax_colors_for};

    #[test]
    fn languages_follow_the_file_viewer_map() {
        assert_eq!(language_for("/a/b.RS"), Some("rust"));
        assert_eq!(language_for("x.mts"), Some("typescript"));
        assert_eq!(language_for("Makefile"), None);
        assert_eq!(language_for("a.md"), None);
    }

    #[test]
    fn a_multi_line_comment_keeps_its_color_on_both_sides() {
        let hunks = parse_unified_diff("@@ -1,3 +1,3 @@\n /* start\n-old */\n+new */\n");
        let colors = syntax_colors_for(SyntaxThemeId::default(), bachuc_theme::Scheme::Dark);
        let runs = highlight_hunks(&hunks, "a.rs", colors).unwrap();
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].len(), 3);
        let color = |row: usize| runs[0][row].first().and_then(|(_, style)| style.color);
        assert_eq!(color(1), Some(colors.comment));
        assert_eq!(color(2), Some(colors.comment));
        assert_eq!(highlight_hunks(&hunks, "a.txt", colors), None);
    }
}
