//! A Space tab's menu (`client/space-menu.tsx`), floating above the tab like the host's menus.
//! A press outside or Escape closes it. Pages: the menu, Rename, Remove (with a confirmation
//! that names the receiving Space), Move projects (every project with its Space), and the
//! Space to move one project to.

use crate::model::{
    MENU_EDGE, MENU_WIDTH, MenuPage, MoveProject, Space, can_save_name, menu_bottom, menu_heading,
    menu_left, order_projects, projects_here, remove_label, remove_prompt, remove_target_name,
};
use crate::state::SpacesState;
use bachuc_icons::{IconName, icon};
use bachuc_theme::metrics::{FONT_SM, ICON_XS, RADIUS_MD, SPACE_0_5, SPACE_1, SPACE_1_5, SPACE_2};
use bachuc_theme::{ShadowSize, Tokens, shadow, tokens, ui_font_size};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::{Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use std::ops::Range;

/// `borderRadius: 10` of the menu frame.
const MENU_RADIUS: f32 = 10.;
/// `maxHeight: 280` of the projects and target lists.
const LIST_MAX_HEIGHT: f32 = 280.;
const ROW_HEIGHT: f32 = 28.;
/// The trailing text: 11 px, at most 90 px wide.
const TRAILING_SIZE: f32 = 11.;
const TRAILING_MAX_WIDTH: f32 = 90.;
const ACTION_HEIGHT: f32 = 26.;
const ACTION_PADDING: f32 = 10.;
const INPUT_HEIGHT: f32 = 28.;
const DISABLED: f32 = 0.5;

/// The menu asks its owner to drop it.
pub(crate) struct MenuClosed;

pub(crate) struct SpaceMenu {
    state: Entity<SpacesState>,
    space_id: String,
    anchor: Bounds<Pixels>,
    page: MenuPage,
    /// The project whose Space the target page picks.
    moving: Option<MoveProject>,
    draft: Entity<InputState>,
    focus: FocusHandle,
    /// The rows of the projects and target lists, set on each draw for the lazy lists.
    ordered: Vec<MoveProject>,
    options: Vec<Space>,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<MenuClosed> for SpaceMenu {}

impl SpaceMenu {
    pub(crate) fn new(
        state: Entity<SpacesState>,
        space_id: String,
        anchor: Bounds<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let name = state
            .read(cx)
            .spaces()
            .and_then(|spaces| spaces.name_of(&space_id))
            .unwrap_or_default()
            .to_string();
        let draft = cx.new(|cx| InputState::new(window, cx).default_value(name));
        let focus = cx.focus_handle();
        focus.focus(window, cx);
        let subscriptions = vec![
            cx.observe(&state, |_, _, cx| cx.notify()),
            cx.subscribe_in(
                &draft,
                window,
                |this: &mut Self, _, event: &InputEvent, _, cx| {
                    if matches!(event, InputEvent::PressEnter { .. }) {
                        this.save_name(cx);
                    }
                },
            ),
        ];
        Self {
            state,
            space_id,
            anchor,
            page: MenuPage::Menu,
            moving: None,
            draft,
            focus,
            ordered: Vec::new(),
            options: Vec::new(),
            _subscriptions: subscriptions,
        }
    }

    pub(crate) fn space_id(&self) -> &str {
        &self.space_id
    }

    fn name(&self, cx: &App) -> String {
        self.state
            .read(cx)
            .spaces()
            .and_then(|spaces| spaces.name_of(&self.space_id))
            .unwrap_or_default()
            .to_string()
    }

    fn close(&mut self, cx: &mut Context<Self>) {
        cx.emit(MenuClosed);
    }

    fn show(&mut self, page: MenuPage, window: &mut Window, cx: &mut Context<Self>) {
        self.page = page;
        if page == MenuPage::Rename {
            let name = self.name(cx);
            self.draft.update(cx, |draft, cx| {
                draft.set_value(name, window, cx);
                draft.focus(window, cx);
            });
        } else {
            self.focus.focus(window, cx);
        }
        cx.notify();
    }

    fn save_name(&mut self, cx: &mut Context<Self>) {
        let name = self.draft.read(cx).value().to_string();
        let busy = self.state.read(cx).busy();
        if !can_save_name(&name, busy) {
            return;
        }
        let id = self.space_id.clone();
        let saved = self
            .state
            .update(cx, |state, cx| state.rename(id, name, cx));
        cx.spawn(async move |this, cx| {
            if saved.await {
                this.update(cx, |this, cx| this.close(cx)).ok();
            }
        })
        .detach();
    }

    fn remove(&mut self, cx: &mut Context<Self>) {
        let id = self.space_id.clone();
        let removed = self.state.update(cx, |state, cx| state.remove(id, cx));
        cx.spawn(async move |this, cx| {
            if removed.await {
                this.update(cx, |this, cx| this.close(cx)).ok();
            }
        })
        .detach();
    }

    /// Moves the project being picked to `space_id`; the Move projects page shows again once
    /// the save lands.
    fn move_to(&mut self, space_id: &str, cx: &mut Context<Self>) {
        let Some(project) = self.moving.clone() else {
            return;
        };
        if space_id == project.space_id {
            self.page = MenuPage::Projects;
            cx.notify();
            return;
        }
        let target = space_id.to_string();
        let moved = self.state.update(cx, |state, cx| {
            state.move_catalog_project_to(project.id, target, cx)
        });
        cx.spawn(async move |this, cx| {
            if moved.await {
                this.update(cx, |this, cx| {
                    this.page = MenuPage::Projects;
                    cx.notify();
                })
                .ok();
            }
        })
        .detach();
    }

    fn project_rows(&mut self, range: Range<usize>, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let busy = self.state.read(cx).busy();
        range
            .filter_map(|ix| self.ordered.get(ix).cloned().map(|project| (ix, project)))
            .map(|(ix, project)| {
                let space = self
                    .options
                    .iter()
                    .find(|space| space.id == project.space_id)
                    .map(|space| space.name.clone())
                    .unwrap_or_default();
                let picked = project.clone();
                menu_row(
                    ("space-menu-project", ix),
                    &project.name,
                    Some(space),
                    false,
                    false,
                    busy,
                    cx,
                )
                .when(!busy, |row| {
                    row.on_click(cx.listener(move |this, _, _, cx| {
                        this.moving = Some(picked.clone());
                        this.page = MenuPage::Target;
                        cx.notify();
                    }))
                })
                .into_any_element()
            })
            .collect()
    }

    fn target_rows(&mut self, range: Range<usize>, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let busy = self.state.read(cx).busy();
        let current = self.moving.as_ref().map(|project| project.space_id.clone());
        range
            .filter_map(|ix| self.options.get(ix).cloned().map(|space| (ix, space)))
            .map(|(ix, space)| {
                let id = space.id.clone();
                menu_row(
                    ("space-menu-target", ix),
                    &space.name,
                    None,
                    current.as_deref() == Some(space.id.as_str()),
                    false,
                    busy,
                    cx,
                )
                .when(!busy, |row| {
                    row.on_click(cx.listener(move |this, _, _, cx| this.move_to(&id, cx)))
                })
                .into_any_element()
            })
            .collect()
    }
}

/// A lazy list under a labeled list node.
fn lazy_list(id: &'static str, label: &'static str, list: impl IntoElement) -> AnyElement {
    div()
        .id(id)
        .role(Role::List)
        .aria_label(label)
        .w_full()
        .child(list)
        .into_any_element()
}

impl Render for SpaceMenu {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = tokens(cx);
        let size = ui_font_size(FONT_SM, cx);
        let (spaces, projects, busy) = {
            let state = self.state.read(cx);
            (state.spaces().cloned(), state.move_projects(), state.busy())
        };
        let Some(spaces) = spaces else {
            return div().into_any_element();
        };
        let name = spaces
            .name_of(&self.space_id)
            .unwrap_or_default()
            .to_string();
        let remove_target = remove_target_name(&spaces, &self.space_id);
        self.ordered = order_projects(&projects, &self.space_id);
        self.options = spaces.spaces.clone();
        let here = projects_here(&projects, &self.space_id);
        let heading = menu_heading(
            self.page,
            &name,
            self.moving.as_ref().map(|project| project.name.as_str()),
        );

        let back = matches!(self.page, MenuPage::Projects | MenuPage::Target).then(|| {
            let to = if self.page == MenuPage::Target {
                MenuPage::Projects
            } else {
                MenuPage::Menu
            };
            menu_row("space-menu-back", "‹ Back", None, false, false, false, cx)
                .on_click(cx.listener(move |this, _, window, cx| this.show(to, window, cx)))
        });

        let body: Vec<AnyElement> = match self.page {
            MenuPage::Projects => vec![lazy_list(
                "space-menu-projects",
                "Projects",
                uniform_list(
                    "space-menu-projects-list",
                    self.ordered.len(),
                    cx.processor(|this, range, _window, cx| this.project_rows(range, cx)),
                )
                .w_full()
                .max_h(px(LIST_MAX_HEIGHT)),
            )],
            MenuPage::Target if self.moving.is_some() => vec![lazy_list(
                "space-menu-targets",
                "Spaces",
                uniform_list(
                    "space-menu-targets-list",
                    self.options.len(),
                    cx.processor(|this, range, _window, cx| this.target_rows(range, cx)),
                )
                .w_full()
                .max_h(px(LIST_MAX_HEIGHT)),
            )],
            MenuPage::Menu | MenuPage::Target => {
                vec![
                    menu_row("space-menu-rename", "Rename…", None, false, false, busy, cx)
                        .when(!busy, |row| {
                            row.on_click(cx.listener(|this, _, window, cx| {
                                this.show(MenuPage::Rename, window, cx)
                            }))
                        })
                        .into_any_element(),
                    menu_row(
                        "space-menu-move",
                        "Move projects…",
                        Some(here.to_string()),
                        false,
                        false,
                        projects.is_empty(),
                        cx,
                    )
                    .when(!projects.is_empty(), |row| {
                        row.on_click(cx.listener(|this, _, window, cx| {
                            this.show(MenuPage::Projects, window, cx)
                        }))
                    })
                    .into_any_element(),
                    div()
                        .h(px(1.))
                        .my(px(SPACE_0_5))
                        .bg(t.border)
                        .into_any_element(),
                    {
                        let disabled = remove_target.is_none() || busy;
                        menu_row(
                            "space-menu-remove",
                            remove_label(remove_target.as_deref()),
                            None,
                            false,
                            remove_target.is_some(),
                            disabled,
                            cx,
                        )
                        .when(!disabled, |row| {
                            row.on_click(cx.listener(|this, _, window, cx| {
                                this.show(MenuPage::Remove, window, cx)
                            }))
                        })
                        .into_any_element()
                    },
                ]
            }
            MenuPage::Rename => {
                let draft = self.draft.read(cx).value().to_string();
                let disabled = !can_save_name(&draft, busy);
                vec![
                    v_flex()
                        .gap(px(SPACE_1_5))
                        .p(px(SPACE_1))
                        .child(Styled::h(
                            Input::new(&self.draft)
                                .accessibility_id("space-menu-name")
                                .aria_label("Space name")
                                .px(px(SPACE_2))
                                .rounded(px(RADIUS_MD))
                                .border_color(t.border)
                                .text_size(size),
                            px(INPUT_HEIGHT),
                        ))
                        .child(
                            actions()
                                .child(
                                    action_button("space-menu-cancel", "Cancel", None, false, t)
                                        .on_click(cx.listener(|this, _, window, cx| {
                                            this.show(MenuPage::Menu, window, cx)
                                        })),
                                )
                                .child(
                                    action_button(
                                        "space-menu-save",
                                        "Save",
                                        Some(t.accent),
                                        disabled,
                                        t,
                                    )
                                    .when(
                                        !disabled,
                                        |button| {
                                            button.on_click(
                                                cx.listener(|this, _, _, cx| this.save_name(cx)),
                                            )
                                        },
                                    ),
                                ),
                        )
                        .into_any_element(),
                ]
            }
            MenuPage::Remove => match remove_target.clone() {
                Some(target) => vec![
                    v_flex()
                        .gap(px(SPACE_2))
                        .p(px(SPACE_1))
                        .child(
                            div()
                                .text_size(size)
                                .text_color(t.foreground)
                                .child(remove_prompt(&name, &target)),
                        )
                        .child(
                            actions()
                                .child(
                                    action_button("space-menu-keep", "Cancel", None, false, t)
                                        .on_click(cx.listener(|this, _, window, cx| {
                                            this.show(MenuPage::Menu, window, cx)
                                        })),
                                )
                                .child(
                                    action_button(
                                        "space-menu-confirm-remove",
                                        "Remove",
                                        Some(t.status_danger),
                                        busy,
                                        t,
                                    )
                                    .when(!busy, |button| {
                                        button
                                            .on_click(cx.listener(|this, _, _, cx| this.remove(cx)))
                                    }),
                                ),
                        )
                        .into_any_element(),
                ],
                None => Vec::new(),
            },
        };

        let viewport = window.viewport_size();
        let left = menu_left(f32::from(self.anchor.origin.x), f32::from(viewport.width));
        let bottom = menu_bottom(f32::from(self.anchor.origin.y));
        let frame = v_flex()
            .id("space-menu")
            .role(Role::Menu)
            .aria_label(format!("{name} menu"))
            .track_focus(&self.focus)
            .w(px(MENU_WIDTH))
            .p(px(SPACE_1))
            .gap(px(SPACE_0_5))
            .border_1()
            .border_color(t.border)
            .rounded(px(MENU_RADIUS))
            .bg(t.surface0)
            .shadow(vec![shadow(ShadowSize::Lg, cx)])
            .occlude()
            .on_mouse_down_out(cx.listener(|this, _, _, cx| this.close(cx)))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                if event.keystroke.key == "escape" {
                    this.close(cx);
                    cx.stop_propagation();
                }
            }))
            .child(
                div()
                    .id("space-menu-heading")
                    .role(Role::Heading)
                    .aria_label(heading.clone())
                    .truncate()
                    .px(px(SPACE_2))
                    .pt(px(SPACE_1))
                    .pb(px(SPACE_0_5))
                    .text_size(ui_font_size(TRAILING_SIZE, cx))
                    .text_color(t.foreground_muted)
                    .child(heading),
            )
            .children(back)
            .children(body);
        deferred(
            anchored()
                .position(point(px(left), px(bottom)))
                .anchor(Anchor::BottomLeft)
                .snap_to_window_with_margin(px(MENU_EDGE))
                .child(frame),
        )
        .with_priority(gpui_kit::base::POPUP_PRIORITY)
        .into_any_element()
    }
}

/// One menu row: the label, an optional muted value at the trailing edge, and a check.
fn menu_row(
    id: impl Into<ElementId>,
    label: &str,
    trailing: Option<String>,
    checked: bool,
    danger: bool,
    disabled: bool,
    cx: &App,
) -> Stateful<Div> {
    let t = tokens(cx);
    h_flex()
        .id(id)
        .role(Role::MenuItem)
        .aria_label(label.to_string())
        .aria_selected(checked)
        .min_h(px(ROW_HEIGHT))
        .px(px(SPACE_2))
        .gap(px(SPACE_2))
        .items_center()
        .rounded(px(RADIUS_MD))
        .when(disabled, |row| row.opacity(DISABLED))
        .when(!disabled, |row| {
            row.cursor_pointer()
                .hover(move |style| style.bg(t.surface2))
        })
        .child(
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .text_size(ui_font_size(FONT_SM, cx))
                .text_color(if danger {
                    t.status_danger
                } else {
                    t.foreground
                })
                .child(label.to_string()),
        )
        .children(trailing.filter(|text| !text.is_empty()).map(|text| {
            div()
                .max_w(px(TRAILING_MAX_WIDTH))
                .truncate()
                .text_size(ui_font_size(TRAILING_SIZE, cx))
                .text_color(t.foreground_muted)
                .child(text)
        }))
        .when(checked, |row| {
            row.child(
                icon(IconName::Check)
                    .with_size(px(ICON_XS))
                    .text_color(t.foreground_muted),
            )
        })
}

fn actions() -> Div {
    h_flex().justify_end().gap(px(SPACE_1_5))
}

/// A 26 px button of the Rename and Remove pages: filled for `fill`, an outline otherwise.
fn action_button(
    id: &'static str,
    label: &'static str,
    fill: Option<Hsla>,
    disabled: bool,
    t: Tokens,
) -> Stateful<Div> {
    div()
        .id(id)
        .role(Role::Button)
        .aria_label(label)
        .h(px(ACTION_HEIGHT))
        .px(px(ACTION_PADDING))
        .flex()
        .items_center()
        .rounded(px(RADIUS_MD))
        .text_size(px(FONT_SM))
        .text_color(if fill.is_some() {
            t.accent_foreground
        } else {
            t.foreground
        })
        .when_some(fill, |button, fill| button.bg(fill))
        .when(fill.is_none(), |button| {
            button.border_1().border_color(t.border)
        })
        .when(disabled, |button| button.opacity(DISABLED))
        .when(!disabled, |button| button.cursor_pointer())
        .child(label)
}
