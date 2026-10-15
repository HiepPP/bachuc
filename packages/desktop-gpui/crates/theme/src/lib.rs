//! Bachuc theme for the GPUI client: light and dark follow the system appearance, and the
//! turn step list gets its Explore, Edit, and Verify colors (ADR-0020, ADR-0022).
//! The label formula is a port of `packages/app/src/styles/turn-label-colors.ts`.

use gpui_kit::component::theme::Theme;
use gpui_kit::{App, Hsla, Rgba, Window};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scheme {
    Light,
    Dark,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TurnLabel {
    Explore,
    Edit,
    Verify,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TurnLabelColors {
    /// Strip segment. Not text, so it is the same in both schemes.
    pub segment: Hsla,
    /// Label text and number.
    pub text: Hsla,
    /// Number border.
    pub border: Hsla,
}

/// Applies gpui-kit's light or dark theme from the system appearance, and switches it when the
/// appearance of a window changes. Call it after `gpui_kit::init`.
pub fn init(cx: &mut App) {
    Theme::sync_system_appearance(None, cx);
    // An appearance observer needs a window, so attach one to each window's root as it opens.
    cx.observe_new(
        |_: &mut gpui_kit::base::Root, window: Option<&mut Window>, _| {
            if let Some(window) = window {
                window
                    .observe_window_appearance(|window, cx| {
                        Theme::sync_system_appearance(Some(window), cx)
                    })
                    .detach();
            }
        },
    )
    .detach();
}

pub fn scheme(cx: &App) -> Scheme {
    if Theme::global(cx).is_dark() {
        Scheme::Dark
    } else {
        Scheme::Light
    }
}

fn hue(label: TurnLabel) -> f64 {
    match label {
        TurnLabel::Explore => 155.,
        TurnLabel::Edit => 55.,
        TurnLabel::Verify => 300.,
    }
}

/// Text and border tone (oklch lightness, chroma) per scheme. Only lightness moves; the hue stays.
fn tones(scheme: Scheme) -> ((f64, f64), (f64, f64)) {
    match scheme {
        Scheme::Light => ((0.52, 0.11), (0.8, 0.06)),
        Scheme::Dark => ((0.75, 0.1), (0.45, 0.07)),
    }
}

/// oklch to 8-bit sRGB channels, rounded as the app rounds them.
fn oklch_to_rgb8(lightness: f64, chroma: f64, hue: f64) -> [u8; 3] {
    let radians = hue.to_radians();
    let a = chroma * radians.cos();
    let b = chroma * radians.sin();
    let l = (lightness + 0.3963377774 * a + 0.2158037573 * b).powi(3);
    let m = (lightness - 0.1055613458 * a - 0.0638541728 * b).powi(3);
    let s = (lightness - 0.0894841775 * a - 1.291485548 * b).powi(3);
    let linear = [
        4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s,
        -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s,
        -0.0041960863 * l - 0.7034186147 * m + 1.707614701 * s,
    ];
    linear.map(|value| {
        let clamped = value.clamp(0., 1.);
        let encoded = if clamped <= 0.0031308 {
            12.92 * clamped
        } else {
            1.055 * clamped.powf(1. / 2.4) - 0.055
        };
        (encoded * 255.).round() as u8
    })
}

fn oklch(lightness: f64, chroma: f64, hue: f64) -> Hsla {
    let [r, g, b] = oklch_to_rgb8(lightness, chroma, hue);
    Rgba {
        r: r as f32 / 255.,
        g: g as f32 / 255.,
        b: b as f32 / 255.,
        a: 1.,
    }
    .into()
}

pub fn turn_label_colors(label: TurnLabel, scheme: Scheme) -> TurnLabelColors {
    let hue = hue(label);
    let (text, border) = tones(scheme);
    TurnLabelColors {
        segment: oklch(0.74, 0.09, hue),
        text: oklch(text.0, text.1, hue),
        border: oklch(border.0, border.1, hue),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// rgb values that `turnLabelColors` in `packages/app/src/styles/turn-label-colors.ts`
    /// returned on 2026-10-15: (scheme, label, segment, text, border).
    const APP_COLORS: [(Scheme, TurnLabel, [u8; 3], [u8; 3], [u8; 3]); 6] = [
        (
            Scheme::Light,
            TurnLabel::Explore,
            [124, 188, 146],
            [38, 123, 76],
            [160, 202, 173],
        ),
        (
            Scheme::Light,
            TurnLabel::Edit,
            [215, 156, 115],
            [152, 85, 33],
            [221, 180, 153],
        ),
        (
            Scheme::Light,
            TurnLabel::Verify,
            [179, 158, 220],
            [115, 89, 158],
            [195, 182, 223],
        ),
        (
            Scheme::Dark,
            TurnLabel::Explore,
            [124, 188, 146],
            [120, 193, 146],
            [51, 97, 68],
        ),
        (
            Scheme::Dark,
            TurnLabel::Edit,
            [215, 156, 115],
            [223, 157, 111],
            [115, 74, 46],
        ),
        (
            Scheme::Dark,
            TurnLabel::Verify,
            [179, 158, 220],
            [183, 160, 228],
            [91, 76, 118],
        ),
    ];

    fn assert_close(actual: Hsla, expected: [u8; 3], what: &str) {
        let rgba = actual.to_rgb();
        let channels = [rgba.r, rgba.g, rgba.b].map(|c| c * 255.);
        for (got, want) in channels.iter().zip(expected) {
            assert!(
                (got - want as f32).abs() <= 1.,
                "{what}: got {channels:?}, want {expected:?}"
            );
        }
    }

    #[test]
    fn turn_label_colors_match_the_app() {
        for (scheme, label, segment, text, border) in APP_COLORS {
            let colors = turn_label_colors(label, scheme);
            assert_close(
                colors.segment,
                segment,
                &format!("{scheme:?} {label:?} segment"),
            );
            assert_close(colors.text, text, &format!("{scheme:?} {label:?} text"));
            assert_close(
                colors.border,
                border,
                &format!("{scheme:?} {label:?} border"),
            );
        }
    }
}
