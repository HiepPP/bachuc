//! `DecisionModal` of `client/decision-modal.tsx`, drawn by the page as an overlay: a backdrop and
//! a centered panel with the ADR's file path and its `parse_markdown` blocks. Inline marks stay
//! plain text. The ADR loads only while the modal is open, with no retry but the Retry button.

use crate::cell::LINE_HEIGHT;
use crate::cells::{OnClick, outline_button};
use crate::model::{Block, Loaded};
use crate::page::WatchtowerPage;
use bachuc_icons::{IconName, icon};
use bachuc_theme::metrics::{
    FONT_BASE, RADIUS_LG, RADIUS_XL, SPACE_2, SPACE_4, SPACE_6, WEIGHT_BOLD, WEIGHT_MEDIUM,
};
use bachuc_theme::{mono_font_family, tokens, ui_font_size};
use gpui_kit::component::Sizable as _;
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use std::rc::Rc;

/// The desktop sheet: a 55% black backdrop with padding 24, the card at most 820 wide and 85%
/// high on `surface1`, radius 12, a `surface2` border; the header 24 by 16 with `gap: 8`.
const BACKDROP: f32 = 0.55;
const CARD_WIDTH: f32 = 820.;
const CARD_HEIGHT: f32 = 0.85;
const TITLE_ICON: f32 = 18.;
const CLOSE_ICON: f32 = 16.;
/// `Modal.Content`'s `padding: 20` and `gap: 8`.
const BODY_PADDING: f32 = 20.;
const BODY_GAP: f32 = 8.;
/// Body text 13 with `lineHeight: 19`; headings 17, 15, and 13.5 with line heights 24 and 21
/// and `marginTop: 8` below level 1.
const TEXT: f32 = 13.;
const TEXT_LINE: f32 = 19.;
const H1: f32 = 17.;
const H2: f32 = 15.;
const H3: f32 = 13.5;
const H1_LINE: f32 = 24.;
const H_LINE: f32 = 21.;
const HEADING_TOP: f32 = 8.;
/// Bullets indent `depth * 14` with `gap: 8`.
const BULLET_INDENT: f32 = 14.;
/// Code: padding 10, radius 8, mono 11.5 with `lineHeight: 17`; the path mono 11.
const CODE_PADDING: f32 = 10.;
const CODE_TEXT: f32 = 11.5;
const CODE_LINE: f32 = 17.;
const PATH_TEXT: f32 = 11.;
const ERROR_GAP: f32 = 10.;

/// The ADR the owner reads.
pub(crate) struct DecisionModal {
    pub id: String,
    pub title: String,
    /// The file path and its blocks.
    pub state: Loaded<(String, Vec<Block>)>,
    pub focus: FocusHandle,
    /// Grows on each load, so a late answer for an older load is dropped.
    pub generation: u64,
}

fn block(block: &Block, cx: &App) -> AnyElement {
    let colors = tokens(cx);
    let text = |cx: &App| {
        div()
            .text_size(ui_font_size(TEXT, cx))
            .line_height(bachuc_theme::zoomed(TEXT_LINE, cx))
            .text_color(colors.foreground)
    };
    match block {
        Block::Heading { level, text: body } => {
            let (size, line) = match level {
                1 => (H1, H1_LINE),
                2 => (H2, H_LINE),
                _ => (H3, H_LINE),
            };
            div()
                .when(*level > 1, |this| {
                    this.pt(bachuc_theme::zoomed(HEADING_TOP, cx))
                })
                .child(
                    div()
                        .id(SharedString::from(format!("wt-adr-h-{body}")))
                        .role(Role::Heading)
                        .aria_label(SharedString::from(body.clone()))
                        .text_size(ui_font_size(size, cx))
                        .line_height(bachuc_theme::zoomed(line, cx))
                        .font_weight(FontWeight(WEIGHT_BOLD))
                        .text_color(colors.foreground)
                        .child(body.clone()),
                )
                .into_any_element()
        }
        Block::Bullet { depth, text: body } => div()
            .flex()
            .flex_row()
            .gap(bachuc_theme::zoomed(SPACE_2, cx))
            .ml(bachuc_theme::zoomed(*depth as f32 * BULLET_INDENT, cx))
            .child(text(cx).text_color(colors.foreground_muted).child("•"))
            .child(text(cx).flex_1().min_w_0().child(body.clone()))
            .into_any_element(),
        Block::Code { text: body } => div()
            .p(bachuc_theme::zoomed(CODE_PADDING, cx))
            .rounded(bachuc_theme::zoomed(RADIUS_LG, cx))
            .bg(colors.surface2)
            .child(
                div()
                    .font_family(mono_font_family(cx))
                    .text_size(ui_font_size(CODE_TEXT, cx))
                    .line_height(bachuc_theme::zoomed(CODE_LINE, cx))
                    .text_color(colors.foreground)
                    .child(body.clone()),
            )
            .into_any_element(),
        Block::Paragraph { text: body } => text(cx).child(body.clone()).into_any_element(),
    }
}

fn body(modal: &DecisionModal, on_retry: OnClick, cx: &App) -> Vec<AnyElement> {
    let colors = tokens(cx);
    match &modal.state {
        Loaded::Loading => vec![
            div()
                .id("wt-adr-loading")
                .role(Role::Status)
                .aria_label(SharedString::from(format!("Loading {}", modal.id)))
                .text_size(ui_font_size(TEXT, cx))
                .text_color(colors.foreground_muted)
                .child("Loading…")
                .into_any_element(),
        ],
        Loaded::Error(error) => {
            let message = format!("Could not load {}. {error}", modal.id);
            vec![
                div()
                    .id("wt-adr-error")
                    .role(Role::Alert)
                    .aria_label(SharedString::from(message.clone()))
                    .flex()
                    .flex_col()
                    .items_start()
                    .gap(bachuc_theme::zoomed(ERROR_GAP, cx))
                    .child(
                        div()
                            .text_size(ui_font_size(TEXT, cx))
                            .text_color(colors.status_danger)
                            .child(message),
                    )
                    .child(outline_button(
                        "wt-adr-retry",
                        "Retry",
                        SharedString::from(format!("Retry loading {}", modal.id)),
                        on_retry,
                        cx,
                    ))
                    .into_any_element(),
            ]
        }
        Loaded::Ready((file, blocks)) => std::iter::once(
            div()
                .font_family(mono_font_family(cx))
                .text_size(ui_font_size(PATH_TEXT, cx))
                .text_color(colors.foreground_muted)
                .child(file.clone())
                .into_any_element(),
        )
        .chain(blocks.iter().map(|item| block(item, cx)))
        .collect(),
    }
}

/// The overlay. Escape, the close button, and a backdrop click close it.
pub(crate) fn decision_overlay(
    modal: &DecisionModal,
    cx: &mut Context<WatchtowerPage>,
) -> AnyElement {
    let colors = tokens(cx);
    let title = format!("{} {}", modal.id, modal.title);
    let weak = cx.entity().downgrade();
    let on_retry: OnClick = Rc::new(move |_, _, cx| {
        weak.update(cx, |page, cx| page.load_decision(cx)).ok();
    });
    let content = body(modal, on_retry, cx);
    div()
        .absolute()
        .top_0()
        .left_0()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .p(bachuc_theme::zoomed(SPACE_6, cx))
        .child(
            div()
                .id("wt-adr-backdrop")
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                .bg(black().opacity(BACKDROP))
                // Blocks the page lists under it, so a click closes only the modal.
                .occlude()
                .on_click(cx.listener(|page, _, window, cx| page.close_decision(window, cx))),
        )
        .child(
            div()
                .id("wt-adr-dialog")
                .role(Role::Dialog)
                .aria_label(SharedString::from(title.clone()))
                .track_focus(&modal.focus)
                .on_key_down(cx.listener(|page, event: &KeyDownEvent, window, cx| {
                    if event.keystroke.key == "escape" {
                        page.close_decision(window, cx);
                        cx.stop_propagation();
                    }
                }))
                .occlude()
                .relative()
                .w_full()
                .max_w(bachuc_theme::zoomed(CARD_WIDTH, cx))
                .max_h(relative(CARD_HEIGHT))
                .flex()
                .flex_col()
                .overflow_hidden()
                .bg(colors.surface1)
                .rounded(bachuc_theme::zoomed(RADIUS_XL, cx))
                .border_1()
                .border_color(colors.surface2)
                .line_height(relative(LINE_HEIGHT))
                .child(
                    div()
                        .flex_shrink_0()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(bachuc_theme::zoomed(SPACE_2, cx))
                        .px(bachuc_theme::zoomed(SPACE_6, cx))
                        .py(bachuc_theme::zoomed(SPACE_4, cx))
                        .border_b_1()
                        .border_color(colors.surface2)
                        .child(
                            icon(IconName::Scale)
                                .with_size(bachuc_theme::zoomed(TITLE_ICON, cx))
                                .text_color(colors.foreground),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .truncate()
                                .text_size(ui_font_size(FONT_BASE, cx))
                                .font_weight(FontWeight(WEIGHT_MEDIUM))
                                .text_color(colors.foreground)
                                .child(title),
                        )
                        .child(
                            div()
                                .id("wt-adr-close")
                                .role(Role::Button)
                                .aria_label("Close")
                                .p(bachuc_theme::zoomed(SPACE_2, cx))
                                .rounded(bachuc_theme::zoomed(RADIUS_LG, cx))
                                .cursor_pointer()
                                .hover(|style| style.bg(colors.surface2))
                                .on_click(cx.listener(|page, _, window, cx| {
                                    page.close_decision(window, cx)
                                }))
                                .child(
                                    icon(IconName::X)
                                        .with_size(bachuc_theme::zoomed(CLOSE_ICON, cx))
                                        .text_color(colors.foreground_muted),
                                ),
                        ),
                )
                .child(
                    div()
                        .id("wt-adr-body")
                        .flex_1()
                        .min_h_0()
                        .overflow_y_scroll()
                        .p(bachuc_theme::zoomed(BODY_PADDING, cx))
                        .flex()
                        .flex_col()
                        .gap(bachuc_theme::zoomed(BODY_GAP, cx))
                        .children(content),
                ),
        )
        .into_any_element()
}
