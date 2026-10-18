//! The Chat outline rules: magnification, the lit prompt, the hover intent, the jump plan, and
//! when the rail shows.

use bachuc_timeline::outline::{
    HORIZONTAL_TRANSIT_PX, HoverIntent, IntentEffect, JumpPlan, accept_prompt_index_epoch,
    pill_size, plan_prompt_jump, prompt_tick_magnification, rail_visible, reading_anchor,
    resolve_active_prompt_seq,
};

#[test]
fn magnification_is_one_at_the_pointer_and_zero_at_three_slots() {
    assert_eq!(prompt_tick_magnification(0.), 1.);
    assert!(prompt_tick_magnification(3.).abs() < 1e-6);
    assert_eq!(prompt_tick_magnification(4.), 0.);
    assert_eq!(prompt_tick_magnification(f32::NAN), 0.);
    assert_eq!(prompt_tick_magnification(f32::INFINITY), 0.);
}

#[test]
fn magnification_is_symmetric_and_falls_off_smoothly() {
    for distance in [0.5, 1., 1.5, 2., 2.9] {
        assert_eq!(
            prompt_tick_magnification(distance),
            prompt_tick_magnification(-distance)
        );
    }
    // Raised cosine: half way at 1.5 slots, and each step lower than the one before.
    assert!((prompt_tick_magnification(1.5) - 0.5).abs() < 1e-6);
    let steps: Vec<f32> = [0., 1., 2., 2.9]
        .into_iter()
        .map(prompt_tick_magnification)
        .collect();
    assert!(steps.windows(2).all(|pair| pair[0] > pair[1]));
}

#[test]
fn no_prompt_is_lit_before_the_first_prompt_or_without_a_position() {
    let prompts = [4, 10, 20];
    assert_eq!(resolve_active_prompt_seq(&prompts, None), None);
    assert_eq!(resolve_active_prompt_seq(&prompts, Some(3)), None);
    assert_eq!(resolve_active_prompt_seq(&[], Some(3)), None);
}

#[test]
fn the_lit_prompt_is_the_last_one_at_or_before_the_anchor() {
    let prompts = [4, 10, 20];
    assert_eq!(resolve_active_prompt_seq(&prompts, Some(4)), Some(4));
    assert_eq!(resolve_active_prompt_seq(&prompts, Some(9)), Some(4));
    assert_eq!(resolve_active_prompt_seq(&prompts, Some(10)), Some(10));
    assert_eq!(resolve_active_prompt_seq(&prompts, Some(500)), Some(20));
}

#[test]
fn the_reading_anchor_skips_rows_without_one() {
    let anchors = [Some(2), None, Some(9), None, None];
    let at = |top| reading_anchor(top, anchors.len(), |index| anchors[index]);
    assert_eq!(at(0), Some(2));
    assert_eq!(at(1), Some(2));
    assert_eq!(at(2), Some(9));
    assert_eq!(at(4), Some(9));
    // Past the end reads the last row; no rows read nothing.
    assert_eq!(at(99), Some(9));
    assert_eq!(reading_anchor(3, 0, |_| Some(1)), None);
    assert_eq!(reading_anchor(0, 2, |_| None), None);
}

#[test]
fn bars_rest_lit_and_magnified_at_the_electron_sizes() {
    assert_eq!(pill_size(false, 0.), (10., 2.));
    assert_eq!(pill_size(true, 0.), (18., 2.));
    assert_eq!(pill_size(false, 1.), (26., 4.));
    assert_eq!(pill_size(true, 1.), (26., 4.));
    assert_eq!(pill_size(false, 0.5), (18., 3.));
    assert_eq!(pill_size(false, 9.), (26., 4.));
}

#[test]
fn the_rail_needs_two_prompts_and_918_px_at_the_zoom() {
    assert!(rail_visible(918., 1., 2));
    assert!(!rail_visible(917.9, 1., 2));
    assert!(!rail_visible(2000., 1., 1));
    assert!(!rail_visible(2000., 1., 0));
    // At 125% the threshold is 1147.5 device px.
    assert!(!rail_visible(1100., 1.25, 5));
    assert!(rail_visible(1148., 1.25, 5));
}

#[test]
fn an_index_of_another_epoch_is_refused() {
    assert!(accept_prompt_index_epoch(None, "e2"));
    assert!(accept_prompt_index_epoch(Some("e2"), "e2"));
    assert!(!accept_prompt_index_epoch(Some("e1"), "e2"));
}

#[test]
fn a_loaded_prompt_scrolls_and_an_older_one_loads_pages() {
    assert_eq!(plan_prompt_jump(true, true), JumpPlan::Scroll);
    assert_eq!(plan_prompt_jump(true, false), JumpPlan::Scroll);
    assert_eq!(plan_prompt_jump(false, true), JumpPlan::LoadOlder);
    assert_eq!(plan_prompt_jump(false, false), JumpPlan::Abandon);
}

#[test]
fn the_first_preview_waits_and_the_next_ones_show_at_once() {
    let mut intent = HoverIntent::default();
    intent.enter((10., 100.));
    assert_eq!(intent.point_at(3), IntentEffect::Schedule);
    // The pointer crosses more bars during the wait: the timer keeps running.
    assert_eq!(intent.point_at(4), IntentEffect::Idle);
    // The timer ends: the latest bar shows.
    assert_eq!(intent.fire(), IntentEffect::Activate(4));
    assert_eq!(intent.point_at(5), IntentEffect::Activate(5));
    assert_eq!(intent.point_at(2), IntentEffect::Activate(2));
}

#[test]
fn leaving_the_rail_hides_the_preview_and_starts_the_wait_again() {
    let mut intent = HoverIntent::default();
    intent.enter((0., 0.));
    intent.point_at(1);
    intent.fire();
    assert_eq!(intent.leave(), IntentEffect::Deactivate);
    intent.enter((0., 0.));
    assert_eq!(intent.point_at(1), IntentEffect::Schedule);
}

#[test]
fn a_firing_without_a_bar_shows_nothing() {
    let mut intent = HoverIntent::default();
    assert_eq!(intent.fire(), IntentEffect::Idle);
}

#[test]
fn a_horizontal_move_restarts_the_wait_and_a_vertical_one_does_not() {
    let mut intent = HoverIntent::default();
    intent.enter((0., 100.));
    intent.point_at(1);
    // Mostly vertical: only the anchor follows.
    assert_eq!(intent.pointer_moved((1., 120.)), IntentEffect::Idle);
    // Wider than tall, but under the transit distance.
    assert_eq!(
        intent.pointer_moved((1. + HORIZONTAL_TRANSIT_PX - 1., 121.)),
        IntentEffect::Idle
    );
    // Wider than tall and past the transit distance: the wait restarts.
    assert_eq!(intent.pointer_moved((12., 122.)), IntentEffect::Schedule);
}

#[test]
fn a_move_does_nothing_once_a_preview_shows_or_before_the_pointer_entered() {
    let mut intent = HoverIntent::default();
    assert_eq!(intent.pointer_moved((50., 0.)), IntentEffect::Idle);
    intent.enter((0., 0.));
    intent.point_at(1);
    intent.fire();
    assert_eq!(intent.pointer_moved((50., 0.)), IntentEffect::Idle);
}

#[test]
fn a_horizontal_move_before_any_bar_schedules_nothing() {
    let mut intent = HoverIntent::default();
    intent.enter((0., 0.));
    assert_eq!(intent.pointer_moved((50., 0.)), IntentEffect::Idle);
}
