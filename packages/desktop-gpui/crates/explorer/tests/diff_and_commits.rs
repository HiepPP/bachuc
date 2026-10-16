//! Tests of the multi-file diff model, the Changes marks and states, the Commits footer model,
//! the Files toolbar order, and the theme syntax colors.

use bachuc_client::{
    ChangedFileDetails, CheckoutDetails, CommitFile, CommitList, CommitSummary, DiffCompare,
    DiffMode, FileDiff, FileDiffStatus, FileEntry,
};
use bachuc_explorer::ExplorerDiffEvent;
use bachuc_explorer::changes::{Change, ChangeRow, CheckoutView, UNKNOWN_BRANCH};
use bachuc_explorer::commits::{
    CommitRow, CommitsState, base_label, commit_rows, parse_iso8601, time_ago,
};
use bachuc_explorer::diff::NoDiff;
use bachuc_explorer::diff_doc::{DiffDoc, DiffRow, FileBody, Latest};
use bachuc_explorer::diff_prefs::{
    DiffPrefs, FocusTracker, ModeOverride, clamp_offset, committed_description, compare_for,
    resolve_mode, uniform_row_height,
};
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

#[test]
fn a_markdown_heading_takes_the_heading_color() {
    let one = ThemeSyntax(syntax_colors_for(SyntaxThemeId::One, Scheme::Dark));
    let lines = highlight("markdown", "# Title\n\ntext\n", &one);
    assert!(
        lines[0]
            .iter()
            .any(|(_, style)| style.color == Some(one.0.heading)),
        "the heading takes the heading color: {:?}",
        lines[0]
    );
}

// ---- focus requests ----

#[test]
fn a_new_focus_revision_scrolls_once_and_a_seen_one_does_not() {
    let mut prefs = DiffPrefs::default();
    let mut tracker = FocusTracker::new(prefs.focus_revision(), None);
    assert!(!tracker.observe(prefs.focus(), "/repo"));
    prefs.request_focus("/repo", "b.rs");
    assert!(tracker.observe(prefs.focus(), "/repo"));
    assert_eq!(tracker.take(true).as_deref(), Some("b.rs"));
    // The same revision again is seen: no second scroll.
    assert!(!tracker.observe(prefs.focus(), "/repo"));
    assert_eq!(tracker.take(true), None);
    // A request for another folder is not for this view.
    prefs.request_focus("/other", "c.rs");
    assert!(!tracker.observe(prefs.focus(), "/repo"));
    assert_eq!(tracker.take(true), None);
}

#[test]
fn a_view_created_by_a_click_takes_its_revision_and_scrolls_by_its_own_path() {
    let mut prefs = DiffPrefs::default();
    prefs.request_focus("/repo", "a.rs");
    let mut tracker = FocusTracker::new(prefs.focus_revision(), Some("a.rs".into()));
    assert!(!tracker.observe(prefs.focus(), "/repo"));
    assert_eq!(tracker.take(true).as_deref(), Some("a.rs"));
    assert_eq!(tracker.take(true), None);
}

#[test]
fn a_focus_request_before_the_first_answer_applies_after_it() {
    let mut prefs = DiffPrefs::default();
    let mut tracker = FocusTracker::new(prefs.focus_revision(), None);
    prefs.request_focus("/repo", "b.rs");
    assert!(tracker.observe(prefs.focus(), "/repo"));
    assert_eq!(tracker.take(false), None);
    assert_eq!(tracker.take(true).as_deref(), Some("b.rs"));
}

#[test]
fn a_focus_request_for_a_collapsed_file_expands_it_then_finds_its_header() {
    let mut doc = DiffDoc::new(vec![ok_diff("a.rs", HUNK), ok_diff("b.rs", HUNK)]);
    doc.toggle_file(1);
    assert!(doc.is_collapsed(1));
    assert_eq!(doc.reveal("b.rs"), Some(5));
    assert!(!doc.is_collapsed(1));
    assert_eq!(doc.rows().len(), 10);
    assert_eq!(doc.rows()[5], DiffRow::Header { file: 1 });
}

#[test]
fn a_focus_request_for_a_path_not_in_the_diff_changes_nothing() {
    let mut doc = DiffDoc::new(vec![ok_diff("a.rs", HUNK)]);
    doc.toggle_file(0);
    let rows = doc.rows().to_vec();
    assert_eq!(doc.reveal("gone.rs"), None);
    assert_eq!(doc.rows(), rows);
    assert!(doc.is_collapsed(0));
    assert_eq!(DiffDoc::default().reveal("a.rs"), None);
}

// ---- long lines ----

#[test]
fn sideways_offsets_clamp_to_the_code_that_does_not_fit() {
    // Longest line 1,000 px, code area 600 px.
    assert_eq!(clamp_offset(0., 250., 1000., 600.), 250.);
    assert_eq!(clamp_offset(250., 500., 1000., 600.), 400.);
    assert_eq!(clamp_offset(400., -1000., 1000., 600.), 0.);
    // A file with no long line never moves.
    assert_eq!(clamp_offset(0., 300., 500., 600.), 0.);
    assert_eq!(clamp_offset(0., -300., 500., 600.), 0.);
}

#[test]
fn each_file_is_sized_by_its_own_longest_drawn_line() {
    let long = "x".repeat(400);
    let text = format!("--- a/x\n+++ b/x\n@@ -1,1 +1,2 @@\n ctx\n+{long}\n");
    let doc = DiffDoc::new(vec![
        ok_diff("long.rs", &text),
        ok_diff("tabs.rs", "@@ -1,1 +1,1 @@\n+\tab\n"),
    ]);
    assert_eq!(doc.longest_columns(0), 400);
    // A tab fills to the next 4-column stop; the hunk header is the widest line here.
    assert_eq!(doc.longest_columns(1), "@@ -1,1 +1,1 @@".len());
    assert_eq!(doc.longest_columns(2), 0);
    let tabs = DiffDoc::new(vec![ok_diff(
        "t.rs",
        "@@ -1 +1 @@\n+\t\tlong_tabbed_line\n",
    )]);
    assert_eq!(tabs.longest_columns(0), 8 + "long_tabbed_line".len());
}

#[test]
fn rows_keep_one_height_while_scrolling_and_measure_while_wrapping() {
    assert!(!DiffPrefs::default().wrap_lines);
    assert_eq!(uniform_row_height(false, 22.), Some(22.));
    assert_eq!(uniform_row_height(true, 22.), None);
}

#[test]
fn sideways_scroll_leaves_the_row_model_unchanged() {
    let long = "y".repeat(1000);
    let doc = DiffDoc::new(vec![ok_diff("a.rs", &format!("@@ -1 +1 @@\n+{long}\n"))]);
    let rows = doc.rows().to_vec();
    let mut offsets = vec![0.; doc.files().len()];
    offsets[0] = clamp_offset(offsets[0], 80., 8. + 1000. * 7.5 + 12., 600.);
    assert_eq!(offsets[0], 80.);
    assert_eq!(doc.rows(), rows);
}

// ---- diff mode ----

#[test]
fn the_mode_defaults_by_the_dirty_state_and_an_override_expires_when_it_changes() {
    let mut prefs = DiffPrefs::default();
    // No status yet: the uncommitted diff, as before modes.
    assert_eq!(prefs.mode("/repo"), DiffMode::Uncommitted);
    prefs.set_status("/repo", false, Some("feature".into()));
    assert_eq!(prefs.mode("/repo"), DiffMode::Base);
    prefs.set_status("/repo", true, Some("feature".into()));
    assert_eq!(prefs.mode("/repo"), DiffMode::Uncommitted);
    // Base picked while dirty holds while the tree stays dirty.
    prefs.select_mode("/repo", DiffMode::Base);
    assert_eq!(prefs.mode("/repo"), DiffMode::Base);
    assert!(!prefs.set_status("/repo", true, Some("feature".into())));
    assert_eq!(prefs.mode("/repo"), DiffMode::Base);
    // The tree goes clean: the override is gone and the clean default applies.
    assert!(prefs.set_status("/repo", false, Some("feature".into())));
    assert_eq!(prefs.checkout("/repo").mode_override, None);
    assert_eq!(prefs.mode("/repo"), DiffMode::Base);
    // Uncommitted picked while clean ends when the tree goes dirty, then clean again.
    prefs.select_mode("/repo", DiffMode::Uncommitted);
    assert_eq!(prefs.mode("/repo"), DiffMode::Uncommitted);
    prefs.set_status("/repo", true, Some("feature".into()));
    prefs.set_status("/repo", false, Some("feature".into()));
    assert_eq!(prefs.mode("/repo"), DiffMode::Base);
    // Each folder keeps its own mode.
    assert_eq!(prefs.mode("/other"), DiffMode::Uncommitted);
}

#[test]
fn a_stale_override_is_masked_even_before_it_expires() {
    let chosen = ModeOverride {
        mode: DiffMode::Uncommitted,
        dirty_at_selection: false,
    };
    assert_eq!(resolve_mode(Some(chosen), false), DiffMode::Uncommitted);
    assert_eq!(resolve_mode(Some(chosen), true), DiffMode::Uncommitted);
    let base = ModeOverride {
        mode: DiffMode::Base,
        dirty_at_selection: false,
    };
    assert_eq!(resolve_mode(Some(base), true), DiffMode::Uncommitted);
    assert_eq!(resolve_mode(None, false), DiffMode::Base);
}

#[test]
fn base_sends_the_loaded_base_ref_and_uncommitted_sends_none() {
    assert_eq!(
        compare_for(DiffMode::Base, Some("origin/main"), false),
        DiffCompare {
            mode: DiffMode::Base,
            base_ref: Some("origin/main".into()),
            ignore_whitespace: false,
        }
    );
    assert_eq!(compare_for(DiffMode::Base, None, false).base_ref, None);
    assert_eq!(
        compare_for(DiffMode::Base, Some("  "), false).base_ref,
        None
    );
    assert_eq!(
        compare_for(DiffMode::Uncommitted, Some("origin/main"), false).base_ref,
        None
    );
}

#[test]
fn committed_picked_before_the_commit_list_loads_asks_without_a_base_ref() {
    let mut prefs = DiffPrefs::default();
    prefs.set_status("/repo", true, Some("feature".into()));
    prefs.select_mode("/repo", DiffMode::Base);
    let before = prefs.compare("/repo");
    assert_eq!((before.mode, before.base_ref), (DiffMode::Base, None));
    assert_eq!(prefs.tree_compare("/repo"), Some(prefs.compare("/repo")));
    assert!(prefs.set_base_ref("/repo", Some("origin/main".into())));
    assert!(!prefs.set_base_ref("/repo", Some("origin/main".into())));
    assert_eq!(
        prefs.compare("/repo").base_ref.as_deref(),
        Some("origin/main")
    );
    // Uncommitted lists the status files, so the tree needs no diff.
    prefs.select_mode("/repo", DiffMode::Uncommitted);
    assert_eq!(prefs.tree_compare("/repo"), None);
}

#[test]
fn an_answer_for_the_mode_before_a_pick_is_dropped() {
    let mut prefs = DiffPrefs::default();
    prefs.set_status("/repo", true, None);
    let mut requests = Latest::default();
    let uncommitted = (requests.begin(), prefs.compare("/repo"));
    prefs.select_mode("/repo", DiffMode::Base);
    let base = (requests.begin(), prefs.compare("/repo"));
    assert_ne!(uncommitted.1, base.1);
    assert!(!requests.is_current(uncommitted.0));
    assert!(requests.is_current(base.0));
}

#[test]
fn the_hide_whitespace_toggle_sets_ignore_whitespace_on_the_next_request() {
    let mut prefs = DiffPrefs::default();
    assert!(!prefs.hide_whitespace);
    assert!(!prefs.compare("/repo").ignore_whitespace);
    prefs.hide_whitespace = true;
    assert!(prefs.compare("/repo").ignore_whitespace);
    prefs.set_status("/repo", false, None);
    let base = prefs.compare("/repo");
    assert_eq!((base.mode, base.ignore_whitespace), (DiffMode::Base, true));
}

#[test]
fn the_committed_description_drops_ref_prefixes_and_skips_equal_or_missing_names() {
    assert_eq!(
        committed_description(Some("feature"), Some("refs/remotes/origin/main")).as_deref(),
        Some("feature -> main")
    );
    assert_eq!(
        committed_description(Some("refs/heads/feature"), Some("origin/main")).as_deref(),
        Some("feature -> main")
    );
    assert_eq!(
        committed_description(Some("main"), Some("origin/main")),
        None
    );
    assert_eq!(committed_description(None, Some("main")), None);
    assert_eq!(committed_description(Some("feature"), None), None);
    assert_eq!(committed_description(Some("feature"), Some(" ")), None);
}

#[test]
fn the_committed_tree_lists_the_base_diff_files() {
    let mut renamed = ok_diff("src/new.rs", HUNK);
    renamed.old_path = Some("src/old.rs".into());
    let view = CheckoutView::from_diffs(
        Some("feature".into()),
        &[renamed, no_text("logo.png", FileDiffStatus::Binary)],
    );
    assert_eq!(view.branch, "feature");
    assert_eq!(view.files.len(), 2);
    assert_eq!(view.files[0].old_path.as_deref(), Some("src/old.rs"));
    assert_eq!(view.files[1].status, "binary");
    assert_eq!(view.label(0), "src/old.rs -> src/new.rs, Modified, +1, -1");
    assert_eq!(
        CheckoutView::from_diffs(None, &[]).branch,
        UNKNOWN_BRANCH.to_string()
    );
}
