//! Parses the unified diff text of `Client::file_diff` into numbered lines.
//!
//! The text is `--- a/<path>` and `+++ b/<path>` (`/dev/null` for a new or deleted file), then
//! per hunk a header line that starts with `@@` and the lines with a `+`, `-`, or space prefix.
//! A unified row shows the old number for a removed line and the new number for the rest, as
//! `buildUnifiedDiffLines` in `packages/app/src/utils/diff-layout.ts` does.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineKind {
    /// A `@@ -a,b +c,d @@` hunk header.
    Header,
    Context,
    Add,
    Remove,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiffLine {
    pub kind: LineKind,
    /// The line without its `+`, `-`, or space prefix; the whole header for `Header`.
    pub text: String,
    /// The number to show; none on a header.
    pub number: Option<u32>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ParsedDiff {
    pub is_new: bool,
    pub is_deleted: bool,
    pub lines: Vec<DiffLine>,
    pub added: u32,
    pub removed: u32,
}

/// The start of the old side and of the new side from `@@ -a,b +c,d @@`.
fn hunk_starts(header: &str) -> Option<(u32, u32)> {
    let mut parts = header.split_whitespace();
    parts.next().filter(|marker| *marker == "@@")?;
    let start = |part: Option<&str>, sign: char| -> Option<u32> {
        let part = part?.strip_prefix(sign)?;
        part.split(',').next()?.parse().ok()
    };
    Some((start(parts.next(), '-')?, start(parts.next(), '+')?))
}

pub fn parse(text: &str) -> ParsedDiff {
    let mut diff = ParsedDiff::default();
    let (mut old, mut new) = (0u32, 0u32);
    let mut in_hunk = false;
    for line in text.lines() {
        if !in_hunk {
            if let Some(path) = line.strip_prefix("--- ") {
                diff.is_new = path == "/dev/null";
                continue;
            }
            if let Some(path) = line.strip_prefix("+++ ") {
                diff.is_deleted = path == "/dev/null";
                continue;
            }
        }
        if line.starts_with("@@") {
            if let Some((old_start, new_start)) = hunk_starts(line) {
                (old, new) = (old_start, new_start);
            }
            in_hunk = true;
            diff.lines.push(DiffLine {
                kind: LineKind::Header,
                text: line.to_string(),
                number: None,
            });
            continue;
        }
        if !in_hunk || line.starts_with('\\') {
            // Before the first hunk, or a "\ No newline at end of file" marker.
            continue;
        }
        let (kind, text) = match line.chars().next() {
            Some('+') => (LineKind::Add, &line[1..]),
            Some('-') => (LineKind::Remove, &line[1..]),
            Some(' ') => (LineKind::Context, &line[1..]),
            // An empty line is an empty context line that lost its space.
            _ => (LineKind::Context, line),
        };
        let number = match kind {
            LineKind::Remove => {
                diff.removed += 1;
                old += 1;
                old - 1
            }
            LineKind::Add => {
                diff.added += 1;
                new += 1;
                new - 1
            }
            _ => {
                old += 1;
                new += 1;
                new - 1
            }
        };
        diff.lines.push(DiffLine {
            kind,
            text: text.to_string(),
            number: Some(number),
        });
    }
    diff
}

/// Why a file has no diff text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoDiff {
    TooLarge,
    Binary,
}

/// Reads the `<path> has no diff: <status>` message of `Client::file_diff`. Only the end counts,
/// because the path may contain the word "binary".
pub fn no_diff_reason(message: &str) -> Option<NoDiff> {
    if message.ends_with("has no diff: too_large") {
        Some(NoDiff::TooLarge)
    } else if message.ends_with("has no diff: binary") {
        Some(NoDiff::Binary)
    } else {
        None
    }
}

/// The widest line number, for the gutter width.
pub fn max_number(lines: &[DiffLine]) -> u32 {
    lines
        .iter()
        .filter_map(|line| line.number)
        .max()
        .unwrap_or(0)
}
