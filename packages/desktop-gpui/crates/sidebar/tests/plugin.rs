use bachuc_client::{AddedProject, ProjectNode, WorkspaceNode};
use bachuc_icons::IconName;
use bachuc_plugin_host::{
    Contributed, ProjectMenuItem, SidebarAction, SidebarItem, SwipeDirection,
};
use bachuc_sidebar::model::{Facts, Row, ViewState, build_rows};
use bachuc_sidebar::plugin::*;
use gpui_kit::{Point, ScrollDelta, SharedString, point, px};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Instant;

fn workspace(project: usize, n: usize, pinned: bool) -> WorkspaceNode {
    WorkspaceNode {
        id: format!("w{project}-{n}"),
        name: format!("ws {n}"),
        cwd: format!("/work/p{project}/{n}"),
        branch: None,
        pinned,
        agent_ids: Vec::new(),
    }
}

fn projects(count: usize) -> Vec<ProjectNode> {
    (0..count)
        .map(|n| ProjectNode {
            id: format!("p{n}"),
            name: format!("project {n}"),
            root: format!("/work/p{n}"),
            workspaces: (0..3)
                .map(|w| workspace(n, w, w == 0 && n % 50 == 0))
                .collect(),
        })
        .collect()
}

fn keys(projects: &[ProjectNode]) -> HashMap<String, String> {
    projects
        .iter()
        .map(|project| (project.id.clone(), format!("key-{}", project.id)))
        .collect()
}

fn row_ids(rows: &[Row]) -> Vec<String> {
    rows.iter()
        .map(|row| match row {
            Row::PinnedHeader { .. } => "pinned".to_string(),
            Row::SectionTitle => "title".to_string(),
            Row::Project(p) => format!("p:{}", p.id),
            Row::Workspace(w) => format!("w:{}", w.id),
            Row::ShowMore { .. } => "more".to_string(),
            Row::Empty => "empty".to_string(),
        })
        .collect()
}

fn item(id: &'static str, surface: &'static str, action: Option<SidebarAction>) -> SidebarItem {
    SidebarItem {
        id,
        title: id.into(),
        icon: IconName::Search,
        surface,
        action,
    }
}

fn action(requires_workspace: bool) -> SidebarAction {
    SidebarAction {
        on_press: Rc::new(|_, _, _| {}),
        requires_workspace,
        new_workspace_panel: None,
    }
}

fn contributed(plugin_id: &'static str, value: SidebarItem) -> Contributed<SidebarItem> {
    Contributed { plugin_id, value }
}

// ---- nav rows ----

#[test]
fn nav_rows_keep_index_order_across_plugins() {
    let items = vec![
        contributed("a", item("one", "s1", None)),
        contributed("a", item("two", "s2", None)),
        contributed("b", item("three", "s3", None)),
    ];
    let rows = nav_rows(&items, None, None);
    let ids: Vec<_> = rows.iter().map(|row| (row.plugin_id, row.id)).collect();
    assert_eq!(ids, [("a", "one"), ("a", "two"), ("b", "three")]);
    assert_eq!(rows[0].label, SharedString::from("one"));
}

#[test]
fn a_requires_workspace_item_is_disabled_without_a_workspace() {
    let items = vec![
        contributed("a", item("needs", "s", Some(action(true)))),
        contributed("a", item("free", "s", Some(action(false)))),
        contributed("a", item("page", "s", None)),
    ];
    let without = nav_rows(&items, None, None);
    assert_eq!(
        without.iter().map(|row| row.disabled).collect::<Vec<_>>(),
        [true, false, false]
    );
    assert_eq!(nav_press(&without[0]), NavPress::Ignored);
    let with = nav_rows(&items, Some("w1"), None);
    assert!(with.iter().all(|row| !row.disabled));
    assert_eq!(nav_press(&with[0]), NavPress::Action);
}

#[test]
fn a_press_without_an_action_opens_the_surface() {
    let items = vec![contributed("a", item("page", "board", None))];
    let rows = nav_rows(&items, None, None);
    assert_eq!(nav_press(&rows[0]), NavPress::OpenSurface);
    assert_eq!(rows[0].surface, "board");
}

#[test]
fn the_row_of_the_open_surface_is_active() {
    let items = vec![
        contributed("a", item("one", "board", None)),
        contributed("b", item("two", "board", None)),
        contributed("a", item("three", "other", None)),
    ];
    let rows = nav_rows(&items, None, Some(("a", "board")));
    assert_eq!(
        rows.iter().map(|row| row.active).collect::<Vec<_>>(),
        [true, false, false],
        "the plugin id counts as well as the surface id"
    );
    let none = nav_rows(&items, None, None);
    assert!(none.iter().all(|row| !row.active));
}

// ---- filters ----

#[test]
fn a_hidden_project_drops_its_workspaces_pinned_ones_too() {
    let all = projects(3);
    let keys = keys(&all);
    // p0 has a pinned workspace.
    let visible = visible_projects(&all, &keys, |project| project.name != "project 0");
    let rows = filtered_rows(
        &all,
        &visible,
        &[],
        &Facts::default(),
        &ViewState::default(),
    );
    let ids = row_ids(&rows);
    assert!(!ids.iter().any(|id| id == "p:p0" || id.starts_with("w:w0-")));
    assert!(ids.contains(&"p:p1".to_string()) && ids.contains(&"w:w2-1".to_string()));
    assert!(
        !ids.contains(&"pinned".to_string()),
        "p0's pin went with it"
    );
}

#[test]
fn two_filters_hide_a_project_when_either_says_so() {
    let all = projects(3);
    let keys = keys(&all);
    let first = |project: &bachuc_plugin_host::SidebarProject| project.name != "project 0";
    let second = |project: &bachuc_plugin_host::SidebarProject| project.view_key != "key-p2";
    let visible = visible_projects(&all, &keys, |project| first(project) && second(project));
    let names: Vec<_> = visible.iter().map(|p| p.id.as_str()).collect();
    assert_eq!(names, ["p1"]);
}

#[test]
fn the_filter_sees_the_view_key_and_falls_back_to_the_project_id() {
    let all = projects(2);
    let mut keys = HashMap::new();
    keys.insert("p0".to_string(), "daemon-key".to_string());
    let seen = RefCell::new(Vec::new());
    visible_projects(&all, &keys, |project| {
        seen.borrow_mut().push((
            project.view_key.clone(),
            project.project_ids.clone(),
            project.name.clone(),
        ));
        true
    });
    assert_eq!(
        seen.into_inner(),
        [
            (
                "daemon-key".to_string(),
                vec!["p0".to_string()],
                "project 0".to_string()
            ),
            (
                "p1".to_string(),
                vec!["p1".to_string()],
                "project 1".to_string()
            ),
        ]
    );
}

#[test]
fn filters_that_hide_every_project_leave_no_onboarding_row() {
    let all = projects(2);
    let visible = visible_projects(&all, &keys(&all), |_| false);
    let rows = filtered_rows(
        &all,
        &visible,
        &[],
        &Facts::default(),
        &ViewState::default(),
    );
    assert_eq!(row_ids(&rows), ["title"]);
    // No projects at all still shows onboarding.
    let empty = filtered_rows(&[], &[], &[], &Facts::default(), &ViewState::default());
    assert_eq!(row_ids(&empty), ["empty"]);
}

#[test]
fn hiding_the_selected_workspace_leaves_the_selection_to_the_app() {
    // The sidebar keeps `selected` apart from the rows: rows come from a pure function of the
    // projects, and nothing here writes the selection or emits an event.
    let all = projects(2);
    let selected = Some("w1-1".to_string());
    let visible = visible_projects(&all, &keys(&all), |project| project.name != "project 1");
    let rows = filtered_rows(
        &all,
        &visible,
        &[],
        &Facts::default(),
        &ViewState::default(),
    );
    assert!(!row_ids(&rows).contains(&"w:w1-1".to_string()));
    assert_eq!(selected.as_deref(), Some("w1-1"));
    // The filter shows the project again: the row returns and the selection still matches it.
    let shown = visible_projects(&all, &keys(&all), |_| true);
    let rows = filtered_rows(&all, &shown, &[], &Facts::default(), &ViewState::default());
    assert!(row_ids(&rows).contains(&format!("w:{}", selected.unwrap())));
}

#[test]
fn the_first_non_empty_trimmed_title_wins() {
    let titles =
        |list: &[Option<&str>]| pick_title(list.iter().map(|title| title.map(SharedString::from)));
    assert_eq!(
        titles(&[Some(""), Some("  Space 2  "), Some("Other")]),
        Some(SharedString::from("Space 2"))
    );
    assert_eq!(titles(&[None, Some("   ")]), None);
    assert_eq!(titles(&[]), None);
}

// ---- swipe ----

fn push(gesture: &mut WheelGesture, dx: f32, dy: f32, time: f64) -> Option<SwipeDirection> {
    gesture.push(dx, dy, time)
}

#[test]
fn three_pushes_inside_the_quiet_gap_make_one_next() {
    let mut gesture = WheelGesture::default();
    assert_eq!(push(&mut gesture, 10., 0., 0.), None);
    assert_eq!(push(&mut gesture, 10., 2., 16.), None);
    assert_eq!(push(&mut gesture, 10., 1., 32.), Some(SwipeDirection::Next));
    assert_eq!(push(&mut gesture, 10., 0., 48.), None, "one per gesture");
}

#[test]
fn a_quiet_gap_restarts_the_count() {
    let mut gesture = WheelGesture::default();
    assert_eq!(push(&mut gesture, 10., 0., 0.), None);
    assert_eq!(push(&mut gesture, 10., 0., 130.), None);
    assert_eq!(
        push(&mut gesture, 10., 0., 260.),
        None,
        "10 + 10 + 10 never sums"
    );
    let mut steady = WheelGesture::default();
    steady.push(10., 0., 0.);
    steady.push(10., 0., 100.);
    assert_eq!(steady.push(10., 0., 200.), Some(SwipeDirection::Next));
}

#[test]
fn a_negative_push_makes_previous() {
    let mut gesture = WheelGesture::default();
    assert_eq!(
        push(&mut gesture, -30., 0., 0.),
        Some(SwipeDirection::Previous)
    );
}

#[test]
fn a_diagonal_push_makes_none() {
    let mut gesture = WheelGesture::default();
    assert_eq!(push(&mut gesture, 20., 20., 0.), None);
    assert_eq!(push(&mut gesture, 20., 20., 16.), None);
}

#[test]
fn a_reversal_starts_a_new_gesture() {
    let mut gesture = WheelGesture::default();
    assert_eq!(push(&mut gesture, 30., 0., 0.), Some(SwipeDirection::Next));
    assert_eq!(
        push(&mut gesture, -30., 0., 16.),
        Some(SwipeDirection::Previous)
    );
}

#[test]
fn a_decaying_tail_stays_quiet_and_a_fresh_stroke_fires_again() {
    let mut gesture = WheelGesture::default();
    assert_eq!(push(&mut gesture, 30., 0., 0.), Some(SwipeDirection::Next));
    // Momentum tail, every push smaller than the one before.
    for (step, dx) in [8., 6., 5., 4., 3., 2., 1.].into_iter().enumerate() {
        let time = 40. + step as f64 * 30.;
        assert_eq!(push(&mut gesture, dx, 0., time), None, "tail push {step}");
    }
    // A new stroke after the tail: no quiet gap, but it speeds up well past the tail.
    let mut fired = None;
    for (step, dx) in [3., 6., 12., 24., 30.].into_iter().enumerate() {
        fired = fired.or(push(&mut gesture, dx, 0., 300. + step as f64 * 20.));
    }
    assert_eq!(fired, Some(SwipeDirection::Next));
}

#[test]
fn scroll_deltas_use_the_web_sign_and_count_lines_as_16_pixels() {
    let pixels = ScrollDelta::Pixels(point(px(20.), px(-4.)));
    assert_eq!(wheel_deltas(pixels), (-20., 4.));
    let lines = ScrollDelta::Lines(Point { x: -2., y: 1. });
    assert_eq!(wheel_deltas(lines), (32., -16.));
    assert!(is_horizontal(30., 4.));
    assert!(!is_horizontal(4., 30.));
}

// ---- slide ----

#[test]
fn the_slide_runs_out_then_in_and_ends_at_170_ms() {
    let start = slide_frame(SwipeDirection::Next, 0.).unwrap();
    assert!(start.offset.abs() < 1e-4 && (start.opacity - 1.).abs() < 1e-4);
    assert!(!start.applied);

    // The out phase ends at 65 ms with the list off to the left and invisible; the rows change
    // there and the new list starts off to the right.
    let apply = slide_frame(SwipeDirection::Next, 65.).unwrap();
    assert!(apply.applied);
    assert!((apply.offset - 1.).abs() < 1e-4, "{apply:?}");
    assert!(apply.opacity.abs() < 1e-4, "{apply:?}");

    let before_apply = slide_frame(SwipeDirection::Next, 64.9).unwrap();
    assert!(!before_apply.applied);
    assert!(before_apply.offset < -0.99, "{before_apply:?}");

    let middle = slide_frame(SwipeDirection::Next, 65. + 52.5).unwrap();
    assert!(middle.offset > 0. && middle.offset < 1.);
    assert!(middle.opacity > 0. && middle.opacity < 1.);

    assert_eq!(slide_frame(SwipeDirection::Next, 170.), None);
    assert_eq!(slide_frame(SwipeDirection::Next, 400.), None);
    let end = slide_frame(SwipeDirection::Next, 169.9).unwrap();
    assert!(end.offset.abs() < 0.01 && end.opacity > 0.99, "{end:?}");
}

#[test]
fn the_slide_goes_the_other_way_for_previous() {
    let next = slide_frame(SwipeDirection::Next, 30.).unwrap();
    let previous = slide_frame(SwipeDirection::Previous, 30.).unwrap();
    assert!(next.offset < 0. && previous.offset > 0.);
    assert!((next.offset + previous.offset).abs() < 1e-6);
    assert_eq!(next.opacity, previous.opacity);
}

// ---- project menu ----

fn menu_item(title: &'static str, checked: bool) -> ProjectMenuItem {
    ProjectMenuItem {
        id: title.into(),
        title: title.into(),
        disabled: false,
        checked,
        items: Vec::new(),
        on_select: None,
    }
}

#[test]
fn a_submenu_ends_with_its_checked_child() {
    let children = [menu_item("a", false), menu_item("b", true)];
    let trailing = submenu_trailing(&children);
    assert_eq!(trailing, Some(SharedString::from("b")));
    assert_eq!(
        submenu_row_label(&SharedString::from("Space"), trailing),
        SharedString::from("Space: b")
    );
    assert_eq!(submenu_trailing(&[menu_item("a", false)]), None);
    assert_eq!(
        submenu_row_label(&SharedString::from("Space"), None),
        SharedString::from("Space")
    );
}

// ---- Add project ----

#[test]
fn an_added_project_gives_filters_its_key_and_id() {
    let project = added_sidebar_project(&AddedProject {
        id: "p9".into(),
        name: "nine".into(),
        root: "/work/nine".into(),
        view_key: "key-9".into(),
    });
    assert_eq!(project.view_key, "key-9");
    assert_eq!(project.project_ids, ["p9"]);
    assert_eq!(project.name, "nine");
}

fn step(
    log: &Rc<RefCell<Vec<&'static str>>>,
    name: &'static str,
    result: Result<(), &'static str>,
) -> impl FnOnce() -> std::future::Ready<anyhow::Result<()>> {
    let log = log.clone();
    move || {
        log.borrow_mut().push(name);
        std::future::ready(result.map_err(|text| anyhow::anyhow!(text)))
    }
}

#[test]
fn filters_run_in_order_and_an_error_stops_the_rest() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let outcome = futures::executor::block_on(run_in_order([
        step(&log, "first", Ok(())),
        step(&log, "second", Err("full")),
        step(&log, "third", Ok(())),
    ]));
    assert_eq!(outcome, Err("full".to_string()));
    assert_eq!(*log.borrow(), ["first", "second"]);

    let log = Rc::new(RefCell::new(Vec::new()));
    let outcome = futures::executor::block_on(run_in_order([
        step(&log, "first", Ok(())),
        step(&log, "second", Ok(())),
    ]));
    assert_eq!(outcome, Ok(()));
    assert_eq!(*log.borrow(), ["first", "second"]);
}

// ---- anti-goal ----

#[test]
fn a_500_project_tree_with_two_filters_rebuilds_rows_under_10_ms() {
    let all = projects(500);
    let keys = keys(&all);
    let facts = Facts::default();
    let state = ViewState::default();
    let rebuild = || {
        let visible = visible_projects(&all, &keys, |project| {
            project.name != "project 7" && project.view_key != "key-p9"
        });
        filtered_rows(&all, &visible, &[], &facts, &state)
    };
    // The first call warms the allocator; the limit holds for the call after it.
    let first = rebuild();
    let started = Instant::now();
    let rows = rebuild();
    let elapsed = started.elapsed();
    println!("500 projects, two filters: {elapsed:?}");
    assert_eq!(rows, first, "the same filters give an equal row list");
    assert!(elapsed.as_millis() < 10, "rebuild took {elapsed:?}");
    // Rows without any filter, as the baseline.
    let baseline = build_rows(&all, &[], &facts, &state);
    assert_eq!(
        baseline.len(),
        rows.len() + 8,
        "p7 and p9 dropped, a project row and 3 workspaces each"
    );
}

#[test]
fn a_filter_change_that_hides_nothing_new_leaves_the_rows_equal() {
    let all = projects(500);
    let keys = keys(&all);
    let facts = Facts::default();
    let state = ViewState::default();
    let build = |hide: &str| {
        let visible = visible_projects(&all, &keys, |project| project.name != hide);
        filtered_rows(&all, &visible, &[], &facts, &state)
    };
    // A `SidebarFilterChanged` after which the filters answer the same: an equal list, so the
    // view's `rows == self.rows` check skips the splice and the redraw.
    assert_eq!(build("project 7"), build("project 7"));
    assert_ne!(build("project 7"), build("project 8"));
}
