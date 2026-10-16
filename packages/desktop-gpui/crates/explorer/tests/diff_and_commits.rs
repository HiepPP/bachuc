//! Tests of the multi-file diff model, the Changes marks and states, the Commits footer model,
//! the Files toolbar order, and the theme syntax colors.

use bachuc_client::{
    ChangedFileDetails, CheckoutDetails, CommitFile, CommitList, CommitSummary, FileDiff,
    FileDiffStatus, FileEntry,
};
use bachuc_explorer::ExplorerDiffEvent;
use bachuc_explorer::changes::{Change, ChangeRow, CheckoutView, UNKNOWN_BRANCH};
use bachuc_explorer::commits::{
    CommitRow, CommitsState, base_label, commit_rows, parse_iso8601, time_ago,
};
use bachuc_explorer::diff::NoDiff;
use bachuc_explorer::diff_doc::{DiffDoc, DiffRow, FileBody, Latest};
use bachuc_explorer::syntax::{ThemeSyntax, highlight};
use bachuc_explorer::tree::{FileTree, ROOT, SortBy};
use bachuc_theme::{Scheme, SyntaxThemeId, syntax_colors_for};
use std::collections::HashSet;
use std::time::Instant;

fn ok_diff(path: &str, text: &str) -> FileDiff {
    FileDiff {
        path: path.into(),
        old_path: None,
        is_new: false,
        is_deleted: false,
        status: FileDiffStatus::Ok,
        additions: 1,
        deletions: 1,
        text: text.into(),
    }
}

fn no_text(path: &str, status: FileDiffStatus) -> FileDiff {
    FileDiff {
        status,
        text: String::new(),
        ..ok_diff(path, "")
    }
}

const HUNK: &str = "--- a/x\n+++ b/x\n@@ -1,2 +1,2 @@\n ctx\n-old\n+new\n";

// ---- multi-file diff ----

#[test]
fn every_file_gets_a_header_in_daemon_order_and_placeholders_keep_their_place() {
    let doc = DiffDoc::new(vec![
        ok_diff("b.rs", HUNK),
        no_text("big.json", FileDiffStatus::TooLarge),
        no_text("logo.png", FileDiffStatus::Binary),
        ok_diff("a.rs", HUNK),
    ]);
    let paths: Vec<&str> = doc.files().iter().map(|file| file.path.as_str()).collect();
    assert_eq!(paths, ["b.rs", "big.json", "logo.png", "a.rs"]);
    // b.rs: header + hunk header + 3 lines; then a header and a placeholder each; then a.rs.
    assert_eq!(
        doc.rows(),
        [
            DiffRow::Header { file: 0 },
            DiffRow::Line { file: 0, line: 0 },
            DiffRow::Line { file: 0, line: 1 },
            DiffRow::Line { file: 0, line: 2 },
            DiffRow::Line { file: 0, line: 3 },
            DiffRow::Header { file: 1 },
            DiffRow::Placeholder {
                file: 1,
                reason: NoDiff::TooLarge
            },
            DiffRow::Header { file: 2 },
            DiffRow::Placeholder {
                file: 2,
                reason: NoDiff::Binary
            },
            DiffRow::Header { file: 3 },
            DiffRow::Line { file: 3, line: 0 },
            DiffRow::Line { file: 3, line: 1 },
            DiffRow::Line { file: 3, line: 2 },
            DiffRow::Line { file: 3, line: 3 },
        ]
    );
    assert!(matches!(doc.files()[0].body, FileBody::Lines(_)));
    assert_eq!(doc.totals(), (4, 4));
    assert_eq!(doc.max_number(), 2);
}

#[test]
fn an_empty_file_list_has_no_rows() {
    let doc = DiffDoc::new(Vec::new());
    assert!(doc.is_empty());
    assert!(doc.rows().is_empty());
    assert_eq!(doc.header_row("a.rs"), None);
    assert!(!doc.all_collapsed());
}

#[test]
fn a_renamed_file_shows_both_paths_and_is_found_by_either() {
    let renamed = FileDiff {
        old_path: Some("src/old.rs".into()),
        ..ok_diff("src/new.rs", HUNK)
    };
    let doc = DiffDoc::new(vec![ok_diff("a.rs", HUNK), renamed]);
    let file = &doc.files()[1];
    assert_eq!(file.display_path(), "src/old.rs -> src/new.rs");
    assert_eq!(file.change, Change::Modified);
    assert_eq!(doc.header_row("src/new.rs"), Some(5));
    assert_eq!(doc.header_row("src/old.rs"), Some(5));
    // An empty old path is no rename.
    let same = FileDiff {
        old_path: Some(String::new()),
        ..ok_diff("c.rs", HUNK)
    };
    assert_eq!(DiffDoc::new(vec![same]).files()[0].display_path(), "c.rs");
}

#[test]
fn a_focus_path_that_is_not_in_the_diff_finds_no_row() {
    let doc = DiffDoc::new(vec![ok_diff("a.rs", HUNK), ok_diff("b.rs", HUNK)]);
    assert_eq!(doc.header_row("b.rs"), Some(5));
    assert_eq!(doc.header_row("gone.rs"), None);
}

#[test]
fn collapsing_keeps_the_header_and_drops_the_body() {
    let mut doc = DiffDoc::new(vec![
        ok_diff("a.rs", HUNK),
        no_text("b.bin", FileDiffStatus::Binary),
    ]);
    doc.toggle_file(0);
    assert!(doc.is_collapsed(0));
    assert_eq!(doc.rows()[0], DiffRow::Header { file: 0 });
    assert_eq!(doc.rows()[1], DiffRow::Header { file: 1 });
    assert_eq!(doc.header_row("b.bin"), Some(1));
    doc.set_all_collapsed(true);
    assert!(doc.all_collapsed());
    assert_eq!(doc.rows().len(), 2);
    doc.set_all_collapsed(false);
    assert_eq!(doc.rows().len(), 7);
}

#[test]
fn new_and_deleted_files_take_their_marks() {
    let added = FileDiff {
        is_new: true,
        ..ok_diff("new.rs", "--- /dev/null\n+++ b/new.rs\n@@ -0,0 +1 @@\n+x\n")
    };
    let deleted = FileDiff {
        is_deleted: true,
        ..ok_diff(
            "gone.rs",
            "--- a/gone.rs\n+++ /dev/null\n@@ -1 +0,0 @@\n-x\n",
        )
    };
    let doc = DiffDoc::new(vec![added, deleted, ok_diff("same.rs", HUNK)]);
    let marks: Vec<Change> = doc.files().iter().map(|file| file.change).collect();
    assert_eq!(marks, [Change::Added, Change::Deleted, Change::Modified]);
}

#[test]
fn an_answer_to_an_older_request_is_dropped() {
    let mut requests = Latest::default();
    let first = requests.begin();
    let second = requests.begin();
    // The first answer arrives after the reload: it is stale.
    assert!(!requests.is_current(first));
    assert!(requests.is_current(second));
    assert_eq!(requests.current(), second);
}

#[test]
fn a_20000_line_diff_of_50_files_builds_its_rows_in_under_200_ms() {
    let mut diffs = Vec::new();
    for file in 0..50 {
        let mut text = format!("--- a/f{file}.rs\n+++ b/f{file}.rs\n@@ -1,400 +1,400 @@\n");
        for line in 0..399 {
            let prefix = ['+', '-', ' '][line % 3];
            text.push_str(&format!(
                "{prefix}let value_{line} = compute({line}, \"text\");\n"
            ));
        }
        diffs.push(ok_diff(&format!("src/f{file}.rs"), &text));
    }
    let start = Instant::now();
    let doc = DiffDoc::new(diffs);
    let elapsed = start.elapsed();
    // 50 headers, 50 hunk headers, and 50 * 399 code lines: 20,050 rows.
    assert_eq!(doc.rows().len(), 50 * 401);
    println!("20,000-line diff rows: {elapsed:?}");
    assert!(
        elapsed.as_millis() < 200,
        "building the rows took {elapsed:?}"
    );
}

// ---- Changes ----

fn details(path: &str) -> ChangedFileDetails {
    ChangedFileDetails {
        path: path.into(),
        old_path: None,
        is_new: false,
        is_deleted: false,
        status: "ok".into(),
        additions: 2,
        deletions: 1,
    }
}

#[test]
fn a_folder_outside_git_shows_the_non_git_state() {
    let view = CheckoutView::new(CheckoutDetails {
        is_git: false,
        branch: None,
        files: Vec::new(),
    });
    assert!(view.is_none());
}

#[test]
fn a_detached_head_stays_a_git_checkout() {
    let view = CheckoutView::new(CheckoutDetails {
        is_git: true,
        branch: None,
        files: vec![details("a.rs")],
    })
    .expect("a git checkout");
    assert_eq!(view.branch, UNKNOWN_BRANCH);
    assert_eq!(view.files.len(), 1);
}

#[test]
fn changes_rows_carry_new_deleted_and_renamed_files() {
    let view = CheckoutView::new(CheckoutDetails {
        is_git: true,
        branch: Some("main".into()),
        files: vec![
            ChangedFileDetails {
                is_new: true,
                ..details("src/new.rs")
            },
            ChangedFileDetails {
                is_deleted: true,
                ..details("src/gone.rs")
            },
            ChangedFileDetails {
                old_path: Some("src/before.rs".into()),
                ..details("src/after.rs")
            },
        ],
    })
    .expect("a git checkout");
    assert_eq!(view.branch, "main");
    assert_eq!(view.change(0), Change::Added);
    assert_eq!(view.change(1), Change::Deleted);
    assert_eq!(view.change(2), Change::Modified);
    assert_eq!(view.label(0), "src/new.rs, New, +2, -1");
    assert_eq!(
        view.label(2),
        "src/before.rs -> src/after.rs, Modified, +2, -1"
    );
    // The tree rows point at the files in daemon order.
    let rows = view.tree.rows(&HashSet::new());
    let indexes: Vec<usize> = rows
        .iter()
        .filter_map(|row| match row {
            ChangeRow::File { index, .. } => Some(*index),
            ChangeRow::Folder { .. } => None,
        })
        .collect();
    assert_eq!(indexes, [2, 1, 0], "after.rs, gone.rs, new.rs by name");
}

// ---- Commits ----

fn commit(sha: &str, on_base: Option<bool>, files: Vec<CommitFile>) -> CommitSummary {
    CommitSummary {
        sha: sha.into(),
        short_sha: sha[..3].into(),
        subject: format!("subject {sha}"),
        author_name: "a".into(),
        author_date: "2026-10-16T01:00:00Z".into(),
        is_on_remote: false,
        is_on_base: on_base,
        files,
    }
}

fn commit_file(path: &str) -> CommitFile {
    CommitFile {
        path: path.into(),
        additions: 1,
        deletions: 0,
        status: Some("modified".into()),
    }
}

#[test]
fn a_failed_commit_list_shows_the_error_state() {
    let state = CommitsState::from_result(Err(anyhow::anyhow!("boom")));
    assert_eq!(state, CommitsState::Failed);
    assert_eq!(state.count(), None);
}

#[test]
fn an_empty_commit_list_names_the_base() {
    let state = CommitsState::from_result(Ok(CommitList {
        base_ref: Some("origin/main".into()),
        commits: Vec::new(),
    }));
    assert_eq!(
        state,
        CommitsState::Empty {
            base: "main".into()
        }
    );
    assert_eq!(state.count(), Some(0));
    // Commits already on the base do not count.
    let only_base = CommitsState::from_result(Ok(CommitList {
        base_ref: None,
        commits: vec![commit("aaaa", Some(true), Vec::new())],
    }));
    assert_eq!(
        only_base,
        CommitsState::Empty {
            base: "base".into()
        }
    );
}

#[test]
fn commits_ahead_of_the_base_list_and_open_to_their_files() {
    let state = CommitsState::from_result(Ok(CommitList {
        base_ref: Some("main".into()),
        commits: vec![
            commit(
                "aaaa",
                Some(false),
                vec![commit_file("a.rs"), commit_file("b.rs")],
            ),
            commit("bbbb", None, vec![commit_file("c.rs")]),
            commit("cccc", Some(true), Vec::new()),
        ],
    }));
    let CommitsState::Ready(commits) = state else {
        panic!("expected commits");
    };
    assert_eq!(commits.len(), 2);
    let closed = commit_rows(&commits, &HashSet::new());
    assert_eq!(
        closed,
        [
            CommitRow::Commit { index: 0 },
            CommitRow::Commit { index: 1 }
        ]
    );
    let open = commit_rows(&commits, &HashSet::from(["aaaa".to_string()]));
    assert_eq!(
        open,
        [
            CommitRow::Commit { index: 0 },
            CommitRow::File { commit: 0, file: 0 },
            CommitRow::File { commit: 0, file: 1 },
            CommitRow::Commit { index: 1 },
        ]
    );
}

#[test]
fn a_commit_file_click_asks_for_its_commit_diff() {
    let summary = commit("abcdef", Some(false), vec![commit_file("src/lib.rs")]);
    assert_eq!(
        ExplorerDiffEvent::commit_file("/repo", &summary, 0),
        Some(ExplorerDiffEvent::OpenCommitDiff {
            cwd: "/repo".into(),
            sha: "abcdef".into(),
            path: "src/lib.rs".into(),
        })
    );
    assert_eq!(ExplorerDiffEvent::commit_file("/repo", &summary, 1), None);
}

#[test]
fn base_labels_drop_ref_prefixes() {
    assert_eq!(base_label(Some("refs/heads/main")), "main");
    assert_eq!(base_label(Some("refs/remotes/origin/dev")), "dev");
    assert_eq!(base_label(Some("HEAD")), "base");
    assert_eq!(base_label(None), "base");
}

#[test]
fn commit_times_read_as_time_ago() {
    let at = parse_iso8601("2026-10-16T01:00:00Z").expect("a time");
    assert_eq!(parse_iso8601("2026-10-16T08:00:00+07:00"), Some(at));
    assert_eq!(parse_iso8601("1970-01-01T00:00:00Z"), Some(0));
    assert_eq!(parse_iso8601("not a date"), None);
    let date = "2026-10-16T01:00:00Z";
    assert_eq!(time_ago(date, at + 5), "just now");
    assert_eq!(time_ago(date, at + 30), "30s ago");
    assert_eq!(time_ago(date, at + 5 * 60), "5m ago");
    assert_eq!(time_ago(date, at + 3 * 3600), "3h ago");
    assert_eq!(time_ago(date, at + 2 * 86_400), "2d ago");
    assert_eq!(time_ago(date, at + 30 * 86_400), "Oct 16");
    assert_eq!(time_ago("garbage", at), "");
}

// ---- Files toolbar ----

fn entry(name: &str, is_dir: bool, size: u64) -> FileEntry {
    FileEntry {
        name: name.into(),
        path: name.into(),
        is_dir,
        size,
    }
}

#[test]
fn the_files_toolbar_sorts_by_size_and_hides_dot_entries() {
    let mut tree = FileTree::new();
    tree.set_listing(
        ROOT,
        vec![
            entry("small.txt", false, 10),
            entry(".env", false, 500),
            entry("src", true, 0),
            entry("big.bin", false, 900),
        ],
    );
    let names = |sort, hidden| -> Vec<String> {
        tree.rows_with(sort, hidden)
            .into_iter()
            .map(|row| row.entry.name)
            .collect()
    };
    assert_eq!(
        names(SortBy::Name, true),
        ["src", ".env", "big.bin", "small.txt"]
    );
    assert_eq!(
        names(SortBy::Size, true),
        ["src", "big.bin", ".env", "small.txt"]
    );
    assert_eq!(names(SortBy::Name, false), ["src", "big.bin", "small.txt"]);
    assert_eq!(SortBy::Name.next(), SortBy::Size);
    assert_eq!(SortBy::Size.next(), SortBy::Name);
    assert!(!tree.only_hidden());

    let mut dots = FileTree::new();
    dots.set_listing(ROOT, vec![entry(".git", true, 0)]);
    assert!(dots.only_hidden());
    assert!(dots.rows_with(SortBy::Name, false).is_empty());
}

// ---- syntax colors ----

#[test]
fn diff_colors_follow_the_selected_syntax_theme() {
    let one = ThemeSyntax(syntax_colors_for(SyntaxThemeId::One, Scheme::Dark));
    let dracula = ThemeSyntax(syntax_colors_for(SyntaxThemeId::Dracula, Scheme::Dark));
    assert_ne!(one, dracula);
    let lines = highlight("rust", "fn main() {}\n", &one);
    assert!(
        lines[0]
            .iter()
            .any(|(range, style)| *range == (0..2) && style.color == Some(one.0.keyword)),
        "fn takes the theme keyword color: {:?}",
        lines[0]
    );
    assert!(
        lines[0]
            .iter()
            .any(|(range, style)| *range == (3..7) && style.color == Some(one.0.function)),
        "main takes the theme function color: {:?}",
        lines[0]
    );
    // A macro name keeps the text color, as the app's grammars give it no role.
    use gpui_kit::base::input::HighlightStyleResolver;
    assert!(one.style("function.macro").is_none());
    assert_eq!(
        one.style("function").and_then(|style| style.color),
        Some(one.0.function)
    );
    assert_eq!(
        one.style("type").and_then(|style| style.color),
        Some(one.0.r#type)
    );
}
