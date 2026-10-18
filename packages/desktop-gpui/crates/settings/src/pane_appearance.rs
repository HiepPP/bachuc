//! The Appearance pane: the theme, the fonts (families and sizes), and the syntax theme with a
//! live preview. A change applies at once and is stored in `appearance.json`.

use crate::appearance::{
    Appearance, CODE_RANGE, CONTENT_RANGE, DEFAULT_FAMILY, INTERFACE_RANGE, apply, family_choice,
    family_label, load_from, parse_size, save_to, tab_close_label, theme_label, with_builtin_theme,
    with_plugin_theme,
};
use crate::appearance_preview::preview;
use crate::plugins::{ThemeItem, TriggerChoice, plugin_themes, theme_items, trigger_choice};
use crate::ui::{
    card, column, error_text, page_title, row, row_text, scroll_page, section, select_trigger,
    select_trigger_with, swatch,
};
use bachuc_icons::{IconName, icon};
use bachuc_plugin_host::PluginHostEvent;
use bachuc_theme::metrics::{FONT_BASE, ICON_MD, RADIUS_MD, SPACE_2, SPACE_3, SPACE_4};
use bachuc_theme::{
    SyntaxThemeId, TabClosePosition, ThemeChoice, installed_font_families, tokens, ui_font_size,
    zoomed,
};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::menu::{DropdownMenu as _, PopupMenuItem};
use gpui_kit::component::theme::Theme;
use gpui_kit::component::{Side, Sizable as _, h_flex, v_flex};
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
    Content,
    Code,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Family {
    Interface,
    Code,
}

pub struct AppearancePane {
    current: Appearance,
    interface: Entity<InputState>,
    content: Entity<InputState>,
    code: Entity<InputState>,
    /// The families the pickers offer, read once when the pane opens.
    families: Vec<SharedString>,
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
        let content =
            cx.new(|cx| InputState::new(window, cx).default_value(size_text(current.content_size)));
        let code =
            cx.new(|cx| InputState::new(window, cx).default_value(size_text(current.code_size)));
        let mut subscriptions = vec![cx.observe_global::<Theme>(|_, cx| cx.notify())];
        if let Some(host) = bachuc_plugin_host::host(cx) {
            subscriptions.push(cx.subscribe(&host, |_, _, event: &PluginHostEvent, cx| {
                if matches!(event, PluginHostEvent::ContributionsChanged) {
                    cx.notify();
                }
            }));
        }
        for (input, which) in [
            (&interface, Size::Interface),
            (&content, Size::Content),
            (&code, Size::Code),
        ] {
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
            content,
            code,
            families: installed_font_families(cx),
            error: None,
            dir,
            scroll: ScrollHandle::new(),
            _subscriptions: subscriptions,
        }
    }

    fn persist(&mut self, next: Appearance, cx: &mut Context<Self>) {
        // The zoom shortcuts change the zoom while the pane shows; keep the level on screen.
        let next = Appearance {
            zoom: bachuc_theme::zoom(cx),
            ..next
        };
        apply(&next, cx);
        self.error = save_to(&self.dir, &next)
            .err()
            .map(|error| format!("Could not save the appearance: {error}").into());
        self.current = next;
        cx.notify();
    }

    fn set_theme(&mut self, theme: ThemeChoice, cx: &mut Context<Self>) {
        if self.current.theme != theme || self.current.plugin_theme.is_some() {
            self.persist(with_builtin_theme(&self.current, theme), cx);
        }
    }

    fn set_plugin_theme(&mut self, id: &str, cx: &mut Context<Self>) {
        if self.current.plugin_theme.as_deref() != Some(id) {
            self.persist(with_plugin_theme(&self.current, id), cx);
        }
    }

    fn set_syntax_theme(&mut self, syntax_theme: SyntaxThemeId, cx: &mut Context<Self>) {
        if self.current.syntax_theme != syntax_theme {
            let mut next = self.current.clone();
            next.syntax_theme = syntax_theme;
            self.persist(next, cx);
        }
    }

    fn set_tab_close_position(&mut self, position: TabClosePosition, cx: &mut Context<Self>) {
        if self.current.tab_close_position != position {
            let mut next = self.current.clone();
            next.tab_close_position = position;
            self.persist(next, cx);
        }
    }

    /// The tab close button side: Left or Right.
    fn tab_close_dropdown(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let current = self.current.tab_close_position;
        let weak = cx.weak_entity();
        select_trigger(
            "tab-close-trigger",
            None,
            tab_close_label(current),
            format!("Tab close button: {}", tab_close_label(current)),
            cx,
        )
        .dropdown_menu_with_anchor(Anchor::TopRight, move |menu, _, app| {
            [TabClosePosition::Left, TabClosePosition::Right]
                .into_iter()
                .fold(
                    menu.min_w(zoomed(200., app)).check_side(Side::Right),
                    |menu, position| {
                        menu.item(Self::text_item(
                            tab_close_label(position).into(),
                            position == current,
                            move |this, cx| this.set_tab_close_position(position, cx),
                            weak.clone(),
                        ))
                    },
                )
        })
    }

    /// `None` picks the default family.
    fn set_family(&mut self, which: Family, family: Option<SharedString>, cx: &mut Context<Self>) {
        let family = family.map(|name| name.to_string());
        let mut next = self.current.clone();
        match which {
            Family::Interface => next.ui_font = family,
            Family::Code => next.code_font = family,
        }
        if next != self.current {
            self.persist(next, cx);
        }
    }

    fn commit_size(&mut self, which: Size, window: &mut Window, cx: &mut Context<Self>) {
        let (input, range, committed) = match which {
            Size::Interface => (
                &self.interface,
                INTERFACE_RANGE,
                self.current.interface_size,
            ),
            Size::Content => (&self.content, CONTENT_RANGE, self.current.content_size),
            Size::Code => (&self.code, CODE_RANGE, self.current.code_size),
        };
        let size = parse_size(&input.read(cx).value(), range).unwrap_or(committed);
        input.clone().update(cx, |input, cx| {
            input.set_value(size_text(size), window, cx);
        });
        if size != committed {
            let mut next = self.current.clone();
            match which {
                Size::Interface => next.interface_size = size,
                Size::Content => next.content_size = size,
                Size::Code => next.code_size = size,
            }
            self.persist(next, cx);
        }
    }

    /// A menu row at the interface size, checked when it is the current choice.
    fn text_item(
        label: SharedString,
        checked: bool,
        on_pick: impl Fn(&mut Self, &mut Context<Self>) + 'static,
        weak: WeakEntity<Self>,
    ) -> PopupMenuItem {
        PopupMenuItem::element(move |_, cx| {
            div()
                .flex_1()
                .truncate()
                .text_size(ui_font_size(FONT_BASE, cx))
                .child(label.clone())
        })
        .checked(checked)
        .on_click(move |_, _, cx| {
            weak.update(cx, |this, cx| on_pick(this, cx)).ok();
        })
    }

    /// A picker of the installed families with "Default" first. A stored family that is not
    /// installed shows "Default", as the theme falls back to it.
    fn family_dropdown(
        &self,
        which: Family,
        id: &'static str,
        label: &'static str,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let stored = match which {
            Family::Interface => self.current.ui_font.as_deref(),
            Family::Code => self.current.code_font.as_deref(),
        };
        let selected = family_choice(stored, &self.families);
        let shown = family_label(stored, &self.families);
        let families = self.families.clone();
        let weak = cx.weak_entity();
        select_trigger(id, None, shown.clone(), format!("{label}: {shown}"), cx)
            .dropdown_menu_with_anchor(Anchor::TopRight, move |menu, _, app| {
                let menu = menu
                    .min_w(zoomed(240., app))
                    .max_h(zoomed(320., app))
                    .scrollable(true)
                    .check_side(Side::Right)
                    .item(Self::text_item(
                        DEFAULT_FAMILY.into(),
                        selected.is_none(),
                        move |this, cx| this.set_family(which, None, cx),
                        weak.clone(),
                    ));
                families.iter().fold(menu, |menu, family| {
                    let picked = family.clone();
                    menu.item(Self::text_item(
                        family.clone(),
                        selected.as_ref() == Some(family),
                        move |this, cx| this.set_family(which, Some(picked.clone()), cx),
                        weak.clone(),
                    ))
                })
            })
    }

    /// `SyntaxRow`: a dropdown of the eight highlight themes.
    fn syntax_dropdown(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let current = self.current.syntax_theme;
        let weak = cx.weak_entity();
        select_trigger(
            "syntax-theme-trigger",
            None,
            current.label(),
            format!("Highlight theme: {}", current.label()),
            cx,
        )
        .dropdown_menu_with_anchor(Anchor::TopRight, move |menu, _, app| {
            SyntaxThemeId::ALL.into_iter().fold(
                menu.min_w(zoomed(200., app)).check_side(Side::Right),
                |menu, theme| {
                    menu.item(Self::text_item(
                        theme.label().into(),
                        theme == current,
                        move |this, cx| this.set_syntax_theme(theme, cx),
                        weak.clone(),
                    ))
                },
            )
        })
    }

    /// `ThemeRow`: a dropdown with the current theme; the menu checks the picked one.
    fn theme_dropdown(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let current = self.current.theme;
        let themes = plugin_themes(cx);
        let selected = self.current.plugin_theme.as_deref();
        let shown = trigger_choice(current, selected, &themes);
        let label = shown.label();
        let id = "theme-trigger";
        let accessibility = format!("Theme: {label}");
        let trigger = match &shown {
            TriggerChoice::Builtin(theme) => {
                select_trigger(id, Some(theme_icon(*theme)), label, accessibility, cx)
            }
            TriggerChoice::Plugin { swatch: color, .. } => select_trigger_with(
                id,
                Some(swatch(*color, cx).into_any_element()),
                label,
                accessibility,
                cx,
            ),
        };
        let items = theme_items(current, selected, &themes, &THEMES);
        let weak = cx.weak_entity();
        trigger.dropdown_menu_with_anchor(Anchor::TopRight, move |menu, _, app| {
            items.iter().cloned().fold(
                menu.min_w(zoomed(200., app)).check_side(Side::Right),
                |menu, item| {
                    let weak = weak.clone();
                    // An element item: `PopupMenuItem::icon` forces an extra-small icon and the
                    // menu sets `text_sm`; the Electron rows use a 16 px icon and 14 px text.
                    match item {
                        ThemeItem::Separator => menu.separator(),
                        ThemeItem::Builtin { theme, checked } => menu.item(
                            PopupMenuItem::element(move |_, cx| {
                                h_flex()
                                    .flex_1()
                                    .items_center()
                                    .gap(zoomed(SPACE_2, cx))
                                    .text_size(ui_font_size(FONT_BASE, cx))
                                    .child(
                                        icon(theme_icon(theme))
                                            .with_size(zoomed(ICON_MD, cx))
                                            .text_color(tokens(cx).foreground_muted),
                                    )
                                    .child(theme_label(theme))
                            })
                            .checked(checked)
                            .on_click(move |_, _, cx| {
                                weak.update(cx, |this, cx| this.set_theme(theme, cx)).ok();
                            }),
                        ),
                        ThemeItem::Plugin {
                            id,
                            name,
                            swatch: color,
                            checked,
                        } => menu.item(
                            PopupMenuItem::element(move |_, cx| {
                                h_flex()
                                    .flex_1()
                                    .items_center()
                                    .gap(zoomed(SPACE_2, cx))
                                    .text_size(ui_font_size(FONT_BASE, cx))
                                    .child(swatch(color, cx))
                                    .child(div().flex_1().truncate().child(name.clone()))
                            })
                            .checked(checked)
                            .on_click(move |_, _, cx| {
                                weak.update(cx, |this, cx| this.set_plugin_theme(&id, cx))
                                    .ok();
                            }),
                        ),
                    }
                },
            )
        })
    }

    fn family_row(
        &self,
        which: Family,
        id: &'static str,
        title: &'static str,
        hint: &'static str,
        first: bool,
        cx: &mut Context<Self>,
    ) -> Div {
        let dropdown = self.family_dropdown(which, id, title, cx);
        row(first, cx)
            .child(row_text(title, Some(hint.into()), cx))
            .child(dropdown)
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
            .px(zoomed(SPACE_3, cx))
            .rounded(zoomed(RADIUS_MD, cx))
            .bg(colors.surface2)
            .border_color(colors.border)
            .text_size(ui_font_size(FONT_BASE, cx))
            .text_right();
        row(first, cx)
            .child(row_text(title, Some(hint.into()), cx))
            .child(
                h_flex()
                    .gap(zoomed(SPACE_2, cx))
                    .items_center()
                    .child(
                        div()
                            .w(zoomed(64., cx))
                            .child(Styled::h(field, zoomed(36., cx))),
                    )
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
        let interface_font = self.family_row(
            Family::Interface,
            "interface-font-trigger",
            "Interface font",
            "Used across the app. Pick Default for the system font",
            true,
            cx,
        );
        let code_font = self.family_row(
            Family::Code,
            "code-font-trigger",
            "Code font",
            "Used in code, diffs, and the terminal output. Pick Default for the system mono font",
            false,
            cx,
        );
        let body = column(cx)
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
                    .child(interface_font)
                    .child(self.size_row(
                        "interface-size-input",
                        "Interface size",
                        "Used for navigation, controls, and labels",
                        "Interface font size",
                        &self.interface,
                        false,
                        cx,
                    ))
                    .child(self.size_row(
                        "content-size-input",
                        "Content size",
                        "Used for chat text and rendered Markdown",
                        "Content font size",
                        &self.content,
                        false,
                        cx,
                    ))
                    .child(code_font)
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
            .child(section(
                "Syntax",
                None,
                v_flex()
                    .gap(zoomed(SPACE_4, cx))
                    .child(
                        card(cx).child(
                            row(true, cx)
                                .child(row_text(
                                    "Highlight theme",
                                    Some("Colors for code, independent of the app theme".into()),
                                    cx,
                                ))
                                .child(self.syntax_dropdown(cx)),
                        ),
                    )
                    .child(preview(
                        "Live preview of content typography, syntax theme, and code font",
                        "Readable content and code preview",
                        cx,
                    )),
                cx,
            ))
            .child(section(
                "Tabs",
                None,
                card(cx).child(
                    row(true, cx)
                        .child(row_text(
                            "Close button",
                            Some("Left shows it in place of the tab icon on hover".into()),
                            cx,
                        ))
                        .child(self.tab_close_dropdown(cx)),
                ),
                cx,
            ))
            .children(self.error.clone().map(|error| error_text(error, cx)));
        scroll_page("appearance-pane", &self.scroll, body, cx)
    }
}
