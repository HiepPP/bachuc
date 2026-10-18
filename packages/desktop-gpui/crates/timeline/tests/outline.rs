//! The Chat outline rules: magnification, the lit prompt, the hover intent, the jump plan, and
//! when the rail shows.

use std::time::Duration;

use bachuc_timeline::outline::{
    BAR_EASE, BarEase, BarLook, HORIZONTAL_TRANSIT_PX, HoverIntent, IntentEffect, JumpPlan,
    accept_prompt_index_epoch, ease_out, pill_size, plan_prompt_jump, prompt_tick_magnification,
    rail_visible, reading_anchor, resolve_active_prompt_seq,
};
use gpui_kit::Rgba;

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

fn look(width: f32, height: f32, color: f32) -> BarLook {
    BarLook {
        width,
        height,
        color: Rgba {
            r: color,
            g: color,
            b: color,
            a: 1.,
        },
    }
}

fn ms(millis: u64) -> Duration {
    Duration::from_millis(millis)
}

#[test]
fn ease_out_runs_from_zero_at_the_start_to_one_at_140_ms() {
    assert_eq!(BAR_EASE, ms(140));
    let progress = |elapsed: Duration| ease_out(elapsed.as_secs_f32() / BAR_EASE.as_secs_f32());
    assert_eq!(progress(Duration::ZERO), 0.);
    assert!((progress(BAR_EASE) - 1.).abs() < 1e-5);
    // Past the end and out-of-range input stay inside 0..=1.
    assert!((progress(ms(500)) - 1.).abs() < 1e-5);
    assert_eq!(ease_out(-1.), 0.);
    assert_eq!(ease_out(f32::NAN), 0.);
    assert_eq!(ease_out(f32::INFINITY), 1.);
}

#[test]
fn ease_out_never_decreases_and_starts_fast() {
    let samples: Vec<f32> = (0..=100).map(|step| ease_out(step as f32 / 100.)).collect();
    assert!(samples.windows(2).all(|pair| pair[0] <= pair[1]));
    // cubic-bezier(0, 0, 0.58, 1) is ahead of linear for the first half.
    assert!(ease_out(0.25) > 0.25);
    assert!(ease_out(0.5) > 0.5);
    // Spot check of the CSS curve: y at t = 0.5 is about 0.68.
    assert!((ease_out(0.5) - 0.68).abs() < 0.02);
}

#[test]
fn a_bar_interpolates_size_and_color_between_two_looks() {
    let rest = look(10., 2., 0.2);
    let big = look(26., 4., 0.8);
    assert_eq!(rest.lerp(big, 0.), rest);
    assert_eq!(rest.lerp(big, 1.), big);
    let half = rest.lerp(big, 0.5);
    assert_eq!((half.width, half.height), (18., 3.));
    assert!((half.color.r - 0.5).abs() < 1e-6);
    assert!((half.color.a - 1.).abs() < 1e-6);
}

#[test]
fn a_new_target_eases_from_the_old_look_over_140_ms() {
    let rest = look(10., 2., 0.2);
    let big = look(26., 4., 0.8);
    let mut bar = BarEase::new(rest);
    // At rest: no ease to run, nothing changes.
    assert!(!bar.retarget(rest, Duration::ZERO));
    assert_eq!(bar.generation(), 0);
    assert!(bar.retarget(big, ms(10_000)));
    assert_eq!(bar.generation(), 1);
    assert_eq!(bar.look_at(Duration::ZERO), rest);
    let mid = bar.look_at(ms(70));
    assert!(mid.width > 10. && mid.width < 26.);
    assert!(mid.height > 2. && mid.height < 4.);
    assert!(mid.color.r > 0.2 && mid.color.r < 0.8);
    assert!(!bar.settled(ms(139)));
    assert!(bar.settled(ms(140)));
    assert_eq!(bar.look_at(ms(140)).width, 26.);
    // The same target again is no change, so no new animation starts.
    assert!(!bar.retarget(big, ms(30)));
    assert_eq!(bar.generation(), 1);
}

#[test]
fn a_retarget_in_mid_ease_starts_from_the_drawn_look() {
    let rest = look(10., 2., 0.2);
    let big = look(26., 4., 0.8);
    let mut bar = BarEase::new(rest);
    bar.retarget(big, ms(1_000));
    let drawn = bar.look_at(ms(50));
    assert!(drawn.width > 10. && drawn.width < 26.);
    // The pointer moves away 50 ms in: the bar heads back to rest from where it is.
    assert!(bar.retarget(rest, ms(50)));
    assert_eq!(bar.from(), drawn);
    assert_eq!(bar.to(), rest);
    assert_eq!(bar.generation(), 2);
    assert_eq!(bar.look_at(Duration::ZERO), drawn);
    assert_eq!(bar.look_at(BAR_EASE).width, 10.);
}
