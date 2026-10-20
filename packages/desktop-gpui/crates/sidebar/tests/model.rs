use bachuc_client::{
    AgentSummary, AttentionReason, ChangedFile, CheckSummary, CheckSummaryState, ProjectNode,
    PullRequestState, WorkspaceLabel, WorkspaceNode, WorkspaceScript,
};
use bachuc_sidebar::model::build_rows as build_rows_with;
use bachuc_sidebar::model::*;
use bachuc_sidebar::prefs::{ChecksDisplay, RowItems, TitleSource, Trailing};
use std::collections::HashSet;

/// The rows with no client fields: no attention, no key, no diff stat, no pin time.
fn build_rows(projects: &[ProjectNode], agents: &[AgentSummary], state: &ViewState) -> Vec<Row> {
    build_rows_with(projects, agents, &Facts::default(), state)
}

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
        ..Default::default()
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
            Row::StatusHeader { bucket, .. } => format!("s:{}", bucket.key()),
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
fn diff_counts_print_compact() {
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
fn archive_from_the_diff_stat_asks_only_for_a_changed_line() {
    assert_eq!(archive_prompt_for_stat("main", None), None, "no stat");
    assert_eq!(
        archive_prompt_for_stat("main", Some((0, 0))),
        None,
        "a 0/0 stat archives at once"
    );
    let prompt = archive_prompt_for_stat("main", Some((4, 1))).expect("risky");
    assert_eq!(prompt.title, "Archive \"main\"?");
    assert_eq!(
        prompt.message,
        "Uncommitted changes (4 added lines, 1 deleted line)"
    );
    let prompt = archive_prompt_for_stat("main", Some((0, 2))).expect("risky");
    assert_eq!(prompt.message, "Uncommitted changes (2 deleted lines)");
}

/// Anti-goal tripwire: the confirm shows in exactly the 5 cases where either signal shows
/// changes, and never when both are clean or unknown.
#[test]
fn archive_from_the_checkout_asks_when_the_stat_or_the_dirty_flag_shows_changes() {
    let mut asked = 0;
    for stat in [None, Some((0, 0)), Some((1, 0))] {
        for is_dirty in [None, Some(false), Some(true)] {
            let prompt = archive_prompt_for_checkout("main", stat, is_dirty);
            let changed = stat == Some((1, 0)) || is_dirty == Some(true);
            assert_eq!(
                prompt.is_some(),
                changed,
                "stat {stat:?}, dirty {is_dirty:?}"
            );
            if let Some(prompt) = prompt {
                asked += 1;
                assert_eq!(prompt.title, "Archive \"main\"?");
                let message = if stat == Some((1, 0)) {
                    "Uncommitted changes (1 added line)"
                } else {
                    "Uncommitted changes"
                };
                assert_eq!(prompt.message, message, "stat {stat:?}, dirty {is_dirty:?}");
            }
        }
    }
    assert_eq!(asked, 5);
}

/// The bucket of a workspace with one agent that has this status and attention data.
fn attention_bucket(requires: bool, reason: Option<AttentionReason>, status: &str) -> StatusBucket {
    let projects = [project("a", vec![workspace("a1", false, &["x"])])];
    let mut facts = Facts::default();
    facts.attention.insert("x".into(), (requires, reason));
    let rows = build_rows_with(
        &projects,
        &[agent("x", status, 0)],
        &facts,
        &ViewState::default(),
    );
    let Some(Row::Workspace(row)) = rows.iter().find(|row| matches!(row, Row::Workspace(_))) else {
        panic!("workspace row")
    };
    row.bucket
}

#[test]
fn an_agent_that_needs_attention_shows_the_attention_status() {
    use StatusBucket::*;
    use bachuc_client::AttentionReason::*;
    assert_eq!(attention_bucket(true, Some(Finished), "idle"), Attention);
    assert_eq!(attention_bucket(true, None, "idle"), Attention);
    assert_eq!(attention_bucket(false, None, "idle"), Done);
    assert_eq!(attention_bucket(true, Some(Error), "idle"), Failed);
    assert_eq!(attention_bucket(true, Some(Permission), "idle"), NeedsInput);
    assert_eq!(
        attention_bucket(true, Some(Finished), "running"),
        Running,
        "running outranks attention"
    );

    // An agent the client has no attention data for stays done.
    let projects = [project("a", vec![workspace("a1", false, &["x"])])];
    let rows = build_rows(&projects, &[agent("x", "idle", 0)], &ViewState::default());
    assert!(matches!(&rows[2], Row::Workspace(w) if w.bucket == Done));

    // A collapsed project takes the urgency of the workspace behind it.
    let mut facts = Facts::default();
    facts
        .attention
        .insert("x".into(), (true, Some(AttentionReason::Finished)));
    let mut state = ViewState::default();
    state.collapsed_projects.insert("a".into());
    let rows = build_rows_with(&projects, &[agent("x", "idle", 0)], &facts, &state);
    assert!(matches!(&rows[1], Row::Project(p) if p.bucket == Attention));
}

#[test]
fn the_letter_color_hashes_the_project_key_and_falls_back_to_the_id() {
    let projects = [
        project("p1", vec![workspace("w1", true, &[])]),
        project("p2", vec![workspace("w2", true, &[])]),
    ];
    let key = "remote:github.com/hieppp/claude-setting";
    let mut facts = Facts::default();
    facts.project_keys.insert("p1".into(), key.into());
    let rows = build_rows_with(&projects, &[], &facts, &ViewState::default());
    let keys: Vec<(&str, &str)> = rows
        .iter()
        .filter_map(|row| match row {
            Row::Project(p) => Some((p.id.as_str(), p.color_key.as_str())),
            _ => None,
        })
        .collect();
    assert_eq!(keys, [("p1", key), ("p2", "p2")]);
    let pinned: Vec<(&str, &str)> = rows
        .iter()
        .filter_map(|row| match row {
            Row::Workspace(w) if w.group == Group::Pinned => {
                Some((w.id.as_str(), w.color_key.as_str()))
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        pinned,
        [("w1", key), ("w2", "p2")],
        "pinned rows lead with it"
    );
    assert_eq!(
        identity_color(key),
        0xa4673a,
        "the .claude project is orange"
    );
    assert_ne!(identity_color(key), identity_color("p1"));
}

#[test]
fn a_row_shows_the_diff_stat_the_workspace_carries() {
    let projects = [project(
        "a",
        vec![
            workspace("none", false, &[]),
            workspace("zero", false, &[]),
            workspace("some", false, &[]),
        ],
    )];
    let mut facts = Facts::default();
    facts.diff_stats.insert("zero".into(), (0, 0));
    facts.diff_stats.insert("some".into(), (12, 3));
    let rows = build_rows_with(&projects, &[], &facts, &ViewState::default());
    let diffs: Vec<(&str, Option<(u32, u32)>)> = rows
        .iter()
        .filter_map(|row| match row {
            Row::Workspace(w) => Some((w.id.as_str(), w.diff)),
            _ => None,
        })
        .collect();
    assert_eq!(
        diffs,
        [
            ("none", None),
            ("some", Some((12, 3))),
            ("zero", Some((0, 0)))
        ],
        "no stat has no count; 0/0 keeps its row, which shows +0 -0"
    );
}

#[test]
fn pinned_rows_go_by_pinned_at_newest_first() {
    let projects = [
        project(
            "a",
            vec![
                workspace("old", true, &[]),
                workspace("new", true, &[]),
                workspace("tie1", true, &[]),
                workspace("tie2", true, &[]),
                workspace("none", true, &[]),
            ],
        ),
        project("b", vec![workspace("b-new", true, &[])]),
    ];
    let mut facts = Facts::default();
    for (id, at) in [
        ("old", "2026-10-01T00:00:00.000Z"),
        ("new", "2026-10-09T00:00:00.000Z"),
        ("tie1", "2026-10-05T00:00:00.000Z"),
        ("tie2", "2026-10-05T00:00:00.000Z"),
        ("b-new", "2026-10-12T00:00:00.000Z"),
    ] {
        facts.pinned_at.insert(id.into(), at.into());
    }
    let rows = build_rows_with(&projects, &[], &facts, &ViewState::default());
    let pinned: Vec<String> = ids(&rows)
        .into_iter()
        .skip(1)
        .take_while(|id| id != "title")
        .collect();
    // Tree order is name order: new, none, old, tie1, tie2. Equal times keep it; a missing
    // time goes last.
    assert_eq!(
        pinned,
        ["w:b-new", "w:new", "w:tie1", "w:tie2", "w:old", "w:none"]
    );

    // Without any time the rows keep the tree order.
    let rows = build_rows(&projects, &[], &ViewState::default());
    let pinned: Vec<String> = ids(&rows)
        .into_iter()
        .skip(1)
        .take_while(|id| id != "title")
        .collect();
    assert_eq!(
        pinned,
        ["w:new", "w:none", "w:old", "w:tie1", "w:tie2", "w:b-new"]
    );
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

/// 50 projects with 10 workspaces each, one agent per workspace, a tenth of them running.
fn large_tree() -> (Vec<ProjectNode>, Vec<AgentSummary>) {
    let mut agents = Vec::new();
    let projects = (0..50)
        .map(|p| {
            let workspaces = (0..10)
                .map(|w| {
                    let id = format!("{p}-{w}");
                    let agent_id = format!("a{id}");
                    let status = if w == 0 { "running" } else { "idle" };
                    agents.push(agent(&agent_id, status, 0));
                    workspace(&id, w == 9, &[agent_id.as_str()])
                })
                .collect();
            project(&p.to_string(), workspaces)
        })
        .collect();
    (projects, agents)
}

/// The median of 21 runs, in milliseconds, so one scheduler stall does not decide the result.
fn median_ms(mut run: impl FnMut()) -> f64 {
    let mut times: Vec<f64> = (0..21)
        .map(|_| {
            let start = std::time::Instant::now();
            run();
            start.elapsed().as_secs_f64() * 1000.
        })
        .collect();
    times.sort_by(|a, b| a.total_cmp(b));
    times[10]
}

#[test]
fn building_500_workspaces_in_project_mode_stays_under_2_ms() {
    let (projects, agents) = large_tree();
    let facts = Facts::default();
    let state = ViewState::default();
    let ms = median_ms(|| {
        std::hint::black_box(build_rows_with(&projects, &agents, &facts, &state));
    });
    eprintln!("build_rows project mode, 50 projects x 10 workspaces: {ms:.3} ms");
    assert!(ms < 2., "{ms:.3} ms");
}

fn status_state() -> ViewState {
    ViewState {
        grouping: Grouping::Status,
        ..Default::default()
    }
}

fn labelled(id: &str, labels: &[&str]) -> WorkspaceNode {
    WorkspaceNode {
        labels: labels
            .iter()
            .map(|name| WorkspaceLabel {
                name: name.to_string(),
                color: "sky".into(),
            })
            .collect(),
        ..workspace(id, false, &[])
    }
}

#[test]
fn status_grouping_lists_workspaces_under_status_headings_in_electron_order() {
    let projects = [
        project(
            "a",
            vec![
                workspace("a-done", false, &[]),
                workspace("a-run", false, &["run"]),
                workspace("a-ask", false, &["ask"]),
            ],
        ),
        project(
            "b",
            vec![
                workspace("b-ready", false, &["ready"]),
                workspace("b-pin", true, &[]),
            ],
        ),
    ];
    let agents = [
        agent("run", "running", 0),
        agent("ask", "idle", 1),
        agent("ready", "idle", 0),
    ];
    let facts = Facts {
        attention: [("ready".to_string(), (true, Some(AttentionReason::Finished)))].into(),
        ..Default::default()
    };
    let rows = build_rows_with(&projects, &agents, &facts, &status_state());
    // Needs input, failed, ready to review, working, done; the pinned section keeps its place;
    // there are no project rows.
    assert_eq!(
        ids(&rows),
        [
            "pinned",
            "w:b-pin",
            "title",
            "s:needs_input",
            "w:a-ask",
            "s:attention",
            "w:b-ready",
            "s:running",
            "w:a-run",
            "s:done",
            "w:a-done",
        ]
    );
    let Row::Workspace(row) = &rows[4] else {
        panic!("workspace row")
    };
    assert_eq!(row.group, Group::Status(StatusBucket::NeedsInput));
    assert!(row.block_end);
}

fn row(id: &str, project: &str, entered: Option<i64>) -> WorkspaceRow {
    let node = WorkspaceNode {
        status_entered_at: entered,
        ..workspace(id, false, &[])
    };
    let projects = [ProjectNode {
        name: project.into(),
        ..self::project(project, vec![node])
    }];
    let rows = build_rows(&projects, &[], &ViewState::default());
    let Some(Row::Workspace(row)) = rows
        .into_iter()
        .find(|row| matches!(row, Row::Workspace(_)))
    else {
        panic!("workspace row")
    };
    row
}

#[test]
fn a_status_group_sorts_by_latest_entry_then_project_then_name() {
    let groups = status_groups(vec![
        row("old", "a", Some(100)),
        row("none", "a", None),
        row("new", "z", Some(900)),
        row("tie-b", "b", Some(500)),
        row("tie-a", "a", Some(500)),
    ]);
    assert_eq!(groups.len(), 1);
    let order: Vec<&str> = groups[0].1.iter().map(|row| row.id.as_str()).collect();
    // Newest first; equal times by project name; a row without a time last.
    assert_eq!(order, ["new", "tie-a", "tie-b", "old", "none"]);
}

#[test]
fn a_collapsed_status_group_keeps_its_heading_only() {
    let projects = [project("a", vec![workspace("a1", false, &[])])];
    let mut state = status_state();
    state.collapsed_statuses.insert(StatusBucket::Done);
    let rows = build_rows(&projects, &[], &state);
    assert_eq!(ids(&rows), ["title", "s:done"]);
    assert!(matches!(
        rows[1],
        Row::StatusHeader {
            collapsed: true,
            block_end: true,
            ..
        }
    ));
}

#[test]
fn the_project_filter_keeps_the_chosen_project_even_without_workspaces() {
    let projects = [
        project("a", vec![workspace("a1", false, &[])]),
        project("b", vec![]),
    ];
    let mut state = ViewState::default();
    state.project_filter.insert("b".into());
    assert_eq!(ids(&build_rows(&projects, &[], &state)), ["title", "p:b"]);
    // A filter whose projects all went filters nothing.
    state.project_filter = ["gone".to_string()].into();
    assert_eq!(
        ids(&build_rows(&projects, &[], &state)),
        ["title", "p:a", "w:a1", "p:b"]
    );
    // Status grouping takes the same filter.
    state.project_filter = ["a".to_string()].into();
    state.grouping = Grouping::Status;
    assert_eq!(
        ids(&build_rows(&projects, &[], &state)),
        ["title", "s:done", "w:a1"]
    );
}

#[test]
fn the_label_filter_keeps_workspaces_with_any_chosen_label_and_drops_empty_projects() {
    let projects = [
        project(
            "a",
            vec![labelled("a1", &["Bug", "UI"]), labelled("a2", &[])],
        ),
        project("b", vec![labelled("b1", &["  ui  "])]),
        project("c", vec![labelled("c1", &["Docs"])]),
    ];
    let mut state = ViewState::default();
    state.label_filter.insert("ui".into());
    // The key ignores case and whitespace; project c has nothing left and goes.
    assert_eq!(
        ids(&build_rows(&projects, &[], &state)),
        ["title", "p:a", "w:a1", "p:b", "w:b1"]
    );
    // The empty key selects a workspace with no label.
    state.label_filter = ["".to_string()].into();
    assert_eq!(
        ids(&build_rows(&projects, &[], &state)),
        ["title", "p:a", "w:a2"]
    );
    // Several keys include a workspace that carries any of them.
    state.label_filter = ["docs".to_string(), "".to_string()].into();
    assert_eq!(
        ids(&build_rows(&projects, &[], &state)),
        ["title", "p:a", "w:a2", "p:c", "w:c1"]
    );
}

#[test]
fn label_options_list_each_label_once_by_name() {
    let projects = [
        project("a", vec![labelled("a1", &["UI", "Bug"])]),
        project("b", vec![labelled("b1", &[" ui "])]),
    ];
    let names: Vec<String> = label_options(&projects)
        .into_iter()
        .map(|option| option.key)
        .collect();
    assert_eq!(names, ["bug", "ui"]);
}

#[test]
fn the_filter_rows_follow_the_electron_rules() {
    assert!(!project_filter_visible(1));
    assert!(project_filter_visible(2));
    assert!(!label_filter_visible(false, false));
    assert!(label_filter_visible(true, false));
    // An active filter keeps its row so the filter can be undone.
    assert!(label_filter_visible(false, true));
    let projects = [
        ("a".to_string(), "Alpha".to_string()),
        ("b".into(), "Beta".into()),
    ];
    assert_eq!(
        project_filter_value(&HashSet::new(), &projects),
        "All projects"
    );
    assert_eq!(
        project_filter_value(&["b".to_string()].into(), &projects),
        "Beta"
    );
    assert_eq!(
        project_filter_value(&["a".to_string(), "b".to_string()].into(), &projects),
        "2 projects"
    );
    assert_eq!(
        project_filter_value(&["gone".to_string()].into(), &projects),
        "All projects"
    );
}

#[test]
fn building_500_workspaces_in_status_mode_stays_under_2_ms() {
    let (projects, agents) = large_tree();
    let facts = Facts::default();
    let state = status_state();
    let ms = median_ms(|| {
        std::hint::black_box(build_rows_with(&projects, &agents, &facts, &state));
    });
    eprintln!("build_rows status mode, 50 projects x 10 workspaces: {ms:.3} ms");
    assert!(ms < 2., "{ms:.3} ms");
}

// ---- row meta items (TASK-262) ----

fn full_row() -> WorkspaceRow {
    let mut row = row("w", "project", Some(1_000));
    row.branch = Some("feat/x".into());
    row.diff = Some((3, 1));
    row.pull_request = Some(RowPullRequest {
        number: 7,
        url: "https://github.com/o/r/pull/7".into(),
        state: PullRequestState::Open,
        forge: "github".into(),
        checks: Some(CheckSummary {
            state: CheckSummaryState::Failed,
            completed: 2,
            total: 2,
        }),
    });
    row.labels = vec![WorkspaceLabel {
        name: "Bug".into(),
        color: "red".into(),
    }];
    row.service = Some(ServiceSummary {
        name: "web".into(),
        health: Some("healthy".into()),
    });
    row
}

fn all_on() -> RowItems {
    RowItems {
        branch: true,
        project: true,
        pull_request: true,
        services: true,
        labels: true,
    }
}

fn kinds(items: &[MetaItem]) -> Vec<&'static str> {
    items
        .iter()
        .map(|item| match item {
            MetaItem::Branch(_) => "branch",
            MetaItem::Project(_) => "project",
            MetaItem::PullRequest(_) => "pr",
            MetaItem::Checks { .. } => "checks",
            MetaItem::Services(_) => "services",
            MetaItem::Labels(_) => "labels",
        })
        .collect()
}

#[test]
fn meta_items_follow_the_electron_order() {
    let row = full_row();
    let items = meta_items(&row, &all_on(), ChecksDisplay::IconAndText, true);
    assert_eq!(
        kinds(&items),
        ["branch", "project", "pr", "checks", "services", "labels"]
    );
    assert!(matches!(items[3], MetaItem::Checks { label: true, .. }));
    let icon_only = meta_items(&row, &all_on(), ChecksDisplay::IconOnly, true);
    assert!(matches!(
        icon_only[3],
        MetaItem::Checks { label: false, .. }
    ));
}

#[test]
fn toggles_off_give_no_items_and_checks_follow_their_own_choice() {
    let row = full_row();
    assert!(meta_items(&row, &RowItems::none(), ChecksDisplay::Hidden, true).is_empty());
    // Checks show without the pull request beside them.
    let items = meta_items(&row, &RowItems::none(), ChecksDisplay::IconAndText, true);
    assert_eq!(kinds(&items), ["checks"]);
    // Each toggle answers for itself.
    let mut only_services = RowItems::none();
    only_services.services = true;
    assert_eq!(
        kinds(&meta_items(
            &row,
            &only_services,
            ChecksDisplay::Hidden,
            true
        )),
        ["services"]
    );
}

#[test]
fn missing_data_hides_an_item() {
    let mut row = full_row();
    row.branch = None;
    row.pull_request = None;
    row.labels.clear();
    row.service = None;
    let items = meta_items(&row, &all_on(), ChecksDisplay::IconAndText, true);
    assert_eq!(kinds(&items), ["project"]);
    row.branch = Some(String::new());
    row.project_name = String::new();
    assert!(meta_items(&row, &all_on(), ChecksDisplay::IconAndText, true).is_empty());
    // A pull request without checks draws no Checks item.
    let mut no_checks = full_row();
    no_checks.pull_request.as_mut().unwrap().checks = None;
    let items = meta_items(&no_checks, &all_on(), ChecksDisplay::IconAndText, true);
    assert!(!kinds(&items).contains(&"checks"));
}

#[test]
fn the_project_name_shows_only_on_a_row_that_leads_with_the_project() {
    let row = full_row();
    let under_heading = meta_items(&row, &all_on(), ChecksDisplay::Hidden, false);
    assert!(!kinds(&under_heading).contains(&"project"));
}

#[test]
fn a_row_with_a_meta_line_has_another_shape() {
    let mut bare = row("w", "project", None);
    bare.branch = None;
    let plain = Row::Workspace(bare);
    let rich = Row::Workspace(full_row());
    let items = all_on();
    assert_ne!(
        plain.shape_with(&items, ChecksDisplay::IconAndText),
        rich.shape_with(&items, ChecksDisplay::IconAndText)
    );
    // Toggling every item off gives the same shape as a row without data.
    assert_eq!(
        rich.shape_with(&RowItems::none(), ChecksDisplay::Hidden)
            .clone(),
        plain.shape_with(&RowItems::none(), ChecksDisplay::Hidden)
    );
    assert!(has_meta_line(&full_row(), &items, ChecksDisplay::Hidden));
    assert!(!has_meta_line(
        &WorkspaceRow {
            branch: None,
            ..row("w", "p", None)
        },
        &items,
        ChecksDisplay::Hidden
    ));
}

#[test]
fn the_trailing_slot_holds_the_diff_or_the_last_activity_never_both() {
    let row = full_row();
    assert_eq!(
        trailing_slot(&row, Some(Trailing::DiffStats)),
        Some(TrailingSlot::Diff {
            added: 3,
            removed: 1
        })
    );
    assert_eq!(
        trailing_slot(&row, Some(Trailing::LastActivity)),
        Some(TrailingSlot::LastActivity(1_000))
    );
    assert_eq!(trailing_slot(&row, None), None);
    let mut bare = full_row();
    bare.diff = None;
    bare.status_entered_at = None;
    assert_eq!(trailing_slot(&bare, Some(Trailing::DiffStats)), None);
    assert_eq!(trailing_slot(&bare, Some(Trailing::LastActivity)), None);
}

#[test]
fn the_branch_title_falls_back_to_the_workspace_name() {
    let mut row = full_row();
    assert_eq!(row_title(&row, TitleSource::Title), "name-w");
    assert_eq!(row_title(&row, TitleSource::Branch), "feat/x");
    row.branch = None;
    assert_eq!(row_title(&row, TitleSource::Branch), "name-w");
    row.branch = Some(String::new());
    assert_eq!(row_title(&row, TitleSource::Branch), "name-w");
    assert_eq!(
        workspace_label_for(&full_row(), false, TitleSource::Branch),
        "feat/x"
    );
}

fn script(name: &str, kind: &str, lifecycle: &str, health: Option<&str>) -> WorkspaceScript {
    WorkspaceScript {
        name: name.into(),
        kind: kind.into(),
        lifecycle: lifecycle.into(),
        health: health.map(String::from),
        terminal_id: None,
        exit_code: None,
        proxy_url: None,
        local_proxy_url: None,
        public_proxy_url: None,
    }
}

#[test]
fn the_service_summary_counts_running_services_and_prefers_an_unhealthy_one() {
    assert_eq!(select_service_summary(&[]), None);
    // A plain script and a stopped service say nothing about the workspace being up.
    assert_eq!(
        select_service_summary(&[
            script("test", "script", "running", None),
            script("old", "service", "stopped", Some("healthy")),
        ]),
        None
    );
    let summary = select_service_summary(&[
        script("api", "service", "running", Some("healthy")),
        script("web", "service", "running", Some("unhealthy")),
    ])
    .unwrap();
    assert_eq!(summary.name, "web");
    assert!(summary.unhealthy());
    let first = select_service_summary(&[
        script("api", "service", "running", Some("healthy")),
        script("web", "service", "running", None),
    ])
    .unwrap();
    assert_eq!(first.name, "api");
}

#[test]
fn the_compact_time_matches_describe_compact_time_ago() {
    let now = 1_760_000_000_000; // 2025-10-09T08:53:20Z
    assert_eq!(compact_time_ago(now - 30_000, now), "now");
    assert_eq!(compact_time_ago(now - 5 * 60_000, now), "5m");
    assert_eq!(compact_time_ago(now - 3 * 3_600_000, now), "3h");
    assert_eq!(compact_time_ago(now - 2 * 86_400_000, now), "2d");
    // A week or more shows the date: 2025-09-20 is 19 days earlier.
    assert_eq!(compact_time_ago(now - 19 * 86_400_000, now), "Sep 20");
    assert_eq!(
        compact_time_ago(1_709_251_200_000 + 3_600_000, now),
        "Mar 1"
    );
}

#[test]
fn a_label_color_name_picks_its_identity_color() {
    assert_eq!(label_color("violet"), Some(IDENTITY_COLORS[0]));
    assert_eq!(label_color("blue"), Some(IDENTITY_COLORS[9]));
    assert_eq!(label_color("plaid"), None);
}

#[test]
fn rows_copy_the_meta_data_when_they_build() {
    let node = WorkspaceNode {
        pull_request: Some(bachuc_client::WorkspacePullRequest {
            number: 9,
            title: "t".into(),
            url: "u".into(),
            state: PullRequestState::Merged,
            is_merged: true,
            is_draft: false,
            forge: "github".into(),
            checks: None,
        }),
        labels: vec![WorkspaceLabel {
            name: "UI".into(),
            color: "sky".into(),
        }],
        ..workspace("w1", false, &[])
    };
    let facts = Facts {
        services: [(
            "w1".to_string(),
            ServiceSummary {
                name: "web".into(),
                health: None,
            },
        )]
        .into(),
        ..Default::default()
    };
    let rows = build_rows_with(
        &[project("a", vec![node])],
        &[],
        &facts,
        &ViewState::default(),
    );
    let Row::Workspace(row) = &rows[2] else {
        panic!("workspace row")
    };
    assert_eq!(row.pull_request.as_ref().map(|pr| pr.number), Some(9));
    assert_eq!(row.labels.len(), 1);
    assert_eq!(row.service.as_ref().map(|s| s.name.as_str()), Some("web"));
}

#[test]
fn building_500_workspaces_with_meta_data_stays_under_2_ms() {
    let (mut projects, agents) = large_tree();
    for node in projects.iter_mut().flat_map(|p| &mut p.workspaces) {
        node.labels = vec![WorkspaceLabel {
            name: "Bug".into(),
            color: "red".into(),
        }];
        node.pull_request = Some(bachuc_client::WorkspacePullRequest {
            number: 1,
            title: "title".into(),
            url: "https://github.com/o/r/pull/1".into(),
            state: PullRequestState::Open,
            is_merged: false,
            is_draft: false,
            forge: "github".into(),
            checks: None,
        });
    }
    let facts = Facts::default();
    let state = ViewState::default();
    let ms = median_ms(|| {
        std::hint::black_box(build_rows_with(&projects, &agents, &facts, &state));
    });
    eprintln!("build_rows project mode with PR and labels: {ms:.3} ms");
    assert!(ms < 2., "{ms:.3} ms");
}

/// Facts with a server id and a view key `key-<id>` for each project.
fn keyed_facts(projects: &[ProjectNode]) -> Facts {
    let mut facts = Facts {
        server_id: Some("srv".into()),
        ..Facts::default()
    };
    for project in projects {
        facts
            .project_keys
            .insert(project.id.clone(), format!("key-{}", project.id));
    }
    facts
}

fn strings(keys: &[&str]) -> Vec<String> {
    keys.iter().map(|key| key.to_string()).collect()
}

#[test]
fn a_saved_order_puts_projects_and_threads_in_its_order() {
    let projects = [
        project(
            "a",
            vec![workspace("a1", false, &[]), workspace("a2", false, &[])],
        ),
        project("b", vec![workspace("b1", false, &[])]),
    ];
    let facts = keyed_facts(&projects);
    let mut state = ViewState::default();
    state.order.project_order = strings(&["key-b", "key-a"]);
    state
        .order
        .workspace_order_by_project
        .insert("key-a".into(), strings(&["srv:a2", "srv:a1"]));
    let rows = build_rows_with(&projects, &[], &facts, &state);
    assert_eq!(ids(&rows), ["title", "p:b", "w:b1", "p:a", "w:a2", "w:a1"]);
}

#[test]
fn an_unsaved_thread_keeps_its_name_slot() {
    let projects = [project(
        "a",
        vec![
            workspace("a1", false, &[]),
            workspace("a2", false, &[]),
            workspace("a3", false, &[]),
        ],
    )];
    let facts = keyed_facts(&projects);
    let mut state = ViewState::default();
    // a2 is not saved, so it keeps slot 2; a3 and a1 fill slots 1 and 3.
    state
        .order
        .workspace_order_by_project
        .insert("key-a".into(), strings(&["srv:a3", "srv:a1"]));
    let rows = build_rows_with(&projects, &[], &facts, &state);
    assert_eq!(ids(&rows), ["title", "p:a", "w:a3", "w:a2", "w:a1"]);
}

#[test]
fn seeding_puts_new_threads_first_and_new_projects_last() {
    let projects = [
        project(
            "a",
            vec![workspace("a1", false, &[]), workspace("a2", false, &[])],
        ),
        project("b", vec![workspace("b1", false, &[])]),
    ];
    let facts = keyed_facts(&projects);
    let mut order = SidebarOrder {
        project_order: strings(&["key-b"]),
        ..SidebarOrder::default()
    };
    order
        .workspace_order_by_project
        .insert("key-a".into(), strings(&["srv:a1"]));
    assert!(seed_order(&mut order, &projects, &facts));
    assert_eq!(order.project_order, strings(&["key-b", "key-a"]));
    assert_eq!(
        order.workspace_order_by_project["key-a"],
        strings(&["srv:a2", "srv:a1"])
    );
    assert_eq!(
        order.workspace_order_by_project["key-b"],
        strings(&["srv:b1"])
    );
    assert!(
        !seed_order(&mut order, &projects, &facts),
        "a second seed with no new key changes nothing"
    );
}

#[test]
fn seeding_keeps_keys_that_are_gone() {
    let projects = [project("a", vec![workspace("a1", false, &[])])];
    let facts = keyed_facts(&projects);
    let mut order = SidebarOrder {
        project_order: strings(&["key-gone", "key-a"]),
        ..SidebarOrder::default()
    };
    order
        .workspace_order_by_project
        .insert("key-a".into(), strings(&["srv:gone", "srv:a1"]));
    assert!(!seed_order(&mut order, &projects, &facts));
    assert_eq!(order.project_order, strings(&["key-gone", "key-a"]));
}

#[test]
fn without_a_server_id_threads_keep_the_name_order_and_are_not_seeded() {
    let projects = [project(
        "a",
        vec![workspace("a1", false, &[]), workspace("a2", false, &[])],
    )];
    let mut facts = keyed_facts(&projects);
    facts.server_id = None;
    let mut state = ViewState::default();
    state
        .order
        .workspace_order_by_project
        .insert("key-a".into(), strings(&["srv:a2", "srv:a1"]));
    let rows = build_rows_with(&projects, &[], &facts, &state);
    assert_eq!(ids(&rows), ["title", "p:a", "w:a1", "w:a2"]);
    let mut order = SidebarOrder::default();
    seed_order(&mut order, &projects, &facts);
    assert!(order.workspace_order_by_project.is_empty());
    assert_eq!(
        order.project_order,
        strings(&["key-a"]),
        "projects still seed"
    );
}

#[test]
fn a_project_without_a_view_key_keeps_its_name_slot_and_is_not_seeded() {
    let projects = [
        project("a", vec![workspace("a1", false, &[])]),
        project("b", vec![workspace("b1", false, &[])]),
        project("c", vec![workspace("c1", false, &[])]),
    ];
    let mut facts = keyed_facts(&projects);
    facts.project_keys.remove("b");
    let mut state = ViewState::default();
    state.order.project_order = strings(&["key-c", "key-a"]);
    let rows = build_rows_with(&projects, &[], &facts, &state);
    assert_eq!(
        ids(&rows),
        ["title", "p:c", "w:c1", "p:b", "w:b1", "p:a", "w:a1"]
    );
    let mut order = SidebarOrder::default();
    seed_order(&mut order, &projects, &facts);
    assert_eq!(order.project_order, strings(&["key-a", "key-c"]));
}

#[test]
fn status_grouping_ignores_the_saved_order() {
    let projects = [project(
        "a",
        vec![workspace("a1", false, &[]), workspace("a2", false, &[])],
    )];
    let facts = keyed_facts(&projects);
    let plain = ViewState {
        grouping: Grouping::Status,
        ..ViewState::default()
    };
    let mut ordered = plain.clone();
    ordered
        .order
        .workspace_order_by_project
        .insert("key-a".into(), strings(&["srv:a2", "srv:a1"]));
    assert_eq!(
        ids(&build_rows_with(&projects, &[], &facts, &ordered)),
        ids(&build_rows_with(&projects, &[], &facts, &plain))
    );
}

#[test]
fn a_saved_pinned_order_applies_after_the_pinned_at_sort() {
    let projects = [project(
        "a",
        vec![workspace("p1", true, &[]), workspace("p2", true, &[])],
    )];
    let mut facts = keyed_facts(&projects);
    facts
        .pinned_at
        .insert("p1".into(), "2026-10-09T00:00:00.000Z".into());
    facts
        .pinned_at
        .insert("p2".into(), "2026-10-01T00:00:00.000Z".into());
    let mut state = ViewState::default();
    state.order.pinned_workspace_order = strings(&["srv:p2", "srv:p1"]);
    let rows = build_rows_with(&projects, &[], &facts, &state);
    assert_eq!(ids(&rows)[..3], ["pinned", "w:p2", "w:p1"]);
}

/// Anti-goal probe: a saved order that lists every key costs at most 1.5 times the same build
/// with no saved order. Run with `--nocapture` to read both times.
#[test]
fn a_full_saved_order_stays_within_one_and_a_half_times_the_unordered_build() {
    let (projects, agents) = large_tree();
    let facts = keyed_facts(&projects);
    let plain = ViewState::default();
    let mut ordered = ViewState::default();
    // Reverse of the name order, so the apply step does real work.
    ordered.order.project_order = projects
        .iter()
        .rev()
        .map(|project| format!("key-{}", project.id))
        .collect();
    for project in &projects {
        ordered.order.workspace_order_by_project.insert(
            format!("key-{}", project.id),
            project
                .workspaces
                .iter()
                .rev()
                .map(|node| format!("srv:{}", node.id))
                .collect(),
        );
    }
    let before = median_ms(|| {
        std::hint::black_box(build_rows_with(&projects, &agents, &facts, &plain));
    });
    let after = median_ms(|| {
        std::hint::black_box(build_rows_with(&projects, &agents, &facts, &ordered));
    });
    eprintln!("build_rows no order: {before:.3} ms, full saved order: {after:.3} ms");
    assert!(
        after <= before * 1.5,
        "{after:.3} ms against {before:.3} ms"
    );
}

fn keys(items: &[&str]) -> Vec<String> {
    items.iter().map(|item| item.to_string()).collect()
}

#[test]
fn a_drop_moves_the_dragged_key_to_the_target_index() {
    let visible = keys(&["a", "b", "c"]);
    assert_eq!(reorder_visible(&visible, "a", "c"), keys(&["b", "c", "a"]));
    assert_eq!(reorder_visible(&visible, "c", "a"), keys(&["c", "a", "b"]));
    assert_eq!(reorder_visible(&visible, "b", "b"), visible);
    assert_eq!(
        reorder_visible(&visible, "x", "a"),
        visible,
        "an unknown dragged key changes nothing"
    );
}

#[test]
fn a_merge_puts_the_visible_order_first_and_keeps_hidden_keys() {
    let current = keys(&["hidden", "a", "b", "c"]);
    assert_eq!(
        merge_with_remainder(&current, &keys(&["c", "a", "b"])),
        keys(&["c", "a", "b", "hidden"])
    );
}

#[test]
fn only_a_changed_visible_order_counts_as_a_change() {
    let current = keys(&["hidden", "a", "b"]);
    assert!(!has_visible_order_changed(&current, &keys(&["a", "b"])));
    assert!(has_visible_order_changed(&current, &keys(&["b", "a"])));
    assert!(
        has_visible_order_changed(&current, &keys(&["a", "b", "new"])),
        "a visible key the saved order lacks counts as a change"
    );
}
