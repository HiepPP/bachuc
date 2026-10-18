//! Bachuc theme for the GPUI client: the Electron design tokens for `light` and the `claude`
//! dark theme (ADR-0043), and the turn step list colors Explore, Edit, and Verify (ADR-0020,
//! ADR-0022). The label formula is a port of `packages/app/src/styles/turn-label-colors.ts`.
//!
//! Views read colors with [`tokens`] and sizes from [`metrics`]. [`init`] and [`set_theme`] also
//! write the tokens into the gpui-kit theme, so gpui-kit components use the same colors.

mod custom;
pub mod metrics;
mod syntax;
mod tokens;

pub use custom::{CustomTheme, CustomThemeColors, active_custom, next_builtin_choice};
pub use syntax::{ROLES as SYNTAX_ROLES, SyntaxColors, SyntaxThemeId, syntax_colors_for};
pub use tokens::Tokens;

use custom::CustomState;
use gpui_kit::component::theme::{Theme, ThemeMode};
use gpui_kit::{App, BoxShadow, Global, Hsla, Pixels, Rgba, SharedString, Window, point, px};

/// The Electron desktop default of `contentFontSize` (`FONT_SIZE.content`).
pub const DEFAULT_CONTENT_FONT_SIZE: f32 = metrics::FONT_CONTENT;
/// `MIN_CONTENT_FONT_SIZE` and `MAX_CONTENT_FONT_SIZE` in `use-settings/storage.ts`.
pub const MIN_CONTENT_FONT_SIZE: f32 = 10.;
pub const MAX_CONTENT_FONT_SIZE: f32 = 21.;

/// The family gpui-kit uses for the interface when no family is set.
const SYSTEM_UI_FONT: &str = ".SystemUIFont";

/// Which theme the app shows. `System` follows the system appearance: `Light` when it is light
/// and `ClaudeDark` when it is dark.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ThemeChoice {
    Light,
    ClaudeDark,
    #[default]
    System,
}

/// Which end of a tab chip shows its close button. `Left` shows it in place of the tab icon
/// on hover; `Right` shows it on an overlay with a scrim at the right end.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TabClosePosition {
    #[default]
    Left,
    Right,
}

struct State {
    choice: ThemeChoice,
    interface_scale: f32,
    code_scale: f32,
    /// The whole-UI zoom factor, 1.0 at 100%.
    zoom: f32,
    tab_close: TabClosePosition,
    /// Installed families only; `None` is the default family.
    ui_family: Option<SharedString>,
    code_family: Option<SharedString>,
    content_size: f32,
    syntax: SyntaxThemeId,
    light: Tokens,
    dark: Tokens,
    /// Plugin themes. The active one wins over `choice`.
    custom: CustomState,
}

impl Global for State {}

fn state(cx: &mut App) -> &mut State {
    if !cx.has_global::<State>() {
        cx.set_global(State {
            choice: ThemeChoice::System,
            interface_scale: 1.,
            code_scale: 1.,
            zoom: 1.,
            tab_close: TabClosePosition::Left,
            ui_family: None,
            code_family: None,
            content_size: DEFAULT_CONTENT_FONT_SIZE,
            syntax: SyntaxThemeId::default(),
            light: Tokens::light(),
            dark: Tokens::claude_dark(),
            custom: CustomState::default(),
        });
    }
    cx.global_mut::<State>()
}

/// The tokens of the theme on screen.
pub fn tokens(cx: &App) -> Tokens {
    if let Some(active) = cx
        .try_global::<State>()
        .and_then(|state| state.custom.active())
    {
        return active.tokens;
    }
    match (cx.try_global::<State>(), scheme(cx)) {
        (Some(state), Scheme::Light) => state.light,
        (Some(state), Scheme::Dark) => state.dark,
        (None, Scheme::Light) => Tokens::light(),
        (None, Scheme::Dark) => Tokens::claude_dark(),
    }
}

/// Switches the theme and writes its tokens into the gpui-kit theme. A selected plugin theme
/// still wins over the choice.
pub fn set_theme(choice: ThemeChoice, cx: &mut App) {
    state(cx).choice = choice;
    resync(None, cx);
}

/// The cycle-theme shortcut: goes to the next built-in theme and leaves the selected plugin
/// theme, if any. Returns the new choice.
pub fn cycle_builtin_theme(cx: &mut App) -> ThemeChoice {
    let state = state(cx);
    state.choice = custom::cycle_builtin(state.choice, &mut state.custom);
    let choice = state.choice;
    resync(None, cx);
    choice
}

/// Replaces the plugin theme list. The theme on screen is written again only when the active
/// plugin theme or its colors changed.
pub fn set_custom_themes(themes: Vec<CustomTheme>, cx: &mut App) {
    if state(cx).custom.set_themes(themes) {
        resync(None, cx);
    }
}

/// The plugin theme list.
pub fn custom_themes(cx: &App) -> Vec<CustomTheme> {
    cx.try_global::<State>()
        .map(|state| state.custom.themes().to_vec())
        .unwrap_or_default()
}

/// Stores the selected plugin theme id. `None` returns to the [`ThemeChoice`]. An id that the
/// list does not hold yet stays stored, and applies when the list holds it.
pub fn set_custom_theme(id: Option<SharedString>, cx: &mut App) {
    if state(cx).custom.select(id) {
        resync(None, cx);
    }
}

/// The stored plugin theme id, whether or not the list holds it.
pub fn custom_theme(cx: &App) -> Option<SharedString> {
    cx.try_global::<State>()
        .and_then(|state| state.custom.selected().cloned())
}

/// The selected plugin theme when the list holds it.
pub fn active_custom_theme(cx: &App) -> Option<CustomTheme> {
    cx.try_global::<State>()
        .and_then(|state| state.custom.active())
        .map(|active| active.theme.clone())
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

/// A `metrics::FONT_*` interface size with the Interface size setting and the zoom applied.
pub fn ui_font_size(base: f32, cx: &App) -> Pixels {
    px(base
        * cx.try_global::<State>()
            .map_or(1., |state| state.interface_scale * state.zoom))
}

/// A `metrics::FONT_CODE` code size with the Code size setting and the zoom applied.
pub fn code_font_size(base: f32, cx: &App) -> Pixels {
    px(base
        * cx.try_global::<State>()
            .map_or(1., |state| state.code_scale * state.zoom))
}

/// GPUI's default rem size, which gpui-kit sizes (`h_8`, `text_sm`, paddings) are made of.
const BASE_REM_SIZE: f32 = 16.;

/// Sets the whole-UI zoom factor, 1.0 at 100%. Text from [`ui_font_size`], [`code_font_size`],
/// and [`content_font_size`], sizes from [`zoomed`], and the rem size from [`rem_size`] follow
/// it. A factor that is not finite or not above 0 selects 1.0. Nothing is persisted.
pub fn set_zoom(factor: f32, cx: &mut App) {
    let factor = if factor.is_finite() && factor > 0. {
        factor
    } else {
        1.
    };
    state(cx).zoom = factor;
    apply(cx);
    cx.refresh_windows();
}

/// The whole-UI zoom factor, 1.0 at 100%.
pub fn zoom(cx: &App) -> f32 {
    cx.try_global::<State>().map_or(1., |state| state.zoom)
}

/// A fixed `metrics` size (a padding, a height, an icon) with the zoom applied.
pub fn zoomed(base: f32, cx: &App) -> Pixels {
    px(base * zoom(cx))
}

/// The distance from the top of a trigger to the top of the menu under it: the trigger height
/// plus [`metrics::POPUP_GAP`]. Place the menu there and add no margin of its own, or the gap
/// doubles.
pub fn popup_offset(trigger_height: f32) -> f32 {
    trigger_height + metrics::POPUP_GAP
}

/// The rem size for the window, with the zoom applied. The root view sets it on each draw
/// with `Window::set_rem_size`, which only stores the value.
pub fn rem_size(cx: &App) -> Pixels {
    zoomed(BASE_REM_SIZE, cx)
}

/// Sets the side of the tab close button. Nothing is persisted.
pub fn set_tab_close_position(position: TabClosePosition, cx: &mut App) {
    if state(cx).tab_close != position {
        state(cx).tab_close = position;
        cx.refresh_windows();
    }
}

/// The side of the tab close button; `Left` until [`set_tab_close_position`] runs.
pub fn tab_close_position(cx: &App) -> TabClosePosition {
    cx.try_global::<State>()
        .map_or_else(TabClosePosition::default, |state| state.tab_close)
}

/// The monospace family: SF Mono when it is installed, else Menlo. Views call this on every
/// draw, and `all_font_names` builds a CoreText collection of every family each time, so the
/// pick is made once per process.
pub fn mono_font_family(cx: &App) -> &'static str {
    static MONO: std::sync::OnceLock<&'static str> = std::sync::OnceLock::new();
    MONO.get_or_init(|| pick_mono_family(&cx.text_system().all_font_names()))
}

/// The first SF Mono name in `installed`, else Menlo.
fn pick_mono_family(installed: &[String]) -> &'static str {
    ["SFMono-Regular", "SF Mono"]
        .into_iter()
        .find(|name| installed.iter().any(|font| font == name))
        .unwrap_or("Menlo")
}

/// Sets the interface and code font families. `None`, an empty name, or a family that is not
/// installed selects the default: the system font for the interface and [`mono_font_family`]
/// for code. Nothing is persisted.
pub fn set_font_families(ui: Option<SharedString>, code: Option<SharedString>, cx: &mut App) {
    let installed = cx.text_system().all_font_names();
    let state = state(cx);
    state.ui_family = installed_family(ui, &installed);
    state.code_family = installed_family(code, &installed);
    apply(cx);
}

/// The interface family: the one set with [`set_font_families`], else the system font.
pub fn ui_font_family(cx: &App) -> SharedString {
    cx.try_global::<State>()
        .and_then(|state| state.ui_family.clone())
        .unwrap_or_else(|| SYSTEM_UI_FONT.into())
}

/// The code family: the one set with [`set_font_families`], else [`mono_font_family`].
pub fn code_font_family(cx: &App) -> SharedString {
    cx.try_global::<State>()
        .and_then(|state| state.code_family.clone())
        .unwrap_or_else(|| mono_font_family(cx).into())
}

/// The families that a picker can offer, sorted and without duplicates or hidden system names.
pub fn installed_font_families(cx: &App) -> Vec<SharedString> {
    let mut names: Vec<String> = cx
        .text_system()
        .all_font_names()
        .into_iter()
        .filter(|name| !name.starts_with('.'))
        .collect();
    names.sort_unstable_by_key(|name| name.to_lowercase());
    names.dedup();
    names.into_iter().map(SharedString::from).collect()
}

/// `requested` when it names an installed family, else `None` (the default).
fn installed_family(requested: Option<SharedString>, installed: &[String]) -> Option<SharedString> {
    let requested = requested?;
    let name = requested.trim();
    installed
        .iter()
        .any(|font| font == name)
        .then(|| name.to_string().into())
}

/// Sets the content font size in px, as `contentFontSize` does: the whole number below it,
/// clamped to 10-21. A value that is not finite selects the default. Nothing is persisted.
pub fn set_content_font_size(px: f32, cx: &mut App) {
    state(cx).content_size = clamp_content_font_size(px);
    apply(cx);
}

/// The content font size for message bodies, the composer input, and Markdown, with the zoom
/// applied.
pub fn content_font_size(cx: &App) -> Pixels {
    px(cx
        .try_global::<State>()
        .map_or(DEFAULT_CONTENT_FONT_SIZE, |state| {
            state.content_size * state.zoom
        }))
}

fn clamp_content_font_size(size: f32) -> f32 {
    if size.is_finite() {
        size.floor()
            .clamp(MIN_CONTENT_FONT_SIZE, MAX_CONTENT_FONT_SIZE)
    } else {
        DEFAULT_CONTENT_FONT_SIZE
    }
}

/// Sets the syntax theme. It follows the light or dark app theme through [`syntax_colors`].
/// Nothing is persisted.
pub fn set_syntax_theme(id: SyntaxThemeId, cx: &mut App) {
    state(cx).syntax = id;
    apply(cx);
}

/// The syntax theme on screen; `One` until [`set_syntax_theme`] runs.
pub fn syntax_theme(cx: &App) -> SyntaxThemeId {
    cx.try_global::<State>()
        .map_or_else(SyntaxThemeId::default, |state| state.syntax)
}

/// The syntax palette of [`syntax_theme`] for the app theme on screen.
pub fn syntax_colors(cx: &App) -> SyntaxColors {
    syntax_colors_for(syntax_theme(cx), scheme(cx))
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
                        let state = state(cx);
                        if state.choice == ThemeChoice::System && state.custom.active().is_none() {
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
    let state = state(cx);
    let custom_scheme = state.custom.active().map(|active| active.theme.scheme);
    match (custom_scheme, state.choice) {
        (Some(Scheme::Light), _) | (None, ThemeChoice::Light) => {
            Theme::change(ThemeMode::Light, None, cx)
        }
        (Some(Scheme::Dark), _) | (None, ThemeChoice::ClaudeDark) => {
            Theme::change(ThemeMode::Dark, None, cx)
        }
        (None, ThemeChoice::System) => Theme::sync_system_appearance(window, cx),
    }
    apply(cx);
}

/// Writes the tokens, radii, and font sizes, all with the zoom applied, into the gpui-kit theme.
fn apply(cx: &mut App) {
    let t = tokens(cx);
    let interface = ui_font_size(metrics::FONT_BASE, cx);
    let code = code_font_size(metrics::FONT_CODE, cx);
    let ui_family = ui_font_family(cx);
    let code_family = code_font_family(cx);
    let radius = zoomed(metrics::RADIUS_MD, cx);
    let radius_lg = zoomed(metrics::RADIUS_LG, cx);
    let clear = gpui_kit::transparent_black();
    Theme::update(cx, |theme| {
        theme.radius = radius;
        theme.radius_lg = radius_lg;
        theme.font_size = interface;
        theme.font_family = ui_family;
        theme.mono_font_size = code;
        theme.mono_font_family = code_family;

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

    #[test]
    fn popup_offset_is_the_trigger_height_plus_one_gap() {
        assert_eq!(metrics::POPUP_GAP, 4.);
        assert_eq!(popup_offset(28.), 32.);
        assert_eq!(popup_offset(0.), metrics::POPUP_GAP);
    }

    /// gpui-kit `dropdown_menu` takes no offset: its `Popover` places the menu `rem / 4` from
    /// the trigger. That must stay the theme gap.
    #[test]
    fn gpui_kit_dropdown_gap_is_the_popup_gap() {
        assert_eq!(BASE_REM_SIZE * 0.25, metrics::POPUP_GAP);
    }

    #[test]
    fn mono_family_prefers_sf_mono_and_falls_back_to_menlo() {
        let names = |list: &[&str]| list.iter().map(|name| name.to_string()).collect::<Vec<_>>();
        assert_eq!(pick_mono_family(&names(&["Menlo", "SF Mono"])), "SF Mono");
        assert_eq!(
            pick_mono_family(&names(&["SFMono-Regular", "SF Mono"])),
            "SFMono-Regular"
        );
        assert_eq!(pick_mono_family(&names(&["Menlo", "Helvetica"])), "Menlo");
        assert_eq!(pick_mono_family(&[]), "Menlo");
    }

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

    fn installed() -> Vec<String> {
        vec!["Menlo".into(), "Inter".into()]
    }

    #[test]
    fn an_installed_family_is_kept_and_trimmed() {
        assert_eq!(
            installed_family(Some(" Inter ".into()), &installed()),
            Some("Inter".into())
        );
    }

    #[test]
    fn a_missing_empty_or_unset_family_falls_back_to_the_default() {
        assert_eq!(
            installed_family(Some("Nope Sans".into()), &installed()),
            None
        );
        assert_eq!(installed_family(Some("  ".into()), &installed()), None);
        assert_eq!(installed_family(None, &installed()), None);
    }

    #[test]
    fn content_size_is_floored_and_clamped_as_in_electron() {
        assert_eq!(clamp_content_font_size(15.), 15.);
        assert_eq!(clamp_content_font_size(15.9), 15.);
        assert_eq!(clamp_content_font_size(3.), MIN_CONTENT_FONT_SIZE);
        assert_eq!(clamp_content_font_size(99.), MAX_CONTENT_FONT_SIZE);
        assert_eq!(clamp_content_font_size(f32::NAN), DEFAULT_CONTENT_FONT_SIZE);
        assert_eq!(
            clamp_content_font_size(f32::INFINITY),
            DEFAULT_CONTENT_FONT_SIZE
        );
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
