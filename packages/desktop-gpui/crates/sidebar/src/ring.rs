//! The running mark: a status dot inside a ring with a quarter arc that turns, as
//! `components/status-ring` draws it. Sizes come from `status-ring/geometry.ts`.
//!
//! Each ring is its own view, so a turn redraws only the ring and not the whole list. It asks the
//! one app ticker (`bachuc_theme::motion`, ADR-0052) for a frame each time it paints, so it
//! redraws at the ticker rate while it is on screen and sleeps when it is scrolled out of view or
//! the window is hidden.

use bachuc_theme::motion::{motion_ms, request_tick};
use bachuc_theme::{tokens, zoomed};
use gpui_kit::*;
use std::f32::consts::PI;

/// `STATUS_INDICATOR_FILLED_DOT_SIZE`, `STATUS_RING_GAP`, `STATUS_RING_STROKE`, and
/// `STATUS_RING_HALO_INSET`.
const DOT: f32 = 6.;
const GAP: f32 = 1.5;
const STROKE: f32 = 1.5;
const HALO: f32 = 1.;
const RING: f32 = DOT + (GAP + STROKE) * 2.;
/// The frame is the knockout around the ring (`STATUS_RING_FRAME_SIZE`).
pub const FRAME: f32 = RING + HALO * 2.;
/// `STATUS_RING_TRACK_OPACITY` and `STATUS_RING_HEAD_OPACITY`.
const TRACK_OPACITY: f32 = 0.3;
const HEAD_OPACITY: f32 = 0.9;
/// `STATUS_RING_PERIOD_MS`: one turn. Every ring reads the shared motion clock, so they turn in
/// step with the loader and the pulse.
const PERIOD_MS: u128 = 900;
/// Points on the arc. The arc is 90 degrees of a 12 px circle.
const ARC_STEPS: usize = 10;

/// Where the ring sits. The fill comes from the token at paint, so a theme change follows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Knockout {
    /// Nothing behind the ring.
    None,
    /// On a project icon: the frame is filled with the sidebar color, so the ring reads as a hole
    /// punched in the icon.
    Sidebar,
    /// On `surface0`: a tab bar with no hover or active fill.
    Surface0,
    /// On `surface1`: a hovered tab or a resting subagent row.
    Surface1,
    /// On `surface2`: an active tab or a hovered subagent row.
    Surface2,
}

pub struct RunningRing {
    knockout: Knockout,
}

impl RunningRing {
    pub fn new(knockout: Knockout, _cx: &mut Context<Self>) -> Self {
        RunningRing { knockout }
    }
}

/// The turn of the arc in turns (0 to 1) from the shared clock. Reduced motion holds the arc
/// still.
fn phase(reduce_motion: bool) -> f32 {
    if reduce_motion {
        return 0.;
    }
    (motion_ms() % PERIOD_MS) as f32 / PERIOD_MS as f32
}

impl Render for RunningRing {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity_id();
        let knockout = self.knockout;
        canvas(
            |_, _, _| (),
            move |bounds, (), window, cx| {
                // A ring that is not painted asks for nothing, so the ticker sleeps with it.
                request_tick(view, cx);
                let t = tokens(cx);
                let color = t.dot_running;
                let center = bounds.center();
                let zoom = bachuc_theme::zoom(cx);
                let scaled = |length: f32| px(length * zoom);
                let square =
                    |side: f32| Bounds::centered_at(center, size(scaled(side), scaled(side)));
                let fill = match knockout {
                    Knockout::None => None,
                    Knockout::Sidebar => Some(t.surface_sidebar),
                    Knockout::Surface0 => Some(t.surface0),
                    Knockout::Surface1 => Some(t.surface1),
                    Knockout::Surface2 => Some(t.surface2),
                };
                if let Some(fill) = fill {
                    window.paint_quad(quad(
                        square(FRAME),
                        Corners::all(scaled(FRAME / 2.)),
                        fill,
                        Edges::all(px(0.)),
                        transparent_black(),
                        BorderStyle::Solid,
                    ));
                }
                window.paint_quad(quad(
                    square(RING),
                    Corners::all(scaled(RING / 2.)),
                    transparent_black(),
                    Edges::all(scaled(STROKE)),
                    color.opacity(TRACK_OPACITY),
                    BorderStyle::Solid,
                ));
                // The CSS arc is the top border of the ring: 90 degrees centered on the top,
                // turned clockwise by the phase.
                let radius = RING / 2. - STROKE / 2.;
                let start = -PI * 3. / 4. + phase(cx.reduce_motion()) * 2. * PI;
                let mut path = PathBuilder::stroke(scaled(STROKE));
                for step in 0..=ARC_STEPS {
                    let angle = start + (PI / 2.) * step as f32 / ARC_STEPS as f32;
                    let point = point(
                        center.x + scaled(radius * angle.cos()),
                        center.y + scaled(radius * angle.sin()),
                    );
                    if step == 0 {
                        path.move_to(point);
                    } else {
                        path.line_to(point);
                    }
                }
                if let Ok(path) = path.build() {
                    window.paint_path(path, color.opacity(HEAD_OPACITY));
                }
                window.paint_quad(quad(
                    square(DOT),
                    Corners::all(scaled(DOT / 2.)),
                    color,
                    Edges::all(px(0.)),
                    transparent_black(),
                    BorderStyle::Solid,
                ));
            },
        )
        .w(zoomed(FRAME, cx))
        .h(zoomed(FRAME, cx))
    }
}
