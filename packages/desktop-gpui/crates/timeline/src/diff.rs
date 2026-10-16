//! Diff lines of an edit or write call, as the app's edit rows show them.

use serde_json::Value;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineKind {
    Add,
    Remove,
    Context,
    Hunk,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiffLine {
    pub kind: LineKind,
    pub text: String,
}

/// More lines than this are cut, so a large write does not lay out thousands of rows.
pub const MAX_LINES: usize = 400;

fn line(kind: LineKind, text: &str) -> DiffLine {
    DiffLine {
        kind,
        text: text.trim_end_matches(['\n', '\r']).to_string(),
    }
}

/// Lines of a unified diff, without its file headers.
pub fn parse_unified(diff: &str) -> Vec<DiffLine> {
    let mut lines = Vec::new();
    for text in diff.lines() {
        let is_header = text.starts_with("diff --git")
            || text.starts_with("index ")
            || text.starts_with("--- ")
            || text.starts_with("+++ ")
            || text.starts_with("\\ No newline");
        if is_header {
            continue;
        }
        let parsed = match text.chars().next() {
            Some('@') => line(LineKind::Hunk, text),
            Some('+') => line(LineKind::Add, &text[1..]),
            Some('-') => line(LineKind::Remove, &text[1..]),
            Some(' ') => line(LineKind::Context, &text[1..]),
            _ => line(LineKind::Context, text),
        };
        lines.push(parsed);
    }
    lines
}

/// A line diff of two texts.
pub fn line_diff(old: &str, new: &str) -> Vec<DiffLine> {
    similar::TextDiff::from_lines(old, new)
        .iter_all_changes()
        .map(|change| {
            let kind = match change.tag() {
                similar::ChangeTag::Insert => LineKind::Add,
                similar::ChangeTag::Delete => LineKind::Remove,
                similar::ChangeTag::Equal => LineKind::Context,
            };
            line(kind, change.value())
        })
        .collect()
}

/// The diff of an edit detail: its unified diff, or a line diff of its old and new strings.
pub fn detail_lines(detail: &Value) -> Vec<DiffLine> {
    let mut lines = match detail["unifiedDiff"].as_str() {
        Some(diff) if !diff.is_empty() => parse_unified(diff),
        _ => line_diff(
            detail["oldString"].as_str().unwrap_or(""),
            detail["newString"].as_str().unwrap_or(""),
        ),
    };
    lines.truncate(MAX_LINES);
    lines
}

/// The diff lines of an edit or write entry, cut at [`MAX_LINES`].
pub fn entry_lines(entry: &Value) -> Vec<DiffLine> {
    let detail = &entry["item"]["detail"];
    let mut lines = match detail["type"].as_str() {
        Some("edit") => detail_lines(detail),
        Some("write") => detail["content"]
            .as_str()
            .unwrap_or("")
            .lines()
            .map(|text| line(LineKind::Add, text))
            .collect(),
        _ => Vec::new(),
    };
    lines.truncate(MAX_LINES);
    lines
}

/// The line number each line shows, as `numberRows` of the edit-diffs plugin: a removed line
/// shows its old number and the others their new one. Numbers start at each hunk header; a write
/// (only added lines, no header) counts from 1, and a line diff without headers shows none.
pub fn numbers(lines: &[DiffLine]) -> Vec<Option<u32>> {
    let is_write = lines.iter().all(|line| line.kind == LineKind::Add);
    let (mut old, mut new) = if is_write {
        (None, Some(1))
    } else {
        (None, None)
    };
    lines
        .iter()
        .map(|line| match line.kind {
            LineKind::Hunk => {
                (old, new) = hunk_starts(&line.text).unzip();
                None
            }
            LineKind::Remove => {
                let number = old;
                old = old.map(|n| n + 1);
                number
            }
            LineKind::Add | LineKind::Context => {
                let number = new;
                new = new.map(|n| n + 1);
                if line.kind == LineKind::Context {
                    old = old.map(|n| n + 1);
                }
                number
            }
        })
        .collect()
}

/// The old and new start lines of `@@ -a,b +c,d @@`.
fn hunk_starts(header: &str) -> Option<(u32, u32)> {
    let mut parts = header.split_whitespace().skip(1);
    let start = |part: Option<&str>, sign: char| -> Option<u32> {
        part?.strip_prefix(sign)?.split(',').next()?.parse().ok()
    };
    Some((start(parts.next(), '-')?, start(parts.next(), '+')?))
}

/// Added and removed line counts.
pub fn stats(lines: &[DiffLine]) -> (usize, usize) {
    let count = |kind| lines.iter().filter(|line| line.kind == kind).count();
    (count(LineKind::Add), count(LineKind::Remove))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn unified_diff_drops_headers_and_marks_lines() {
        let diff = "--- a/x.ts\n+++ b/x.ts\n@@ -1,2 +1,2 @@\n keep\n-old\n+new\n";
        let kinds: Vec<LineKind> = parse_unified(diff).iter().map(|l| l.kind).collect();
        assert_eq!(
            kinds,
            [
                LineKind::Hunk,
                LineKind::Context,
                LineKind::Remove,
                LineKind::Add
            ]
        );
    }

    #[test]
    fn edit_without_a_diff_uses_a_line_diff() {
        let entry = json!({"item": {"detail": {
            "type": "edit", "filePath": "/a", "oldString": "a\nb\n", "newString": "a\nc\n"
        }}});
        let lines = entry_lines(&entry);
        assert_eq!(stats(&lines), (1, 1));
        assert_eq!(lines[0], line(LineKind::Context, "a"));
    }
}
