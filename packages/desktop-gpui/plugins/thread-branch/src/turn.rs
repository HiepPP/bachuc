//! The turn diff rules of `shared/turn-diff.ts` and `client/turn-diff-card.tsx` and
//! `turn-diff-panel.tsx`, one function per TS function with the same results, over the
//! `thread-branch-turn-diff` item and the `thread-branch.file-diff` answer.

use base64::Engine as _;
use serde::{Deserialize, Serialize};

/// Blocks in the diffstat bar of a file row (`BAR_BLOCKS`).
pub(crate) const BAR_BLOCKS: usize = 5;
/// The most diff rows the panel shows (`MAX_ROWS`).
pub(crate) const MAX_ROWS: usize = 2_000;
/// The panel code size, which sets the gutter width (`FONT_SIZE`).
pub(crate) const CODE_SIZE: f32 = 12.;

/// `turnDiffSchema`: one turn's changes.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TurnDiff {
    pub file_count: u32,
    pub added: u32,
    pub deleted: u32,
    pub files: Vec<TurnFile>,
    pub commits: Vec<Commit>,
    /// Another agent ran a turn in the same folder at the same time.
    pub shared: bool,
    /// Work-tree snapshots of the turn. A row without them cannot open a file diff.
    #[serde(default)]
    pub source: Option<TurnSource>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct TurnFile {
    pub path: String,
    /// Null counts mean a binary file.
    pub added: Option<u32>,
    pub deleted: Option<u32>,
    /// An untracked file too large to snapshot: listed, with no diff.
    #[serde(default)]
    pub large: bool,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct Commit {
    pub sha: String,
    pub subject: String,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TurnSource {
    pub workspace_id: String,
    pub root: String,
    pub from: String,
    pub to: String,
}

/// `turnHistoryEntrySchema`.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TurnEntry {
    pub key: String,
    pub ended_at: String,
    pub diff: TurnDiff,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct TurnHistory {
    pub turns: Vec<TurnEntry>,
}

/// The `thread-branch.file-diff` and `thread-branch.file-image` input.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize)]
pub(crate) struct FileRequest {
    pub root: String,
    pub from: String,
    pub to: String,
    pub path: String,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct FileDiffReply {
    pub diff: String,
    pub truncated: bool,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FileImageReply {
    /// `data:` URIs of the file before and after the turn; null when that side has no file.
    pub before: Option<String>,
    pub after: Option<String>,
    pub too_large: bool,
}

pub(crate) fn plural(count: usize, one: &str, many: &str) -> String {
    format!("{count} {}", if count == 1 { one } else { many })
}

/// `2 files changed +14 -3 · 1 commit`; a turn with no files shows its commits (`turnDiffHeader`).
pub(crate) fn turn_diff_header(data: &TurnDiff) -> String {
    let files = format!(
        "{} changed",
        plural(data.file_count as usize, "file", "files")
    );
    let commits =
        (!data.commits.is_empty()).then(|| plural(data.commits.len(), "commit", "commits"));
    if data.file_count == 0 {
        return commits.unwrap_or(files);
    }
    let mut header = format!("{files} +{} -{}", data.added, data.deleted);
    if let Some(commits) = commits {
        header.push_str(" · ");
        header.push_str(&commits);
    }
    header
}

/// The image MIME type of a path by its extension, or `None` when it is not a previewable
/// image (`imageType`).
pub(crate) fn image_type(path: &str) -> Option<&'static str> {
    let extension = path.rsplit('.').next()?.to_lowercase();
    Some(match extension.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        "ico" => "image/x-icon",
        "avif" => "image/avif",
        "svg" => "image/svg+xml",
        _ => return None,
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum DiffRow {
    Hunk(String),
    Note(String),
    Add {
        text: String,
        new_line: u32,
    },
    Remove {
        text: String,
        old_line: u32,
    },
    Context {
        text: String,
        old_line: u32,
        new_line: u32,
    },
}

impl DiffRow {
    pub(crate) fn text(&self) -> &str {
        match self {
            DiffRow::Hunk(text)
            | DiffRow::Note(text)
            | DiffRow::Add { text, .. }
            | DiffRow::Remove { text, .. }
            | DiffRow::Context { text, .. } => text,
        }
    }

    /// The larger of the old and new line numbers, for the gutter width.
    fn max_line(&self) -> u32 {
        match self {
            DiffRow::Add { new_line, .. } => *new_line,
            DiffRow::Remove { old_line, .. } => *old_line,
            DiffRow::Context {
                old_line, new_line, ..
            } => (*old_line).max(*new_line),
            DiffRow::Hunk(_) | DiffRow::Note(_) => 0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FileStatus {
    Modified,
    Added,
    Deleted,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ParsedDiff {
    pub status: FileStatus,
    pub binary: bool,
    pub added: u32,
    pub removed: u32,
    pub rows: Vec<DiffRow>,
}

/// The text after the first character, which a diff line spends on its marker.
fn after_marker(line: &str) -> &str {
    line.chars().next().map_or("", |c| &line[c.len_utf8()..])
}

/// `@@ -3,3 +3,3 @@ fn` gives `(3, 3)`: the old and new start lines.
fn hunk_start(line: &str) -> Option<(u32, u32)> {
    fn number(text: &str) -> Option<(u32, &str)> {
        let digits = text.chars().take_while(char::is_ascii_digit).count();
        let (value, rest) = text.split_at(digits);
        let rest = rest.strip_prefix(',').map_or(rest, |counted| {
            let count = counted.chars().take_while(char::is_ascii_digit).count();
            // A comma with no digits after it is not part of the header.
            if count == 0 { rest } else { &counted[count..] }
        });
        Some((value.parse().ok()?, rest))
    }
    let (old, rest) = number(line.strip_prefix("@@ -")?)?;
    let (new, rest) = number(rest.strip_prefix(" +")?)?;
    rest.starts_with(" @@").then_some((old, new))
}

/// Parses a one-file `git diff` into numbered rows, like the gutters of the main diff view
/// (`parseUnifiedDiff`).
pub(crate) fn parse_unified_diff(diff: &str) -> ParsedDiff {
    let mut parsed = ParsedDiff {
        status: FileStatus::Modified,
        binary: false,
        added: 0,
        removed: 0,
        rows: Vec::new(),
    };
    let (mut old_line, mut new_line) = (0u32, 0u32);
    let mut in_hunk = false;
    for line in diff.strip_suffix('\n').unwrap_or(diff).split('\n') {
        if let Some((old, new)) = hunk_start(line) {
            in_hunk = true;
            (old_line, new_line) = (old, new);
            parsed.rows.push(DiffRow::Hunk(line.to_string()));
        } else if !in_hunk {
            if line.starts_with("new file mode") {
                parsed.status = FileStatus::Added;
            } else if line.starts_with("deleted file mode") {
                parsed.status = FileStatus::Deleted;
            } else if line.starts_with("Binary files") {
                parsed.binary = true;
            }
        } else if line.starts_with('+') {
            parsed.rows.push(DiffRow::Add {
                text: after_marker(line).to_string(),
                new_line,
            });
            new_line += 1;
            parsed.added += 1;
        } else if line.starts_with('-') {
            parsed.rows.push(DiffRow::Remove {
                text: after_marker(line).to_string(),
                old_line,
            });
            old_line += 1;
            parsed.removed += 1;
        } else if line.starts_with('\\') {
            // `\ No newline at end of file`: the note spends two characters on its marker.
            parsed
                .rows
                .push(DiffRow::Note(after_marker(after_marker(line)).to_string()));
        } else {
            parsed.rows.push(DiffRow::Context {
                text: after_marker(line).to_string(),
                old_line,
                new_line,
            });
            old_line += 1;
            new_line += 1;
        }
    }
    parsed
}

/// The badge of the file header (`TurnDiffPanel`): binary first, then the status.
pub(crate) fn badge_label(parsed: &ParsedDiff) -> Option<&'static str> {
    if parsed.binary {
        return Some("Binary");
    }
    match parsed.status {
        FileStatus::Added => Some("New file"),
        FileStatus::Deleted => Some("Deleted"),
        FileStatus::Modified => None,
    }
}

/// The highest line number a row shows, which sets the gutter width.
pub(crate) fn max_line(rows: &[DiffRow]) -> u32 {
    rows.iter().map(DiffRow::max_line).max().unwrap_or(0)
}

/// The main diff view's gutter formula: at least 2 digits, each about 0.62 of the code size,
/// plus padding (`gutterWidth`).
pub(crate) fn gutter_width(max_line: u32, code_size: f32) -> f32 {
    let digits = max_line.to_string().len().max(2) as f32;
    digits * (0.62 * code_size).ceil() + 12.
}

/// Whether the rows exceed what the panel draws, or the server cut the diff short.
pub(crate) fn too_long(rows: usize, truncated: bool) -> bool {
    rows > MAX_ROWS || truncated
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Block {
    Add,
    Remove,
    None,
}

/// GitHub-style diffstat blocks: the share of added and removed lines in one file, as counts
/// of add and remove blocks (`barBlocks`).
pub(crate) fn bar_blocks(added: u32, deleted: u32) -> (usize, usize) {
    let total = added as u64 + deleted as u64;
    if total == 0 {
        return (0, 0);
    }
    let add = (BAR_BLOCKS as f64 * added as f64 / total as f64).round() as usize;
    (add, BAR_BLOCKS - add)
}

/// The five blocks of a file row. A binary file (`added` null) has none colored.
pub(crate) fn bar(file: &TurnFile) -> [Block; BAR_BLOCKS] {
    let (add, remove) = match file.added {
        None => (0, 0),
        Some(added) => bar_blocks(added, file.deleted.unwrap_or(0)),
    };
    std::array::from_fn(|index| {
        if index < add {
            Block::Add
        } else if index < add + remove {
            Block::Remove
        } else {
            Block::None
        }
    })
}

/// A file row opens its diff only when the turn has snapshots and the file is not `large`.
pub(crate) fn file_row_pressable(source: Option<&TurnSource>, file: &TurnFile) -> bool {
    source.is_some() && !file.large
}

/// The icon of a file row: images, text files, and binary files differ (`FileRow`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FileKind {
    Image,
    Code,
    Binary,
}

pub(crate) fn file_kind(file: &TurnFile) -> FileKind {
    if image_type(&file.path).is_some() {
        FileKind::Image
    } else if file.added.is_none() {
        FileKind::Binary
    } else {
        FileKind::Code
    }
}

/// The directory with its trailing slash, then the file name (`splitPath`).
pub(crate) fn split_path(path: &str) -> (&str, &str) {
    match path.rfind('/') {
        Some(slash) => path.split_at(slash + 1),
        None => ("", path),
    }
}

/// The turn time in local time, as `Oct 16, 03:45 PM`: the en-US output of `formatTime`.
pub(crate) fn turn_time(iso: &str) -> String {
    match chrono::DateTime::parse_from_rfc3339(iso) {
        Ok(time) => format_turn_time(&time.with_timezone(&chrono::Local)),
        Err(_) => iso.to_string(),
    }
}

fn format_turn_time<Tz: chrono::TimeZone>(time: &chrono::DateTime<Tz>) -> String
where
    Tz::Offset: std::fmt::Display,
{
    time.format("%b %-d, %I:%M %p").to_string()
}

/// Splits a `data:<mime>;base64,<data>` URI into its MIME type and bytes. `None` for another
/// form of URI or bad base64.
pub(crate) fn decode_data_uri(uri: &str) -> Option<(String, Vec<u8>)> {
    let (head, data) = uri.strip_prefix("data:")?.split_once(',')?;
    let mime = head.strip_suffix(";base64")?;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(data)
        .ok()?;
    Some((mime.to_string(), bytes))
}

#[cfg(test)]
mod tests {
    use super::{
        Block, Commit, DiffRow, FileKind, FileStatus, TurnDiff, TurnFile, TurnSource, badge_label,
        bar, bar_blocks, decode_data_uri, file_kind, file_row_pressable, format_turn_time,
        gutter_width, image_type, max_line, parse_unified_diff, split_path, too_long,
        turn_diff_header, turn_time,
    };
    use chrono::TimeZone as _;

    fn turn(file_count: u32, added: u32, deleted: u32, commits: usize) -> TurnDiff {
        TurnDiff {
            file_count,
            added,
            deleted,
            files: Vec::new(),
            commits: (0..commits)
                .map(|index| Commit {
                    sha: format!("abc{index}"),
                    subject: "subject".into(),
                })
                .collect(),
            shared: false,
            source: None,
        }
    }

    fn file(path: &str, added: Option<u32>, deleted: Option<u32>, large: bool) -> TurnFile {
        TurnFile {
            path: path.into(),
            added,
            deleted,
            large,
        }
    }

    fn source() -> TurnSource {
        TurnSource {
            workspace_id: "w1".into(),
            root: "/repo".into(),
            from: "a".repeat(40),
            to: "b".repeat(40),
        }
    }

    #[test]
    fn numbers_old_and_new_lines_and_reads_the_file_status() {
        let parsed = parse_unified_diff(
            "diff --git a/x b/x\n--- a/x\n+++ b/x\n@@ -3,3 +3,3 @@ fn\n keep\n-old\n+new\n\\ No newline at end of file\n",
        );
        assert_eq!(parsed.status, FileStatus::Modified);
        assert_eq!((parsed.added, parsed.removed), (1, 1));
        assert_eq!(
            parsed.rows,
            vec![
                DiffRow::Hunk("@@ -3,3 +3,3 @@ fn".into()),
                DiffRow::Context {
                    text: "keep".into(),
                    old_line: 3,
                    new_line: 3
                },
                DiffRow::Remove {
                    text: "old".into(),
                    old_line: 4
                },
                DiffRow::Add {
                    text: "new".into(),
                    new_line: 4
                },
                DiffRow::Note("No newline at end of file".into()),
            ]
        );
    }

    #[test]
    fn parse_unified_diff_reads_added_deleted_and_binary_files() {
        let binary = parse_unified_diff(
            "diff --git a/i b/i\nnew file mode 100644\nBinary files /dev/null and b/i differ\n",
        );
        assert!(binary.binary);
        assert_eq!(binary.status, FileStatus::Added);
        assert!(binary.rows.is_empty());
        let added = parse_unified_diff(
            "diff --git a/n b/n\nnew file mode 100644\n--- /dev/null\n+++ b/n\n@@ -0,0 +1,2 @@\n+n1\n+n2\n",
        );
        assert_eq!(added.status, FileStatus::Added);
        assert_eq!(added.added, 2);
        assert_eq!(
            added.rows[1..],
            [
                DiffRow::Add {
                    text: "n1".into(),
                    new_line: 1
                },
                DiffRow::Add {
                    text: "n2".into(),
                    new_line: 2
                },
            ]
        );
        let deleted = parse_unified_diff(
            "diff --git a/g b/g\ndeleted file mode 100644\n--- a/g\n+++ /dev/null\n@@ -1,2 +0,0 @@\n-x\n-y\n",
        );
        assert_eq!(deleted.status, FileStatus::Deleted);
        assert_eq!(deleted.removed, 2);
        // The hunk header with no counts, and an empty diff.
        let single = parse_unified_diff("@@ -7 +7 @@\n-a\n+b\n");
        assert_eq!(
            single.rows[1],
            DiffRow::Remove {
                text: "a".into(),
                old_line: 7
            }
        );
        assert_eq!(
            single.rows[2],
            DiffRow::Add {
                text: "b".into(),
                new_line: 7
            }
        );
        assert!(parse_unified_diff("").rows.is_empty());
    }

    #[test]
    fn badges_follow_binary_then_status() {
        assert_eq!(
            badge_label(&parse_unified_diff("Binary files a and b differ")),
            Some("Binary")
        );
        let added = parse_unified_diff("new file mode 100644\n@@ -0,0 +1 @@\n+x\n");
        assert_eq!(badge_label(&added), Some("New file"));
        let deleted = parse_unified_diff("deleted file mode 100644\n@@ -1 +0,0 @@\n-x\n");
        assert_eq!(badge_label(&deleted), Some("Deleted"));
        assert_eq!(
            badge_label(&parse_unified_diff("@@ -1 +1 @@\n-a\n+b\n")),
            None
        );
    }

    #[test]
    fn knows_which_paths_are_previewable_images() {
        assert_eq!(image_type("docs/a/after-1664.png"), Some("image/png"));
        assert_eq!(image_type("x.JPG"), Some("image/jpeg"));
        assert_eq!(image_type("a.PNG"), Some("image/png"));
        assert_eq!(image_type("icon.svg"), Some("image/svg+xml"));
        assert_eq!(image_type("a.txt"), None);
        assert_eq!(image_type("notes.txt"), None);
        assert_eq!(image_type("Makefile"), None);
    }

    #[test]
    fn turn_diff_header_cases() {
        // Zero files with commits: only the commits.
        assert_eq!(turn_diff_header(&turn(0, 0, 0, 2)), "2 commits");
        assert_eq!(turn_diff_header(&turn(0, 0, 0, 1)), "1 commit");
        assert_eq!(turn_diff_header(&turn(0, 0, 0, 0)), "0 files changed");
        // One file.
        assert_eq!(turn_diff_header(&turn(1, 1, 0, 0)), "1 file changed +1 -0");
        // Several files, with and without commits.
        assert_eq!(turn_diff_header(&turn(4, 4, 4, 0)), "4 files changed +4 -4");
        assert_eq!(
            turn_diff_header(&turn(1, 1, 0, 2)),
            "1 file changed +1 -0 · 2 commits"
        );
        assert_eq!(
            turn_diff_header(&turn(3, 9, 2, 1)),
            "3 files changed +9 -2 · 1 commit"
        );
    }

    #[test]
    fn bar_blocks_rule() {
        // 3 added and 1 deleted: 4 add blocks and 1 remove block.
        assert_eq!(bar_blocks(3, 1), (4, 1));
        assert_eq!(bar_blocks(0, 0), (0, 0));
        assert_eq!(bar_blocks(7, 0), (5, 0));
        assert_eq!(bar_blocks(0, 7), (0, 5));
        // JS `Math.round` takes a half up.
        assert_eq!(bar_blocks(1, 1), (3, 2));
        let blocks = bar(&file("a.rs", Some(3), Some(1), false));
        assert_eq!(
            blocks,
            [
                Block::Add,
                Block::Add,
                Block::Add,
                Block::Add,
                Block::Remove
            ]
        );
        assert_eq!(
            bar(&file("a.rs", Some(0), Some(0), false)),
            [Block::None; 5]
        );
        // A binary file has no colored block, even with a deleted count.
        assert_eq!(bar(&file("a.png", None, Some(4), false)), [Block::None; 5]);
        // A deleted count of null counts as zero.
        assert_eq!(bar(&file("a.rs", Some(2), None, false))[4], Block::Add);
    }

    #[test]
    fn gutter_width_rule() {
        // At least 2 digits, each ceil(0.62 * 12) = 8 px, plus 12 px.
        assert_eq!(gutter_width(0, 12.), 28.);
        assert_eq!(gutter_width(99, 12.), 28.);
        assert_eq!(gutter_width(100, 12.), 36.);
        assert_eq!(gutter_width(12_345, 12.), 52.);
        let rows = parse_unified_diff("@@ -98,2 +120,2 @@\n a\n-b\n+c\n").rows;
        assert_eq!(max_line(&rows), 121);
        assert_eq!(max_line(&[]), 0);
    }

    #[test]
    fn file_row_pressable_rule() {
        let plain = file("a.rs", Some(1), Some(0), false);
        assert!(file_row_pressable(Some(&source()), &plain));
        // No snapshots, or a large file: not pressable.
        assert!(!file_row_pressable(None, &plain));
        let large = file("big.bin", None, None, true);
        assert!(!file_row_pressable(Some(&source()), &large));
        assert!(!file_row_pressable(None, &large));
    }

    #[test]
    fn file_rows_pick_their_icon_from_the_path_and_counts() {
        assert_eq!(
            file_kind(&file("a.png", Some(0), Some(0), false)),
            FileKind::Image
        );
        assert_eq!(
            file_kind(&file("a.png", None, None, false)),
            FileKind::Image
        );
        assert_eq!(
            file_kind(&file("a.rs", Some(1), Some(0), false)),
            FileKind::Code
        );
        assert_eq!(
            file_kind(&file("a.dat", None, None, false)),
            FileKind::Binary
        );
        assert_eq!(split_path("src/lib/a.rs"), ("src/lib/", "a.rs"));
        assert_eq!(split_path("a.rs"), ("", "a.rs"));
    }

    #[test]
    fn the_panel_cuts_long_diffs() {
        assert!(!too_long(2_000, false));
        assert!(too_long(2_001, false));
        assert!(too_long(5, true));
    }

    #[test]
    fn data_uri_decodes() {
        let (mime, bytes) = decode_data_uri("data:image/png;base64,iVBORw0=").unwrap();
        assert_eq!(mime, "image/png");
        assert_eq!(bytes, [0x89, 0x50, 0x4e, 0x47, 0x0d]);
        assert!(decode_data_uri("https://x/y.png").is_none());
        assert!(decode_data_uri("data:image/png,plain").is_none());
        assert!(decode_data_uri("data:image/png;base64,!!!").is_none());
    }

    #[test]
    fn turn_time_format() {
        let afternoon = chrono::Utc
            .with_ymd_and_hms(2026, 10, 16, 15, 45, 0)
            .unwrap();
        assert_eq!(format_turn_time(&afternoon), "Oct 16, 03:45 PM");
        // The day has no leading zero; the hour has one.
        let morning = chrono::Utc.with_ymd_and_hms(2026, 1, 5, 9, 5, 0).unwrap();
        assert_eq!(format_turn_time(&morning), "Jan 5, 09:05 AM");
        let midnight = chrono::Utc.with_ymd_and_hms(2026, 1, 5, 0, 0, 0).unwrap();
        assert_eq!(format_turn_time(&midnight), "Jan 5, 12:00 AM");
        // An unreadable time stays as written.
        assert_eq!(turn_time("later"), "later");
    }

    #[test]
    fn turn_and_history_decode_from_the_server_json() {
        let turn: TurnDiff = serde_json::from_value(serde_json::json!({
            "fileCount": 2, "added": 3, "deleted": 1,
            "files": [
                {"path": "a.rs", "added": 3, "deleted": 1},
                {"path": "big.bin", "added": null, "deleted": null, "large": true},
            ],
            "commits": [{"sha": "abc1234", "subject": "feat: x"}],
            "shared": true,
            "source": {"workspaceId": "w1", "root": "/r", "from": "f", "to": "t"},
        }))
        .unwrap();
        assert!(!turn.files[0].large);
        assert!(turn.files[1].large);
        assert_eq!(turn.source.unwrap().workspace_id, "w1");
        let bare: TurnDiff = serde_json::from_value(serde_json::json!({
            "fileCount": 0, "added": 0, "deleted": 0, "files": [], "commits": [], "shared": false,
        }))
        .unwrap();
        assert!(bare.source.is_none());
    }
}
