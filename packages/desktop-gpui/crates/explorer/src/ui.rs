//! Small elements that the three views share: the centered state text, the file icon, the
//! `+N -M` count, and the tree row look of `tree-primitives.tsx`.

pub use crate::changes::Change;
use crate::changes::format_count;
use crate::file_icons::file_icon;
use bachuc_icons::{IconName, icon};
use bachuc_theme::metrics::{
    FONT_BASE, FONT_SM, ICON_MD, ICON_SM, SPACE_1, SPACE_2, SPACE_3, SPACE_4, SPACE_6,
};
use bachuc_theme::{Tokens, tokens, ui_font_size};
use gpui_kit::component::{Sizable as _, h_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

/// `theme.spacing[16]`, the top inset of the Electron state screens.
pub const SPACE_16: f32 = 64.;
/// `TREE_INDENT_PER_LEVEL`, `WORKSPACE_TREE_ICON_SIZE`, and `WORKSPACE_TREE_ICON_LABEL_GAP`.
pub const TREE_INDENT: f32 = 12.;
pub const TREE_ICON: f32 = ICON_MD;
pub const TREE_ICON_GAP: f32 = SPACE_2;
/// A Files row: 6 px above and below one 17 px text line, as `WORKSPACE_FILE_ROW_VERTICAL_PADDING`
/// gives the Electron row.
pub const FILE_ROW_HEIGHT: f32 = 29.;
/// A Changes row is taller: the `DiffStat` is 20 px high.
pub const CHANGE_ROW_HEIGHT: f32 = 32.;

/// `treeRowPaddingLeft`.
pub fn tree_padding_left(depth: usize) -> f32 {
    SPACE_3 + depth as f32 * TREE_INDENT
}

/// A message centered in the pane below a 64 px inset, as the Electron `emptyContainer` and
/// `errorContainer` draw it.
pub fn state_message(
    id: &'static str,
    text: impl Into<SharedString>,
    error: bool,
    cx: &App,
) -> Stateful<Div> {
    let colors = tokens(cx);
    let text: SharedString = text.into();
    div()
        .id(id)
        .role(Role::Status)
        .aria_label(text.clone())
        .flex_1()
        .min_h_0()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .pt(px(SPACE_16))
        .px(px(SPACE_6))
        .text_size(ui_font_size(FONT_BASE, cx))
        .text_center()
        .text_color(if error {
            colors.destructive
        } else {
            colors.foreground_muted
        })
        .child(text)
}

/// A message centered both ways in the pane, as the Files `centerState` draws it.
pub fn centered_text(
    id: &'static str,
    text: impl Into<SharedString>,
    error: bool,
    cx: &App,
) -> Stateful<Div> {
    let colors = tokens(cx);
    let text: SharedString = text.into();
    div()
        .id(id)
        .role(Role::Status)
        .aria_label(text.clone())
        .flex_1()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(SPACE_2))
        .p(px(SPACE_4))
        .text_size(ui_font_size(FONT_BASE, cx))
        .text_center()
        .text_color(if error {
            colors.destructive
        } else {
            colors.foreground_muted
        })
        .child(text)
}

/// The file viewer placeholder: a muted title and the file size under it, centered
/// (`emptyText` and `binaryMetaText` in `file-pane/pane.tsx`).
pub fn file_meta_state(
    id: &'static str,
    title: &'static str,
    meta: Option<String>,
    cx: &App,
) -> Stateful<Div> {
    let colors = tokens(cx);
    div()
        .id(id)
        .role(Role::Status)
        .aria_label(title)
        .flex_1()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(SPACE_3))
        .p(px(SPACE_4))
        .text_size(ui_font_size(FONT_BASE, cx))
        .text_color(colors.foreground_muted)
        .child(div().text_center().child(title))
        .when_some(meta, |this, meta| {
            this.child(div().mt(px(SPACE_2)).child(meta))
        })
}

/// The colored file icon of a row.
pub fn file_icon_element(file_name: &str) -> impl IntoElement + use<> {
    div()
        .size(px(TREE_ICON))
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_center()
        .child(img(file_icon(file_name)).size(px(TREE_ICON)))
}

/// The disclosure chevron of a folder row.
pub fn chevron(expanded: bool, colors: &Tokens) -> impl IntoElement + use<> {
    div()
        .size(px(TREE_ICON))
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_center()
        .child(
            icon(if expanded {
                IconName::ChevronDown
            } else {
                IconName::ChevronRight
            })
            .with_size(px(TREE_ICON))
            .text_color(colors.foreground_extra_muted),
        )
}

/// The slot of a folder whose listing is on its way.
pub fn loading_marker(colors: &Tokens) -> impl IntoElement + use<> {
    div()
        .size(px(TREE_ICON))
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_center()
        .child(
            icon(IconName::RotateCw)
                .with_size(px(ICON_SM))
                .text_color(colors.foreground_extra_muted),
        )
}

/// `DiffStat`: `+N` in the success color and `-M` in the danger color.
pub fn diff_stat(added: u32, removed: u32, cx: &App) -> impl IntoElement + use<> {
    let colors = tokens(cx);
    h_flex()
        .h(px(20.))
        .gap(px(SPACE_1))
        .flex_shrink_0()
        .text_size(ui_font_size(FONT_SM, cx))
        .child(
            div()
                .text_color(colors.status_success)
                .child(format!("+{}", format_count(added))),
        )
        .child(
            div()
                .text_color(colors.status_danger)
                .child(format!("-{}", format_count(removed))),
        )
}

pub fn change_icon(change: Change, colors: &Tokens) -> impl IntoElement + use<> {
    let (name, color) = match change {
        Change::Added => (IconName::SquarePlus, colors.status_success),
        Change::Deleted => (IconName::SquareMinus, colors.status_danger),
        Change::Modified => (IconName::SquareDot, colors.status_warning),
    };
    icon(name).with_size(px(ICON_SM)).text_color(color)
}

/// The padding that every pane uses at the right edge: `WORKSPACE_PANE_TRAILING_GLYPH_RAIL`.
pub const TRAILING_RAIL: f32 = SPACE_2;
