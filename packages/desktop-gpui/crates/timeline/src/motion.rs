//! The running marks of the timeline: the synced 6-dot loader (`synced-loader.tsx`), the
//! running pulse of `usePulse`, and the tool-row shimmer of `message.tsx`.
//!
//! Every mark is its own small view, like `sidebar/src/ring.rs`, so a frame redraws the mark
//! and not the timeline list. It asks for a frame every `interval` while it was painted in the
//! last 200 ms and sleeps otherwise. All marks read one clock, so they move in step.

use std::cell::Cell;
use std::rc::Rc;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use bachuc_theme::tokens;
use gpui_kit::*;

/// How often a painted mark asks for the next frame.
pub const FRAME_INTERVAL: Duration = Duration::from_millis(33);
/// A mark that was not painted for this long stops asking for frames.
const SLEEP_AFTER: Duration = Duration::from_millis(200);

/// One loader loop, in six steps (`SYNCED_LOADER_DURATION_MS`).
pub const LOADER_PERIOD_MS: u128 = 950;
pub const LOADER_DOTS: usize = 6;
const LOADER_OPACITY_STATES: [[f32; LOADER_DOTS]; 6] = [
    [1., 0., 0.78, 0., 0.56, 0.34],
    [0.78, 1., 0.56, 0., 0.34, 0.],
    [0.56, 0.78, 0.34, 1., 0., 0.],
    [0.34, 0.56, 0., 0.78, 0., 1.],
    [0., 0.34, 0., 0.56, 1., 0.78],
    [0., 0., 1., 0.34, 0.78, 0.56],
];

/// One pulse half, in `PULSE_MS`: 1 to 0.3 and back.
pub const PULSE_MS: u128 = 700;
const PULSE_LOW: f32 = 0.3;

/// Milliseconds on the one shared motion clock.
pub fn motion_ms() -> u128 {
    static START: OnceLock<Instant> = OnceLock::new();
    START.get_or_init(Instant::now).elapsed().as_millis()
}

/// A mark that redraws itself at `interval` while it is on screen.
pub struct Motion {
    interval: Duration,
    draw: Box<dyn Fn(u128, &mut Window, &mut App) -> AnyElement>,
    painted: Rc<Cell<Instant>>,
    _tick: Task<()>,
}

impl Motion {
    pub fn new(
        interval: Duration,
        draw: impl Fn(u128, &mut Window, &mut App) -> AnyElement + 'static,
        cx: &mut Context<Motion>,
    ) -> Motion {
        let painted = Rc::new(Cell::new(Instant::now()));
        let seen = painted.clone();
        let tick = cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(interval).await;
                let alive = this.update(cx, |_, cx| {
                    if seen.get().elapsed() < SLEEP_AFTER {
                        cx.notify();
                    }
                });
                if alive.is_err() {
                    break;
                }
            }
        });
        Motion {
            interval,
            draw: Box::new(draw),
            painted,
            _tick: tick,
        }
    }

    pub fn interval(&self) -> Duration {
        self.interval
    }

    pub fn set_draw(
        &mut self,
        draw: impl Fn(u128, &mut Window, &mut App) -> AnyElement + 'static,
        cx: &mut Context<Motion>,
    ) {
        self.draw = Box::new(draw);
        cx.notify();
    }
}

impl Render for Motion {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.painted.set(Instant::now());
        // Reduced motion draws the still state, as `useReducedMotion` does.
        let time = if cx.reduce_motion() { 0 } else { motion_ms() };
        (self.draw)(time, window, cx)
    }
}

/// The loader draw of `turn-footer.tsx`: a 14 px box holding the 2 x 3 dot grid.
pub fn loader_draw() -> impl Fn(u128, &mut Window, &mut App) -> AnyElement {
    |ms, _window, cx| {
        let zoom = bachuc_theme::zoom(cx);
        let geometry = loader_geometry(14.);
        let step = loader_step(ms);
        let box_size = 14. * zoom;
        let (dot, gap) = (geometry.dot * zoom, geometry.gap * zoom);
        let (grid_w, grid_h) = (geometry.width * zoom, geometry.height * zoom);
        canvas(
            |_, _, _| (),
            move |bounds, (), window, cx| {
                let color = tokens(cx).foreground;
                let origin = point(
                    bounds.left() + px((box_size - grid_w) / 2.),
                    bounds.top() + px((box_size - grid_h) / 2.),
                );
                for index in 0..LOADER_DOTS {
                    let opacity = loader_dot_opacity(step, index);
                    if opacity <= 0. {
                        continue;
                    }
                    let column = index % 2;
                    let row = index / 2;
                    let at = point(
                        origin.x + px(column as f32 * (dot + gap)),
                        origin.y + px(row as f32 * (dot + gap)),
                    );
                    window.paint_quad(quad(
                        Bounds::new(at, size(px(dot), px(dot))),
                        Corners::all(px(dot / 2.)),
                        color.opacity(opacity),
                        Edges::all(px(0.)),
                        transparent_black(),
                        BorderStyle::Solid,
                    ));
                }
            },
        )
        .w(px(box_size))
        .h(px(box_size))
        .into_any_element()
    }
}

/// `getSyncedLoaderStep`: one loop is 950 ms in six steps.
pub fn loader_step(now_ms: u128) -> usize {
    ((now_ms % LOADER_PERIOD_MS) * LOADER_DOTS as u128 / LOADER_PERIOD_MS) as usize
}

/// `getSyncedLoaderDotOpacity`; an unknown step or dot draws 0.
pub fn loader_dot_opacity(step: usize, dot: usize) -> f32 {
    LOADER_OPACITY_STATES
        .get(step)
        .and_then(|states| states.get(dot))
        .copied()
        .unwrap_or(0.)
}

/// The 2 x 3 loader grid inside a `size` box.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LoaderGeometry {
    pub gap: f32,
    pub dot: f32,
    pub width: f32,
    pub height: f32,
}

/// `SyncedLoader`: the gap is `max(1, round(size * 0.12))` and the dot
/// `max(2, (size - 2 * gap) / 3)`; the grid is 2 dots wide and 3 high.
pub fn loader_geometry(size: f32) -> LoaderGeometry {
    let gap = (size * 0.12).round().max(1.);
    let dot = ((size - 2. * gap) / 3.).max(2.);
    LoaderGeometry {
        gap,
        dot,
        width: dot * 2. + gap,
        height: dot * 3. + gap * 2.,
    }
}

fn ease_in_out(t: f32) -> f32 {
    if t < 0.5 {
        4. * t * t * t
    } else {
        let u = -2. * t + 2.;
        1. - u * u * u / 2.
    }
}

/// `usePulse`: 1 to 0.3 in 700 ms and back in 700 ms, on the default ease in-out curve.
pub fn pulse_opacity(ms: u128) -> f32 {
    let phase = (ms % (PULSE_MS * 2)) as f32 / PULSE_MS as f32;
    if phase < 1. {
        1. - (1. - PULSE_LOW) * ease_in_out(phase)
    } else {
        PULSE_LOW + (1. - PULSE_LOW) * ease_in_out(phase - 1.)
    }
}

/// The characters the shimmer timing counts: the trimmed label and summary, in UTF-16 units
/// as JS `length` counts them.
pub fn shimmer_chars(label: &str, summary: Option<&str>) -> usize {
    label.trim().encode_utf16().count()
        + summary.map_or(0, |summary| summary.trim().encode_utf16().count())
}

/// The shimmer loop in seconds: shorter text sweeps faster, and a loop of 12 characters or
/// fewer is 0.25 s shorter.
pub fn shimmer_duration(chars: usize) -> f64 {
    let short_adjustment = if chars <= 12 { 0.25 } else { 0. };
    (1.25 + chars as f64 * 0.008 - short_adjustment).clamp(1., 2.3)
}

/// The moving band's width: 22% of the span, between 42 and 120 px.
pub fn shimmer_band_width(span: f32) -> f32 {
    (span * 0.22).clamp(42., 120.)
}

/// Where the band's left edge sits at `phase` 0 to 1: at phase 0 it ends at the span start, and
/// at phase 1 it starts at the span end.
pub fn shimmer_band_left(span_start: f32, span_end: f32, band_width: f32, phase: f32) -> f32 {
    let track_start = span_start - band_width;
    track_start + (span_end - track_start) * phase.clamp(0., 1.)
}

/// The band's white alpha at `position` 0 to 1 across it, from `SHIMMER_GRADIENT`: transparent
/// at both ends, 0.45 at 24% and 76%, and solid white between 40% and 60%.
pub fn shimmer_alpha(position: f32) -> f32 {
    const STOPS: [(f32, f32); 6] = [
        (0., 0.),
        (0.24, 0.45),
        (0.40, 1.),
        (0.60, 1.),
        (0.76, 0.45),
        (1., 0.),
    ];
    let position = position.clamp(0., 1.);
    for pair in STOPS.windows(2) {
        let (x0, a0) = pair[0];
        let (x1, a1) = pair[1];
        if position <= x1 {
            let t = if x1 > x0 {
                (position - x0) / (x1 - x0)
            } else {
                0.
            };
            return a0 + (a1 - a0) * t;
        }
    }
    0.
}

/// The band's phase on the shared clock for a loop of `duration` seconds.
pub fn shimmer_phase(ms: u128, duration: f64) -> f32 {
    if duration <= 0. {
        return 0.;
    }
    ((ms as f64 / (duration * 1000.)) % 1.) as f32
}
