use bachuc_client::{AgentSummary, ChangedFile, ProjectNode, WorkspaceNode};
use bachuc_sidebar::model::*;

fn agent(id: &str, status: &str, pending: usize) -> AgentSummary {
    AgentSummary {
        id: id.into(),
        title: id.into(),
        status: status.into(),
        cwd: "/tmp".into(),
        workspace_id: None,
        provider: "mock".into(),
        model: None,
        pending_permissions: pending,
    }
}

fn workspace(id: &str, pinned: bool, agent_ids: &[&str]) -> WorkspaceNode {
    WorkspaceNode {
        id: id.into(),
        name: format!("name-{id}"),
        cwd: format!("/work/{id}"),
        branch: Some("main".into()),
        pinned,
        agent_ids: agent_ids.iter().map(|id| id.to_string()).collect(),
    }
}

fn project(id: &str, workspaces: Vec<WorkspaceNode>) -> ProjectNode {
    ProjectNode {
        id: id.into(),
        name: format!("project-{id}"),
        root: format!("/work/{id}"),
        workspaces,
    }
}

fn ids(rows: &[Row]) -> Vec<String> {
    rows.iter()
        .map(|row| match row {
            Row::PinnedHeader { .. } => "pinned".to_string(),
            Row::SectionTitle => "title".to_string(),
            Row::Project(p) => format!("p:{}", p.id),
            Row::Workspace(w) => format!("w:{}", w.id),
            Row::ShowMore { group, .. } => format!("more:{group:?}"),
            Row::Empty => "empty".to_string(),
        })
        .collect()
}

fn file(added: u32, removed: u32) -> ChangedFile {
    ChangedFile {
        path: "a.rs".into(),
        status: "M".into(),
        added,
        removed,
    }
}

#[test]
fn no_projects_gives_the_empty_state() {
    assert_eq!(ids(&build_rows(&[], &[], &ViewState::default())), ["empty"]);
}

#[test]
fn projects_and_workspaces_sort_by_name_like_electron() {
    let named = |id: &str, name: &str| WorkspaceNode {
        name: name.into(),
        ..workspace(id, false, &[])
    };
    let projects = [
        ProjectNode {
            name: "wp-test".into(),
            ..project("w", vec![])
        },
        ProjectNode {
            name: ".claude".into(),
            ..project(
                "c",
                vec![
                    named("c1", "Research Jump Desktop"),
                    named("c2", "Re-enable Hammerspoon"),
                    named("c3", "Check chase-goal"),
                    named("c4", "/caveman ultra"),
                    named("c5", "Check Claude CLI"),
                ],
            )
        },
        ProjectNode {
            name: "calendar-ref".into(),
            ..project("r", vec![])
        },
        ProjectNode {
            name: "App".into(),
            ..project("a", vec![])
        },
    ];
    let rows = build_rows(&projects, &[], &ViewState::default());
    assert_eq!(
        ids(&rows),
        [
            "title", "p:c", "w:c4", "w:c3", "w:c5", "w:c2", "w:c1", "p:a", "p:r", "p:w"
        ],
        "the reference shows .claude, App, calendar-ref, ... and /caveman first"
    );
}

#[test]
fn names_compare_without_case_and_with_numbers_by_value() {
    use std::cmp::Ordering::*;
    assert_eq!(natural_compare("app", "App"), Equal);
    assert_eq!(natural_compare("task-2", "task-10"), Less);
    assert_eq!(natural_compare(".claude", "App"), Less);
    assert_eq!(
        natural_compare("a b", "a-b"),
        Less,
        "a space sorts before punctuation"
    );
    assert_eq!(
        natural_compare("a_b", "a1"),
        Less,
        "punctuation sorts before digits"
    );
    assert_eq!(natural_compare("abc", "ab"), Greater);
}

#[test]
fn diff_counts_sum_the_files_and_print_compact() {
    assert_eq!(diff_stat(&[]), None);
    assert_eq!(
        diff_stat(&[file(0, 0)]),
        None,
        "a clean checkout shows nothing"
    );
    assert_eq!(diff_stat(&[file(50, 30), file(5, 5)]), Some((55, 35)));
    assert_eq!(diff_stat(&[file(742, 0)]), Some((742, 0)));
    assert_eq!(format_diff_count(0), "0");
    assert_eq!(format_diff_count(999), "999");
    assert_eq!(format_diff_count(1000), "1k");
    assert_eq!(format_diff_count(1234), "1.2k");
    assert_eq!(format_diff_count(15_300), "15.3k");
    assert_eq!(format_diff_count(999_950), "1m");
    assert_eq!(format_diff_count(2_500_000), "2.5m");
}

#[test]
fn the_tree_lists_projects_and_workspaces_in_name_order() {
    let projects = [
        project(
            "a",
            vec![workspace("a1", false, &[]), workspace("a2", false, &[])],
        ),
        project("b", vec![workspace("b1", false, &[])]),
    ];
    let rows = build_rows(&projects, &[], &ViewState::default());
    assert_eq!(
        ids(&rows),
        ["title", "p:a", "w:a1", "w:a2", "p:b", "w:b1"],
        "no agent rows, and no pinned header without a pinned workspace"
    );
}

#[test]
fn a_collapsed_project_hides_its_workspaces_and_shows_the_urgent_status() {
    let projects = [project(
        "a",
        vec![
            workspace("a1", false, &["x"]),
            workspace("a2", false, &["y"]),
        ],
    )];
    let agents = [agent("x", "idle", 0), agent("y", "running", 0)];
    let mut state = ViewState::default();
    state.collapsed_projects.insert("a".into());
    let rows = build_rows(&projects, &agents, &state);
    assert_eq!(ids(&rows), ["title", "p:a"]);
    let Row::Project(row) = &rows[1] else {
        panic!("project row")
    };
    assert!(row.collapsed);
    assert_eq!(row.bucket, StatusBucket::Running);
}

#[test]
fn pinned_workspaces_move_to_a_section_in_tree_order() {
    let projects = [
        project(
            "a",
            vec![workspace("a1", true, &[]), workspace("a2", false, &[])],
        ),
        project("b", vec![workspace("b1", true, &[])]),
    ];
    let rows = build_rows(&projects, &[], &ViewState::default());
    assert_eq!(
        ids(&rows),
        ["pinned", "w:a1", "w:b1", "title", "p:a", "w:a2", "p:b"],
        "pinned first in tree order; the projects keep only their unpinned workspaces"
    );
    let Row::Workspace(last_pinned) = &rows[2] else {
        panic!("workspace row")
    };
    assert!(
        last_pinned.block_end,
        "the pinned section keeps space under it"
    );
    assert_eq!(last_pinned.project_name, "project-b");

    let state = ViewState {
        pinned_collapsed: true,
        ..ViewState::default()
    };
    let rows = build_rows(&projects, &[], &state);
    assert_eq!(ids(&rows)[..2], ["pinned", "title"]);
}

#[test]
fn a_long_group_is_cut_with_show_more_until_expanded() {
    let many: Vec<WorkspaceNode> = (0..25)
        .map(|n| workspace(&format!("w{n}"), false, &[]))
        .collect();
    let projects = [project("a", many)];
    let rows = build_rows(&projects, &[], &ViewState::default());
    assert_eq!(rows.len(), 2 + INITIAL_VISIBLE + 1);
    assert!(matches!(
        rows.last(),
        Some(Row::ShowMore {
            expanded: false,
            block_end: true,
            ..
        })
    ));

    let mut state = ViewState::default();
    state.expanded_groups.insert(Group::Project("a".into()));
    let rows = build_rows(&projects, &[], &state);
    assert_eq!(rows.len(), 2 + 25 + 1);
    assert!(matches!(
        rows.last(),
        Some(Row::ShowMore { expanded: true, .. })
    ));
}

#[test]
fn an_agent_bucket_follows_the_electron_rule() {
    use StatusBucket::*;
    assert_eq!(bucket_for_agent("running", 0, false, None), Running);
    assert_eq!(bucket_for_agent("running", 2, false, None), NeedsInput);
    assert_eq!(bucket_for_agent("idle", 1, false, None), NeedsInput);
    assert_eq!(bucket_for_agent("error", 0, false, None), Failed);
    assert_eq!(bucket_for_agent("idle", 0, true, Some("error")), Failed);
    assert_eq!(
        bucket_for_agent("idle", 0, false, Some("permission")),
        NeedsInput
    );
    assert_eq!(
        bucket_for_agent("idle", 0, true, Some("finished")),
        Attention
    );
    assert_eq!(bucket_for_agent("idle", 0, false, None), Done);
    assert_eq!(bucket_for_agent("initializing", 0, false, None), Done);
    assert_eq!(bucket_for_agent("closed", 0, false, None), Done);
}

#[test]
fn a_workspace_takes_the_most_urgent_bucket_of_its_agents() {
    use StatusBucket::*;
    assert_eq!(most_urgent([]), Done);
    assert_eq!(most_urgent([Done, Attention]), Attention);
    assert_eq!(
        most_urgent([Attention, Running]),
        Running,
        "running outranks attention"
    );
    assert_eq!(most_urgent([Running, Failed, Done]), Failed);
    assert_eq!(most_urgent([Failed, NeedsInput, Running]), NeedsInput);

    let projects = [project(
        "a",
        vec![
            workspace("idle", false, &["i"]),
            workspace("busy", false, &["i", "r"]),
            workspace("asks", false, &["r", "p"]),
            workspace("empty", false, &[]),
            workspace("gone", false, &["missing"]),
        ],
    )];
    let agents = [
        agent("i", "idle", 0),
        agent("r", "running", 0),
        agent("p", "running", 1),
    ];
    let rows = build_rows(&projects, &agents, &ViewState::default());
    let buckets: Vec<(&str, StatusBucket)> = rows
        .iter()
        .filter_map(|row| match row {
            Row::Workspace(w) => Some((w.id.as_str(), w.bucket)),
            _ => None,
        })
        .collect();
    assert_eq!(
        buckets,
        [
            ("asks", NeedsInput),
            ("busy", Running),
            ("empty", Done),
            ("gone", Done),
            ("idle", Done)
        ]
    );
}

#[test]
fn a_status_change_keeps_the_row_shapes() {
    let projects = [project("a", vec![workspace("a1", false, &["x"])])];
    let idle = build_rows(&projects, &[agent("x", "idle", 0)], &ViewState::default());
    let busy = build_rows(
        &projects,
        &[agent("x", "running", 0)],
        &ViewState::default(),
    );
    assert_ne!(idle, busy);
    let shapes = |rows: &[Row]| rows.iter().map(Row::shape).collect::<Vec<_>>();
    assert_eq!(
        changed_range(&shapes(&idle), &shapes(&busy)),
        None,
        "a status change redraws the rows without a new measure of the list"
    );
}

#[test]
fn a_changed_range_covers_only_the_rows_that_moved() {
    let rows = |names: &[&str]| -> Vec<RowShape> {
        let ws: Vec<WorkspaceNode> = names.iter().map(|id| workspace(id, false, &[])).collect();
        build_rows(&[project("a", ws)], &[], &ViewState::default())
            .iter()
            .map(Row::shape)
            .collect()
    };
    // title, project, then the workspaces.
    assert_eq!(changed_range(&rows(&["1", "2"]), &rows(&["1", "2"])), None);
    assert_eq!(
        changed_range(&rows(&["1", "2", "3"]), &rows(&["1", "3"])),
        Some((3..4, 0)),
        "removing the second workspace"
    );
    assert_eq!(
        changed_range(&rows(&["1"]), &rows(&["1", "2"])),
        Some((2..3, 2)),
        "adding one at the end also changes the old last row, which loses its end mark"
    );
}

#[test]
fn archive_asks_only_when_the_checkout_has_changed_files() {
    assert_eq!(
        archive_prompt("main", &[]),
        None,
        "a clean workspace archives at once"
    );
    let prompt = archive_prompt("main", &[file(3, 1), file(1, 0)]).expect("risky");
    assert_eq!(prompt.title, "Archive \"main\"?");
    assert_eq!(
        prompt.message,
        "Uncommitted changes (4 added lines, 1 deleted line)"
    );
    let prompt = archive_prompt("main", &[file(0, 0)]).expect("risky");
    assert_eq!(prompt.message, "Uncommitted changes");
    let prompt = archive_prompt("main", &[file(1, 0)]).expect("risky");
    assert_eq!(prompt.message, "Uncommitted changes (1 added line)");
}

#[test]
fn the_width_stays_between_the_electron_limits() {
    assert_eq!(clamp_width(100.), 200.);
    assert_eq!(clamp_width(320.), 320.);
    assert_eq!(clamp_width(900.), 600.);
}

#[test]
fn project_icons_pick_a_color_and_an_initial_like_electron() {
    // hashIdentityKey("a") is 97, and 97 % 10 is 7: "red".
    assert_eq!(identity_color("a"), 0xb06260);
    // "ab" is 97 * 31 + 98 = 3105, and 3105 % 10 is 5: "indigo".
    assert_eq!(identity_color("ab"), 0x6a70b8);
    // The reference daemon's `.claude` projectKey hashes to "orange", as Electron draws it.
    assert_eq!(
        identity_color("remote:github.com/hieppp/claude-setting"),
        0xa4673a
    );
    assert_eq!(project_initial("bachuc"), "B");
    assert_eq!(project_initial("org/paseo"), "P");
    assert_eq!(project_initial("  "), "");
}

#[test]
fn a_workspace_label_names_its_status_unless_done() {
    let projects = [project("a", vec![workspace("a1", false, &["r"])])];
    let rows = build_rows(
        &projects,
        &[agent("r", "running", 0)],
        &ViewState::default(),
    );
    let Row::Workspace(row) = &rows[2] else {
        panic!("workspace row")
    };
    assert_eq!(workspace_label(row, false), "name-a1, Working");
    assert_eq!(workspace_label(row, true), "project-a, name-a1, Working");
}
