//! `packages/app/src/composer/input/input.tsx`: the composer card. From the top: the docked
//! track strip and the attachment tray, the text input, and the button row with attach, the
//! toolbar slot, the agent controls, and on the right the meter slot and the primary button.

use crate::pill_styles::{SURFACE_PADDING_X, SURFACE_PADDING_Y};
use bachuc_icons::{IconName, icon};
use bachuc_theme::metrics::{ICON_MD, RADIUS_2XL, SPACE_1, SPACE_1_5};
use bachuc_theme::{tokens, zoomed};
use gpui_kit::component::v_flex;
use gpui_kit::component::{Sizable as _, h_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

/// Size of the round toolbar buttons (attach, send, stop).
pub const CONTROL: f32 = 28.;
/// `buttonRow` margin: the row reaches 6 px into the card padding.
const BUTTON_ROW_INSET: f32 = 6.;

/// A round icon button with a role and a label.
pub fn round_button(
    id: impl Into<ElementId>,
    glyph: IconName,
    label: &'static str,
    size: f32,
    background: Option<Hsla>,
    foreground: Hsla,
    hover: Hsla,
    cx: &App,
) -> Stateful<Div> {
    div()
        .id(id)
        .role(Role::Button)
        .aria_label(label)
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .size(zoomed(size, cx))
        .rounded_full()
        .cursor_pointer()
        .when_some(background, |button, color| button.bg(color))
        .hover(move |button| button.bg(hover))
        .child(
            icon(glyph)
                .with_size(zoomed(ICON_MD, cx))
                .text_color(foreground),
        )
}

/// The card. Each slot is optional except the input.
#[derive(IntoElement)]
pub struct MessageInput {
    header: Option<AnyElement>,
    tray: Option<AnyElement>,
    input: AnyElement,
    attach: Option<AnyElement>,
    toolbar: Vec<AnyElement>,
    controls: Option<AnyElement>,
    trailing: Vec<AnyElement>,
    primary: Option<AnyElement>,
}

impl MessageInput {
    pub fn new(input: impl IntoElement) -> Self {
        Self {
            header: None,
            tray: None,
            input: input.into_any_element(),
            attach: None,
            toolbar: Vec::new(),
            controls: None,
            trailing: Vec::new(),
            primary: None,
        }
    }

    /// The docked track strip at the top of the card.
    pub fn header(mut self, header: Option<AnyElement>) -> Self {
        self.header = header;
        self
    }

    /// The attachment tray under the strip.
    pub fn tray(mut self, tray: Option<AnyElement>) -> Self {
        self.tray = tray;
        self
    }

    pub fn attach(mut self, attach: Option<AnyElement>) -> Self {
        self.attach = attach;
        self
    }

    /// The slot after the attach button, before the agent controls.
    pub fn toolbar(mut self, toolbar: Vec<AnyElement>) -> Self {
        self.toolbar = toolbar;
        self
    }

    pub fn controls(mut self, controls: Option<AnyElement>) -> Self {
        self.controls = controls;
        self
    }

    /// Before the primary button: the context window meter slot.
    pub fn trailing(mut self, trailing: Vec<AnyElement>) -> Self {
        self.trailing = trailing;
        self
    }

    /// Send, Stop, or the submit button.
    pub fn primary(mut self, primary: Option<AnyElement>) -> Self {
        self.primary = primary;
        self
    }
}

impl RenderOnce for MessageInput {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = tokens(cx);
        v_flex()
            .gap(zoomed(SPACE_1_5, cx))
            .px(zoomed(SURFACE_PADDING_X, cx))
            .py(zoomed(SURFACE_PADDING_Y, cx))
            .bg(t.surface1)
            .border_1()
            .border_color(t.border_accent)
            .rounded(zoomed(RADIUS_2XL, cx))
            .children(self.header)
            .children(self.tray)
            .child(self.input)
            .child(
                h_flex()
                    .mx(zoomed(-BUTTON_ROW_INSET, cx))
                    .items_end()
                    .justify_between()
                    .child(
                        h_flex()
                            .min_w_0()
                            .flex_1()
                            .items_center()
                            .children(self.attach)
                            .children(self.toolbar)
                            .children(self.controls),
                    )
                    .child(
                        h_flex()
                            .flex_none()
                            .items_center()
                            .gap(zoomed(SPACE_1, cx))
                            .children(self.trailing)
                            .children(self.primary),
                    ),
            )
    }
}
