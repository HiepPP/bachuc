//! `AutorunStrip` of `client/run-cells.tsx`: the run state with its dot, then label and value
//! pairs, on `surface1`.

use crate::cell::LINE_HEIGHT;
use crate::model::{StripModel, StripTone};
use bachuc_theme::metrics::{FONT_SM, WEIGHT_BOLD};
use bachuc_theme::{tokens, ui_font_size};
use gpui_kit::*;

/// `columnGap: 14`, `rowGap: 4`, padding 12 by 8, `borderRadius: 8`; the dot is 8 wide with a
/// `gap: 6` to the state.
const COLUMN_GAP: f32 = 14.;
const ROW_GAP: f32 = 4.;
const PADDING_X: f32 = 12.;
const PADDING_Y: f32 = 8.;
const RADIUS: f32 = 8.;
const DOT: f32 = 8.;
const DOT_GAP: f32 = 6.;

pub(crate) fn autorun_strip(strip: &StripModel, cx: &App) -> AnyElement {
    let colors = tokens(cx);
    let color = match strip.tone {
        StripTone::Live => colors.status_success,
        StripTone::Stopped => colors.status_warning,
        StripTone::Finished => colors.foreground,
    };
    let label = std::iter::once(strip.state.to_string())
        .chain(
            strip
                .items
                .iter()
                .map(|(label, value)| format!("{label} {value}")),
        )
        .collect::<Vec<_>>()
        .join(", ");
    div()
        .id("wt-autorun-strip")
        .role(Role::Group)
        .aria_label(SharedString::from(label))
        .flex()
        .flex_row()
        .flex_wrap()
        .items_center()
        .gap_x(px(COLUMN_GAP))
        .gap_y(px(ROW_GAP))
        .px(px(PADDING_X))
        .py(px(PADDING_Y))
        .rounded(px(RADIUS))
        .border_1()
        .border_color(colors.border)
        .bg(colors.surface1)
        .line_height(relative(LINE_HEIGHT))
        .text_size(ui_font_size(FONT_SM, cx))
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(DOT_GAP))
                .child(div().size(px(DOT)).rounded(px(DOT / 2.)).bg(color))
                .child(
                    div()
                        .font_weight(FontWeight(WEIGHT_BOLD))
                        .text_color(color)
                        .child(strip.state),
                ),
        )
        .children(strip.items.iter().map(|(label, value)| {
            let text = format!("{label} {value}");
            let muted = HighlightStyle {
                color: Some(colors.foreground_muted),
                ..HighlightStyle::default()
            };
            div()
                .text_color(colors.foreground)
                .child(StyledText::new(text).with_highlights([(0..label.len() + 1, muted)]))
        }))
        .into_any_element()
}
