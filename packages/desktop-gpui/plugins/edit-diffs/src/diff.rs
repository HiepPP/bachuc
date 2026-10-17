//! The diff rules of `shared/diff.ts` and `shared/change-bar.ts`, one function per TS function
//! with the same results.

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum RowKind {
    Add,
    Remove,
    Context,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Row {
    pub kind: RowKind,
    pub text: String,
}

/// Starts are 1-based file lines; `None` until the hunk is located in the file.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Hunk {
    pub old_start: Option<i64>,
    pub new_start: Option<i64>,
    pub rows: Vec<Row>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct EditSource<'a> {
    pub old_string: Option<&'a str>,
    pub new_string: Option<&'a str>,
    pub unified_diff: Option<&'a str>,
    /// A whole written file; every line is new.
    pub content: Option<&'a str>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct NumberedRow {
    pub kind: RowKind,
    pub text: String,
    pub line: Option<i64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Side {
    Old,
    New,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Located {
    pub line: i64,
    pub side: Side,
}

/// Above this many cells the line diff skips LCS and shows old lines removed, new lines added.
const MAX_LCS_CELLS: usize = 250_000;

pub(crate) const CHANGE_BAR_SQUARES: usize = 7;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Square {
    Add,
    Remove,
}

fn row(kind: RowKind, text: &str) -> Row {
    Row {
        kind,
        text: text.to_string(),
    }
}

fn split_lines(text: &str) -> Vec<&str> {
    if text.is_empty() {
        return Vec::new();
    }
    let mut lines: Vec<&str> = text.split('\n').collect();
    if lines.last() == Some(&"") {
        lines.pop();
    }
    lines
}

/// `lineDiff`.
pub(crate) fn line_diff(old_text: &str, new_text: &str) -> Vec<Row> {
    let a = split_lines(old_text);
    let b = split_lines(new_text);
    if a.len() * b.len() > MAX_LCS_CELLS {
        return a
            .iter()
            .map(|text| row(RowKind::Remove, text))
            .chain(b.iter().map(|text| row(RowKind::Add, text)))
            .collect();
    }
    let width = b.len() + 1;
    let mut lcs = vec![0u32; (a.len() + 1) * width];
    for i in (0..a.len()).rev() {
        for j in (0..b.len()).rev() {
            lcs[i * width + j] = if a[i] == b[j] {
                lcs[(i + 1) * width + j + 1] + 1
            } else {
                lcs[(i + 1) * width + j].max(lcs[i * width + j + 1])
            };
        }
    }
    let mut rows = Vec::with_capacity(a.len() + b.len());
    let (mut i, mut j) = (0, 0);
    while i < a.len() && j < b.len() {
        if a[i] == b[j] {
            rows.push(row(RowKind::Context, a[i]));
            i += 1;
            j += 1;
        } else if lcs[(i + 1) * width + j] >= lcs[i * width + j + 1] {
            rows.push(row(RowKind::Remove, a[i]));
            i += 1;
        } else {
            rows.push(row(RowKind::Add, b[j]));
            j += 1;
        }
    }
    rows.extend(a[i..].iter().map(|text| row(RowKind::Remove, text)));
    rows.extend(b[j..].iter().map(|text| row(RowKind::Add, text)));
    rows
}

fn is_file_header(lines: &[&str], index: usize) -> bool {
    let line = lines[index];
    let next_is_new = lines.get(index + 1).is_some_and(|l| l.starts_with("+++ "));
    let previous_is_old = index > 0 && lines[index - 1].starts_with("--- ");
    line.starts_with("diff --git ")
        || line.starts_with("index ")
        || (line.starts_with("--- ") && next_is_new)
        || (line.starts_with("+++ ") && previous_is_old)
}

/// `^@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@`: the old start and count, the new start and
/// count.
fn hunk_header(line: &str) -> Option<(i64, Option<i64>, i64, Option<i64>)> {
    fn digits(text: &str) -> Option<(i64, &str)> {
        let end = text
            .find(|c: char| !c.is_ascii_digit())
            .unwrap_or(text.len());
        Some((text[..end].parse().ok()?, &text[end..]))
    }
    fn range(text: &str) -> Option<(i64, Option<i64>, &str)> {
        let (start, rest) = digits(text)?;
        match rest.strip_prefix(',') {
            Some(rest) => {
                let (count, rest) = digits(rest)?;
                Some((start, Some(count), rest))
            }
            None => Some((start, None, rest)),
        }
    }
    let rest = line.strip_prefix("@@ -")?;
    let (old_start, old_count, rest) = range(rest)?;
    let rest = rest.strip_prefix(" +")?;
    let (new_start, new_count, rest) = range(rest)?;
    rest.starts_with(" @@")
        .then_some((old_start, old_count, new_start, new_count))
}

/// `parseUnifiedDiff`.
pub(crate) fn parse_unified_diff(diff: &str) -> Vec<Hunk> {
    let lines: Vec<&str> = diff.split('\n').collect();
    let mut hunks: Vec<Hunk> = Vec::new();
    let mut open = false;
    // Remaining counts from the hunk header; `None` for Codex's bare `@@`, which has none.
    let mut old_left: Option<i64> = None;
    let mut new_left: Option<i64> = None;
    for (index, &line) in lines.iter().enumerate() {
        if line.starts_with("@@") {
            let header = hunk_header(line);
            hunks.push(Hunk {
                old_start: header.map(|h| h.0),
                new_start: header.map(|h| h.2),
                rows: Vec::new(),
            });
            open = true;
            old_left = header.map(|h| h.1.unwrap_or(1));
            new_left = header.map(|h| h.3.unwrap_or(1));
            continue;
        }
        if open && old_left == Some(0) && new_left == Some(0) {
            open = false;
            old_left = None;
            new_left = None;
        }
        let counted = old_left.is_some();
        if !counted && is_file_header(&lines, index) {
            continue;
        }
        let marker = line.chars().next();
        if !open {
            if !matches!(marker, Some('+' | '-')) {
                continue;
            }
            hunks.push(Hunk {
                old_start: None,
                new_start: None,
                rows: Vec::new(),
            });
            open = true;
        }
        let text = marker.map_or("", |m| &line[m.len_utf8()..]);
        let hunk = hunks.last_mut().expect("an open hunk");
        match marker {
            Some('+') => {
                hunk.rows.push(row(RowKind::Add, text));
                new_left = new_left.map(|n| n - 1);
            }
            Some('-') => {
                hunk.rows.push(row(RowKind::Remove, text));
                old_left = old_left.map(|n| n - 1);
            }
            // Some tools strip the space from empty context lines; only trust that inside a
            // counted hunk.
            Some(' ') => {
                hunk.rows.push(row(RowKind::Context, text));
                old_left = old_left.map(|n| n - 1);
                new_left = new_left.map(|n| n - 1);
            }
            None if counted && index < lines.len() - 1 => {
                hunk.rows.push(row(RowKind::Context, ""));
                old_left = old_left.map(|n| n - 1);
                new_left = new_left.map(|n| n - 1);
            }
            _ => {}
        }
    }
    hunks.retain(|hunk| !hunk.rows.is_empty());
    hunks
}

/// `buildHunks`.
pub(crate) fn build_hunks(source: &EditSource) -> Vec<Hunk> {
    if let Some(content) = source.content {
        let rows: Vec<Row> = split_lines(content)
            .into_iter()
            .map(|text| row(RowKind::Add, text))
            .collect();
        return if rows.is_empty() {
            Vec::new()
        } else {
            vec![Hunk {
                old_start: Some(0),
                new_start: Some(1),
                rows,
            }]
        };
    }
    if let Some(diff) = source.unified_diff.filter(|diff| !diff.is_empty()) {
        return parse_unified_diff(diff);
    }
    let rows = line_diff(
        source.old_string.unwrap_or(""),
        source.new_string.unwrap_or(""),
    );
    if rows.is_empty() {
        Vec::new()
    } else {
        vec![Hunk {
            old_start: None,
            new_start: None,
            rows,
        }]
    }
}

/// `sideText`.
pub(crate) fn side_text(hunk: &Hunk, side: Side) -> String {
    let skip = match side {
        Side::Old => RowKind::Add,
        Side::New => RowKind::Remove,
    };
    hunk.rows
        .iter()
        .filter(|row| row.kind != skip)
        .map(|row| row.text.as_str())
        .collect::<Vec<_>>()
        .join("\n")
}

/// `needsLocating`.
pub(crate) fn needs_locating(hunks: &[Hunk]) -> bool {
    hunks
        .iter()
        .any(|hunk| hunk.old_start.is_none() || hunk.new_start.is_none())
}

/// `applyLocations`: fills unknown starts from file positions; a hunk's old and new starts differ
/// by the earlier hunks' net line change.
pub(crate) fn apply_locations(hunks: &[Hunk], located: &[Option<Located>]) -> Vec<Hunk> {
    let mut delta: i64 = 0;
    hunks
        .iter()
        .enumerate()
        .map(|(index, hunk)| {
            let mut next = hunk.clone();
            if (next.old_start.is_none() || next.new_start.is_none())
                && let Some(Some(found)) = located.get(index)
            {
                next.old_start = Some(match found.side {
                    Side::Old => found.line,
                    Side::New => found.line - delta,
                });
                next.new_start = Some(match found.side {
                    Side::New => found.line,
                    Side::Old => found.line + delta,
                });
            }
            for row in &hunk.rows {
                match row.kind {
                    RowKind::Add => delta += 1,
                    RowKind::Remove => delta -= 1,
                    RowKind::Context => {}
                }
            }
            next
        })
        .collect()
}

/// `numberRows`, Claude Code style: removed rows show the old line number, other rows the new
/// one.
pub(crate) fn number_rows(hunk: &Hunk) -> Vec<NumberedRow> {
    let mut old_line = hunk.old_start;
    let mut new_line = hunk.new_start;
    hunk.rows
        .iter()
        .map(|row| {
            let line = if row.kind == RowKind::Remove {
                let line = old_line;
                old_line = old_line.map(|n| n + 1);
                line
            } else {
                let line = new_line;
                new_line = new_line.map(|n| n + 1);
                if row.kind == RowKind::Context {
                    old_line = old_line.map(|n| n + 1);
                }
                line
            };
            NumberedRow {
                kind: row.kind,
                text: row.text.clone(),
                line,
            }
        })
        .collect()
}

/// `countChanges`: the added and removed row counts.
pub(crate) fn count_changes(hunks: &[Hunk]) -> (usize, usize) {
    let rows = || hunks.iter().flat_map(|hunk| &hunk.rows);
    (
        rows().filter(|row| row.kind == RowKind::Add).count(),
        rows().filter(|row| row.kind == RowKind::Remove).count(),
    )
}

/// `changeBar`: splits the squares by the added share; a side that changed keeps at least one
/// square.
pub(crate) fn change_bar(added: usize, removed: usize) -> Vec<Square> {
    let total = added + removed;
    if total == 0 {
        return Vec::new();
    }
    let mut add = ((added as f64 / total as f64) * CHANGE_BAR_SQUARES as f64).round() as usize;
    if added > 0 && removed > 0 {
        add = add.clamp(1, CHANGE_BAR_SQUARES - 1);
    }
    (0..CHANGE_BAR_SQUARES)
        .map(|index| {
            if index < add {
                Square::Add
            } else {
                Square::Remove
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    fn kinds_lines(rows: &[NumberedRow]) -> Vec<(RowKind, Option<i64>)> {
        rows.iter().map(|row| (row.kind, row.line)).collect()
    }

    fn starts(hunks: &[Hunk]) -> Vec<(Option<i64>, Option<i64>)> {
        hunks.iter().map(|h| (h.old_start, h.new_start)).collect()
    }

    // diff.test.ts

    #[test]
    fn line_diff_keeps_shared_lines_as_context() {
        assert_eq!(
            line_diff("a\nb\nc", "a\nB\nc\nd"),
            vec![
                row(RowKind::Context, "a"),
                row(RowKind::Remove, "b"),
                row(RowKind::Add, "B"),
                row(RowKind::Context, "c"),
                row(RowKind::Add, "d"),
            ]
        );
    }

    #[test]
    fn unified_diff_hunks_number_removed_rows_by_old_line_and_others_by_new_line() {
        let diff = [
            "diff --git a/x.ts b/x.ts",
            "--- a/x.ts",
            "+++ b/x.ts",
            "@@ -10,3 +10,3 @@ fn",
            " keep",
            "-old",
            "+new",
            " tail",
            "@@ -40 +40,2 @@",
            "-gone",
            "+one",
            "+two",
        ]
        .join("\n");
        let hunks = parse_unified_diff(&diff);
        assert_eq!(hunks.len(), 2);
        assert_eq!(
            kinds_lines(&number_rows(&hunks[0])),
            vec![
                (RowKind::Context, Some(10)),
                (RowKind::Remove, Some(11)),
                (RowKind::Add, Some(11)),
                (RowKind::Context, Some(12)),
            ]
        );
        let lines: Vec<Option<i64>> = number_rows(&hunks[1]).iter().map(|r| r.line).collect();
        assert_eq!(lines, vec![Some(40), Some(40), Some(41)]);
        assert_eq!(count_changes(&hunks), (3, 2));
    }

    #[test]
    fn a_removed_line_that_looks_like_a_file_header_stays_a_row_inside_a_counted_hunk() {
        let hunks = parse_unified_diff("@@ -1,2 +1,2 @@\n--- a\n+++ b\n");
        assert_eq!(
            hunks[0].rows,
            vec![row(RowKind::Remove, "-- a"), row(RowKind::Add, "++ b")]
        );
    }

    #[test]
    fn bare_codex_hunks_have_no_starts_until_located() {
        let hunks = parse_unified_diff("--- a/x\n+++ b/x\n@@\n ctx\n-a\n+b\n+c\n@@\n-d\n+e");
        assert_eq!(starts(&hunks), vec![(None, None), (None, None)]);
        assert_eq!(side_text(&hunks[0], Side::New), "ctx\nb\nc");
        assert_eq!(side_text(&hunks[0], Side::Old), "ctx\na");
        // The first hunk adds a net line, so the second hunk's old start trails its new start by
        // one.
        let located = apply_locations(
            &hunks,
            &[
                Some(Located {
                    line: 5,
                    side: Side::New,
                }),
                Some(Located {
                    line: 20,
                    side: Side::New,
                }),
            ],
        );
        assert_eq!(
            starts(&located),
            vec![(Some(5), Some(5)), (Some(19), Some(20))]
        );
    }

    #[test]
    fn old_and_new_strings_become_one_hunk_numbered_after_locating() {
        let hunks = build_hunks(&EditSource {
            old_string: Some("const a = 1;"),
            new_string: Some("const a = 2;\nconst b = 3;"),
            ..Default::default()
        });
        assert_eq!(hunks.len(), 1);
        let lines: Vec<Option<i64>> = number_rows(&hunks[0]).iter().map(|r| r.line).collect();
        assert_eq!(lines, vec![None, None, None]);
        let located = apply_locations(
            &hunks,
            &[Some(Located {
                line: 7,
                side: Side::New,
            })],
        );
        assert_eq!(
            kinds_lines(&number_rows(&located[0])),
            vec![
                (RowKind::Remove, Some(7)),
                (RowKind::Add, Some(7)),
                (RowKind::Add, Some(8)),
            ]
        );
    }

    #[test]
    fn written_content_becomes_added_rows_numbered_from_line_1_with_no_locating() {
        let hunks = build_hunks(&EditSource {
            content: Some("one\ntwo\n"),
            ..Default::default()
        });
        assert_eq!(
            kinds_lines(&number_rows(&hunks[0])),
            vec![(RowKind::Add, Some(1)), (RowKind::Add, Some(2))]
        );
        assert!(!needs_locating(&hunks));
        assert_eq!(
            build_hunks(&EditSource {
                content: Some(""),
                ..Default::default()
            }),
            vec![]
        );
    }

    /// The spec case: a write of `a\nb\n` is one hunk of two add rows numbered 1 and 2.
    #[test]
    fn a_two_line_write_is_one_hunk_of_two_numbered_adds() {
        let hunks = build_hunks(&EditSource {
            content: Some("a\nb\n"),
            ..Default::default()
        });
        assert_eq!(hunks.len(), 1);
        assert_eq!(
            kinds_lines(&number_rows(&hunks[0])),
            vec![(RowKind::Add, Some(1)), (RowKind::Add, Some(2))]
        );
    }

    #[test]
    fn a_600_by_600_line_diff_falls_back_to_removes_then_adds() {
        let old: String = (0..600).map(|i| format!("old {i}\n")).collect();
        let new: String = (0..600).map(|i| format!("new {i}\n")).collect();
        let rows = line_diff(&old, &new);
        assert_eq!(rows.len(), 1200);
        assert!(rows[..600].iter().all(|r| r.kind == RowKind::Remove));
        assert!(rows[600..].iter().all(|r| r.kind == RowKind::Add));
        assert_eq!(rows[0].text, "old 0");
        assert_eq!(rows[600].text, "new 0");
        // Under the cell limit the LCS runs and keeps shared lines.
        let small = line_diff("same\nold\n", "same\nnew\n");
        assert_eq!(small[0], row(RowKind::Context, "same"));
    }

    /// Anti-goal: a 1,000 by 1,000 line edit builds its hunks and numbered rows in under 50 ms.
    #[test]
    fn a_1000_by_1000_line_edit_builds_in_under_50_ms() {
        let old: String = (0..1000).map(|i| format!("let a{i} = {i};\n")).collect();
        let new: String = (0..1000).map(|i| format!("let b{i} = {i};\n")).collect();
        let started = Instant::now();
        let hunks = build_hunks(&EditSource {
            old_string: Some(&old),
            new_string: Some(&new),
            ..Default::default()
        });
        let numbered: Vec<Vec<NumberedRow>> = hunks.iter().map(number_rows).collect();
        let (added, removed) = count_changes(&hunks);
        let elapsed = started.elapsed();
        println!("1000x1000 edit built in {elapsed:?}");
        assert_eq!((added, removed), (1000, 1000));
        assert_eq!(numbered[0].len(), 2000);
        assert!(elapsed.as_millis() < 50, "took {elapsed:?}");
    }

    #[test]
    fn hunk_headers_need_the_full_form() {
        assert_eq!(hunk_header("@@ -1,2 +3 @@"), Some((1, Some(2), 3, None)));
        assert_eq!(
            hunk_header("@@ -1 +3,4 @@ fn x"),
            Some((1, None, 3, Some(4)))
        );
        assert_eq!(hunk_header("@@"), None);
        assert_eq!(hunk_header("@@ -1 +3"), None);
    }

    // change-bar.test.ts

    fn counts(bar: &[Square]) -> (usize, usize) {
        (
            bar.iter().filter(|s| **s == Square::Add).count(),
            bar.iter().filter(|s| **s == Square::Remove).count(),
        )
    }

    #[test]
    fn a_write_fills_all_7_squares_green() {
        let (added, removed) = count_changes(&build_hunks(&EditSource {
            content: Some("one\ntwo\nthree\n"),
            ..Default::default()
        }));
        assert_eq!(CHANGE_BAR_SQUARES, 7);
        assert_eq!(change_bar(added, removed), vec![Square::Add; 7]);
    }

    #[test]
    fn a_pure_delete_fills_all_7_squares_red() {
        let (added, removed) = count_changes(&build_hunks(&EditSource {
            old_string: Some("a\nb"),
            new_string: Some(""),
            ..Default::default()
        }));
        assert_eq!(change_bar(added, removed), vec![Square::Remove; 7]);
    }

    #[test]
    fn a_mixed_edit_splits_the_squares_by_share_green_first() {
        assert_eq!(counts(&change_bar(4, 3)), (4, 3));
        assert_eq!(
            change_bar(4, 3),
            [[Square::Add; 4].as_slice(), [Square::Remove; 3].as_slice()].concat()
        );
        assert_eq!(counts(&change_bar(1, 1)), (4, 3));
        // The spec case: 3 to 1 rounds to 5 green squares.
        assert_eq!(counts(&change_bar(3, 1)), (5, 2));
    }

    #[test]
    fn a_small_side_keeps_one_square_when_both_sides_changed() {
        assert_eq!(counts(&change_bar(100, 1)), (6, 1));
        assert_eq!(counts(&change_bar(1, 100)), (1, 6));
    }

    #[test]
    fn no_changes_draw_no_squares() {
        assert_eq!(change_bar(0, 0), vec![]);
    }
}
