use bachuc_workspace::SplitAxis;
use bachuc_workspace::model::{Layout, MIN_SPLIT_SIZE, Node, Rect, child_rects, resize_pair};
use bachuc_workspace::tab_layout::{self, TAB_MAX_WIDTH, TAB_MIN_WIDTH};

fn layout(keys: &[&str]) -> Layout<String> {
    let mut layout = Layout::new();
    for key in keys {
        layout.open(key.to_string());
    }
    layout
}

fn keys(layout: &Layout<String>, group: u64) -> Vec<&str> {
    layout
        .group(group)
        .unwrap()
        .items
        .iter()
        .map(String::as_str)
        .collect()
}

fn group_ids(layout: &Layout<String>) -> Vec<u64> {
    layout.groups().iter().map(|group| group.id).collect()
}

#[test]
fn open_appends_and_activates_and_an_open_key_is_activated_in_place() {
    let mut layout = layout(&["a", "b", "c"]);
    assert_eq!(keys(&layout, 1), ["a", "b", "c"]);
    assert_eq!(layout.active_key(), Some("c"));
    layout.open("a".into());
    assert_eq!(keys(&layout, 1), ["a", "b", "c"]);
    assert_eq!(layout.active_key(), Some("a"));
}

#[test]
fn closing_the_active_tab_activates_the_next_then_the_previous() {
    let mut layout = layout(&["a", "b", "c"]);
    layout.activate_index(1);
    let closed = layout.close("b").unwrap();
    assert_eq!(closed.item, "b");
    assert_eq!(closed.activated.as_deref(), Some("c"));
    let closed = layout.close("c").unwrap();
    assert_eq!(closed.activated.as_deref(), Some("a"));
    assert_eq!(layout.active_key(), Some("a"));
}

#[test]
fn closing_an_inactive_tab_keeps_the_active_one() {
    let mut layout = layout(&["a", "b", "c"]);
    assert_eq!(layout.active_key(), Some("c"));
    let closed = layout.close("a").unwrap();
    assert_eq!(closed.activated, None);
    assert_eq!(layout.active_key(), Some("c"));
    assert_eq!(layout.group(1).unwrap().active, 1);
}

#[test]
fn closing_the_last_tab_of_the_only_group_keeps_an_empty_group() {
    let mut layout = layout(&["a"]);
    layout.close("a").unwrap();
    assert_eq!(group_ids(&layout), [1]);
    assert_eq!(layout.active_key(), None);
    assert!(layout.close("a").is_none());
}

#[test]
fn split_adds_an_empty_focused_group_and_reuses_a_split_on_the_same_axis() {
    let mut layout = layout(&["a"]);
    let right = layout.split_focused(SplitAxis::Right);
    assert_eq!(layout.focused(), right);
    assert_eq!(layout.active_key(), None);
    layout.open("b".into());
    let third = layout.split_focused(SplitAxis::Right);
    let Node::Split(split) = layout.root() else {
        panic!("the root is a split");
    };
    assert_eq!(split.axis, SplitAxis::Right);
    assert_eq!(split.children.len(), 3);
    assert_eq!(split.ratios, [0.5, 0.25, 0.25]);
    assert_eq!(group_ids(&layout), [1, right, third]);
}

#[test]
fn split_down_inside_a_row_nests_a_column() {
    let mut layout = layout(&["a"]);
    let right = layout.split_focused(SplitAxis::Right);
    let below = layout.split_focused(SplitAxis::Down);
    assert_eq!(group_ids(&layout), [1, right, below]);
    let Node::Split(row) = layout.root() else {
        panic!("the root is a split");
    };
    let Node::Split(column) = &row.children[1] else {
        panic!("the right child is a split");
    };
    assert_eq!(column.axis, SplitAxis::Down);
    assert_eq!(column.ratios, [0.5, 0.5]);
}

#[test]
fn closing_the_last_tab_of_a_split_group_removes_the_group_and_its_divider() {
    let mut layout = layout(&["a"]);
    let right = layout.split_focused(SplitAxis::Right);
    layout.open("b".into());
    let closed = layout.close("b").unwrap();
    assert_eq!(group_ids(&layout), [1]);
    assert!(
        matches!(layout.root(), Node::Group(_)),
        "the split collapsed"
    );
    assert_eq!(layout.focused(), 1, "the focus went to the group before it");
    assert_eq!(closed.activated.as_deref(), Some("a"));
    assert!(layout.group(right).is_none());
}

#[test]
fn removing_a_group_from_three_renormalizes_the_others() {
    let mut layout = layout(&["a"]);
    let second = layout.split_focused(SplitAxis::Right);
    layout.open("b".into());
    let third = layout.split_focused(SplitAxis::Right);
    layout.open("c".into());
    layout.close("b").unwrap();
    assert_eq!(group_ids(&layout), [1, third]);
    assert!(layout.group(second).is_none());
    let Node::Split(split) = layout.root() else {
        panic!("two groups remain in a split");
    };
    let total: f32 = split.ratios.iter().sum();
    assert!((total - 1.).abs() < 1e-6);
    assert!((split.ratios[0] - 2. / 3.).abs() < 1e-6);
    assert_eq!(
        layout.focused(),
        third,
        "closing in an unfocused group keeps the focus"
    );
}

#[test]
fn closing_the_first_group_focuses_the_next_one() {
    let mut layout = layout(&["a"]);
    let right = layout.split_focused(SplitAxis::Right);
    layout.open("b".into());
    layout.focus(1);
    layout.close("a").unwrap();
    assert_eq!(group_ids(&layout), [right]);
    assert_eq!(layout.focused(), right);
    assert_eq!(layout.active_key(), Some("b"));
}

#[test]
fn move_reorders_inside_a_group() {
    let mut layout = layout(&["a", "b", "c", "d"]);
    // Drop "a" into the slot before "c": it lands between "b" and "c".
    assert!(layout.move_item("a", 1, 2));
    assert_eq!(keys(&layout, 1), ["b", "a", "c", "d"]);
    assert_eq!(layout.active_key(), Some("a"));
    // Drop "d" into the first slot.
    assert!(layout.move_item("d", 1, 0));
    assert_eq!(keys(&layout, 1), ["d", "b", "a", "c"]);
    // Drop "b" past the end.
    assert!(layout.move_item("b", 1, 99));
    assert_eq!(keys(&layout, 1), ["d", "a", "c", "b"]);
    // Dropping a tab on its own slot changes nothing.
    assert!(!layout.move_item("a", 1, 1));
    assert!(!layout.move_item("a", 1, 2));
}

#[test]
fn move_across_groups_activates_and_focuses_the_target() {
    let mut layout = layout(&["a", "b"]);
    let right = layout.split_focused(SplitAxis::Right);
    layout.open("c".into());
    layout.focus(1);
    assert!(layout.move_item("b", right, 0));
    assert_eq!(keys(&layout, 1), ["a"]);
    assert_eq!(keys(&layout, right), ["b", "c"]);
    assert_eq!(layout.focused(), right);
    assert_eq!(layout.active_key(), Some("b"));
    assert_eq!(layout.group(1).unwrap().active, 0);
}

#[test]
fn moving_the_last_tab_out_of_a_group_removes_that_group() {
    let mut layout = layout(&["a"]);
    let right = layout.split_focused(SplitAxis::Right);
    layout.open("b".into());
    assert!(layout.move_item("a", right, 2));
    assert_eq!(group_ids(&layout), [right]);
    assert!(matches!(layout.root(), Node::Group(_)));
    assert_eq!(keys(&layout, right), ["b", "a"]);
    assert_eq!(layout.focused(), right);
}

#[test]
fn move_to_a_missing_group_or_key_does_nothing() {
    let mut layout = layout(&["a"]);
    assert!(!layout.move_item("a", 42, 0));
    assert!(!layout.move_item("x", 1, 0));
    assert_eq!(keys(&layout, 1), ["a"]);
}

#[test]
fn resize_clamps_each_side_to_the_min_split_size() {
    assert_eq!(resize_pair(&[0.5, 0.5], 0, 0.7), [0.7, 0.3]);
    let low = resize_pair(&[0.5, 0.5], 0, -1.);
    assert!((low[0] - MIN_SPLIT_SIZE).abs() < 1e-6 && (low[1] - 0.9).abs() < 1e-6);
    let high = resize_pair(&[0.5, 0.5], 0, 2.);
    assert!((high[0] - 0.9).abs() < 1e-6 && (high[1] - MIN_SPLIT_SIZE).abs() < 1e-6);
    // A pair smaller than two minimums splits evenly at most.
    let small = resize_pair(&[0.9, 0.05, 0.05], 1, 0.);
    assert_eq!(small[0], 0.9);
    assert!((small[1] - 0.05).abs() < 1e-6 && (small[2] - 0.05).abs() < 1e-6);
    // An index without a right neighbor is left alone.
    assert_eq!(resize_pair(&[0.5, 0.5], 1, 0.2), [0.5, 0.5]);
}

#[test]
fn layout_resize_changes_only_the_pair_at_the_divider() {
    let mut layout = layout(&["a"]);
    layout.split_focused(SplitAxis::Right);
    layout.split_focused(SplitAxis::Right);
    let Node::Split(split) = layout.root() else {
        panic!("split");
    };
    let id = split.id;
    assert!(layout.resize(id, 1, 0.05));
    let split = layout.split(id).unwrap();
    assert_eq!(split.ratios[0], 0.5);
    assert!((split.ratios[1] - MIN_SPLIT_SIZE).abs() < 1e-6);
    assert!((split.ratios[2] - 0.4).abs() < 1e-6);
    assert!(
        !layout.resize(id, 1, 0.05),
        "a clamped repeat changes nothing"
    );
}

#[test]
fn child_rects_leave_one_pixel_per_divider() {
    let rect = Rect {
        x: 0.,
        y: 0.,
        w: 201.,
        h: 100.,
    };
    let rects = child_rects(rect, SplitAxis::Right, &[0.5, 0.5]);
    assert_eq!(rects[0].w, 100.);
    assert_eq!(rects[1].x, 101.);
    assert_eq!(rects[1].h, 100.);
    let rects = child_rects(rect, SplitAxis::Down, &[0.25, 0.75]);
    assert_eq!(rects[0].h, 24.75);
    assert_eq!(rects[1].y, 25.75);
}

#[test]
fn tabs_take_their_natural_width_up_to_the_max() {
    let layout = tab_layout::compute(1000., &[20., 500.]);
    assert!(!layout.scrolls);
    assert_eq!(layout.widths, [TAB_MIN_WIDTH, TAB_MAX_WIDTH]);
    let layout = tab_layout::compute(1000., &[90.]);
    assert_eq!(layout.widths, [124.]);
}

#[test]
fn many_tabs_shrink_to_the_min_width_then_scroll() {
    // Row 600 leaves 600 - 36 - 27 = 537 after the buttons, and 517 after the row padding and
    // three gaps.
    let labels = [200.; 4];
    let layout = tab_layout::compute(600., &labels);
    assert!(!layout.scrolls);
    let total: f32 = layout.widths.iter().sum();
    assert!(total <= 517. + 2., "{:?}", layout.widths);
    assert!(
        layout
            .widths
            .iter()
            .all(|width| *width >= TAB_MIN_WIDTH && *width < TAB_MAX_WIDTH)
    );

    let labels = [200.; 12];
    let layout = tab_layout::compute(600., &labels);
    assert!(layout.scrolls);
    assert!(layout.widths.iter().all(|width| *width == TAB_MIN_WIDTH));
}

#[test]
fn the_drop_slot_follows_tab_centers() {
    let widths = [100., 100.];
    // Row padding 4, each slot 104 wide.
    assert_eq!(tab_layout::drop_slot(&widths, 0.), 0);
    assert_eq!(tab_layout::drop_slot(&widths, 55.), 0);
    assert_eq!(tab_layout::drop_slot(&widths, 57.), 1);
    assert_eq!(tab_layout::drop_slot(&widths, 159.), 1);
    assert_eq!(tab_layout::drop_slot(&widths, 161.), 2);
    assert_eq!(tab_layout::drop_slot(&[], 10.), 0);
}
