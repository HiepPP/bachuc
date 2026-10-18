//! `packages/app/src/composer/agent-controls/control.tsx`: the toolbar trigger of a control, an
//! icon and the current value, 28 px high and fully round.

use super::layout::CONTROL_SIZE;
use bachuc_icons::{IconName, icon};
use bachuc_theme::metrics::{FONT_SM, ICON_SM, RADIUS_2XL, SPACE_1, SPACE_2};
use bachuc_theme::{tokens, ui_font_size, zoomed};
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::component::{Selectable, Sizable as _, h_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use std::sync::Arc;

/// The toolbar glyph size (`resolveComposerToolbarGlyphSize` on web).
pub const GLYPH: f32 = 14.;
/// The value text stops growing here.
const VALUE_MAX_WIDTH: f32 = 160.;

/// The icon of a control: a lucide icon tinted by the control, or an image in its own color.
#[derive(Clone)]
pub enum Glyph {
    Icon(IconName),
    Image(Arc<Image>),
}

pub fn glyph_element(glyph: Glyph, color: Hsla, cx: &App) -> AnyElement {
    match glyph {
        Glyph::Icon(name) => div()
            .flex_none()
            .child(icon(name).with_size(zoomed(GLYPH, cx)).text_color(color))
            .into_any_element(),
        Glyph::Image(image) => img(image)
            .flex_none()
            .size(zoomed(GLYPH, cx))
            .into_any_element(),
    }
}

/// How much of the trigger shows at the current density.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TriggerLook {
    pub show_caret: bool,
    /// Icon only, 28 px wide: the tight mode pill.
    pub icon_only: bool,
}

/// The trigger: icon, value, and in full density the caret.
#[derive(IntoElement)]
pub struct PillTrigger {
    pub id: ElementId,
    pub kind: String,
    pub glyph: Glyph,
    pub label: String,
    pub tooltip: Option<String>,
    pub disabled: bool,
    pub open: bool,
    pub look: TriggerLook,
}

impl Selectable for PillTrigger {
    fn selected(mut self, selected: bool) -> Self {
        self.open = selected;
        self
    }

    fn is_selected(&self) -> bool {
        self.open
    }
}

impl RenderOnce for PillTrigger {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = tokens(cx);
        let size = ui_font_size(FONT_SM, cx);
        let disabled = self.disabled;
        let look = self.look;
        h_flex()
            .id(self.id)
            .role(Role::Button)
            .aria_label(format!("{}: {}", self.kind, self.label))
            .aria_expanded(self.open)
            .h(zoomed(CONTROL_SIZE, cx))
            .items_center()
            .rounded(zoomed(RADIUS_2XL, cx))
            .text_size(size)
            .text_color(t.foreground_muted)
            .map(|pill| {
                if look.icon_only {
                    pill.w(zoomed(CONTROL_SIZE, cx))
                        .flex_none()
                        .justify_center()
                } else {
                    // The label text shrinks first, as `container` and `modelControl` do on web.
                    pill.min_w_0()
                        .flex_shrink(1.)
                        .px(zoomed(SPACE_2, cx))
                        .gap(zoomed(SPACE_1, cx))
                }
            })
            .when(self.open, |pill| pill.bg(t.surface2))
            .when(disabled, |pill| pill.opacity(0.5))
            .when(!disabled, |pill| {
                pill.cursor_pointer().hover(|pill| pill.bg(t.surface2))
            })
            .when_some(self.tooltip, |pill, text| {
                pill.tooltip(move |window, cx| Tooltip::new(text.clone()).build(window, cx))
            })
            .child(glyph_element(self.glyph, t.foreground_muted, cx))
            .when(!look.icon_only, |pill| {
                pill.child(
                    div()
                        .truncate()
                        .min_w_0()
                        .flex_shrink(1.)
                        .max_w(zoomed(VALUE_MAX_WIDTH, cx))
                        .child(self.label),
                )
            })
            // `ComboboxTrigger` draws a 14 px muted chevron 1 px low after the toolbar value.
            .when(look.show_caret && !look.icon_only, |pill| {
                pill.child(
                    div().flex_none().mt(zoomed(1., cx)).child(
                        icon(IconName::ChevronDown)
                            .with_size(zoomed(ICON_SM, cx))
                            .text_color(t.foreground_muted),
                    ),
                )
            })
    }
}
