//! Splits message markdown into text and diagram blocks. Closed ` ```mermaid ` fences and
//! display math (`$$...$$`) render through `bachuc-diagram` as images (ADR-0037); the rest
//! stays markdown for gpui-kit's text view.

use std::sync::Arc;

use bachuc_diagram::{DiagramKind, DiagramState};
use gpui_kit::Image;

/// One part of a message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Segment {
    Markdown(String),
    Diagram { kind: DiagramKind, source: String },
}

/// What a row draws for one segment.
#[derive(Clone, Debug)]
pub enum Block {
    Markdown(String),
    Image {
        image: Arc<Image>,
        label: &'static str,
    },
    Pending {
        label: &'static str,
    },
    /// A diagram that failed shows its source as a code block.
    Code {
        markdown: String,
        error: String,
    },
}

fn fence_of(line: &str) -> Option<(char, usize, &str)> {
    let trimmed = line.trim_start();
    if line.len() - trimmed.len() > 3 {
        return None;
    }
    let marker = trimmed.chars().next().filter(|c| *c == '`' || *c == '~')?;
    let length = trimmed.chars().take_while(|c| *c == marker).count();
    (length >= 3).then(|| (marker, length, trimmed[length..].trim()))
}

fn closes(line: &str, marker: char, length: usize) -> bool {
    fence_of(line).is_some_and(|(m, l, info)| m == marker && l >= length && info.is_empty())
}

fn push_diagram(
    output: &mut Vec<Segment>,
    markdown: &mut String,
    kind: DiagramKind,
    source: String,
) {
    if !markdown.trim().is_empty() {
        output.push(Segment::Markdown(std::mem::take(markdown)));
    }
    markdown.clear();
    output.push(Segment::Diagram { kind, source });
}

/// Splits `text` into segments. An unclosed fence or math block stays markdown, so a block
/// that is still streaming shows as text until it ends.
pub fn segments(text: &str) -> Vec<Segment> {
    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    let mut output: Vec<Segment> = Vec::new();
    let mut markdown = String::new();
    let mut index = 0;
    while index < lines.len() {
        let line = lines[index];
        if let Some((marker, length, info)) = fence_of(line) {
            let end = (index + 1..lines.len()).find(|&i| closes(lines[i], marker, length));
            let is_mermaid = info.split_whitespace().next() == Some("mermaid");
            match end {
                Some(end) if is_mermaid => {
                    let source = lines[index + 1..end].concat().trim_end().to_string();
                    push_diagram(&mut output, &mut markdown, DiagramKind::Mermaid, source);
                    index = end + 1;
                }
                Some(end) => {
                    markdown.push_str(&lines[index..=end].concat());
                    index = end + 1;
                }
                None => {
                    markdown.push_str(&lines[index..].concat());
                    index = lines.len();
                }
            }
            continue;
        }
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("$$") {
            // `$$ x $$` on one line, or `$$` lines around the formula.
            if let Some(inner) = rest.strip_suffix("$$") {
                push_diagram(
                    &mut output,
                    &mut markdown,
                    DiagramKind::Math,
                    inner.trim().to_string(),
                );
                index += 1;
                continue;
            }
            let end = (index + 1..lines.len()).find(|&i| lines[i].trim_end().ends_with("$$"));
            if let Some(end) = end {
                let last = lines[end].trim_end();
                let mut source = rest.trim().to_string();
                for middle in &lines[index + 1..end] {
                    source.push('\n');
                    source.push_str(middle.trim_end_matches('\n'));
                }
                source.push('\n');
                source.push_str(&last[..last.len() - 2]);
                push_diagram(
                    &mut output,
                    &mut markdown,
                    DiagramKind::Math,
                    source.trim().to_string(),
                );
                index = end + 1;
                continue;
            }
        }
        markdown.push_str(line);
        index += 1;
    }
    if !markdown.trim().is_empty() {
        output.push(Segment::Markdown(markdown));
    }
    output
}

/// The blocks of one message. `render` asks the diagram renderer for each diagram.
pub fn blocks(
    segments: &[Segment],
    mut render: impl FnMut(DiagramKind, &str) -> DiagramState,
) -> Vec<Block> {
    segments
        .iter()
        .map(|segment| match segment {
            Segment::Markdown(text) => Block::Markdown(text.clone()),
            Segment::Diagram { kind, source } => match render(*kind, source) {
                DiagramState::Ready(image) => Block::Image {
                    image,
                    label: kind.label(),
                },
                DiagramState::Pending => Block::Pending {
                    label: kind.label(),
                },
                DiagramState::Failed(error) => Block::Code {
                    markdown: fallback_markdown(*kind, source),
                    error,
                },
            },
        })
        .collect()
}

/// The source of a failed diagram as a fenced code block.
pub fn fallback_markdown(kind: DiagramKind, source: &str) -> String {
    let lang = match kind {
        DiagramKind::Mermaid => "mermaid",
        DiagramKind::Math => "latex",
    };
    format!("```{lang}\n{source}\n```\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    const BROKEN: &str = "Here is the flow:\n\n```mermaid\nflowchart LR\n  A --> (\n```\n\nDone.\n";

    #[test]
    fn broken_mermaid_renders_as_a_code_block_when_the_renderer_fails() {
        let parts = segments(BROKEN);
        assert_eq!(
            parts,
            vec![
                Segment::Markdown("Here is the flow:\n\n".into()),
                Segment::Diagram {
                    kind: DiagramKind::Mermaid,
                    source: "flowchart LR\n  A --> (".into()
                },
                Segment::Markdown("\nDone.\n".into()),
            ]
        );
        let shown = blocks(&parts, |_, _| {
            DiagramState::Failed("Parse error on line 2".into())
        });
        match &shown[1] {
            Block::Code { markdown, error } => {
                assert_eq!(markdown, "```mermaid\nflowchart LR\n  A --> (\n```\n");
                assert_eq!(error, "Parse error on line 2");
            }
            other => panic!("expected a code block, got {other:?}"),
        }
        assert!(matches!(&shown[0], Block::Markdown(text) if text.starts_with("Here")));
    }

    #[test]
    fn broken_mermaid_still_streaming_stays_markdown() {
        let text = "```mermaid\nflowchart LR\n  A -->";
        assert_eq!(segments(text), vec![Segment::Markdown(text.into())]);
    }

    #[test]
    fn math_block_becomes_a_math_request() {
        let text = "The sum:\n\n$$\n\\sum_{n=1}^{\\infty} \\frac{1}{n^2}\n$$\n\nInline $$x$$ stays text.\n\n$$e^{i\\pi} + 1 = 0$$\n";
        let mut requests = Vec::new();
        let shown = blocks(&segments(text), |kind, source| {
            requests.push((kind, source.to_string()));
            DiagramState::Pending
        });
        assert_eq!(
            requests,
            vec![
                (
                    DiagramKind::Math,
                    "\\sum_{n=1}^{\\infty} \\frac{1}{n^2}".to_string()
                ),
                (DiagramKind::Math, "e^{i\\pi} + 1 = 0".to_string()),
            ]
        );
        assert!(matches!(
            shown[1],
            Block::Pending {
                label: "Math formula"
            }
        ));
    }

    #[test]
    fn math_block_inside_a_code_fence_stays_code() {
        let text = "```text\n$$x$$\n```\n";
        assert_eq!(segments(text), vec![Segment::Markdown(text.into())]);
    }

    #[test]
    fn other_fences_and_text_stay_one_markdown_segment() {
        let text = "Intro\n\n```rust\nfn main() {}\n```\n\nOutro";
        assert_eq!(segments(text), vec![Segment::Markdown(text.into())]);
    }
}
