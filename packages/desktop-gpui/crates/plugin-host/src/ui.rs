//! Element helpers for plugin settings pages and panels, drawn as the settings panes draw them
//! (`crates/settings/src/ui.rs`, after `components/settings/index.tsx`). Copied, because this
//! crate never depends on `bachuc-settings`.

use crate::composer::SelectOption;
use bachuc_icons::{IconName, icon};
use bachuc_theme::metrics::{
    CONTROL_SM, FONT_2XL, FONT_BASE, FONT_CODE, FONT_LG, FONT_SM, FONT_XL, ICON_SM, RADIUS_LG,
    RADIUS_XL, SPACE_1, SPACE_2, SPACE_3, SPACE_4, SPACE_6,
};
use bachuc_theme::{
    code_font_size, content_font_size, mono_font_family, rem_size, tokens, ui_font_size, zoom,
    zoomed,
};
use gpui_kit::base::text::{TextView, TextViewStyle};
use gpui_kit::component::button::Button;
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::component::menu::{DropdownMenu, PopupMenuItem};
use gpui_kit::component::switch::Switch;
use gpui_kit::component::{
    ActiveTheme as _, Disableable as _, Selectable, Side, Sizable as _, h_flex, v_flex,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use std::rc::Rc;

/// `note` text: one muted 13 px line. No metric has 13.
const NOTE_SIZE: f32 = 13.;
/// The space between two markdown blocks, less the 1 px GPUI's whole-pixel line box adds
/// (the timeline's `PARAGRAPH_GAP`).
const BLOCK_GAP: f32 = SPACE_6;
const PARAGRAPH_GAP: f32 = BLOCK_GAP - 1.;
const FENCE_LINE_HEIGHT: f32 = 1.45;
const FONT_3XL: f32 = 22.;
const FONT_4XL: f32 = 26.;
const HEADING_LINE_HEIGHT: f32 = 1.3;
/// `DropdownMenuContent width={220}` of `SettingsSelect`.
const SELECT_MENU_WIDTH: f32 = 220.;

/// The text column of a row: a title, a muted hint, and an error line.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RowText {
    pub label: SharedString,
    pub hint: Option<SharedString>,
    pub error: Option<SharedString>,
}

/// Whether row `index` of a card gets a divider above it (`SettingsCard`'s `rowBorder`).
pub fn divider_above(index: usize) -> bool {
    index > 0
}

/// A section: a muted heading, then the body (`SettingsSection`).
pub fn section(title: impl Into<SharedString>, body: impl IntoElement, cx: &App) -> AnyElement {
    let colors = tokens(cx);
    let title = title.into();
    v_flex()
        .mb(zoomed(SPACE_6, cx))
        .child(
            div()
                .id(SharedString::from(format!("heading-{title}")))
                .role(Role::Heading)
                .aria_label(title.clone())
                .mb(zoomed(SPACE_3, cx))
                .ml(zoomed(SPACE_1, cx))
                .text_size(ui_font_size(FONT_SM, cx))
                .text_color(colors.foreground_muted)
                .child(title),
        )
        .child(body)
        .into_any_element()
}

/// The rounded surface that holds rows, with a divider above every row but the first.
pub fn card(rows: Vec<AnyElement>, cx: &App) -> AnyElement {
    let colors = tokens(cx);
    v_flex()
        .w_full()
        .bg(colors.surface1)
        .rounded(zoomed(RADIUS_LG, cx))
        .border_1()
        .border_color(colors.border)
        .overflow_hidden()
        .children(rows.into_iter().enumerate().map(|(index, row)| {
            div()
                .w_full()
                .when(divider_above(index), |this| {
                    this.border_t_1().border_color(colors.border)
                })
                .child(row)
        }))
        .into_any_element()
}

/// `SettingsRow`: the text column, then the control.
fn row(text: RowText, control: impl IntoElement, cx: &App) -> AnyElement {
    let colors = tokens(cx);
    h_flex()
        .w_full()
        .items_center()
        .justify_between()
        .gap(zoomed(SPACE_3, cx))
        .px(zoomed(SPACE_4, cx))
        .py(zoomed(SPACE_4, cx))
        .child(
            v_flex()
                .flex_1()
                .min_w_0()
                .gap(zoomed(SPACE_1, cx))
                .child(
                    div()
                        .text_size(ui_font_size(FONT_BASE, cx))
                        .text_color(colors.foreground)
                        .child(text.label),
                )
                .children(text.hint.map(|hint| {
                    div()
                        .text_size(ui_font_size(FONT_SM, cx))
                        .text_color(colors.foreground_muted)
                        .child(hint)
                }))
                .children(text.error.map(|error| {
                    div()
                        .id("row-error")
                        .role(Role::Alert)
                        .aria_label(error.clone())
                        .text_size(ui_font_size(FONT_SM, cx))
                        .text_color(colors.destructive)
                        .child(error)
                })),
        )
        .child(div().flex_shrink_0().child(control))
        .into_any_element()
}

/// `SettingsSwitch`.
pub fn switch_row(
    id: impl Into<ElementId>,
    text: RowText,
    value: bool,
    disabled: bool,
    on_change: impl Fn(bool, &mut Window, &mut App) + 'static,
    cx: &App,
) -> AnyElement {
    let switch = Switch::new(id)
        .checked(value)
        .disabled(disabled)
        .accessibility_label(text.label.clone())
        .on_click(move |checked: &bool, window, cx| on_change(*checked, window, cx));
    row(text, switch, cx)
}

/// The anchor of the select menu; the menu marks it selected while it is open.
#[derive(IntoElement)]
struct SelectTrigger {
    base: Stateful<Div>,
    open: bool,
    open_fill: Hsla,
}

impl Selectable for SelectTrigger {
    fn selected(mut self, selected: bool) -> Self {
        self.open = selected;
        self
    }

    fn is_selected(&self) -> bool {
        self.open
    }
}

impl Styled for SelectTrigger {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl InteractiveElement for SelectTrigger {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}

impl DropdownMenu for SelectTrigger {}

impl RenderOnce for SelectTrigger {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let fill = self.open_fill;
        self.base.when(self.open, |base| base.bg(fill))
    }
}

/// `SettingsSelect`: a `DropdownTrigger` of size `sm` with the selected label and a menu
/// that checks the selected option.
pub fn select_row(
    id: impl Into<ElementId>,
    text: RowText,
    options: Vec<SelectOption>,
    value: SharedString,
    disabled: bool,
    on_change: impl Fn(&str, &mut Window, &mut App) + 'static,
    cx: &App,
) -> AnyElement {
    let colors = tokens(cx);
    let shown = options
        .iter()
        .find(|option| option.id == value)
        .map(|option| option.label.clone())
        .unwrap_or_else(|| value.clone());
    let base = div()
        .id(id)
        .role(Role::Button)
        .aria_label(SharedString::from(format!("{}: {shown}", text.label)))
        .h(zoomed(CONTROL_SM, cx))
        .px(zoomed(SPACE_3, cx))
        .flex()
        .flex_row()
        .items_center()
        .gap(zoomed(SPACE_2, cx))
        .rounded(zoomed(RADIUS_XL, cx))
        .border_1()
        .border_color(colors.border_accent)
        .when(disabled, |this| this.opacity(0.5))
        .when(!disabled, |this| {
            this.cursor_pointer().hover(|style| style.opacity(0.85))
        })
        .child(
            div()
                .max_w(zoomed(SELECT_MENU_WIDTH, cx))
                .truncate()
                .text_size(ui_font_size(FONT_BASE, cx))
                .text_color(colors.foreground)
                .child(shown),
        )
        .child(
            div().pt(zoomed(1., cx)).child(
                icon(IconName::ChevronDown)
                    .with_size(zoomed(ICON_SM, cx))
                    .text_color(colors.foreground_muted),
            ),
        );
    let trigger = SelectTrigger {
        base,
        open: false,
        open_fill: colors.surface2,
    };
    if disabled {
        return row(text, trigger, cx);
    }
    let on_change = Rc::new(on_change);
    let menu_width = zoomed(SELECT_MENU_WIDTH, cx);
    let control = trigger.dropdown_menu_with_anchor(Anchor::TopRight, move |menu, _, _| {
        options.iter().fold(
            menu.min_w(menu_width).check_side(Side::Right),
            |menu, option| {
                let on_change = on_change.clone();
                let picked = option.id.clone();
                menu.item(
                    PopupMenuItem::new(option.label.clone())
                        .checked(option.id == value)
                        .on_click(move |_, window, cx| on_change(&picked, window, cx)),
                )
            },
        )
    });
    row(text, control, cx)
}

/// `SettingsInput`. The caller creates `input` and reads its text.
pub fn input_row(
    text: RowText,
    input: &Entity<InputState>,
    disabled: bool,
    cx: &App,
) -> AnyElement {
    let colors = tokens(cx);
    let field = Input::new(input)
        .aria_label(text.label.clone())
        .disabled(disabled)
        .px(zoomed(SPACE_3, cx))
        .rounded(zoomed(RADIUS_LG, cx))
        .bg(colors.surface2)
        .border_color(colors.border)
        .text_size(ui_font_size(FONT_BASE, cx));
    // `Styled::h`: the Input's own `h` sets only the multi-line height.
    row(
        text,
        div()
            .w(zoomed(SELECT_MENU_WIDTH, cx))
            .child(Styled::h(field, zoomed(CONTROL_SM, cx))),
        cx,
    )
}

/// `SettingsAction`: an outline button of size `sm`.
pub fn action_row(
    id: impl Into<ElementId>,
    text: RowText,
    action_label: impl Into<SharedString>,
    disabled: bool,
    on_press: impl Fn(&mut Window, &mut App) + 'static,
    cx: &App,
) -> AnyElement {
    let action_label = action_label.into();
    let button = Button::new(id)
        .outline()
        .small()
        .label(action_label.clone())
        .accessibility_label(action_label)
        .disabled(disabled)
        .on_click(move |_, window, cx| on_press(window, cx));
    row(text, button, cx)
}

/// One muted 13 px line, for loading and error states.
pub fn note(text: impl Into<SharedString>, cx: &App) -> AnyElement {
    let colors = tokens(cx);
    div()
        .text_size(ui_font_size(NOTE_SIZE, cx))
        .text_color(colors.foreground_muted)
        .child(text.into())
        .into_any_element()
}

/// The timeline's rich text style (`markdown-styles.ts` headings, 24 px block gaps, 1.45 fence
/// line height) without its code highlighter. Setting a style replaces the installed default,
/// so its colors are made again here.
pub fn markdown_style(cx: &App) -> TextViewStyle {
    let theme = cx.theme();
    let t = tokens(cx);
    let radius = theme.semantic_tokens().radius.md;
    let mut corners = StyleRefinement::default();
    corners.corner_radii.top_left = Some(radius.into());
    corners.corner_radii.top_right = Some(radius.into());
    corners.corner_radii.bottom_left = Some(radius.into());
    corners.corner_radii.bottom_right = Some(radius.into());
    let code = code_font_size(FONT_CODE, cx);
    let code_block = corners
        .clone()
        .relative()
        .top(zoomed(SPACE_3, cx))
        .mb(zoomed(SPACE_3, cx))
        .p(zoomed(SPACE_3, cx))
        .font_family(mono_font_family(cx))
        .text_size(code)
        .line_height((code * FENCE_LINE_HEIGHT).round());
    let mut table = corners.clone();
    table.overflow.x = Some(Overflow::Scroll);
    let table_head = StyleRefinement::default()
        .bg(theme.table_head)
        .text_color(theme.table_head_foreground);
    let content = content_font_size(cx);
    let border = t.border;
    let zoom = zoom(cx);
    TextViewStyle::default()
        .with_foreground(theme.foreground)
        .with_muted_foreground(theme.muted_foreground)
        .with_link(theme.link)
        .with_selection(theme.selection)
        .with_code_background(t.surface2)
        .with_border(theme.border)
        .with_paragraph_gap(rems(
            f32::from(zoomed(PARAGRAPH_GAP, cx)) / f32::from(rem_size(cx)),
        ))
        .with_code_block(code_block)
        .with_table(table)
        .with_table_head(table_head)
        .with_inline_code(HighlightStyle {
            background_color: Some(theme.accent),
            ..Default::default()
        })
        .with_dark(theme.is_dark())
        .with_heading(move |level| heading_style(level, content, border, zoom))
}

/// One heading level of `markdown-styles.ts`, as the timeline's `heading_style`.
fn heading_style(level: u8, content: Pixels, border: Hsla, zoom: f32) -> StyleRefinement {
    let block = BLOCK_GAP - SPACE_3;
    let (tier, top, bottom) = match level {
        1 => (FONT_4XL, SPACE_6, SPACE_3),
        2 => (FONT_3XL, SPACE_6, SPACE_3),
        3 => (FONT_2XL, SPACE_4, SPACE_2),
        4 => (FONT_XL, SPACE_4, SPACE_2),
        _ => (FONT_LG, SPACE_3, SPACE_1),
    };
    let size = (content * (tier / FONT_BASE)).round();
    let style = StyleRefinement::default()
        .text_size(size)
        .line_height((size * HEADING_LINE_HEIGHT).round())
        .pt(px(top * zoom));
    if level <= 2 {
        style
            .font_weight(FontWeight::BOLD)
            .pb(px(SPACE_2 * zoom))
            .mb(px((bottom + block) * zoom))
            .border_b_1()
            .border_color(border)
    } else {
        style
            .font_weight(FontWeight::SEMIBOLD)
            .pb(px((bottom + block) * zoom))
    }
}

/// Compact Markdown in the content font size with `markdown_style`.
pub fn markdown(id: impl Into<ElementId>, text: impl Into<SharedString>, cx: &App) -> TextView {
    TextView::markdown(id, text)
        .style(markdown_style(cx))
        .text_size(content_font_size(cx))
}

#[cfg(test)]
mod tests {
    // Not `super::*`: the glob brings in gpui's `test` macro, which shadows `#[test]`.
    use super::divider_above;

    #[test]
    fn card_draws_no_divider_above_the_first_row() {
        assert!(!divider_above(0));
        assert!(divider_above(1));
        assert!(divider_above(2));
    }
}
