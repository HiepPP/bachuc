//! Bachuc theme for the GPUI client: the Electron design tokens for `light` and the `claude`
//! dark theme (ADR-0043), and the turn step list colors Explore, Edit, and Verify (ADR-0020,
//! ADR-0022). The label formula is a port of `packages/app/src/styles/turn-label-colors.ts`.
//!
//! Views read colors with [`tokens`] and sizes from [`metrics`]. [`init`] and [`set_theme`] also
//! write the tokens into the gpui-kit theme, so gpui-kit components use the same colors.

pub mod metrics;
mod tokens;

pub use tokens::Tokens;

use gpui_kit::component::theme::{Theme, ThemeMode};
use gpui_kit::{App, BoxShadow, Global, Hsla, Pixels, Rgba, Window, point, px};

/// Which theme the app shows. `System` follows the system appearance: `Light` when it is light
/// and `ClaudeDark` when it is dark.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ThemeChoice {
    Light,
    ClaudeDark,
    #[default]
    System,
}

struct State {
    choice: ThemeChoice,
    interface_scale: f32,
    code_scale: f32,
    light: Tokens,
    dark: Tokens,
}

impl Global for State {}

fn state(cx: &mut App) -> &mut State {
    if !cx.has_global::<State>() {
        cx.set_global(State {
            choice: ThemeChoice::System,
            interface_scale: 1.,
            code_scale: 1.,
            light: Tokens::light(),
            dark: Tokens::claude_dark(),
        });
    }
    cx.global_mut::<State>()
}

/// The tokens of the theme on screen.
pub fn tokens(cx: &App) -> Tokens {
    match (cx.try_global::<State>(), scheme(cx)) {
        (Some(state), Scheme::Light) => state.light,
        (Some(state), Scheme::Dark) => state.dark,
        (None, Scheme::Light) => Tokens::light(),
        (None, Scheme::Dark) => Tokens::claude_dark(),
    }
}

/// Switches the theme and writes its tokens into the gpui-kit theme.
pub fn set_theme(choice: ThemeChoice, cx: &mut App) {
    state(cx).choice = choice;
    resync(None, cx);
}

/// Sets the Appearance font sizes as factors of the defaults: 1.0 is 14 px for the interface
/// and 12 px for code. They scale [`ui_font_size`], [`code_font_size`], and the gpui-kit base
/// sizes.
pub fn set_font_scale(interface: f32, code: f32, cx: &mut App) {
    let state = state(cx);
    state.interface_scale = interface;
    state.code_scale = code;
    apply(cx);
}

/// A `metrics::FONT_*` interface size with the Interface size setting applied.
pub fn ui_font_size(base: f32, cx: &App) -> Pixels {
    px(base
        * cx.try_global::<State>()
            .map_or(1., |state| state.interface_scale))
}

/// A `metrics::FONT_CODE` code size with the Code size setting applied.
pub fn code_font_size(base: f32, cx: &App) -> Pixels {
    px(base
        * cx.try_global::<State>()
            .map_or(1., |state| state.code_scale))
}

/// The monospace family: SF Mono when it is installed, else Menlo.
pub fn mono_font_family(cx: &App) -> &'static str {
    let installed = cx.text_system().all_font_names();
    ["SFMono-Regular", "SF Mono"]
        .into_iter()
        .find(|name| installed.iter().any(|font| font == name))
        .unwrap_or("Menlo")
}

/// `sm`, `md`, or `lg` from the `shadow` object of the theme on screen.
pub fn shadow(size: ShadowSize, cx: &App) -> BoxShadow {
    let recipes = match scheme(cx) {
        Scheme::Light => metrics::LIGHT_SHADOWS,
        Scheme::Dark => metrics::DARK_SHADOWS,
    };
    let recipe = recipes[size as usize];
    BoxShadow {
        color: Rgba {
            r: 0.,
            g: 0.,
            b: 0.,
            a: recipe.alpha,
        }
        .into(),
        offset: point(px(0.), px(recipe.offset_y)),
        blur_radius: px(recipe.blur),
        spread_radius: px(0.),
        inset: false,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShadowSize {
    Sm,
    Md,
    Lg,
}

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

/// Applies the Bachuc theme with the `System` choice, and follows the system appearance when a
/// window's appearance changes. Call it after `gpui_kit::init`. A later [`set_theme`] replaces
/// the choice.
pub fn init(cx: &mut App) {
    state(cx).choice = ThemeChoice::System;
    resync(None, cx);
    // An appearance observer needs a window, so attach one to each window's root as it opens.
    cx.observe_new(
        |_: &mut gpui_kit::base::Root, window: Option<&mut Window>, _| {
            if let Some(window) = window {
                window
                    .observe_window_appearance(|window, cx| {
                        if state(cx).choice == ThemeChoice::System {
                            resync(Some(window), cx);
                        }
                    })
                    .detach();
            }
        },
    )
    .detach();
}

/// Picks the gpui-kit mode for the choice, then writes the tokens over the colors that the mode
/// loaded.
fn resync(window: Option<&mut Window>, cx: &mut App) {
    match state(cx).choice {
        ThemeChoice::Light => Theme::change(ThemeMode::Light, None, cx),
        ThemeChoice::ClaudeDark => Theme::change(ThemeMode::Dark, None, cx),
        ThemeChoice::System => Theme::sync_system_appearance(window, cx),
    }
    apply(cx);
}

/// Writes the tokens, radii, and font sizes into the gpui-kit theme.
fn apply(cx: &mut App) {
    let t = tokens(cx);
    let interface = ui_font_size(metrics::FONT_BASE, cx);
    let code = code_font_size(metrics::FONT_CODE, cx);
    let mono = mono_font_family(cx);
    let clear = gpui_kit::transparent_black();
    Theme::update(cx, |theme| {
        theme.radius = px(metrics::RADIUS_MD);
        theme.radius_lg = px(metrics::RADIUS_LG);
        theme.font_size = interface;
        theme.mono_font_size = code;
        theme.mono_font_family = mono.into();

        let c = &mut theme.colors;
        c.background = t.surface0;
        c.foreground = t.foreground;
        c.border = t.border;
        c.input = t.border;
        c.ring = t.ring;
        c.caret = t.foreground;
        c.selection = t.terminal_selection;
        c.overlay = Rgba {
            r: 0.,
            g: 0.,
            b: 0.,
            a: 0.4,
        }
        .into();

        c.muted = t.surface2;
        c.muted_foreground = t.foreground_muted;
        c.accent = t.surface2;
        c.accent_foreground = t.foreground;
        c.popover = t.popover;
        c.popover_foreground = t.foreground;
        c.link = t.accent;
        c.link_hover = t.accent_bright;
        c.link_active = t.accent;

        // The default button is the one CTA, filled with `accent` (docs/design.md).
        c.primary = t.accent;
        c.primary_hover = t.accent_bright;
        c.primary_active = t.accent;
        c.primary_foreground = t.accent_foreground;
        c.secondary = t.surface2;
        c.secondary_hover = t.surface3;
        c.secondary_active = t.surface3;
        c.secondary_foreground = t.foreground;
        c.button = t.surface2;
        c.button_hover = t.surface3;
        c.button_active = t.surface3;
        c.button_foreground = t.foreground;
        c.danger = t.destructive;
        c.danger_hover = t.destructive;
        c.danger_active = t.destructive;
        c.danger_foreground = t.destructive_foreground;
        c.success = t.status_success;
        c.success_hover = t.status_success;
        c.success_active = t.status_success;
        c.success_foreground = t.destructive_foreground;
        c.warning = t.status_warning;
        c.warning_hover = t.status_warning;
        c.warning_active = t.status_warning;
        c.warning_foreground = t.destructive_foreground;
        c.info = t.dot_running;
        c.info_hover = t.dot_running;
        c.info_active = t.dot_running;
        c.info_foreground = t.destructive_foreground;

        c.sidebar = t.surface_sidebar;
        c.sidebar_foreground = t.foreground;
        c.sidebar_border = t.border;
        c.sidebar_accent = t.surface_sidebar_selected;
        c.sidebar_accent_foreground = t.foreground;
        c.sidebar_primary = t.accent;
        c.sidebar_primary_foreground = t.accent_foreground;

        c.list = t.surface0;
        c.list_even = t.surface0;
        c.list_head = t.surface1;
        c.list_hover = t.interaction_highlight;
        c.list_active = t.surface_sidebar_selected;
        c.list_active_border = t.border;

        c.table = t.surface0;
        c.table_even = t.surface0;
        c.table_head = t.surface1;
        c.table_head_foreground = t.foreground_muted;
        c.table_foot = t.surface1;
        c.table_foot_foreground = t.foreground_muted;
        c.table_hover = t.interaction_highlight;
        c.table_active = t.surface_sidebar_selected;
        c.table_active_border = t.border;
        c.table_row_border = t.border;

        c.tab = clear;
        c.tab_bar = t.surface_workspace;
        c.tab_bar_segmented = t.surface2;
        c.tab_active = t.surface_workspace;
        c.tab_foreground = t.foreground_muted;
        c.tab_active_foreground = t.foreground;
        c.title_bar = t.surface_sidebar;
        c.title_bar_border = t.border;
        c.status_bar = t.surface_sidebar;
        c.status_bar_border = t.border;
        c.window_border = t.border;

        c.scrollbar = clear;
        c.scrollbar_thumb = t.surface4;
        c.scrollbar_thumb_hover = t.foreground_extra_muted;
        c.switch = t.surface3;
        c.switch_thumb = t.surface0;
        c.progress_bar = t.accent;
        c.slider_bar = t.accent;
        c.slider_thumb = t.surface0;
        c.skeleton = t.surface2;
        c.drag_border = t.accent;
        c.group_box = t.surface1;
        c.group_box_foreground = t.foreground;
        c.accordion = t.surface0;
    });
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
