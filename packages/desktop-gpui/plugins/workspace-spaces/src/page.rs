//! The Spaces page (`client/page.tsx`): the title, a Refresh button, the rename row, the
//! project cards of the selected Space, a swipe hint, and the numbered tabs with "+ New".
//! A horizontal drag across the project list switches Space.

use crate::model::{
    CatalogView, PageItem, adjacent_space, can_save_name, drag_direction, page_items,
    selected_space, subtitle,
};
use crate::state::SpacesState;
use bachuc_plugin_host::ui::{RowText, select_row};
use bachuc_plugin_host::{PluginContext, SelectOption};
use bachuc_theme::metrics::{
    FONT_BASE, FONT_XL, RADIUS_LG, SPACE_2, SPACE_3, SPACE_4, SPACE_6, WEIGHT_SEMIBOLD,
};
use bachuc_theme::{Tokens, tokens, ui_font_size};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::{h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

/// The page title is 24 px.
const TITLE_SIZE: f32 = 24.;
/// `minHeight: 100` of the project region.
const REGION_MIN_HEIGHT: f32 = 100.;
/// `minWidth: 44` of a numbered tab, and `minWidth: 120` of the rename field.
const TAB_MIN_WIDTH: f32 = 44.;
const RENAME_MIN_WIDTH: f32 = 120.;
/// The "Move to workspace" button's padding, and a workspace row's vertical padding.
const MOVE_PADDING: f32 = 6.;
const WORKSPACE_PADDING: f32 = 10.;
const DISABLED: f32 = 0.5;
const OVERDRAW: f32 = 400.;

const HINT: &str = "Swipe left or right across the project list to switch.";
const CATALOG_FAILED: &str = "Cannot refresh projects. Check the host connection and retry.";
const LOAD_FAILED: &str = "Cannot load Spaces. Refresh to retry; saved data is preserved.";

pub(crate) fn build(
    ctx: PluginContext,
    state: Entity<SpacesState>,
    window: &mut Window,
    cx: &mut App,
) -> AnyView {
    cx.new(|cx| SpacesPage::new(ctx, state, window, cx)).into()
}

struct SpacesPage {
    ctx: PluginContext,
    state: Entity<SpacesState>,
    list: ListState,
    items: Vec<PageItem>,
    /// The project whose "Move to workspace" select is open.
    moving: Option<String>,
    /// The Space being renamed.
    renaming: Option<String>,
    rename_input: Entity<InputState>,
    notice: String,
    /// Where a mouse drag across the project list started.
    drag_start: Option<(f32, f32)>,
    _subscriptions: Vec<Subscription>,
}

impl SpacesPage {
    fn new(
        ctx: PluginContext,
        state: Entity<SpacesState>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let rename_input = cx.new(|cx| InputState::new(window, cx));
        let subscriptions = vec![
            cx.observe(&state, |this, _, cx| this.sync(cx)),
            cx.subscribe_in(
                &rename_input,
                window,
                |this: &mut Self, _, event: &InputEvent, _, cx| {
                    if matches!(event, InputEvent::PressEnter { .. }) {
                        this.rename(cx);
                    }
                },
            ),
        ];
        let mut page = Self {
            ctx,
            state,
            list: ListState::new(0, ListAlignment::Top, px(OVERDRAW)),
            items: Vec::new(),
            moving: None,
            renaming: None,
            rename_input,
            notice: String::new(),
            drag_start: None,
            _subscriptions: subscriptions,
        };
        page.sync(cx);
        page
    }

    fn compute_items(&self, cx: &App) -> Vec<PageItem> {
        let state = self.state.read(cx);
        let (Some(spaces), Some(host)) = (state.spaces(), state.host()) else {
            return Vec::new();
        };
        let selected = selected_space(spaces, state.selection());
        let view = match (state.catalog(), state.catalog_error().is_empty()) {
            (None, true) => CatalogView::Pending,
            (None, false) => CatalogView::Failed(&[]),
            (Some(projects), true) => CatalogView::Ready(projects),
            (Some(projects), false) => CatalogView::Failed(projects),
        };
        page_items(view, host, spaces, selected, self.moving.as_deref())
    }

    /// Rebuilds the list rows; a changed row is measured again.
    fn sync(&mut self, cx: &mut Context<Self>) {
        let items = self.compute_items(cx);
        if items != self.items {
            self.list.splice(0..self.items.len(), items.len());
            self.items = items;
        }
        cx.notify();
    }

    fn select(&mut self, id: &str, cx: &mut Context<Self>) {
        self.state.update(cx, |state, cx| state.select(id, cx));
        self.moving = None;
        self.renaming = None;
        self.notice.clear();
        self.sync(cx);
    }

    fn start_rename(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some((id, name)) = self.state.read(cx).spaces().and_then(|spaces| {
            let id = selected_space(spaces, self.state.read(cx).selection());
            spaces
                .name_of(id)
                .map(|name| (id.to_string(), name.to_string()))
        }) else {
            return;
        };
        self.notice.clear();
        self.renaming = Some(id);
        self.rename_input.update(cx, |input, cx| {
            input.set_value(name, window, cx);
            input.focus(window, cx);
        });
        cx.notify();
    }

    fn rename(&mut self, cx: &mut Context<Self>) {
        let Some(id) = self.renaming.clone() else {
            return;
        };
        let name = self.rename_input.read(cx).value().to_string();
        let busy = self.state.read(cx).busy() || self.state.read(cx).saving(cx);
        if !can_save_name(&name, busy) {
            return;
        }
        let saved = self
            .state
            .update(cx, |state, cx| state.rename(id, name, cx));
        cx.spawn(async move |this, cx| {
            if saved.await {
                this.update(cx, |this, cx| {
                    this.renaming = None;
                    cx.notify();
                })
                .ok();
            }
        })
        .detach();
    }

    fn create(&mut self, cx: &mut Context<Self>) {
        let created = self.state.update(cx, |state, cx| state.create(cx));
        cx.spawn(async move |this, cx| {
            if created.await {
                this.update(cx, |this, cx| {
                    this.moving = None;
                    this.renaming = None;
                    this.notice.clear();
                    this.sync(cx);
                })
                .ok();
            }
        })
        .detach();
    }

    fn move_project(&mut self, project_id: String, target: String, cx: &mut Context<Self>) {
        let known = self
            .state
            .read(cx)
            .catalog()
            .is_some_and(|projects| projects.iter().any(|project| project.id == project_id));
        if !known {
            self.notice = "Project unavailable. Refresh and try again.".into();
            cx.notify();
            return;
        }
        let moved = self.state.update(cx, |state, cx| {
            state.move_page_project(project_id, target, cx)
        });
        cx.spawn(async move |this, cx| {
            if moved.await {
                this.update(cx, |this, cx| {
                    this.moving = None;
                    this.notice = "Project moved.".into();
                    this.sync(cx);
                })
                .ok();
            }
        })
        .detach();
    }

    fn reload(&mut self, cx: &mut Context<Self>) {
        self.state.update(cx, |state, cx| state.reload(cx));
    }

    /// A drag across the project list that passes `touch_direction` switches Space.
    fn finish_drag(&mut self, end: Point<Pixels>, cx: &mut Context<Self>) {
        let Some(start) = self.drag_start.take() else {
            return;
        };
        let end = (f32::from(end.x), f32::from(end.y));
        let Some(direction) = drag_direction(start, end) else {
            return;
        };
        let target = {
            let state = self.state.read(cx);
            state
                .spaces()
                .and_then(|spaces| adjacent_space(spaces, state.selection(), direction))
        };
        if let Some(id) = target {
            self.select(&id, cx);
        }
    }

    fn render_row(
        &mut self,
        index: usize,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Some(item) = self.items.get(index).cloned() else {
            return div().into_any_element();
        };
        let t = tokens(cx);
        let size = ui_font_size(FONT_BASE, cx);
        let body = match item {
            PageItem::Loading => div()
                .text_size(size)
                .text_color(t.foreground)
                .child("Loading projects…")
                .into_any_element(),
            PageItem::Failed => alert("spaces-catalog-failed", CATALOG_FAILED, t, cx),
            PageItem::Empty { first } => v_flex()
                .p(px(SPACE_6))
                .gap(px(SPACE_3))
                .child(
                    div()
                        .text_size(size)
                        .text_color(t.foreground)
                        .child("No projects in this workspace."),
                )
                .child(
                    div()
                        .text_size(size)
                        .text_color(t.foreground_muted)
                        .child("Open another workspace and use “Move to workspace” on a project."),
                )
                .children(first.map(|(id, name)| {
                    page_button("spaces-open-first", &format!("Open {name}"), false, t, cx)
                        .self_start()
                        .on_click(cx.listener(move |this, _, _, cx| this.select(&id, cx)))
                }))
                .into_any_element(),
            PageItem::Card { project, moving } => self.card(index, project, moving, t, cx),
        };
        div()
            .w_full()
            .pb(px(SPACE_3))
            .child(body)
            .into_any_element()
    }

    fn card(
        &self,
        index: usize,
        project: crate::model::CatalogProject,
        moving: bool,
        t: Tokens,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let size = ui_font_size(FONT_BASE, cx);
        let (spaces, selected, busy, failed) = {
            let state = self.state.read(cx);
            let spaces = state.spaces().cloned().unwrap_or_default();
            let selected = selected_space(&spaces, state.selection()).to_string();
            (
                spaces,
                selected,
                state.busy(),
                !state.catalog_error().is_empty(),
            )
        };
        let disabled = busy || failed;
        let move_id = project.id.clone();
        let page = cx.entity();
        let move_button = div()
            .id(("space-card-move", index))
            .role(Role::Button)
            .aria_label(format!("Move {} to workspace", project.name))
            .p(px(MOVE_PADDING))
            .text_size(size)
            .text_color(t.foreground)
            .when(disabled, |button| button.opacity(DISABLED))
            .when(!disabled, |button| {
                button.cursor_pointer().on_click(cx.listener({
                    let id = move_id.clone();
                    move |this, _, _, cx| {
                        this.moving = if this.moving.as_deref() == Some(id.as_str()) {
                            None
                        } else {
                            Some(id.clone())
                        };
                        this.sync(cx);
                    }
                }))
            })
            .child("Move to workspace");
        let options: Vec<SelectOption> = spaces
            .spaces
            .iter()
            .map(|space| SelectOption {
                id: space.id.clone().into(),
                label: space.name.clone().into(),
                description: None,
            })
            .collect();
        let project_id = project.id.clone();
        v_flex()
            .id(("space-card", index))
            .role(Role::Group)
            .aria_label(project.name.clone())
            .p(px(SPACE_4))
            .gap(px(SPACE_2))
            .rounded(px(bachuc_theme::metrics::RADIUS_XL))
            .bg(t.surface1)
            .child(
                h_flex()
                    .flex_wrap()
                    .justify_between()
                    .gap(px(SPACE_2))
                    .child(
                        div()
                            .text_size(ui_font_size(FONT_XL, cx))
                            .font_weight(FontWeight(WEIGHT_SEMIBOLD))
                            .text_color(t.foreground)
                            .child(project.name.clone()),
                    )
                    .child(move_button),
            )
            .child(
                div()
                    .truncate()
                    .text_size(size)
                    .text_color(t.foreground_muted)
                    .child(project.path.clone()),
            )
            .when(moving, |card| {
                card.child(select_row(
                    ("space-card-select", index),
                    RowText {
                        label: format!("Move {}", project.name).into(),
                        hint: None,
                        error: None,
                    },
                    options,
                    selected.clone().into(),
                    disabled,
                    move |target, _window, cx| {
                        let (id, target) = (project_id.clone(), target.to_string());
                        page.update(cx, |page, cx| page.move_project(id, target, cx));
                    },
                    cx,
                ))
            })
            .when(project.workspaces.is_empty(), |card| {
                card.child(
                    div()
                        .text_size(size)
                        .text_color(t.foreground_muted)
                        .child("No active coding workspaces."),
                )
            })
            .children(project.workspaces.iter().map(|workspace| {
                let id = workspace.id.clone();
                let label = format!("{} →", workspace.name);
                div()
                    .id(SharedString::from(format!(
                        "space-workspace-{}",
                        workspace.id
                    )))
                    .role(Role::Button)
                    .aria_label(label.clone())
                    .py(px(WORKSPACE_PADDING))
                    .cursor_pointer()
                    .text_size(size)
                    .text_color(t.foreground)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.ctx.open_workspace(&id, cx);
                    }))
                    .child(label)
            }))
            .into_any_element()
    }
}

impl Render for SpacesPage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = tokens(cx);
        let size = ui_font_size(FONT_BASE, cx);
        let (spaces, selection, host_label, busy, saving, loading, save_error) = {
            let state = self.state.read(cx);
            (
                state.spaces().cloned(),
                state.selection().map(str::to_string),
                state.host_label(cx),
                state.busy(),
                state.saving(cx),
                state.loading(),
                state.save_error().to_string(),
            )
        };
        let selected = spaces
            .as_ref()
            .map(|spaces| selected_space(spaces, selection.as_deref()).to_string());
        let header = h_flex()
            .items_center()
            .justify_between()
            .gap(px(SPACE_3))
            .child(
                v_flex()
                    .flex_1()
                    .child(
                        div()
                            .id("spaces-title")
                            .role(Role::Heading)
                            .aria_label("Spaces")
                            .text_size(ui_font_size(TITLE_SIZE, cx))
                            .font_weight(FontWeight(WEIGHT_SEMIBOLD))
                            .text_color(t.foreground)
                            .child("Spaces"),
                    )
                    .child(
                        div()
                            .text_size(size)
                            .text_color(t.foreground_muted)
                            .child(subtitle(&host_label, spaces.as_ref(), selection.as_deref())),
                    ),
            )
            .child(
                page_button("spaces-refresh", "Refresh", busy, t, cx).when(!busy, |button| {
                    button.on_click(cx.listener(|this, _, _, cx| this.reload(cx)))
                }),
            );
        let root = v_flex()
            .id("spaces-page")
            .role(Role::Group)
            .aria_label("Spaces")
            .size_full()
            .bg(t.surface0)
            .p(px(SPACE_6))
            .gap(px(SPACE_3))
            .child(header);
        let (Some(spaces), Some(selected)) = (spaces, selected) else {
            let text = if loading {
                "Loading Spaces…"
            } else {
                LOAD_FAILED
            };
            return root
                .child(alert("spaces-load-status", text, t, cx))
                .into_any_element();
        };

        let rename_row = if self.renaming.is_some() {
            let draft = self.rename_input.read(cx).value().to_string();
            let save_disabled = !can_save_name(&draft, busy || saving);
            h_flex()
                .flex_wrap()
                .gap(px(SPACE_2))
                .child(
                    div().flex_1().min_w(px(RENAME_MIN_WIDTH)).child(
                        Input::new(&self.rename_input)
                            .accessibility_id("spaces-rename-input")
                            .aria_label("Workspace name")
                            .disabled(busy)
                            .px(px(SPACE_3))
                            .rounded(px(RADIUS_LG))
                            .bg(t.surface1)
                            .text_size(size),
                    ),
                )
                .child(
                    page_button("spaces-rename-save", "Save", save_disabled, t, cx)
                        .when(!save_disabled, |button| {
                            button.on_click(cx.listener(|this, _, _, cx| this.rename(cx)))
                        }),
                )
                .child(
                    page_button("spaces-rename-cancel", "Cancel", busy, t, cx).when(
                        !busy,
                        |button| {
                            button.on_click(cx.listener(|this, _, _, cx| {
                                this.renaming = None;
                                cx.notify();
                            }))
                        },
                    ),
                )
                .into_any_element()
        } else {
            let disabled = busy || saving;
            page_button("spaces-rename", "Rename workspace", disabled, t, cx)
                .self_start()
                .when(!disabled, |button| {
                    button
                        .on_click(cx.listener(|this, _, window, cx| this.start_rename(window, cx)))
                })
                .into_any_element()
        };

        let region = div()
            .id("spaces-projects")
            .role(Role::List)
            .aria_label("Projects")
            .flex_1()
            .min_h(px(REGION_MIN_HEIGHT))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, event: &MouseDownEvent, _, _| {
                    this.drag_start =
                        Some((f32::from(event.position.x), f32::from(event.position.y)));
                }),
            )
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, event: &MouseUpEvent, _, cx| {
                    this.finish_drag(event.position, cx)
                }),
            )
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(|this, event: &MouseUpEvent, _, cx| {
                    this.finish_drag(event.position, cx)
                }),
            )
            .child(list(self.list.clone(), cx.processor(Self::render_row)).size_full());

        let tabs = h_flex()
            .id("spaces-tabs")
            .flex_1()
            .gap(px(SPACE_2))
            .overflow_x_scroll()
            .children(spaces.spaces.iter().enumerate().map(|(index, space)| {
                let active = space.id == selected;
                let id = space.id.clone();
                div()
                    .id(SharedString::from(format!("spaces-tab-{}", space.id)))
                    .role(Role::Tab)
                    .aria_label(space.name.clone())
                    .aria_selected(active)
                    .min_w(px(TAB_MIN_WIDTH))
                    .p(px(SPACE_3))
                    .flex()
                    .justify_center()
                    .rounded(px(RADIUS_LG))
                    .bg(t.surface1)
                    .border_2()
                    .border_color(if active {
                        t.foreground
                    } else {
                        hsla(0., 0., 0., 0.)
                    })
                    .cursor_pointer()
                    .text_size(size)
                    .text_color(t.foreground)
                    .on_click(cx.listener(move |this, _, _, cx| this.select(&id, cx)))
                    .child((index + 1).to_string())
            }));
        let create_disabled = busy || saving;
        let footer = h_flex().items_center().gap(px(SPACE_2)).child(tabs).child(
            page_button("spaces-create", "+ New", create_disabled, t, cx)
                .aria_label("Create workspace tab")
                .when(!create_disabled, |button| {
                    button.on_click(cx.listener(|this, _, _, cx| this.create(cx)))
                }),
        );

        root.when(!save_error.is_empty(), |page| {
            page.child(alert(
                "spaces-save-error",
                &format!("{save_error} Refresh before retrying."),
                t,
                cx,
            ))
        })
        .when(!self.notice.is_empty(), |page| {
            page.child(alert("spaces-notice", &self.notice, t, cx))
        })
        .child(rename_row)
        .child(region)
        .child(
            div()
                .text_size(size)
                .text_color(t.foreground_muted)
                .child(HINT),
        )
        .child(footer)
        .into_any_element()
    }
}

/// An alert line in the foreground color.
fn alert(id: &'static str, text: &str, t: Tokens, cx: &App) -> AnyElement {
    div()
        .id(id)
        .role(Role::Alert)
        .aria_label(text.to_string())
        .text_size(ui_font_size(FONT_BASE, cx))
        .text_color(t.foreground)
        .child(text.to_string())
        .into_any_element()
}

/// A page button: padding 12, radius 8, on `surface1`.
fn page_button(
    id: &'static str,
    label: &str,
    disabled: bool,
    t: Tokens,
    cx: &App,
) -> Stateful<Div> {
    div()
        .id(id)
        .role(Role::Button)
        .aria_label(label.to_string())
        .p(px(SPACE_3))
        .rounded(px(RADIUS_LG))
        .bg(t.surface1)
        .text_size(ui_font_size(FONT_BASE, cx))
        .text_color(t.foreground)
        .when(disabled, |button| button.opacity(DISABLED))
        .when(!disabled, |button| button.cursor_pointer())
        .child(label.to_string())
}
