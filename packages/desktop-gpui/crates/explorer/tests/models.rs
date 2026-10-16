//! Tests of the pure models: the Files tree, the Changes tree, the diff parser, the line
//! splitter of the highlighter, and the file icon lookup.

use bachuc_client::{ChangedFile, FileEntry};
use bachuc_explorer::changes::{ChangeRow, ChangeTree, format_count};
use bachuc_explorer::diff::{LineKind, max_number, parse};
use bachuc_explorer::file_icons::icon_name;
use bachuc_explorer::syntax::{Run, expand_tabs, language_for, split_lines};
use bachuc_explorer::tree::{FileTree, ROOT, RootState, Toggled};
use gpui_kit::{HighlightStyle, hsla};
use std::collections::HashSet;

fn entry(path: &str, is_dir: bool) -> FileEntry {
    FileEntry {
        name: path.rsplit('/').next().unwrap().to_string(),
        path: path.to_string(),
        is_dir,
        size: 0,
    }
}

fn paths(tree: &FileTree) -> Vec<(String, usize)> {
    tree.rows()
        .into_iter()
        .map(|row| (row.entry.path, row.depth))
        .collect()
}

// ---- Files tree ----

#[test]
fn tree_lists_folders_first_then_names_without_case() {
    let mut tree = FileTree::new();
    tree.set_listing(
        ROOT,
        vec![
            entry("zebra.txt", false),
            entry("Beta.md", false),
            entry("src", true),
            entry("alpha.rs", false),
            entry("Docs", true),
            entry(".gitignore", false),
        ],
    );
    let order: Vec<String> = paths(&tree).into_iter().map(|(path, _)| path).collect();
    assert_eq!(
        order,
        [
            "Docs",
            "src",
            ".gitignore",
            "alpha.rs",
            "Beta.md",
            "zebra.txt"
        ]
    );
}

#[test]
fn tree_root_state_follows_the_listing() {
    let mut tree = FileTree::new();
    assert_eq!(tree.root_state(), RootState::Loading);
    tree.start_load(ROOT);
    assert_eq!(tree.root_state(), RootState::Loading);
    tree.set_listing(ROOT, vec![]);
    assert_eq!(tree.root_state(), RootState::Empty);
    tree.set_listing(ROOT, vec![entry("a", false)]);
    assert_eq!(tree.root_state(), RootState::Ready);
    tree.set_failed(ROOT, "no access".into());
    assert_eq!(tree.root_state(), RootState::Failed("no access".into()));
}

#[test]
fn tree_lists_only_expanded_folders_and_loads_them_lazily() {
    let mut tree = FileTree::new();
    tree.set_listing(ROOT, vec![entry("src", true), entry("docs", true)]);
    assert_eq!(paths(&tree).len(), 2, "closed folders show no children");

    assert_eq!(tree.toggle("src"), Toggled::Expanded { needs_load: true });
    let rows = tree.rows();
    assert!(rows[1].expanded || rows[0].expanded);
    assert!(
        rows.iter()
            .any(|row| row.entry.path == "src" && row.loading),
        "an open folder without a listing shows the loading mark"
    );

    tree.start_load("src");
    assert!(
        !tree.needs_load("src"),
        "a requested folder is not requested twice"
    );
    tree.set_listing(
        "src",
        vec![entry("src/main.rs", false), entry("src/ui", true)],
    );
    assert_eq!(
        paths(&tree),
        [
            ("docs".to_string(), 0),
            ("src".to_string(), 0),
            ("src/ui".to_string(), 1),
            ("src/main.rs".to_string(), 1),
        ]
    );

    assert_eq!(tree.toggle("src"), Toggled::Collapsed);
    assert_eq!(paths(&tree).len(), 2);
    // The listing stays, so the next open needs no request.
    assert_eq!(tree.toggle("src"), Toggled::Expanded { needs_load: false });
}

#[test]
fn tree_with_many_entries_lists_only_expanded_folders() {
    let mut tree = FileTree::new();
    let folders: Vec<FileEntry> = (0..50)
        .map(|i| entry(&format!("dir{i:02}"), true))
        .collect();
    tree.set_listing(ROOT, folders);
    // A folder with 12,000 files is not expanded, so no row of it exists.
    let files: Vec<FileEntry> = (0..12_000)
        .map(|i| entry(&format!("dir07/file{i:05}.txt"), false))
        .collect();
    tree.set_listing("dir07", files);
    assert_eq!(tree.rows().len(), 50);
    tree.toggle("dir07");
    assert_eq!(tree.rows().len(), 50 + 12_000);
}

#[test]
fn tree_invalidate_keeps_the_open_folders() {
    let mut tree = FileTree::new();
    tree.set_listing(ROOT, vec![entry("src", true)]);
    tree.toggle("src");
    tree.invalidate();
    assert_eq!(tree.expanded_paths(), ["src"]);
    assert_eq!(tree.root_state(), RootState::Loading);
    tree.reset();
    assert!(tree.expanded_paths().is_empty());
}

// ---- Changes tree ----

fn changed(path: &str, added: u32, removed: u32) -> ChangedFile {
    ChangedFile {
        path: path.to_string(),
        status: "ok".to_string(),
        added,
        removed,
    }
}

fn labels(rows: &[ChangeRow]) -> Vec<String> {
    rows.iter()
        .map(|row| match row {
            ChangeRow::Folder {
                display_name,
                depth,
                added,
                removed,
                ..
            } => format!("{depth}:{display_name}/ +{added} -{removed}"),
            ChangeRow::File { name, depth, .. } => format!("{depth}:{name}"),
        })
        .collect()
}

#[test]
fn changes_group_by_folder_and_merge_single_child_chains() {
    let files = [
        changed("packages/app/src/a.ts", 3, 1),
        changed("packages/app/src/b.ts", 2, 0),
        changed("README.md", 1, 1),
        changed("docs/guide/x.md", 4, 4),
    ];
    let tree = ChangeTree::build(&files);
    let rows = tree.rows(&HashSet::new());
    assert_eq!(
        labels(&rows),
        [
            "0:docs/guide/ +4 -4",
            "1:x.md",
            "0:packages/app/src/ +5 -1",
            "1:a.ts",
            "1:b.ts",
            "0:README.md",
        ]
    );
}

#[test]
fn changes_folder_with_a_file_and_a_folder_does_not_merge() {
    let files = [changed("a/x.rs", 1, 0), changed("a/b/y.rs", 1, 0)];
    let rows = ChangeTree::build(&files).rows(&HashSet::new());
    assert_eq!(
        labels(&rows),
        ["0:a/ +2 -0", "1:b/ +1 -0", "2:y.rs", "1:x.rs"]
    );
}

#[test]
fn changes_collapse_hides_children_but_keeps_the_counts() {
    let files = [changed("src/a.ts", 3, 1), changed("src/b.ts", 2, 0)];
    let tree = ChangeTree::build(&files);
    let collapsed = HashSet::from(["src".to_string()]);
    assert_eq!(labels(&tree.rows(&collapsed)), ["0:src/ +5 -1"]);
    assert_eq!(tree.folder_paths(), ["src"]);
}

#[test]
fn changes_row_index_points_into_the_input_files() {
    let files = [changed("z/last.rs", 1, 0), changed("a/first.rs", 1, 0)];
    let tree = ChangeTree::build(&files);
    let indexes: Vec<usize> = tree
        .rows(&HashSet::new())
        .iter()
        .filter_map(|row| match row {
            ChangeRow::File { index, .. } => Some(*index),
            ChangeRow::Folder { .. } => None,
        })
        .collect();
    assert_eq!(indexes, [1, 0]);
    assert_eq!(tree.files()[1].path, "a/first.rs");
}

#[test]
fn counts_use_the_compact_form() {
    let shown: Vec<String> = [0, 7, 999, 1000, 1234, 12_345, 999_999, 1_500_000]
        .into_iter()
        .map(format_count)
        .collect();
    assert_eq!(
        shown,
        ["0", "7", "999", "1k", "1.2k", "12.3k", "1m", "1.5m"]
    );
}

// ---- diff parser ----

const DIFF: &str = "--- a/src/lib.rs\n+++ b/src/lib.rs\n@@ -1,4 +1,5 @@ fn main\n keep one\n-old two\n+new two\n+new three\n keep four\n\\ No newline at end of file\n@@ -20,2 +21,2 @@\n tail\n-gone\n+here\n";

#[test]
fn diff_numbers_removed_lines_by_the_old_side_and_the_rest_by_the_new_side() {
    let parsed = parse(DIFF);
    let shown: Vec<(LineKind, Option<u32>, &str)> = parsed
        .lines
        .iter()
        .map(|line| (line.kind, line.number, line.text.as_str()))
        .collect();
    assert_eq!(
        shown,
        [
            (LineKind::Header, None, "@@ -1,4 +1,5 @@ fn main"),
            (LineKind::Context, Some(1), "keep one"),
            (LineKind::Remove, Some(2), "old two"),
            (LineKind::Add, Some(2), "new two"),
            (LineKind::Add, Some(3), "new three"),
            (LineKind::Context, Some(4), "keep four"),
            (LineKind::Header, None, "@@ -20,2 +21,2 @@"),
            (LineKind::Context, Some(21), "tail"),
            (LineKind::Remove, Some(21), "gone"),
            (LineKind::Add, Some(22), "here"),
        ]
    );
    assert_eq!((parsed.added, parsed.removed), (3, 2));
    assert!(!parsed.is_new && !parsed.is_deleted);
    assert_eq!(max_number(&parsed.lines), 22);
}

#[test]
fn diff_marks_new_and_deleted_files_by_dev_null() {
    let new = parse("--- /dev/null\n+++ b/a.txt\n@@ -0,0 +1,2 @@\n+one\n+two\n");
    assert!(new.is_new && !new.is_deleted);
    assert_eq!(new.added, 2);
    let deleted = parse("--- a/a.txt\n+++ /dev/null\n@@ -1,1 +0,0 @@\n-one\n");
    assert!(deleted.is_deleted && !deleted.is_new);
}

#[test]
fn diff_keeps_content_that_looks_like_a_file_header_inside_a_hunk() {
    let parsed = parse("--- a/x\n+++ b/x\n@@ -1,2 +1,2 @@\n--- not a header\n+++ also content\n");
    let kinds: Vec<LineKind> = parsed.lines.iter().map(|line| line.kind).collect();
    assert_eq!(kinds, [LineKind::Header, LineKind::Remove, LineKind::Add]);
    assert_eq!(parsed.lines[1].text, "-- not a header");
}

#[test]
fn diff_of_nothing_has_no_lines() {
    assert!(parse("").lines.is_empty());
    assert!(parse("--- a/x\n+++ b/x\n").lines.is_empty());
}

#[test]
fn no_diff_reason_reads_the_end_of_the_client_message() {
    use bachuc_explorer::diff::{NoDiff, no_diff_reason};
    assert_eq!(
        no_diff_reason("a/big.lock has no diff: too_large"),
        Some(NoDiff::TooLarge)
    );
    assert_eq!(
        no_diff_reason("img/logo.png has no diff: binary"),
        Some(NoDiff::Binary)
    );
    // A path that holds the word "binary" is not a binary file.
    assert_eq!(no_diff_reason("binary/too_large.rs: connection lost"), None);
}

// ---- highlighter helpers ----

fn colored() -> HighlightStyle {
    HighlightStyle {
        color: Some(hsla(0.5, 1., 0.5, 1.)),
        ..Default::default()
    }
}

#[test]
fn split_lines_cuts_a_range_that_crosses_line_breaks() {
    let text = "ab\ncd\nef";
    let style = colored();
    let runs: Vec<Run> = vec![(1..7, style)];
    let lines = split_lines(text, &runs);
    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0], vec![(1..2, style)]);
    assert_eq!(lines[1], vec![(0..2, style)]);
    assert_eq!(lines[2], vec![(0..1, style)]);
}

#[test]
fn split_lines_skips_uncolored_ranges_and_empty_lines() {
    let text = "a\n\nb";
    let plain = HighlightStyle::default();
    let style = colored();
    let runs: Vec<Run> = vec![(0..4, plain), (3..4, style)];
    let lines = split_lines(text, &runs);
    assert_eq!(lines[0], vec![]);
    assert_eq!(lines[1], vec![]);
    assert_eq!(lines[2], vec![(0..1, style)]);
}

#[test]
fn tabs_expand_to_the_next_stop_and_move_the_runs() {
    let style = colored();
    let (text, runs) = expand_tabs("\tab\tc", &[(1..3, style), (4..5, style)]);
    assert_eq!(text, "    ab  c");
    assert_eq!(runs, vec![(4..6, style), (8..9, style)]);
    let (same, same_runs) = expand_tabs("no tabs", &[(0..2, style)]);
    assert_eq!(same, "no tabs");
    assert_eq!(same_runs, vec![(0..2, style)]);
}

#[test]
fn the_highlighter_colors_rust_and_json_by_line() {
    use bachuc_explorer::syntax::{SyntaxColors, highlight};
    use bachuc_theme::Scheme;

    let colors = SyntaxColors::for_scheme(Scheme::Light);
    let lines = highlight("rust", "fn main() {\n    let x = \"a\"; // c\n}\n", &colors);
    assert_eq!(
        lines.len(),
        4,
        "three lines and the empty one after the last break"
    );
    let keyword = colors.keyword;
    assert!(
        lines[0]
            .iter()
            .any(|(range, style)| *range == (0..2) && style.color == Some(keyword)),
        "fn is a keyword: {:?}",
        lines[0]
    );
    assert!(
        lines[1]
            .iter()
            .any(|(_, style)| style.color == Some(colors.string)),
        "the string is colored: {:?}",
        lines[1]
    );
    assert!(
        lines[1]
            .iter()
            .any(|(_, style)| style.color == Some(colors.comment)),
        "the comment is colored: {:?}",
        lines[1]
    );

    let json = highlight("json", "{\"a\": 1}", &colors);
    assert!(!json[0].is_empty(), "json has colors: {:?}", json[0]);

    let plain = highlight("text", "just words", &colors);
    assert!(plain[0].is_empty());
}

#[test]
fn languages_resolve_for_the_enabled_grammars_only() {
    assert_eq!(language_for("src/lib.rs"), Some("rust"));
    assert_eq!(language_for("a/b.TSX"), Some("tsx"));
    assert_eq!(language_for("run.sh"), Some("bash"));
    assert_eq!(language_for("data.json"), Some("json"));
    assert_eq!(language_for("style.css"), None);
    assert_eq!(language_for("Makefile"), None);
}

// ---- file icons ----

#[test]
fn file_icons_follow_the_extension_table() {
    assert_eq!(icon_name("main.rs"), "rust");
    assert_eq!(icon_name("App.TSX"), "react_ts");
    assert_eq!(icon_name("pic.PNG"), "image");
    assert_eq!(icon_name("Makefile"), "_default");
    assert_eq!(icon_name("trailing."), "_default");
    assert_eq!(icon_name("a.unknownext"), "_default");
}
