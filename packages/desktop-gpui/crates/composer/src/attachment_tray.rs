//! The attachment tray at the top of the card (`renderAttachmentTray` in
//! `packages/app/src/composer/index.tsx`): one chip per attachment in a wrapping row.
//! `attachments.rs` keeps the image model.

use crate::attachments::Attachment;
use crate::input::round_button;
use bachuc_icons::{IconName, icon};
use bachuc_plugin_host::ComposerAttachment;
use bachuc_theme::metrics::{FONT_SM, ICON_SM, RADIUS_MD, SPACE_2};
use bachuc_theme::{tokens, ui_font_size};
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::component::{Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use std::rc::Rc;

/// The image chip side, and the height of a plugin chip.
const CHIP: f32 = 48.;
/// A plugin chip stops growing here.
const PLUGIN_CHIP_MAX_WIDTH: f32 = 280.;
/// The remove button sits over the chip corner.
const REMOVE: f32 = 24.;

/// The image chips, each with its remove button.
pub fn image_chips(
    attachments: &[Attachment],
    on_remove: impl Fn(usize, &mut App) + 'static,
    cx: &App,
) -> Vec<AnyElement> {
    let t = tokens(cx);
    let on_remove = Rc::new(on_remove);
    attachments
        .iter()
        .enumerate()
        .map(|(index, attachment)| {
            let on_remove = on_remove.clone();
            div()
                .relative()
                .child(
                    div()
                        .id(("composer-chip", index))
                        .role(Role::ListItem)
                        .aria_label(format!("Attachment {}", attachment.name))
                        .size(px(CHIP))
                        .overflow_hidden()
                        .border_1()
                        .border_color(t.border_accent)
                        .rounded(px(RADIUS_MD))
                        .child(
                            img(attachment.preview.clone())
                                .size_full()
                                .object_fit(ObjectFit::Cover),
                        ),
                )
                .child(
                    div().absolute().top(px(-8.)).left(px(-8.)).child(
                        round_button(
                            ("composer-chip-remove", index),
                            IconName::X,
                            "Remove attachment",
                            REMOVE,
                            Some(t.surface2),
                            t.foreground_muted,
                            t.surface3,
                        )
                        .border_1()
                        .border_color(t.border_accent)
                        .on_click(move |_, _, cx| on_remove(index, cx)),
                    ),
                )
                .into_any_element()
        })
        .collect()
}

/// A plugin chip (`AttachmentFrame` with `AttachmentLabel`): the source icon and the item
/// title, a comment field when the chip is commentable, and its remove button. A press opens
/// the item through the source's `on_open` when it has one.
pub fn plugin_chip(
    index: usize,
    chip: &ComposerAttachment,
    comment: Option<&Entity<InputState>>,
    on_open: Option<Rc<dyn Fn(&mut Window, &mut App)>>,
    on_remove: impl Fn(usize, &mut App) + 'static,
    cx: &App,
) -> AnyElement {
    let t = tokens(cx);
    let title = format!("{} {}", chip.item.identifier, chip.item.title);
    h_flex()
        .id(("composer-plugin-chip", index))
        .role(Role::ListItem)
        .aria_label(format!("{}: {title}", chip.source_title))
        .items_center()
        .gap(px(SPACE_2))
        .h(px(CHIP))
        .px(px(SPACE_2))
        .max_w(px(PLUGIN_CHIP_MAX_WIDTH))
        .border_1()
        .border_color(t.border_accent)
        .rounded(px(RADIUS_MD))
        .bg(t.surface1)
        .when_some(on_open, |chip, on_open| {
            chip.cursor_pointer()
                .hover(move |chip| chip.bg(t.surface2))
                .on_click(move |_, window, cx| on_open(window, cx))
        })
        .child(
            icon(chip.icon)
                .with_size(px(ICON_SM))
                .text_color(t.foreground_muted),
        )
        .child(
            v_flex()
                .min_w_0()
                .flex_1()
                .child(
                    div()
                        .truncate()
                        .text_size(ui_font_size(FONT_SM, cx))
                        .text_color(t.foreground)
                        .child(title),
                )
                .children(comment.map(|comment| {
                    Input::new(comment)
                        .xsmall()
                        .appearance(false)
                        .aria_label("Comment")
                })),
        )
        .child(
            round_button(
                ("composer-plugin-chip-remove", index),
                IconName::X,
                "Remove attachment",
                REMOVE,
                None,
                t.foreground_muted,
                t.surface3,
            )
            .on_click(move |_, _, cx| {
                cx.stop_propagation();
                on_remove(index, cx);
            }),
        )
        .into_any_element()
}

/// The tray; `None` without a chip. The padding leaves room for the remove buttons.
pub fn attachment_tray(chips: Vec<AnyElement>) -> Option<AnyElement> {
    if chips.is_empty() {
        return None;
    }
    Some(
        h_flex()
            .id("composer-attachment-tray")
            .role(Role::List)
            .aria_label("Attachments")
            .flex_wrap()
            .gap(px(SPACE_2))
            .pt(px(SPACE_2))
            .pl(px(SPACE_2))
            .children(chips)
            .into_any_element(),
    )
}
