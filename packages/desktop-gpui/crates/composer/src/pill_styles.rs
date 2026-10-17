//! `packages/app/src/composer/pill-styles.ts`: the composer surface padding and the pills that
//! dock into the track strip.

use bachuc_theme::metrics::{FONT_SM, RADIUS_MD, SPACE_1_5, SPACE_2, SPACE_3};
use bachuc_theme::{tokens, ui_font_size};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

/// `COMPOSER_SURFACE_PADDING`: inside the bordered card. The strip bleeds through it.
pub const SURFACE_PADDING_Y: f32 = SPACE_2;
pub const SURFACE_PADDING_X: f32 = SPACE_3;
/// `COMPOSER_DOCKED_STRIP_HEIGHT`.
pub const STRIP_HEIGHT: f32 = 28.;
/// `COMPOSER_DOCKED_PILL_HEIGHT`.
pub const DOCKED_PILL_HEIGHT: f32 = 24.;
pub const DOCKED_PILL_GAP: f32 = SPACE_1_5;
pub const DOCKED_PILL_PADDING: f32 = SPACE_1_5;
pub const DOCKED_PILL_RADIUS: f32 = RADIUS_MD;

/// The body of a docked pill (`dockedComposerPillStyles.body`): no border and no fill, with
/// `surface2` on hover, press, or while its panel is open. The label is `sm` in
/// `foregroundMuted`.
pub fn docked_pill(id: impl Into<ElementId>, open: bool, cx: &App) -> Stateful<Div> {
    let t = tokens(cx);
    div()
        .id(id)
        .flex()
        .flex_none()
        .items_center()
        .gap(px(DOCKED_PILL_GAP))
        .min_h(px(DOCKED_PILL_HEIGHT))
        .px(px(DOCKED_PILL_PADDING))
        .rounded(px(DOCKED_PILL_RADIUS))
        .text_size(ui_font_size(FONT_SM, cx))
        .text_color(if open {
            t.foreground
        } else {
            t.foreground_muted
        })
        .when(open, |pill| pill.bg(t.surface2))
        .hover(move |pill| pill.bg(t.surface2))
        .active(move |pill| pill.bg(t.surface2))
}
