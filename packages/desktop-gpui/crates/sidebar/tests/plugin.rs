use bachuc_client::{AddedProject, ProjectNode, WorkspaceNode};
use bachuc_icons::IconName;
use bachuc_plugin_host::{
    Contributed, ProjectMenuItem, SidebarAction, SidebarItem, SwipeDirection,
};
use bachuc_sidebar::model::{Facts, Row, ViewState, build_rows};
use bachuc_sidebar::plugin::*;
use gpui_kit::{Point, ScrollDelta, SharedString, TouchPhase, point, px};
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

const WIDTH: f32 = 240.;

/// Feeds `moves` (dx, dy) 8 ms apart after a `Started` at `start`; returns the last step and
/// the time of the last move.
fn drag(gesture: &mut SwipeGesture, start: f64, moves: &[(f32, f32)]) -> (SwipeStep, f64) {
    let mut step = gesture.push(0., 0., TouchPhase::Started, start, WIDTH);
    let mut time = start;
    for &(dx, dy) in moves {
        time += 8.;
        step = gesture.push(dx, dy, TouchPhase::Moved, time, WIDTH);
    }
    (step, time)
}

fn lift(gesture: &mut SwipeGesture, time: f64) -> SwipeStep {
    gesture.push(0., 0., TouchPhase::Ended, time, WIDTH)
}

#[test]
fn the_list_follows_the_finger_one_to_one() {
    let mut gesture = SwipeGesture::default();
    let (step, _) = drag(
        &mut gesture,
        0.,
        &[(4., 0.), (6., 1.), (10., 0.), (25., 0.)],
    );
    assert_eq!(step, SwipeStep::Drag { offset: 45. });
    let frame = drag_frame(45., WIDTH, false);
    assert!(
        (frame.offset + 45. / WIDTH).abs() < 1e-6,
        "a left finger moves the list left"
    );
}

#[test]
fn a_slow_release_past_half_the_width_switches() {
    let mut gesture = SwipeGesture::default();
    let (_, time) = drag(&mut gesture, 0., &[(40., 0.); 4]);
    // The fingers rest before they lift, so only the travel counts.
    assert_eq!(
        lift(&mut gesture, time + 200.),
        SwipeStep::Commit {
            direction: SwipeDirection::Next,
            offset: 160.
        }
    );
}

#[test]
fn a_slow_release_short_of_half_snaps_back() {
    let mut gesture = SwipeGesture::default();
    let (_, time) = drag(&mut gesture, 0., &[(-30., 0.); 3]);
    assert_eq!(
        lift(&mut gesture, time + 200.),
        SwipeStep::SnapBack { offset: -90. }
    );
}

#[test]
fn a_fast_flick_switches_short_of_half() {
    let mut gesture = SwipeGesture::default();
    let (_, time) = drag(&mut gesture, 0., &[(-15., 0.), (-20., 0.)]);
    assert_eq!(
        lift(&mut gesture, time + 4.),
        SwipeStep::Commit {
            direction: SwipeDirection::Previous,
            offset: -35.
        }
    );
}

#[test]
fn a_vertical_scroll_is_never_a_swipe() {
    let mut gesture = SwipeGesture::default();
    let (step, time) = drag(
        &mut gesture,
        0.,
        &[(1., 10.), (30., 0.), (30., 0.), (30., 0.)],
    );
    assert_eq!(step, SwipeStep::Ignore, "the axis locks on the first 8 px");
    assert_eq!(gesture.timeout_ms(), None);
    assert_eq!(lift(&mut gesture, time + 8.), SwipeStep::Ignore);
    let mut diagonal = SwipeGesture::default();
    let (step, _) = drag(&mut diagonal, 0., &[(20., 20.), (20., 20.)]);
    assert_eq!(step, SwipeStep::Ignore);
}

#[test]
fn the_direction_follows_the_finger_when_it_reverses() {
    let mut back = SwipeGesture::default();
    let (step, time) = drag(
        &mut back,
        0.,
        &[(40., 0.), (40., 0.), (40., 0.), (-50., 0.), (-50., 0.)],
    );
    assert_eq!(step, SwipeStep::Drag { offset: 20. });
    assert_eq!(
        lift(&mut back, time + 200.),
        SwipeStep::SnapBack { offset: 20. }
    );

    let mut over = SwipeGesture::default();
    let (_, time) = drag(&mut over, 0., &[(30., 0.), (-40., 0.), (-40., 0.)]);
    assert_eq!(
        lift(&mut over, time + 4.),
        SwipeStep::Commit {
            direction: SwipeDirection::Previous,
            offset: -50.
        },
        "a flick back past the start goes the other way"
    );
}

#[test]
fn the_momentum_tail_never_swipes_again_and_the_next_gesture_does() {
    let mut gesture = SwipeGesture::default();
    let (_, time) = drag(&mut gesture, 0., &[(70., 0.); 2]);
    assert!(matches!(
        lift(&mut gesture, time + 2.),
        SwipeStep::Commit { .. }
    ));
    for step in 0..30 {
        let tail = gesture.push(
            20.,
            0.,
            TouchPhase::Moved,
            time + 10. + step as f64 * 16.,
            WIDTH,
        );
        assert_eq!(tail, SwipeStep::Ignore, "tail event {step}");
    }
    // A new stroke right after the tail, with no quiet gap, starts with `Started`.
    let (_, again) = drag(&mut gesture, time + 500., &[(70., 0.); 2]);
    assert!(matches!(
        lift(&mut gesture, again + 2.),
        SwipeStep::Commit { .. }
    ));
}

#[test]
fn ten_quick_swipes_each_switch() {
    let mut gesture = SwipeGesture::default();
    let mut start = 0.;
    for swipe in 0..10 {
        let (_, time) = drag(&mut gesture, start, &[(20., 0.), (30., 0.), (30., 0.)]);
        assert!(
            matches!(lift(&mut gesture, time + 2.), SwipeStep::Commit { .. }),
            "swipe {swipe}"
        );
        // Momentum between the swipes.
        gesture.push(10., 0., TouchPhase::Moved, time + 12., WIDTH);
        start = time + 30.;
    }
}

#[test]
fn a_wheel_without_phases_ends_after_a_quiet_gap() {
    let mut gesture = SwipeGesture::default();
    for step in 0..9 {
        gesture.push(16., 0., TouchPhase::Moved, step as f64 * 40., WIDTH);
    }
    let last = 8. * 40.;
    assert_eq!(gesture.timeout_ms(), Some(120.));
    assert_eq!(
        gesture.tick(last + 60., WIDTH),
        SwipeStep::Ignore,
        "too early"
    );
    assert_eq!(
        gesture.tick(last + 120., WIDTH),
        SwipeStep::Commit {
            direction: SwipeDirection::Next,
            offset: 144.
        }
    );
    assert_eq!(gesture.timeout_ms(), None);
}

#[test]
fn a_phased_drag_waits_for_its_lift_and_a_lost_one_ends_by_travel() {
    let mut gesture = SwipeGesture::default();
    let (_, time) = drag(&mut gesture, 0., &[(-50., 0.); 3]);
    assert_eq!(
        gesture.tick(time + 400., WIDTH),
        SwipeStep::Ignore,
        "resting fingers"
    );
    assert_eq!(
        gesture.tick(time + 1500., WIDTH),
        SwipeStep::Commit {
            direction: SwipeDirection::Previous,
            offset: -150.
        }
    );
}

#[test]
fn a_cancel_or_a_new_start_mid_drag_snaps_back() {
    let mut gesture = SwipeGesture::default();
    let (_, time) = drag(&mut gesture, 0., &[(40., 0.); 4]);
    assert_eq!(
        gesture.push(0., 0., TouchPhase::Cancelled, time + 8., WIDTH),
        SwipeStep::SnapBack { offset: 160. }
    );
    let mut restarted = SwipeGesture::default();
    let (_, time) = drag(&mut restarted, 0., &[(40., 0.); 2]);
    assert_eq!(
        restarted.push(0., 0., TouchPhase::Started, time + 8., WIDTH),
        SwipeStep::SnapBack { offset: 80. }
    );
}

#[test]
fn a_drag_past_the_last_space_rubber_bands() {
    let free = drag_frame(200., WIDTH, false);
    let banded = drag_frame(200., WIDTH, true);
    assert!(
        banded.offset < 0. && banded.offset > free.offset,
        "{banded:?}"
    );
    let far = drag_frame(5000., WIDTH, true);
    assert!(far.offset > -0.5, "never past half the width: {far:?}");
    assert!(rubber_band(-200., WIDTH) < 0.);
    assert_eq!(rubber_band(0., WIDTH), 0.);
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
    let next = Some(SwipeDirection::Next);
    let start = slide_frame(next, 0., 0.).unwrap();
    assert!(start.offset.abs() < 1e-4 && (start.opacity - 1.).abs() < 1e-4);
    assert!(!start.applied);

    // The out phase ends at 65 ms with the list off to the left and invisible; the rows change
    // there and the new list starts off to the right.
    let apply = slide_frame(next, 0., 65.).unwrap();
    assert!(apply.applied);
    assert!((apply.offset - 1.).abs() < 1e-4, "{apply:?}");
    assert!(apply.opacity.abs() < 1e-4, "{apply:?}");

    let before_apply = slide_frame(next, 0., 64.9).unwrap();
    assert!(!before_apply.applied);
    assert!(before_apply.offset < -0.99, "{before_apply:?}");

    let middle = slide_frame(next, 0., 65. + 52.5).unwrap();
    assert!(middle.offset > 0. && middle.offset < 1.);
    assert!(middle.opacity > 0. && middle.opacity < 1.);

    assert_eq!(slide_frame(next, 0., 170.), None);
    assert_eq!(slide_frame(next, 0., 400.), None);
    let end = slide_frame(next, 0., 169.9).unwrap();
    assert!(end.offset.abs() < 0.01 && end.opacity > 0.99, "{end:?}");
}

#[test]
fn the_slide_goes_the_other_way_for_previous() {
    let next = slide_frame(Some(SwipeDirection::Next), 0., 30.).unwrap();
    let previous = slide_frame(Some(SwipeDirection::Previous), 0., 30.).unwrap();
    assert!(next.offset < 0. && previous.offset > 0.);
    assert!((next.offset + previous.offset).abs() < 1e-6);
    assert_eq!(next.opacity, previous.opacity);
}

#[test]
fn a_released_swipe_slides_on_from_the_finger() {
    let next = Some(SwipeDirection::Next);
    let from = drag_frame(180., WIDTH, false).offset;
    assert!((slide_out_ms(SwipeDirection::Next, from) - 65. * 0.25).abs() < 1e-3);
    let first = slide_frame(next, from, 0.).unwrap();
    assert!(
        (first.offset - from).abs() < 1e-4,
        "no jump at release: {first:?}"
    );
    assert!((first.opacity - drag_frame(180., WIDTH, false).opacity).abs() < 1e-4);
    assert!(slide_frame(next, from, 16.3).unwrap().applied);
    assert_eq!(slide_frame(next, from, 16.25 + 105.), None);
}

#[test]
fn a_snap_back_returns_to_the_start() {
    let first = slide_frame(None, -0.3, 0.).unwrap();
    assert!((first.offset + 0.3).abs() < 1e-4 && !first.applied);
    let middle = slide_frame(None, -0.3, 60.).unwrap();
    assert!(middle.offset > -0.3 && middle.offset < 0.);
    assert_eq!(slide_frame(None, -0.3, 180.), None);
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
