//! The stop button face (`client/stop-orb.tsx`): the solving orb in the stop button's red, on a
//! round button that gains the composer's icon-button background under the pointer. The orb
//! draws on a GPUI `canvas` and asks for the next frame only while it shows and the agent is not
//! stopping.

use crate::orb::{DOT_SIZE, Options, Orb, OrbClock, frame_rubik, ink_rgb, solving_orb};
use bachuc_plugin_host::StopButtonProps;
use bachuc_theme::{Scheme, scheme, tokens};
use gpui_kit::*;
use std::sync::OnceLock;
use std::time::Instant;

/// The red of Paseo's own stop button (palette red 600), the same in every theme: it has no
/// `bachuc_theme` token (`hiep-plugins/plugins/stop-orb/client/stop-orb.tsx:8`).
const STOP_RED: [u8; 3] = [0xdc, 0x26, 0x26];

/// Seconds on a clock that starts with the first frame, as `performance.now() / 1000`.
fn seconds() -> f64 {
    static START: OnceLock<Instant> = OnceLock::new();
    START.get_or_init(Instant::now).elapsed().as_secs_f64()
}

/// One dot on the button: its offset from the center, radius, and color.
struct Painted {
    dx: f32,
    dy: f32,
    radius: f32,
    color: Hsla,
}

pub(crate) struct OrbFace {
    size: Pixels,
    icon_size: Pixels,
    cancelling: bool,
    hovered: bool,
    orb: Orb,
    options: Options,
    clock: OrbClock,
}

impl OrbFace {
    pub(crate) fn new(props: &StopButtonProps) -> Self {
        let orb = solving_orb(f64::from(f32::from(props.icon_size)));
        Self {
            size: props.size,
            icon_size: props.icon_size,
            cancelling: props.cancelling,
            hovered: false,
            orb,
            options: Options::solving(orb.size, DOT_SIZE),
            clock: OrbClock::default(),
        }
    }

    /// Takes the props of the composer's render; redraws only when one changed.
    pub(crate) fn set_props(&mut self, props: &StopButtonProps, cx: &mut Context<Self>) {
        let changed = self.size != props.size
            || self.icon_size != props.icon_size
            || self.cancelling != props.cancelling;
        if !changed {
            return;
        }
        if self.icon_size != props.icon_size {
            self.orb = solving_orb(f64::from(f32::from(props.icon_size)));
            self.options = Options::solving(self.orb.size, DOT_SIZE);
        }
        self.size = props.size;
        self.icon_size = props.icon_size;
        self.cancelling = props.cancelling;
        cx.notify();
    }

    fn painted_dots(&mut self, dark: bool) -> Vec<Painted> {
        let t = self
            .clock
            .time(seconds(), self.options.speed, self.hovered, self.cancelling);
        let half = f64::from(self.orb.size) / 2.;
        let scale = self.orb.scale;
        frame_rubik(self.orb.size, t, &self.options)
            .into_iter()
            .map(|dot| {
                let [r, g, b] = ink_rgb(dot.white, dark, STOP_RED);
                Painted {
                    dx: ((dot.x - half) * scale) as f32,
                    dy: ((dot.y - half) * scale) as f32,
                    radius: (dot.r * scale) as f32,
                    color: Rgba {
                        r: f32::from(r) / 255.,
                        g: f32::from(g) / 255.,
                        b: f32::from(b) / 255.,
                        a: 1.,
                    }
                    .into(),
                }
            })
            .collect()
    }
}

impl Render for OrbFace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = tokens(cx);
        let dots = self.painted_dots(scheme(cx) == Scheme::Dark);
        if !self.cancelling {
            window.request_animation_frame();
        }
        // The hover background is the composer's icon-button one. Only the background and the
        // speed change on hover; the layout stays.
        let background = if self.hovered && !self.cancelling {
            colors.surface2
        } else {
            transparent_black()
        };
        div()
            .id("stop-orb-face")
            .relative()
            .size(self.size)
            .flex_none()
            .rounded_full()
            .bg(background)
            .on_hover(cx.listener(|this, hovered: &bool, _window, cx| {
                if this.hovered != *hovered {
                    this.hovered = *hovered;
                    cx.notify();
                }
            }))
            .child(
                canvas(
                    |_, _, _| (),
                    move |bounds, (), window, _cx| {
                        let center = bounds.center();
                        for dot in &dots {
                            let at = point(center.x + px(dot.dx), center.y + px(dot.dy));
                            let side = px(dot.radius * 2.);
                            window.paint_quad(quad(
                                Bounds::centered_at(at, size(side, side)),
                                Corners::all(px(dot.radius)),
                                dot.color,
                                Edges::all(px(0.)),
                                transparent_black(),
                                BorderStyle::Solid,
                            ));
                        }
                    },
                )
                .size_full(),
            )
    }
}
