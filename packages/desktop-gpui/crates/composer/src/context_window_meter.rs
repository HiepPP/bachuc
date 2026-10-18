//! `packages/app/src/components/context-window-meter.tsx`: the ring that shows how full the
//! context window is, with its tooltip. It sits in a 28 px slot before the primary button.

use bachuc_client::AgentUsage;
use bachuc_theme::metrics::{FONT_BASE, FONT_SM, SPACE_1_5};
use bachuc_theme::{tokens, ui_font_size, zoom, zoomed};
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::component::v_flex;
use gpui_kit::*;
use std::f32::consts::PI;

/// `palette.amber[500]` (`packages/app/src/styles/theme.ts` line 76); no token carries it.
pub const AMBER_500: u32 = 0xf59e0b;
/// `contextWindowMeterSlot`.
pub const SLOT: f32 = 28.;
/// `SVG_SIZE` and `STROKE_WIDTH`.
const GLYPH: f32 = 14.;
const STROKE: f32 = 2.;
/// Points on a full progress arc.
const ARC_STEPS: usize = 48;

/// `getUsagePercentage`: `None` for a missing or zero maximum.
pub fn usage_percent(usage: &AgentUsage) -> Option<f64> {
    let max = usage.context_window_max_tokens.filter(|max| *max > 0)?;
    let used = usage.context_window_used_tokens?;
    Some(used as f64 / max as f64 * 100.)
}

/// `clampPercentage`.
pub fn clamp_percent(percent: f64) -> f64 {
    percent.clamp(0., 100.)
}

/// `getMeterColors`: the progress color of a clamped percentage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MeterTone {
    Destructive,
    Amber,
    Muted,
}

pub fn meter_tone(clamped: f64) -> MeterTone {
    if clamped > 90. {
        MeterTone::Destructive
    } else if clamped >= 70. {
        MeterTone::Amber
    } else {
        MeterTone::Muted
    }
}

/// `formatSessionCost`: four decimals under one cent, else two; nothing for zero.
pub fn format_cost(value: f64) -> Option<String> {
    if !value.is_finite() || value <= 0. {
        None
    } else if value < 0.01 {
        Some(format!("${value:.4}"))
    } else {
        Some(format!("${value:.2}"))
    }
}

/// `formatTokenCount`.
pub fn format_token_count(value: u64) -> String {
    if value >= 1_000_000 {
        format!("{}m", (value as f64 / 1_000_000.).round())
    } else if value >= 1_000 {
        format!("{}k", (value as f64 / 1_000.).round())
    } else {
        value.to_string()
    }
}

/// What the slot draws.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Meter {
    Hidden,
    /// No usage yet while a turn runs: the track only, so the ring fades in without a shift.
    Pending,
    Shown {
        percent: f64,
    },
}

pub fn meter(usage: Option<&AgentUsage>, pending: bool) -> Meter {
    match usage.and_then(usage_percent) {
        Some(percent) => Meter::Shown { percent },
        None if pending => Meter::Pending,
        None => Meter::Hidden,
    }
}

/// The ring in its 28 px slot; `None` hides the slot's content.
pub fn context_window_meter(
    usage: Option<AgentUsage>,
    pending: bool,
    cx: &App,
) -> Option<AnyElement> {
    let t = tokens(cx);
    let track = t.surface3;
    // The paint closure has no `App`, so it takes the zoom factor.
    let zoom = zoom(cx);
    let (percent, progress) = match meter(usage.as_ref(), pending) {
        Meter::Hidden => return None,
        Meter::Pending => (None, track),
        Meter::Shown { percent } => {
            let clamped = clamp_percent(percent);
            let color = match meter_tone(clamped) {
                MeterTone::Destructive => t.destructive,
                MeterTone::Amber => rgb(AMBER_500).into(),
                MeterTone::Muted => t.foreground_muted,
            };
            (Some(percent), color)
        }
    };
    let ring = canvas(
        |_, _, _| (),
        move |bounds, (), window, _| {
            let center = bounds.center();
            let side = px((GLYPH - STROKE) * zoom);
            window.paint_quad(quad(
                Bounds::centered_at(center, size(px(GLYPH * zoom), px(GLYPH * zoom))),
                Corners::all(px(GLYPH / 2. * zoom)),
                transparent_black(),
                Edges::all(px(STROKE * zoom)),
                track,
                BorderStyle::Solid,
            ));
            let Some(percent) = percent else {
                return;
            };
            let sweep = clamp_percent(percent) as f32 / 100. * 2. * PI;
            if sweep <= 0. {
                return;
            }
            // The SVG turns -90 degrees, so the arc starts at the top and runs clockwise.
            let radius = f32::from(side) / 2.;
            let steps = ((ARC_STEPS as f32 * sweep / (2. * PI)).ceil() as usize).max(1);
            let mut path = PathBuilder::stroke(px(STROKE * zoom));
            for step in 0..=steps {
                let angle = -PI / 2. + sweep * step as f32 / steps as f32;
                let point = point(
                    center.x + px(radius * angle.cos()),
                    center.y + px(radius * angle.sin()),
                );
                if step == 0 {
                    path.move_to(point);
                } else {
                    path.line_to(point);
                }
            }
            if let Ok(path) = path.build() {
                window.paint_path(path, progress);
            }
        },
    )
    .size(zoomed(GLYPH, cx));
    let Some(percent) = percent else {
        return Some(ring.into_any_element());
    };
    let usage = usage.unwrap_or_default();
    let rounded = percent.round() as i64;
    let used = format_token_count(usage.context_window_used_tokens.unwrap_or_default());
    let max = format_token_count(usage.context_window_max_tokens.unwrap_or_default());
    let cost = usage.total_cost_usd.and_then(format_cost);
    Some(
        div()
            .id("context-window-meter")
            .role(Role::Image)
            .aria_label(format!("Context window {rounded}% used"))
            .size(zoomed(SLOT, cx))
            .flex()
            .items_center()
            .justify_center()
            .rounded_full()
            .tooltip(move |window, cx| {
                let (used, max, cost) = (used.clone(), max.clone(), cost.clone());
                Tooltip::element(move |_, cx| {
                    let t = tokens(cx);
                    let base = ui_font_size(FONT_BASE, cx);
                    let small = ui_font_size(FONT_SM, cx);
                    v_flex()
                        .gap(zoomed(SPACE_1_5, cx))
                        .min_w(zoomed(200., cx))
                        .child(
                            div()
                                .text_size(base)
                                .text_color(t.foreground)
                                .child("Context window"),
                        )
                        .child(
                            div()
                                .text_size(base)
                                .text_color(t.foreground)
                                .child(format!("{rounded}% used")),
                        )
                        .child(
                            div()
                                .text_size(small)
                                .text_color(t.foreground_muted)
                                .child(format!("{used} / {max} tokens")),
                        )
                        .children(cost.clone().map(|cost| {
                            div()
                                .text_size(small)
                                .text_color(t.foreground_muted)
                                .child(format!("Session cost {cost}"))
                        }))
                })
                .build(window, cx)
            })
            .child(ring)
            .into_any_element(),
    )
}

#[cfg(test)]
mod tests {
    // Named imports: `use super::*` would bring in GPUI's `test` macro over `#[test]`.
    use super::{
        AMBER_500, Meter, MeterTone, clamp_percent, format_cost, format_token_count, meter,
        meter_tone,
    };
    use bachuc_client::AgentUsage;

    fn usage(max: Option<u64>, used: Option<u64>) -> AgentUsage {
        AgentUsage {
            context_window_max_tokens: max,
            context_window_used_tokens: used,
            ..AgentUsage::default()
        }
    }

    #[test]
    fn no_context_window_data_draws_no_meter() {
        assert_eq!(meter(Some(&usage(None, Some(10))), false), Meter::Hidden);
        assert_eq!(meter(Some(&usage(Some(0), Some(10))), false), Meter::Hidden);
        assert_eq!(meter(None, false), Meter::Hidden);
        assert_eq!(meter(None, true), Meter::Pending);
        assert_eq!(
            meter(Some(&usage(Some(200), Some(50))), false),
            Meter::Shown { percent: 25. }
        );
    }

    #[test]
    fn the_percent_clamps_to_the_ring() {
        assert_eq!(clamp_percent(130.), 100.);
        assert_eq!(clamp_percent(-5.), 0.);
        assert_eq!(clamp_percent(42.5), 42.5);
    }

    #[test]
    fn colors_turn_amber_at_70_and_destructive_over_90() {
        assert_eq!(meter_tone(91.), MeterTone::Destructive);
        assert_eq!(meter_tone(90.), MeterTone::Amber);
        assert_eq!(meter_tone(70.), MeterTone::Amber);
        assert_eq!(meter_tone(69.9), MeterTone::Muted);
        assert_eq!(AMBER_500, 0xf59e0b);
    }

    #[test]
    fn cost_has_four_decimals_under_a_cent() {
        assert_eq!(format_cost(0.0042).as_deref(), Some("$0.0042"));
        assert_eq!(format_cost(1.234).as_deref(), Some("$1.23"));
        assert_eq!(format_cost(0.), None);
        assert_eq!(format_cost(f64::NAN), None);
    }

    #[test]
    fn token_counts_round_to_k_and_m() {
        assert_eq!(format_token_count(950), "950");
        assert_eq!(format_token_count(12_400), "12k");
        assert_eq!(format_token_count(200_000), "200k");
        assert_eq!(format_token_count(1_600_000), "2m");
    }
}
