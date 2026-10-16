//! The Appearance pane: the theme and the interface and code font sizes. A change applies at
//! once and is stored in `appearance.json`.

use crate::appearance::{
    Appearance, CODE_RANGE, INTERFACE_RANGE, apply, load_from, parse_size, save_to, theme_label,
};
use crate::ui::{
    card, column, error_text, page_title, row, row_text, scroll_page, section, select_trigger,
};
use bachuc_icons::{IconName, icon};
use bachuc_theme::metrics::{FONT_BASE, ICON_MD, RADIUS_MD, SPACE_2, SPACE_3};
use bachuc_theme::{ThemeChoice, tokens, ui_font_size};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::menu::{DropdownMenu as _, PopupMenuItem};
use gpui_kit::component::theme::Theme;
use gpui_kit::component::{Side, Sizable as _, h_flex};
use gpui_kit::*;
use std::path::PathBuf;

/// The built-in themes in `THEME_OPTIONS` order.
const THEMES: [ThemeChoice; 3] = [
    ThemeChoice::Light,
    ThemeChoice::ClaudeDark,
    ThemeChoice::System,
];

/// `ThemeLeading`: sun, moon, or monitor.
fn theme_icon(theme: ThemeChoice) -> IconName {
    match theme {
        ThemeChoice::Light => IconName::Sun,
        ThemeChoice::ClaudeDark => IconName::Moon,
        ThemeChoice::System => IconName::Monitor,
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Size {
    Interface,
    Code,
}

pub struct AppearancePane {
    current: Appearance,
    interface: Entity<InputState>,
    code: Entity<InputState>,
    error: Option<SharedString>,
    dir: PathBuf,
    scroll: ScrollHandle,
    _subscriptions: Vec<Subscription>,
}

fn size_text(size: f32) -> String {
    format!("{}", size.round() as i32)
}

impl AppearancePane {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self::in_dir(bachuc_command::data_dir(), window, cx)
    }

    /// `new` with the folder of `appearance.json` given.
    pub fn in_dir(dir: PathBuf, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let current = load_from(&dir);
        let interface = cx
            .new(|cx| InputState::new(window, cx).default_value(size_text(current.interface_size)));
        let code =
            cx.new(|cx| InputState::new(window, cx).default_value(size_text(current.code_size)));
        let mut subscriptions = vec![cx.observe_global::<Theme>(|_, cx| cx.notify())];
        for (input, which) in [(&interface, Size::Interface), (&code, Size::Code)] {
            subscriptions.push(cx.subscribe_in(
                input,
                window,
                move |this: &mut AppearancePane, _, event: &InputEvent, window, cx| {
                    if matches!(event, InputEvent::Blur | InputEvent::PressEnter { .. }) {
                        this.commit_size(which, window, cx);
                    }
                },
            ));
        }
        Self {
            current,
            interface,
            code,
            error: None,
            dir,
            scroll: ScrollHandle::new(),
            _subscriptions: subscriptions,
        }
    }

    fn persist(&mut self, next: Appearance, cx: &mut Context<Self>) {
        self.current = next;
        apply(&next, cx);
        self.error = save_to(&self.dir, &next)
            .err()
            .map(|error| format!("Could not save the appearance: {error}").into());
        cx.notify();
    }

    fn set_theme(&mut self, theme: ThemeChoice, cx: &mut Context<Self>) {
        if self.current.theme != theme {
            self.persist(
                Appearance {
                    theme,
                    ..self.current
                },
                cx,
            );
        }
    }

    fn commit_size(&mut self, which: Size, window: &mut Window, cx: &mut Context<Self>) {
        let (input, range, committed) = match which {
            Size::Interface => (
                &self.interface,
                INTERFACE_RANGE,
                self.current.interface_size,
            ),
            Size::Code => (&self.code, CODE_RANGE, self.current.code_size),
        };
        let size = parse_size(&input.read(cx).value(), range).unwrap_or(committed);
        input.clone().update(cx, |input, cx| {
            input.set_value(size_text(size), window, cx);
        });
        if size != committed {
            let mut next = self.current;
            match which {
                Size::Interface => next.interface_size = size,
                Size::Code => next.code_size = size,
            }
            self.persist(next, cx);
        }
    }

    /// `ThemeRow`: a dropdown with the current theme; the menu checks the picked one.
    fn theme_dropdown(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let current = self.current.theme;
        let label = theme_label(current);
        let weak = cx.weak_entity();
        select_trigger(
            "theme-trigger",
            theme_icon(current),
            label,
            format!("Theme: {label}"),
            cx,
        )
        .dropdown_menu_with_anchor(Anchor::TopRight, move |menu, _, _| {
            THEMES.into_iter().fold(
                menu.min_w(px(200.)).check_side(Side::Right),
                |menu, theme| {
                    let weak = weak.clone();
                    // An element item: `PopupMenuItem::icon` forces an extra-small icon and the
                    // menu sets `text_sm`; the Electron rows use a 16 px icon and 14 px text.
                    menu.item(
                        PopupMenuItem::element(move |_, cx| {
                            h_flex()
                                .flex_1()
                                .items_center()
                                .gap(px(SPACE_2))
                                .text_size(ui_font_size(FONT_BASE, cx))
                                .child(
                                    icon(theme_icon(theme))
                                        .with_size(px(ICON_MD))
                                        .text_color(tokens(cx).foreground_muted),
                                )
                                .child(theme_label(theme))
                        })
                        .checked(theme == current)
                        .on_click(move |_, _, cx| {
                            weak.update(cx, |this, cx| this.set_theme(theme, cx)).ok();
                        }),
                    )
                },
            )
        })
    }

    fn size_row(
        &self,
        id: &'static str,
        title: &'static str,
        hint: &'static str,
        label: &'static str,
        input: &Entity<InputState>,
        first: bool,
        cx: &App,
    ) -> Div {
        let colors = tokens(cx);
        // `sizeInput`: 64 x 36, grey fill, right-aligned text. `Styled::h` because the Input's
        // own `h` sets only the multi-line height.
        let field = Input::new(input)
            .accessibility_id(id)
            .aria_label(label)
            .px(px(SPACE_3))
            .rounded(px(RADIUS_MD))
            .bg(colors.surface2)
            .border_color(colors.border)
            .text_size(ui_font_size(FONT_BASE, cx))
            .text_right();
        row(first, cx)
            .child(row_text(title, Some(hint.into()), cx))
            .child(
                h_flex()
                    .gap(px(SPACE_2))
                    .items_center()
                    .child(div().w(px(64.)).child(Styled::h(field, px(36.))))
                    .child(
                        div()
                            .text_size(ui_font_size(FONT_BASE, cx))
                            .text_color(colors.foreground_muted)
                            .child("px"),
                    ),
            )
    }
}

impl Render for AppearancePane {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let body = column()
            .child(page_title("Appearance", cx))
            .child(section(
                "Theme",
                None,
                card(cx).child(
                    row(true, cx)
                        .child(row_text("Theme", None, cx))
                        .child(self.theme_dropdown(cx)),
                ),
                cx,
            ))
            .child(section(
                "Fonts",
                None,
                card(cx)
                    .child(self.size_row(
                        "interface-size-input",
                        "Interface size",
                        "Used for navigation, controls, and labels",
                        "Interface font size",
                        &self.interface,
                        true,
                        cx,
                    ))
                    .child(self.size_row(
                        "code-size-input",
                        "Code size",
                        "Used for code, diffs, and terminal output",
                        "Code font size",
                        &self.code,
                        false,
                        cx,
                    )),
                cx,
            ))
            .children(self.error.clone().map(|error| error_text(error, cx)));
        scroll_page("appearance-pane", &self.scroll, body, cx)
    }
}
