//! `OverviewCell`, `ShowMore`, and `Empty` of `client/overview-cells.tsx`. A cell that holds rows
//! draws as slices, one list item each: the top slice with the title, middle slices, and a bottom
//! slice, so a long list stays lazy.

use bachuc_theme::metrics::{FONT_SM, RADIUS_XL, SPACE_2, WEIGHT_MEDIUM, WEIGHT_SEMIBOLD};
use bachuc_theme::{tokens, ui_font_size};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

/// The cell's `padding: 14`; no metric has 14.
pub(crate) const CELL_PADDING: f32 = 14.;
/// `borderRadius: 12`.
pub(crate) const CELL_RADIUS: f32 = RADIUS_XL;
/// `LIST_LIMIT`: lists show this many items until the owner expands them.
pub(crate) const LIST_LIMIT: usize = 5;
/// The title row's `marginBottom: 10`.
const TITLE_GAP: f32 = 10.;
/// The title's `fontSize: 12.5`.
const TITLE_SIZE: f32 = 12.5;
/// `ShowMore`'s `fontSize: 11.5`, `paddingVertical: 4`, `marginTop: 4`.
const SHOW_MORE_SIZE: f32 = 11.5;
const SHOW_MORE_SPACE: f32 = 4.;
/// `Empty`'s `fontSize: 12.5`.
const EMPTY_SIZE: f32 = 12.5;
/// The warning tint's `opacity: 0.05`.
const WARNING_TINT: f32 = 0.05;
/// Electron text sets about 1.2; GPUI's default is 1.618.
pub(crate) const LINE_HEIGHT: f32 = 1.2;

/// Which part of a cell a list item draws.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Slice {
    Whole,
    Top,
    Middle,
    Bottom,
}

/// The bordered frame of one slice: each slice draws its own sides, corners, and padding, so the
/// slices of a cell join into one card.
pub(crate) fn cell_slice(slice: Slice, warning: bool, cx: &App) -> Div {
    let colors = tokens(cx);
    let border = if warning {
        colors.status_warning
    } else {
        colors.border
    };
    let top = matches!(slice, Slice::Whole | Slice::Top);
    let bottom = matches!(slice, Slice::Whole | Slice::Bottom);
    div()
        .relative()
        .w_full()
        .flex()
        .flex_col()
        .px(px(CELL_PADDING))
        .bg(colors.surface0)
        .border_color(border)
        .border_l_1()
        .border_r_1()
        .line_height(relative(LINE_HEIGHT))
        .when(top, |this| {
            this.border_t_1()
                .rounded_t(px(CELL_RADIUS))
                .pt(px(CELL_PADDING))
        })
        .when(bottom, |this| {
            this.border_b_1()
                .rounded_b(px(CELL_RADIUS))
                .pb(px(CELL_PADDING))
        })
        .when(warning, |this| {
            // The tint sits under the content, as the absolute tint view does.
            this.child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full()
                    .bg(colors.status_warning.opacity(WARNING_TINT)),
            )
        })
}

/// The title row: the title, and an aside such as a count on the right.
pub(crate) fn cell_title(
    title: impl Into<SharedString>,
    aside: Option<SharedString>,
    cx: &App,
) -> Div {
    let colors = tokens(cx);
    let title = title.into();
    div()
        .flex()
        .flex_row()
        .justify_between()
        .items_baseline()
        .gap(px(SPACE_2))
        .mb(px(TITLE_GAP))
        .child(
            div()
                .id(SharedString::from(format!("cell-{title}")))
                .role(Role::Heading)
                .aria_label(title.clone())
                .text_size(ui_font_size(TITLE_SIZE, cx))
                .font_weight(FontWeight(WEIGHT_SEMIBOLD))
                .text_color(colors.foreground)
                .child(title),
        )
        .children(aside.map(|aside| {
            div()
                .min_w_0()
                .flex_shrink(1.)
                .truncate()
                .text_size(ui_font_size(FONT_SM, cx))
                .text_color(colors.foreground_muted)
                .child(aside)
        }))
}

/// A whole bordered cell with a title row. `warning` tints a cell that needs attention. It grows
/// to fill a grid column, so cells in one row end on the same line.
pub(crate) fn overview_cell(
    title: impl Into<SharedString>,
    aside: Option<SharedString>,
    warning: bool,
    body: impl IntoElement,
    cx: &App,
) -> AnyElement {
    cell_slice(Slice::Whole, warning, cx)
        .flex_grow_1()
        .child(cell_title(title, aside, cx))
        .child(body)
        .into_any_element()
}

/// The label of the Show more toggle, or `None` when the list fits.
pub(crate) fn show_more_label(total: usize, expanded: bool) -> Option<String> {
    if total <= LIST_LIMIT {
        return None;
    }
    Some(if expanded {
        "Show less".into()
    } else {
        format!("Show {} more", total - LIST_LIMIT)
    })
}

/// The first `LIST_LIMIT` items until the owner expands the list.
pub(crate) fn limit<T>(items: &[T], expanded: bool) -> &[T] {
    if expanded {
        items
    } else {
        &items[..items.len().min(LIST_LIMIT)]
    }
}

/// The Show more toggle of a list longer than `LIST_LIMIT`.
pub(crate) fn show_more(
    id: impl Into<ElementId>,
    total: usize,
    expanded: bool,
    on_toggle: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> Option<AnyElement> {
    let label = show_more_label(total, expanded)?;
    let colors = tokens(cx);
    Some(
        div()
            .id(id)
            .role(Role::Button)
            .aria_label(SharedString::from(label.clone()))
            .aria_expanded(expanded)
            .self_start()
            .mt(px(SHOW_MORE_SPACE))
            .py(px(SHOW_MORE_SPACE))
            .cursor_pointer()
            .active(|style| style.opacity(0.6))
            .text_size(ui_font_size(SHOW_MORE_SIZE, cx))
            .font_weight(FontWeight(WEIGHT_MEDIUM))
            .text_color(colors.accent)
            .on_click(on_toggle)
            .child(label)
            .into_any_element(),
    )
}

/// A muted line for an empty or loading list.
pub(crate) fn empty(text: impl Into<SharedString>, cx: &App) -> Div {
    let colors = tokens(cx);
    div()
        .text_size(ui_font_size(EMPTY_SIZE, cx))
        .text_color(colors.foreground_muted)
        .child(text.into())
}

#[cfg(test)]
mod tests {
    // Not `super::*`: the gpui glob brings in a `test` macro that shadows `#[test]`.
    use super::{LIST_LIMIT, limit, show_more_label};

    #[test]
    fn show_more_appears_past_the_limit_of_5() {
        assert_eq!(LIST_LIMIT, 5);
        assert_eq!(show_more_label(5, false), None);
        assert_eq!(show_more_label(8, false).as_deref(), Some("Show 3 more"));
        assert_eq!(show_more_label(8, true).as_deref(), Some("Show less"));
        assert_eq!(limit(&[1, 2, 3, 4, 5, 6, 7], false), [1, 2, 3, 4, 5]);
        assert_eq!(limit(&[1, 2, 3, 4, 5, 6, 7], true).len(), 7);
        assert_eq!(limit(&[1, 2], false), [1, 2]);
    }
}
