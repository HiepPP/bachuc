//! The popover of the Skills pill, drawn like `client/popover.tsx` with the rows of
//! `client/menu-row.tsx`.

use crate::catalog::{CATALOG, toggle};
use crate::pins::Pins;
use bachuc_icons::{IconName, icon};
use bachuc_plugin_host::ComposerTarget;
use bachuc_theme::metrics::{FONT_BASE, FONT_SM, ICON_MD, RADIUS_MD, SPACE_1, SPACE_2};
use bachuc_theme::{tokens, ui_font_size};
use gpui_kit::component::{Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use std::rc::Rc;

/// The menu width. The composer's pill menus use 200 (`plugin_slots.rs`); this one is wider so
/// a description fits two lines. UNVERIFIED against Electron until the UI check.
const MENU_MIN_WIDTH: f32 = 240.;
/// `minHeight` of `MenuRow` for a pointer.
const ROW_MIN_HEIGHT: f32 = 28.;
/// The row's horizontal padding, `paddingHorizontal` of `MenuRow`.
const ROW_PAD_X: f32 = 9.;
/// The row's vertical padding, `paddingVertical` of `MenuRow`.
const ROW_PAD_Y: f32 = 5.;
/// The header and error lines' horizontal padding, `note` of `popover.tsx`.
const NOTE_PAD_X: f32 = 13.;
const LABEL_LINE: f32 = 18.;
const SMALL_LINE: f32 = 16.;
const CLEAR_ALL: &str = "Bỏ chọn tất cả";

/// What a row draws.
struct Row {
    id: ElementId,
    label: &'static str,
    description: Option<&'static str>,
    warn: Option<&'static str>,
    /// `Some` marks a toggle and shows a check when true.
    selected: Option<bool>,
    muted: bool,
    disabled: bool,
}

pub(crate) struct SkillsMenu {
    pins: Rc<Pins>,
    target: ComposerTarget,
}

impl SkillsMenu {
    pub(crate) fn new(pins: Rc<Pins>, target: ComposerTarget, cx: &mut Context<Self>) -> Self {
        pins.track_menu(cx.entity().downgrade());
        Self { pins, target }
    }

    fn save(&mut self, next: Vec<String>, cx: &mut Context<Self>) {
        self.pins.save(&self.target, next, cx);
    }

    fn row(
        &self,
        row: Row,
        on_press: Option<Box<dyn Fn(&mut Self, &mut Context<Self>)>>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let t = tokens(cx);
        let Row {
            id,
            label,
            description,
            warn,
            selected,
            muted,
            disabled,
        } = row;
        let pressable = !disabled;
        let base = h_flex()
            .id(id)
            .role(Role::MenuItem)
            .aria_label(label)
            .min_h(px(ROW_MIN_HEIGHT))
            .mx(px(SPACE_1))
            .px(px(ROW_PAD_X))
            .py(px(ROW_PAD_Y))
            .gap(px(SPACE_2))
            .items_center()
            .rounded(px(RADIUS_MD))
            .when(disabled, |row| row.opacity(0.5))
            .when(pressable, |row| {
                row.cursor_pointer().hover(move |row| row.bg(t.surface2))
            });
        let base = match selected {
            Some(selected) => base.aria_selected(selected),
            None => base,
        };
        let base = match on_press {
            Some(on_press) if pressable => {
                base.on_click(cx.listener(move |this, _, _, cx| on_press(this, cx)))
            }
            _ => base,
        };
        base.child(
            v_flex()
                .flex_1()
                .min_w_0()
                .child(
                    div()
                        .truncate()
                        .text_size(ui_font_size(FONT_BASE, cx))
                        .line_height(px(LABEL_LINE))
                        .text_color(if muted {
                            t.foreground_muted
                        } else {
                            t.foreground
                        })
                        .child(label),
                )
                .children(description.map(|text| {
                    div()
                        .mt(px(2.))
                        .line_clamp(2)
                        .text_size(ui_font_size(FONT_SM, cx))
                        .line_height(px(SMALL_LINE))
                        .text_color(t.foreground_muted)
                        .child(text)
                }))
                .children(warn.map(|text| {
                    div()
                        .line_clamp(2)
                        .text_size(ui_font_size(FONT_SM, cx))
                        .line_height(px(SMALL_LINE))
                        .text_color(t.status_warning)
                        .child(text)
                })),
        )
        .when(selected == Some(true), |row| {
            row.child(
                icon(IconName::Check)
                    .with_size(px(ICON_MD))
                    .text_color(t.foreground_muted),
            )
        })
        .into_any_element()
    }
}

impl Render for SkillsMenu {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = tokens(cx);
        let skills = self.pins.current(&self.target, cx);
        let error = self.pins.error(&self.target);
        let pinned = skills.clone().unwrap_or_default();
        let unknown = skills.is_none();
        let small = ui_font_size(FONT_SM, cx);
        let note = |text: String, color: Hsla| {
            div()
                .px(px(NOTE_PAD_X))
                .text_size(small)
                .line_height(px(SMALL_LINE))
                .text_color(color)
                .child(text)
        };
        let mut rows: Vec<AnyElement> = Vec::new();
        for (index, skill) in CATALOG.iter().enumerate() {
            let on = pinned.iter().any(|id| id == skill.id);
            let next = toggle(&pinned, skill.id);
            rows.push(self.row(
                Row {
                    id: ("skill-pins-row", index).into(),
                    label: skill.label,
                    description: Some(skill.description),
                    warn: skill.warn,
                    selected: Some(on),
                    muted: false,
                    disabled: unknown,
                },
                // A toggle keeps the menu open, so several skills can be picked in a row.
                Some(Box::new(move |this, cx| this.save(next.clone(), cx))),
                cx,
            ));
        }
        let clear = self.row(
            Row {
                id: "skill-pins-clear".into(),
                label: CLEAR_ALL,
                description: None,
                warn: None,
                selected: None,
                muted: true,
                disabled: pinned.is_empty(),
            },
            Some(Box::new(|this, cx| this.save(Vec::new(), cx))),
            cx,
        );
        v_flex()
            .id("skill-pins-menu")
            .role(Role::Menu)
            .aria_label("Pinned skills")
            .min_w(px(MENU_MIN_WIDTH))
            .py(px(SPACE_1))
            .child(
                div()
                    .pt(px(SPACE_2))
                    .pb(px(SPACE_1))
                    .child(note("Pinned for this chat".into(), t.foreground_muted)),
            )
            .children(error.map(|error| div().pb(px(SPACE_1)).child(note(error, t.status_danger))))
            .children(rows)
            .child(div().h(px(1.)).my(px(SPACE_1)).bg(t.border))
            .child(clear)
    }
}
