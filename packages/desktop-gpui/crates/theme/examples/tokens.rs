//! Shows the design tokens of both themes side by side: swatches, type sizes, and 40 icons at
//! 14 and 16 px. The three buttons call `set_theme`, so the gpui-kit button and its text show
//! the mapped colors.
//!
//! Usage: tokens

use bachuc_icons::{IconName, icon};
use bachuc_theme::{ThemeChoice, Tokens, metrics, set_theme};
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::{Sizable as _, h_flex, v_flex};
use gpui_kit::*;

struct Example;

fn swatches(t: Tokens) -> Vec<(&'static str, Hsla)> {
    let mut all = vec![
        ("surface0", t.surface0),
        ("surface1", t.surface1),
        ("surface2", t.surface2),
        ("surface3", t.surface3),
        ("surface4", t.surface4),
        ("surface_sidebar", t.surface_sidebar),
        ("sidebar_hover", t.surface_sidebar_hover),
        ("sidebar_selected", t.surface_sidebar_selected),
        ("surface_workspace", t.surface_workspace),
        ("surface_diff_empty", t.surface_diff_empty),
        ("interaction_highlight", t.interaction_highlight),
        ("foreground", t.foreground),
        ("foreground_muted", t.foreground_muted),
        ("foreground_extra_muted", t.foreground_extra_muted),
        ("border", t.border),
        ("border_accent", t.border_accent),
        ("accent", t.accent),
        ("accent_bright", t.accent_bright),
        ("accent_foreground", t.accent_foreground),
        ("destructive", t.destructive),
        ("destructive_foreground", t.destructive_foreground),
        ("ring", t.ring),
        ("popover", t.popover),
        ("status_success", t.status_success),
        ("status_danger", t.status_danger),
        ("status_warning", t.status_warning),
        ("status_merged", t.status_merged),
        ("tint_success", t.tint_success),
        ("tint_danger", t.tint_danger),
        ("tint_warning", t.tint_warning),
        ("tint_merged", t.tint_merged),
        ("dot_success", t.dot_success),
        ("dot_danger", t.dot_danger),
        ("dot_warning", t.dot_warning),
        ("dot_running", t.dot_running),
        ("diff_addition", t.diff_addition),
        ("diff_deletion", t.diff_deletion),
        ("terminal_background", t.terminal_background),
        ("terminal_foreground", t.terminal_foreground),
        ("terminal_cursor", t.terminal_cursor),
        ("terminal_selection", t.terminal_selection),
    ];
    const ANSI: [&str; 16] = [
        "ansi black",
        "ansi red",
        "ansi green",
        "ansi yellow",
        "ansi blue",
        "ansi magenta",
        "ansi cyan",
        "ansi white",
        "ansi bright black",
        "ansi bright red",
        "ansi bright green",
        "ansi bright yellow",
        "ansi bright blue",
        "ansi bright magenta",
        "ansi bright cyan",
        "ansi bright white",
    ];
    all.extend(ANSI.into_iter().zip(t.ansi));
    all
}

fn panel(label: &'static str, t: Tokens) -> impl IntoElement {
    let swatch = |(name, color): (&'static str, Hsla)| {
        h_flex()
            .w(px(168.))
            .gap(px(metrics::SPACE_1_5))
            .items_center()
            .child(
                div()
                    .size(px(16.))
                    .flex_shrink_0()
                    .rounded(px(metrics::RADIUS_BASE))
                    .border_1()
                    .border_color(t.border)
                    .bg(color),
            )
            .child(
                div()
                    .text_size(px(metrics::FONT_SM))
                    .text_color(t.foreground_muted)
                    .child(name),
            )
    };
    let types = [
        ("code 12", metrics::FONT_CODE),
        ("sm 12", metrics::FONT_SM),
        ("base 14", metrics::FONT_BASE),
        ("content 15", metrics::FONT_CONTENT),
        ("lg 16", metrics::FONT_LG),
        ("xl 18", metrics::FONT_XL),
        ("2xl 20", metrics::FONT_2XL),
    ];
    let icons = |size: f32| {
        h_flex().flex_wrap().gap(px(metrics::SPACE_2)).children(
            IconName::ALL
                .iter()
                .take(40)
                .map(|name| icon(*name).with_size(px(size)).text_color(t.foreground)),
        )
    };
    v_flex()
        .flex_1()
        .gap(px(metrics::SPACE_3))
        .p(px(metrics::SPACE_4))
        .bg(t.surface_workspace)
        .text_color(t.foreground)
        .child(
            div()
                .text_size(px(metrics::FONT_XL))
                .font_weight(FontWeight::SEMIBOLD)
                .child(label),
        )
        .child(
            h_flex()
                .flex_wrap()
                .gap_x(px(metrics::SPACE_2))
                .gap_y(px(metrics::SPACE_1))
                .children(swatches(t).into_iter().map(swatch)),
        )
        .child(
            v_flex()
                .gap(px(metrics::SPACE_1))
                .children(types.map(|(name, size)| {
                    div()
                        .text_size(px(size))
                        .child(format!("{name} - The quick brown fox"))
                })),
        )
        .child(icons(metrics::ICON_SM))
        .child(icons(metrics::ICON_MD))
        .child(
            div()
                .bg(t.accent)
                .text_color(t.accent_foreground)
                .rounded(px(metrics::RADIUS_MD))
                .h(px(metrics::CONTROL_XS))
                .px(px(metrics::SPACE_3))
                .flex()
                .items_center()
                .text_size(px(metrics::FONT_BASE))
                .child("Accent button"),
        )
}

impl Render for Example {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = bachuc_theme::tokens(cx);
        v_flex()
            .size_full()
            .bg(t.surface0)
            .child(
                h_flex()
                    .gap(px(metrics::SPACE_2))
                    .p(px(metrics::SPACE_2))
                    .child(
                        Button::new("system")
                            .label("System")
                            .small()
                            .on_click(|_, _, cx| set_theme(ThemeChoice::System, cx)),
                    )
                    .child(
                        Button::new("light")
                            .label("Light")
                            .small()
                            .on_click(|_, _, cx| set_theme(ThemeChoice::Light, cx)),
                    )
                    .child(
                        Button::new("dark")
                            .label("Claude dark")
                            .small()
                            .primary()
                            .on_click(|_, _, cx| set_theme(ThemeChoice::ClaudeDark, cx)),
                    ),
            )
            .child(
                h_flex()
                    .flex_1()
                    .child(panel("light", Tokens::light()))
                    .child(panel("claude dark", Tokens::claude_dark())),
            )
    }
}

fn main() {
    gpui_kit::application()
        .with_assets(bachuc_icons::Assets)
        .run(|cx| {
            gpui_kit::init(cx);
            bachuc_theme::init(cx);
            cx.activate(true);
            let options = WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                    None,
                    size(px(1180.), px(900.)),
                    cx,
                ))),
                titlebar: Some(TitlebarOptions {
                    title: Some("Bachuc tokens".into()),
                    ..Default::default()
                }),
                ..Default::default()
            };
            gpui_kit::open_window(options, cx, |_, cx| cx.new(|_| Example)).expect("open window");
        });
}
