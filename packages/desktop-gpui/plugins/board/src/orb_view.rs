//! `Orb` and `OrbAvatar` of `client/orb.tsx`: the thinking orb drawn on a GPUI `canvas` over
//! the running card's avatar. The canvas asks for the next frame from its paint, so a frame is
//! requested only while an orb is painted on screen.

use crate::avatar::avatar_image;
use crate::model::settings::{orb_preset, sphere_canvas};
use crate::model::{OrbSettings, OrbState};
use crate::orb::{frame, ink_gray, resolve_preset};
use bachuc_theme::{Scheme, scheme};
use gpui_kit::*;
use std::sync::OnceLock;
use std::time::Instant;

/// The avatar's share of the footprint; the orb's silhouette is its edge.
const AVATAR_SHARE: f32 = 0.9;
/// `filter: saturate(0.6)`: GPUI has no saturate filter, so a grayscale copy shows through the
/// color copy drawn at this opacity.
const SATURATION: f32 = 0.6;

/// Seconds on a clock that starts with the first frame, as `performance.now() / 1000`.
fn seconds() -> f64 {
    static START: OnceLock<Instant> = OnceLock::new();
    START.get_or_init(Instant::now).elapsed().as_secs_f64()
}

fn gray(white: f64, alpha: f64, dark: bool) -> Hsla {
    let level = f32::from(ink_gray(white, dark)) / 255.;
    hsla(0., 0., level, alpha as f32)
}

/// The dots and lines of the frame at `seconds`, offset from the canvas center.
struct Painted {
    dots: Vec<(f32, f32, f32, Hsla)>,
    lines: Vec<(f32, f32, f32, f32, f32, Hsla)>,
}

/// The frame of `state` drawn `canvas` px wide: the nearest preset, scaled about its center.
fn painted(state: OrbState, canvas: f32, dark: bool) -> Painted {
    let (size, scale) = orb_preset(canvas);
    let resolved = resolve_preset(state, size);
    let half = f64::from(size) / 2.;
    let scale = f64::from(scale);
    let at = |value: f64| ((value - half) * scale) as f32;
    let frame = frame(
        resolved.mode,
        f64::from(size),
        seconds() * resolved.speed,
        &resolved.opts,
    );
    Painted {
        dots: frame
            .dots
            .iter()
            .map(|dot| {
                (
                    at(dot.x),
                    at(dot.y),
                    (dot.r * scale) as f32,
                    gray(dot.white, dot.a, dark),
                )
            })
            .collect(),
        lines: frame
            .lines
            .iter()
            .map(|line| {
                (
                    at(line.x1),
                    at(line.y1),
                    at(line.x2),
                    at(line.y2),
                    (line.w * scale) as f32,
                    gray(line.white, line.a, dark),
                )
            })
            .collect(),
    }
}

/// Whether an orb's bounds show inside the visible area, so it may ask for the next frame.
pub(crate) fn in_view(bounds: Bounds<Pixels>, visible: Bounds<Pixels>) -> bool {
    bounds.intersects(&visible)
}

/// The orb of `state`, `size` px square, painted about the center of its bounds.
fn orb_canvas(state: OrbState, size_px: f32, dark: bool) -> AnyElement {
    let painted = painted(state, size_px, dark);
    canvas(
        |_, _, _| (),
        move |bounds, (), window, _cx| {
            // A list paints its overdraw too; an orb outside the visible area stays still.
            if !in_view(bounds, window.content_mask().bounds) {
                return;
            }
            let center = bounds.center();
            for &(x1, y1, x2, y2, width, color) in &painted.lines {
                let mut path = PathBuilder::stroke(px(width));
                path.move_to(point(center.x + px(x1), center.y + px(y1)));
                path.line_to(point(center.x + px(x2), center.y + px(y2)));
                if let Ok(path) = path.build() {
                    window.paint_path(path, color);
                }
            }
            for &(x, y, radius, color) in &painted.dots {
                let side = px(radius * 2.);
                window.paint_quad(quad(
                    Bounds::centered_at(
                        point(center.x + px(x), center.y + px(y)),
                        size(side, side),
                    ),
                    Corners::all(px(radius)),
                    color,
                    Edges::all(px(0.)),
                    transparent_black(),
                    BorderStyle::Solid,
                ));
            }
            // Only a painted orb asks for the next frame.
            window.request_animation_frame();
        },
    )
    .absolute()
    .size_full()
    .into_any_element()
}

/// `OrbAvatar`: the bot in a circle whose edge is the orb's silhouette, in the footprint of
/// the plain avatar.
pub(crate) fn orb_avatar(agent_id: &str, size: Pixels, orb: &OrbSettings, cx: &App) -> AnyElement {
    let footprint = f32::from(size);
    let avatar = footprint * AVATAR_SHARE;
    let canvas = sphere_canvas(avatar, orb.state);
    let dark = scheme(cx) == Scheme::Dark;
    let image = avatar_image(agent_id);
    div()
        .relative()
        .flex_none()
        .size(size)
        .flex()
        .items_center()
        .justify_center()
        .child(
            div()
                .relative()
                .size(px(avatar))
                .rounded_full()
                .overflow_hidden()
                .opacity(f32::from(orb.avatar_opacity) / 100.)
                .child(img(image.clone()).absolute().size_full().grayscale(true))
                .child(img(image).absolute().size_full().opacity(SATURATION)),
        )
        .child(
            div()
                .absolute()
                .left(px((footprint - canvas) / 2.))
                .top(px((footprint - canvas) / 2.))
                .size(px(canvas))
                .child(orb_canvas(orb.state, canvas, dark)),
        )
        .into_any_element()
}

#[cfg(test)]
mod tests {
    // Not a glob: `gpui_kit::*` brings GPUI's own `test` attribute.
    use super::in_view;
    use gpui_kit::{Bounds, point, px, size};

    #[test]
    fn only_an_orb_inside_the_visible_area_asks_for_frames() {
        let visible = Bounds::new(point(px(0.), px(0.)), size(px(400.), px(300.)));
        let orb = |y: f32| Bounds::new(point(px(10.), px(y)), size(px(36.), px(36.)));
        assert!(in_view(orb(100.), visible));
        assert!(in_view(orb(280.), visible));
        // The list's overdraw below the fold.
        assert!(!in_view(orb(320.), visible));
        assert!(!in_view(orb(-80.), visible));
    }
}
