//! The pure motion rules of TASK-266 and TASK-267: the synced loader, the running pulse, the
//! shimmer band, and the running footer's clock start.

use std::time::Duration;

use bachuc_client::ActiveTurn;
use bachuc_timeline::motion;
use bachuc_timeline::running_start_ms;

#[test]
fn the_loader_follows_its_six_steps() {
    assert_eq!(motion::loader_step(0), 0);
    assert_eq!(motion::loader_step(159), 1);
    assert_eq!(motion::loader_step(949), 5);
    assert_eq!(motion::loader_step(950), 0);
}

#[test]
fn the_loader_dot_opacity_reads_its_table() {
    let step0: Vec<f32> = (0..6)
        .map(|dot| motion::loader_dot_opacity(0, dot))
        .collect();
    assert_eq!(step0, [1., 0., 0.78, 0., 0.56, 0.34]);
    assert_eq!(motion::loader_dot_opacity(6, 0), 0.);
    assert_eq!(motion::loader_dot_opacity(0, 6), 0.);
}

#[test]
fn the_loader_geometry_fills_the_14_px_box() {
    let geometry = motion::loader_geometry(14.);
    assert_eq!(geometry.gap, 2.);
    assert_eq!(geometry.dot, 10. / 3.);
    assert!((geometry.width - 8.666_667).abs() < 0.001, "2 dots wide");
    assert!(
        (geometry.height - 14.).abs() < 0.001,
        "3 dots and 2 gaps high"
    );
}

#[test]
fn the_pulse_fades_to_0_3_and_back() {
    assert_eq!(motion::pulse_opacity(0), 1.);
    assert_eq!(motion::pulse_opacity(700), 0.3);
    assert_eq!(motion::pulse_opacity(1400), 1.);
    let mid = motion::pulse_opacity(350);
    assert!(mid > 0.3 && mid < 1., "mid-pulse is {mid}");
    assert_eq!(motion::pulse_opacity(2100), 0.3);
}

#[test]
fn the_shimmer_duration_follows_the_character_count() {
    let close = |got: f64, want: f64| (got - want).abs() < 1e-9;
    assert!(close(motion::shimmer_duration(10), 1.08));
    assert!(close(motion::shimmer_duration(12), 1.096));
    assert!(close(motion::shimmer_duration(13), 1.354));
    assert!(close(motion::shimmer_duration(100), 2.05));
    assert!(close(motion::shimmer_duration(200), 2.3));
    assert_eq!(motion::shimmer_chars("  bash  ", Some(" ls ")), 6);
    assert_eq!(motion::shimmer_chars("bash", None), 4);
}

#[test]
fn the_shimmer_band_width_is_clamped() {
    assert_eq!(motion::shimmer_band_width(100.), 42.);
    assert_eq!(motion::shimmer_band_width(300.), 66.);
    assert_eq!(motion::shimmer_band_width(1000.), 120.);
    assert_eq!(motion::shimmer_band_width(10.), 42.);
}

#[test]
fn the_shimmer_band_runs_from_the_span_start_to_its_end() {
    let span_start = 0.;
    let span_end = 300.;
    let band = 66.;
    let at_start = motion::shimmer_band_left(span_start, span_end, band, 0.);
    assert!(
        (at_start + band - span_start).abs() < 0.001,
        "ends at the start"
    );
    assert!((motion::shimmer_band_left(span_start, span_end, band, 1.) - span_end).abs() < 0.001);
}

#[test]
fn the_shimmer_gradient_fades_at_both_ends() {
    assert_eq!(motion::shimmer_alpha(0.), 0.);
    assert_eq!(motion::shimmer_alpha(0.24), 0.45);
    assert_eq!(motion::shimmer_alpha(0.5), 1.);
    assert_eq!(motion::shimmer_alpha(0.76), 0.45);
    assert_eq!(motion::shimmer_alpha(1.), 0.);
    assert_eq!(motion::shimmer_phase(0, 1.08), 0.);
    assert!((motion::shimmer_phase(540, 1.08) - 0.5).abs() < 0.001);
}

#[test]
fn a_running_tool_call_gets_its_shimmer_motion() {
    // The daemon wraps the item: status lives at `entry["item"]["status"]`, and the gate in
    // `ensure_motions` creates `MotionKey::Badge` only when this returns true.
    let running = serde_json::json!({
        "seqStart": 7,
        "item": {"type": "tool_call", "status": "running"},
    });
    let executing = serde_json::json!({
        "seqStart": 8,
        "item": {"type": "tool_call", "status": "executing"},
    });
    let completed = serde_json::json!({
        "seqStart": 9,
        "item": {"type": "tool_call", "status": "completed"},
    });
    let wrapper_status = serde_json::json!({
        "seqStart": 10,
        "status": "running",
        "item": {"type": "tool_call"},
    });
    assert!(motion::tool_entry_running(&running));
    assert!(motion::tool_entry_running(&executing));
    assert!(!motion::tool_entry_running(&completed));
    assert!(
        !motion::tool_entry_running(&wrapper_status),
        "a status on the wrapper is not the item's status"
    );
    // The shimmer of that entry moves across its loop.
    let duration = motion::shimmer_duration(motion::shimmer_chars("bash", Some("sleep 90")));
    assert_ne!(
        motion::shimmer_state(0, duration),
        motion::shimmer_state((duration * 500.) as u128, duration)
    );
}

#[test]
fn an_unchanged_quantized_frame_is_skipped() {
    let interval = Duration::from_millis(66);
    assert!(!motion::frame_draws(
        Duration::from_millis(10),
        interval,
        true
    ));
    assert!(!motion::frame_draws(
        Duration::from_millis(80),
        interval,
        false
    ));
    assert!(motion::frame_draws(
        Duration::from_millis(80),
        interval,
        true
    ));
    // The loader state skips the frames between its 158 ms steps.
    assert_eq!(
        motion::loader_step(100),
        motion::loader_step(150),
        "one loader step"
    );
    // The pulse quantizes to levels: times in one level draw the same frame.
    assert_eq!(
        motion::pulse_state(0),
        motion::pulse_state(5),
        "both are full opacity"
    );
    assert_ne!(motion::pulse_state(0), motion::pulse_state(700));
    let duration = motion::shimmer_duration(20);
    assert_eq!(
        motion::shimmer_state(10, duration),
        motion::shimmer_state(11, duration),
        "11 ms is inside one band step"
    );
}

#[test]
fn the_footer_start_prefers_the_active_turn() {
    let active = |started: Option<&str>| ActiveTurn {
        turn_id: "t1".to_string(),
        started_at: started.map(str::to_string),
    };
    let parsed = running_start_ms(Some(active(Some("2026-10-19T10:39:00.123Z"))), Some(1));
    let Some(parsed) = parsed else {
        panic!("a readable startedAt gives a start");
    };
    // Milliseconds survive the parse.
    assert_eq!(parsed % 1000, 123);
    assert_eq!(
        running_start_ms(Some(active(None)), Some(5)),
        Some(5),
        "no startedAt takes the user message"
    );
    assert_eq!(
        running_start_ms(Some(active(Some("not a date"))), Some(5)),
        Some(5),
        "an unreadable startedAt takes the user message"
    );
    assert_eq!(running_start_ms(Some(active(None)), None), None);
    assert_eq!(running_start_ms(None, None), None);
}
