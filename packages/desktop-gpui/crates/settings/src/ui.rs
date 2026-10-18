//! Shared pieces of the settings panes: the page column, section headings, cards, rows, and
//! small controls, drawn with the Electron tokens (`styles/settings.ts`, `SettingsSection`).

use bachuc_icons::{IconName, icon};
use bachuc_theme::metrics::{
    CONTROL_SM, FONT_BASE, FONT_SM, ICON_SM, RADIUS_BASE, RADIUS_LG, RADIUS_MD, RADIUS_XL,
    SETTINGS_CONTENT_WIDTH, SPACE_1, SPACE_2, SPACE_3, SPACE_4, SPACE_6, WEIGHT_MEDIUM,
};
use bachuc_theme::{tokens, ui_font_size, zoomed};
use gpui_kit::component::menu::DropdownMenu;
use gpui_kit::component::{Selectable, Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

/// `theme.fontSize["4xl"]`, the page title size. `bachuc_theme::metrics` stops at `FONT_2XL`.
const FONT_4XL: f32 = 26.;

/// The browser's `normal` line height, which the Electron settings text uses. GPUI's default
/// (1.618) makes every text row taller: sidebar rows, card rows, and the page title.
pub const LINE_HEIGHT_NORMAL: f32 = 1.2;

/// The content column of a pane: centered, 720 px at most, padded 16 px (24 px above).
pub fn column(cx: &App) -> Div {
    v_flex()
        .w_full()
        .max_w(zoomed(SETTINGS_CONTENT_WIDTH, cx))
        .mx_auto()
        .p(zoomed(SPACE_4, cx))
        .pt(zoomed(SPACE_6, cx))
}

/// The page heading at the top of the column (`pageTitle` in `settings-screen.tsx`).
pub fn page_title(title: impl Into<SharedString>, cx: &App) -> Stateful<Div> {
    let colors = tokens(cx);
    let title = title.into();
    div()
        .id("page-title")
        .role(Role::Heading)
        .aria_label(title.clone())
        .ml(zoomed(SPACE_1, cx))
        .mb(zoomed(SPACE_6, cx))
        .text_size(ui_font_size(FONT_4XL, cx))
        .font_weight(FontWeight(WEIGHT_MEDIUM))
        .text_color(colors.foreground)
        .child(title)
}

/// The anchor of a gpui-kit dropdown menu. The menu marks it selected while it is open, and it
/// keeps its `open` fill then.
#[derive(IntoElement)]
pub struct MenuTrigger {
    base: Stateful<Div>,
    open: bool,
    open_fill: Hsla,
}

impl Selectable for MenuTrigger {
    fn selected(mut self, selected: bool) -> Self {
        self.open = selected;
        self
    }

    fn is_selected(&self) -> bool {
        self.open
    }
}

impl Styled for MenuTrigger {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl InteractiveElement for MenuTrigger {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}

impl DropdownMenu for MenuTrigger {}

impl RenderOnce for MenuTrigger {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let fill = self.open_fill;
        self.base.when(self.open, |base| base.bg(fill))
    }
}

/// `DropdownTrigger` size `sm`: the selected value after `leading` (if any), then a chevron, in
/// the outline button chrome. Chain `.dropdown_menu_with_anchor` on it.
pub fn select_trigger(
    id: impl Into<ElementId>,
    leading: Option<IconName>,
    value: impl Into<SharedString>,
    label: impl Into<SharedString>,
    cx: &App,
) -> MenuTrigger {
    let leading = leading.map(|name| {
        icon(name)
            .with_size(zoomed(bachuc_theme::metrics::ICON_MD, cx))
            .text_color(tokens(cx).foreground_muted)
            .into_any_element()
    });
    select_trigger_with(id, leading, value, label, cx)
}

/// [`select_trigger`] with any leading element, such as a [`swatch`].
pub fn select_trigger_with(
    id: impl Into<ElementId>,
    leading: Option<AnyElement>,
    value: impl Into<SharedString>,
    label: impl Into<SharedString>,
    cx: &App,
) -> MenuTrigger {
    let colors = tokens(cx);
    let base = div()
        .id(id)
        .role(Role::Button)
        .aria_label(label)
        .h(zoomed(CONTROL_SM, cx))
        .px(zoomed(SPACE_3, cx))
        .flex()
        .flex_row()
        .items_center()
        .gap(zoomed(SPACE_2, cx))
        .flex_shrink_0()
        .rounded(zoomed(RADIUS_XL, cx))
        .border_1()
        .border_color(colors.border_accent)
        .cursor_pointer()
        .hover(|style| style.opacity(0.85))
        .children(leading)
        .child(
            div()
                .max_w(zoomed(220., cx))
                .truncate()
                .text_size(ui_font_size(FONT_BASE, cx))
                .text_color(colors.foreground)
                .child(value.into()),
        )
        .child(
            div().pt(px(1.)).child(
                icon(IconName::ChevronDown)
                    .with_size(zoomed(ICON_SM, cx))
                    .text_color(colors.foreground_muted),
            ),
        );
    MenuTrigger {
        base,
        open: false,
        open_fill: colors.surface2,
    }
}

/// `ThemeSwatch`: a 16 px round dot of a theme's background with a 1 px `border` ring.
pub fn swatch(color: Hsla, cx: &App) -> Div {
    div()
        .size(zoomed(bachuc_theme::metrics::ICON_MD, cx))
        .flex_shrink_0()
        .rounded_full()
        .border_1()
        .border_color(tokens(cx).border)
        .bg(color)
}

/// The 32 px `...` button of a row menu (`ShortcutActionsMenu`). Chain
/// `.dropdown_menu_with_anchor` on it.
pub fn more_trigger(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    cx: &App,
) -> MenuTrigger {
    let colors = tokens(cx);
    let fg = colors.foreground;
    let base = div()
        .id(id)
        .role(Role::Button)
        .aria_label(label)
        .size(zoomed(CONTROL_SM, cx))
        .flex()
        .items_center()
        .justify_center()
        .flex_shrink_0()
        .rounded(zoomed(RADIUS_LG, cx))
        .cursor_pointer()
        .text_color(colors.foreground_muted)
        .hover(move |style| style.bg(colors.surface2).text_color(fg))
        .child(icon(IconName::MoreHorizontal).with_size(zoomed(ICON_SM, cx)));
    MenuTrigger {
        base,
        open: false,
        open_fill: colors.surface2,
    }
}

/// A scrolling pane that holds `body`, normally a `column`.
pub fn scroll_page(
    id: &'static str,
    scroll: &ScrollHandle,
    body: impl IntoElement,
    cx: &App,
) -> Stateful<Div> {
    let colors = tokens(cx);
    div()
        .id(id)
        .size_full()
        .overflow_y_scroll()
        .track_scroll(scroll)
        .bg(colors.surface0)
        .text_size(ui_font_size(FONT_BASE, cx))
        .text_color(colors.foreground)
        .child(body)
}

/// A section: a muted title, an optional trailing control, then the body.
pub fn section(title: &str, trailing: Option<AnyElement>, body: impl IntoElement, cx: &App) -> Div {
    let colors = tokens(cx);
    v_flex()
        .mb(zoomed(SPACE_6, cx))
        .child(
            h_flex()
                .items_center()
                .justify_between()
                .mb(zoomed(SPACE_3, cx))
                .ml(zoomed(SPACE_1, cx))
                .child(
                    div()
                        .id(SharedString::from(format!("heading-{title}")))
                        .role(Role::Heading)
                        .aria_label(title.to_string())
                        .text_size(ui_font_size(FONT_SM, cx))
                        .text_color(colors.foreground_muted)
                        .child(title.to_string()),
                )
                .children(trailing),
        )
        .child(body)
}

/// The rounded surface that holds rows.
pub fn card(cx: &App) -> Div {
    let colors = tokens(cx);
    v_flex()
        .w_full()
        .bg(colors.surface1)
        .rounded(zoomed(RADIUS_LG, cx))
        .border_1()
        .border_color(colors.border)
        .overflow_hidden()
}

/// One row of a card. `first` rows have no divider above them.
pub fn row(first: bool, cx: &App) -> Div {
    let colors = tokens(cx);
    h_flex()
        .w_full()
        .items_center()
        .justify_between()
        .gap(zoomed(SPACE_3, cx))
        .px(zoomed(SPACE_4, cx))
        .py(zoomed(SPACE_4, cx))
        .when(!first, |this| this.border_t_1().border_color(colors.border))
}

/// A row title above a muted hint; it takes the free width.
pub fn row_text(title: impl Into<SharedString>, hint: Option<SharedString>, cx: &App) -> Div {
    let colors = tokens(cx);
    v_flex()
        .flex_1()
        .min_w_0()
        .gap(zoomed(SPACE_1, cx))
        .child(
            div()
                .text_size(ui_font_size(FONT_BASE, cx))
                .text_color(colors.foreground)
                .truncate()
                .child(title.into()),
        )
        .children(hint.map(|hint| {
            div()
                .text_size(ui_font_size(FONT_SM, cx))
                .text_color(colors.foreground_muted)
                .truncate()
                .child(hint)
        }))
}

/// A muted line of text, for empty states and notes.
pub fn note(text: impl Into<SharedString>, cx: &App) -> Div {
    let colors = tokens(cx);
    div()
        .text_size(ui_font_size(FONT_BASE, cx))
        .text_color(colors.foreground_muted)
        .child(text.into())
}

/// A small error line in the destructive color.
pub fn error_text(text: impl Into<SharedString>, cx: &App) -> Stateful<Div> {
    let colors = tokens(cx);
    div()
        .id("error-text")
        .role(Role::Alert)
        .text_size(ui_font_size(FONT_SM, cx))
        .text_color(colors.destructive)
        .child(text.into())
}

/// An icon-only button, 28 px square, for row actions. Add `.on_click` at the call site.
pub fn icon_button(
    id: impl Into<ElementId>,
    name: IconName,
    label: impl Into<SharedString>,
    cx: &App,
) -> Stateful<Div> {
    let colors = tokens(cx);
    div()
        .id(id)
        .role(Role::Button)
        .aria_label(label)
        .size(zoomed(bachuc_theme::metrics::CONTROL_XS, cx))
        .flex()
        .items_center()
        .justify_center()
        .flex_shrink_0()
        .rounded(zoomed(RADIUS_LG, cx))
        .cursor_pointer()
        .hover(|style| style.bg(colors.surface2))
        .child(
            icon(name)
                .with_size(zoomed(ICON_SM, cx))
                .text_color(colors.foreground_muted),
        )
}

/// A status tag such as "Connected": tinted background, colored text.
pub fn tag(text: impl Into<SharedString>, color: Hsla, tint: Hsla, cx: &App) -> Div {
    div()
        .flex_shrink_0()
        .px(zoomed(SPACE_2, cx))
        .py(zoomed(SPACE_1 / 2., cx))
        .rounded(zoomed(RADIUS_BASE, cx))
        .bg(tint)
        .text_size(ui_font_size(FONT_SM, cx))
        .text_color(color)
        .child(text.into())
}

/// A field label above an input.
pub fn field_label(text: &str, cx: &App) -> Div {
    let colors = tokens(cx);
    div()
        .text_size(ui_font_size(FONT_SM, cx))
        .text_color(colors.foreground_muted)
        .child(text.to_string())
}

/// The border and surface of a callout (stale config, host removed, and so on).
pub fn callout(cx: &App) -> Div {
    let colors = tokens(cx);
    v_flex()
        .w_full()
        .gap(zoomed(SPACE_2, cx))
        .p(zoomed(SPACE_3, cx))
        .rounded(zoomed(RADIUS_MD, cx))
        .border_1()
        .border_color(colors.border)
        .bg(colors.surface2)
}
