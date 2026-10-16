//! The sidebar view. Sizes and states follow `sidebar-workspace-list.tsx` (lines 2541-2760),
//! `left-sidebar.tsx`, `sidebar-header-row.tsx`, and the row and menu files next to them.
//!
//! Hover, pressed, and the kebab use GPUI hover styles on a row group, so moving the pointer
//! redraws nothing in the view. The list rebuilds only when the tree changes, and a status change
//! that keeps every row's height redraws without a new measure.

use crate::model::{
    Group, ProjectRow, Row, RowShape, StatusBucket, ViewState, WorkspaceRow, archive_prompt,
    build_rows, changed_range, clamp_width, identity_color, project_initial, workspace_label,
};
use crate::ring::{FRAME as RING_FRAME, Knockout, RunningRing};
use bachuc_client::{Client, ClientEvent, DirectoryEvent};
use bachuc_icons::{IconName, icon};
use bachuc_theme::metrics::{
    FONT_BASE, FONT_SM, ICON_MD, ICON_SM, ICON_XS, RADIUS_BASE, RADIUS_LG, RADIUS_MD,
    SIDEBAR_WIDTH, SPACE_0_5, SPACE_1, SPACE_1_5, SPACE_2, SPACE_3, SPACE_4, SPACE_6,
    WEIGHT_MEDIUM,
};
use bachuc_theme::{Tokens, tokens, ui_font_size};
use gpui_kit::component::button::ButtonVariant;
use gpui_kit::component::dialog::DialogButtonProps;
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::component::menu::{DropdownMenu, PopupMenu, PopupMenuItem};
use gpui_kit::component::theme::Theme;
use gpui_kit::component::{Selectable, Sizable as _, WindowExt as _, h_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use std::cell::Cell;
use std::collections::{HashMap, HashSet};
use std::time::Duration;

/// How far the list draws beyond the visible rows.
const OVERDRAW: f32 = 400.;
/// `workspaceRow` and `projectRow` margins under a row.
const WORKSPACE_GAP: f32 = SPACE_0_5;
const PROJECT_GAP: f32 = SPACE_1;
/// `projectBlockExpanded` padding under a project block, and the `pinnedSection` margin.
const PROJECT_BLOCK_GAP: f32 = SPACE_3;
const PINNED_BLOCK_GAP: f32 = SPACE_1;
/// Row sizes: `minHeight` of project, workspace, and pinned header rows, and the title line.
const ROW_HEIGHT: f32 = 36.;
const LINE_HEIGHT: f32 = 20.;
/// The workspace kebab: 14 px icon, padding 2, so 18 px, pulled 7 px past the content rail.
const WORKSPACE_KEBAB: f32 = 18.;
/// The project kebab: 24 px, pulled 6 px past the row padding.
const PROJECT_KEBAB: f32 = 24.;
/// `TrailingActionScrim`: 48 px wide, solid after 55%.
const SCRIM_WIDTH: f32 = 48.;
const SCRIM_SOLID: f32 = 0.55;
/// The resize handle straddles the border, 10 px wide (`POINTER_HANDLE_WIDTH`).
const HANDLE_WIDTH: f32 = 10.;
/// A workspace that did not leave the tree after this long shows its row again.
const ARCHIVE_TIMEOUT: Duration = Duration::from_secs(10);
/// `STATUS_BADGE_SIZE` and `STATUS_BADGE_OFFSET` of the corner mark on a project icon.
const BADGE: f32 = 12.;
const BADGE_OFFSET: f32 = -4.;
/// `getStatusRingOffset(BADGE_OFFSET, BADGE)`: the ring grows around the badge center.
const RING_OFFSET: f32 = BADGE_OFFSET - (RING_FRAME - BADGE) / 2.;
/// `STATUS_INDICATOR_FILLED_DOT_SIZE`.
const DOT: f32 = 6.;

/// What the sidebar asks the app to do.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SidebarEvent {
    OpenWorkspace { workspace_id: String },
    OpenSettings,
    OpenCommandCenter,
}

impl EventEmitter<SidebarEvent> for Sidebar {}

#[derive(Clone, Copy)]
struct ResizeDrag;

/// What follows the pointer while the handle is dragged: nothing.
struct NoPreview;

impl Render for NoPreview {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

/// A small icon button that anchors a gpui-kit dropdown menu and stays visible while the menu is
/// open. It paints only a fill on hover; the glyph comes from its child.
#[derive(IntoElement)]
struct KebabButton {
    base: Stateful<Div>,
    open: bool,
    group: SharedString,
    fill: Hsla,
}

impl Selectable for KebabButton {
    fn selected(mut self, selected: bool) -> Self {
        self.open = selected;
        self
    }

    fn is_selected(&self) -> bool {
        self.open
    }
}

impl Styled for KebabButton {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl InteractiveElement for KebabButton {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}

impl DropdownMenu for KebabButton {}

impl RenderOnce for KebabButton {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let fill = self.fill;
        let group = self.group;
        self.base
            .when(self.open, |button| button.bg(fill))
            .when(!self.open, |button| {
                button
                    .invisible()
                    .group_hover(group, |style| style.visible())
            })
    }
}

pub struct Sidebar {
    client: Entity<Client>,
    selected: Option<String>,
    state: ViewState,
    rows: Vec<Row>,
    shapes: Vec<RowShape>,
    list: ListState,
    width: f32,
    rings: HashMap<String, Entity<RunningRing>>,
    archiving: HashSet<String>,
    _subscriptions: Vec<Subscription>,
}

impl Sidebar {
    pub fn new(client: Entity<Client>, _window: &mut Window, cx: &mut Context<Sidebar>) -> Sidebar {
        let subscriptions = vec![
            cx.subscribe(&client, |this, _, event: &ClientEvent, cx| match event {
                ClientEvent::Connected
                | ClientEvent::Disconnected { .. }
                | ClientEvent::AgentsChanged => this.refresh(cx),
                _ => {}
            }),
            cx.subscribe(&client, |this, _, event: &DirectoryEvent, cx| {
                if *event == DirectoryEvent::ProjectsChanged {
                    this.refresh(cx);
                }
            }),
            // set_theme and set_font_scale write the gpui-kit theme global.
            cx.observe_global::<Theme>(|_, cx| cx.notify()),
        ];
        let mut sidebar = Sidebar {
            client,
            selected: None,
            state: ViewState::default(),
            rows: Vec::new(),
            shapes: Vec::new(),
            list: ListState::new(0, ListAlignment::Top, px(OVERDRAW)),
            width: SIDEBAR_WIDTH,
            rings: HashMap::new(),
            archiving: HashSet::new(),
            _subscriptions: subscriptions,
        };
        sidebar.refresh(cx);
        sidebar
    }

    /// Marks the workspace the app shows, or none.
    pub fn set_selected_workspace(
        &mut self,
        workspace_id: Option<String>,
        cx: &mut Context<Sidebar>,
    ) {
        if self.selected != workspace_id {
            self.selected = workspace_id;
            cx.notify();
        }
    }

    /// Builds the rows again from the client and redraws only when something changed.
    fn refresh(&mut self, cx: &mut Context<Self>) {
        let client = self.client.read(cx);
        let rows = if client.projects().is_empty() && !client.is_connected() {
            // Nothing to say before the first answer.
            Vec::new()
        } else {
            build_rows(client.projects(), client.agents(), &self.state)
        };
        if rows == self.rows {
            return;
        }
        let shapes: Vec<RowShape> = rows.iter().map(Row::shape).collect();
        if let Some((range, count)) = changed_range(&self.shapes, &shapes) {
            self.list.splice(range, count);
        }
        let gone: Vec<String> = self
            .archiving
            .iter()
            .filter(|id| {
                !rows
                    .iter()
                    .any(|row| matches!(row, Row::Workspace(w) if &w.id == *id))
            })
            .cloned()
            .collect();
        for id in gone {
            self.archiving.remove(&id);
        }
        let live: HashSet<String> = rows.iter().filter_map(ring_key).collect();
        self.rings.retain(|key, _| live.contains(key));
        self.shapes = shapes;
        self.rows = rows;
        cx.notify();
    }

    fn toggle_project(&mut self, id: &str, cx: &mut Context<Self>) {
        if !self.state.collapsed_projects.remove(id) {
            self.state.collapsed_projects.insert(id.to_string());
        }
        self.refresh(cx);
    }

    fn toggle_pinned(&mut self, cx: &mut Context<Self>) {
        self.state.pinned_collapsed = !self.state.pinned_collapsed;
        self.refresh(cx);
    }

    fn toggle_group(&mut self, group: Group, cx: &mut Context<Self>) {
        if !self.state.expanded_groups.remove(&group) {
            self.state.expanded_groups.insert(group);
        }
        self.refresh(cx);
    }

    fn open_workspace(&mut self, id: &str, cx: &mut Context<Self>) {
        self.selected = Some(id.to_string());
        cx.emit(SidebarEvent::OpenWorkspace {
            workspace_id: id.to_string(),
        });
        cx.notify();
    }

    fn pin(&mut self, id: &str, pinned: bool, cx: &mut Context<Self>) {
        self.client
            .update(cx, |client, cx| client.pin_workspace(id, pinned, cx));
    }

    fn mark_unread(&mut self, id: &str, cx: &mut Context<Self>) {
        self.client
            .update(cx, |client, cx| client.mark_workspace_unread(id, cx));
    }

    fn rename(&mut self, id: String, current: String, window: &mut Window, cx: &mut Context<Self>) {
        let input = cx.new(|cx| InputState::new(window, cx).default_value(current));
        let weak = cx.weak_entity();
        // The builder runs again whenever the dialog layer is rebuilt; focus once.
        let focus_once = Cell::new(true);
        window.open_dialog(cx, move |dialog, window, cx| {
            if focus_once.replace(false) {
                input.update(cx, |input, cx| input.focus(window, cx));
            }
            let weak = weak.clone();
            let id = id.clone();
            let field = input.clone();
            dialog
                .title("Rename workspace")
                .w(px(420.))
                .child(Input::new(&input).aria_label("Workspace name"))
                .button_props(
                    DialogButtonProps::default()
                        .ok_text("Rename")
                        .show_cancel(true)
                        .on_ok(move |_, _, cx| {
                            let title = field.read(cx).value().trim().to_string();
                            if !title.is_empty() {
                                weak.update(cx, |this, cx| {
                                    this.client.update(cx, |client, cx| {
                                        client.rename_workspace(&id, &title, cx)
                                    });
                                })
                                .ok();
                            }
                            true
                        }),
                )
        });
    }

    fn copy_path(&mut self, path: String, window: &mut Window, cx: &mut Context<Self>) {
        cx.write_to_clipboard(ClipboardItem::new_string(path));
        window.push_notification("Path copied", cx);
    }

    /// Asks for a confirm only when the checkout lists changed files; a clean workspace goes at
    /// once. A failed status read also goes at once, as Electron does with no risk data.
    fn archive(
        &mut self,
        id: String,
        name: String,
        cwd: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.archiving.contains(&id) {
            return;
        }
        let status = self
            .client
            .update(cx, |client, cx| client.checkout_status(&cwd, cx));
        cx.spawn_in(window, async move |this, cx| {
            let files = match status.await {
                Ok(status) => status.files,
                Err(error) => {
                    log::warn!("checkout status of {cwd} failed: {error}");
                    Vec::new()
                }
            };
            this.update_in(cx, |this, window, cx| match archive_prompt(&name, &files) {
                None => this.start_archive(id, cx),
                Some(prompt) => {
                    let weak = cx.weak_entity();
                    window.open_alert_dialog(cx, move |dialog, _, _| {
                        let weak = weak.clone();
                        let id = id.clone();
                        dialog
                            .title(prompt.title.clone())
                            .description(prompt.message.clone())
                            .confirm()
                            .ok_text("Archive")
                            .ok_variant(ButtonVariant::Danger)
                            .cancel_text("Cancel")
                            .on_ok(move |_, _, cx| {
                                let id = id.clone();
                                weak.update(cx, |this, cx| this.start_archive(id, cx)).ok();
                                true
                            })
                    });
                }
            })
            .ok();
        })
        .detach();
    }

    fn start_archive(&mut self, id: String, cx: &mut Context<Self>) {
        self.client
            .update(cx, |client, cx| client.archive_workspace(&id, cx));
        self.archiving.insert(id.clone());
        cx.notify();
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(ARCHIVE_TIMEOUT).await;
            this.update(cx, |this, cx| {
                if this.archiving.remove(&id) {
                    cx.notify();
                }
            })
            .ok();
        })
        .detach();
    }

    fn ring(&mut self, key: String, knockout: Knockout, cx: &mut Context<Self>) -> AnyElement {
        self.rings
            .entry(key)
            .or_insert_with(|| cx.new(|cx| RunningRing::new(knockout, cx)))
            .clone()
            .into_any_element()
    }

    // ---- rows ----

    fn render_row(&mut self, ix: usize, _: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let Some(row) = self.rows.get(ix).cloned() else {
            return div().into_any_element();
        };
        let t = tokens(cx);
        let last = ix + 1 == self.rows.len();
        let body = match &row {
            Row::PinnedHeader { collapsed, .. } => self.render_pinned_header(*collapsed, &t, cx),
            Row::SectionTitle => render_section_title(&t, cx),
            Row::Project(project) => self.render_project(ix, project, &t, cx),
            Row::Workspace(workspace) => self.render_workspace(ix, workspace, &t, cx),
            Row::ShowMore {
                group, expanded, ..
            } => self.render_show_more(ix, group.clone(), *expanded, &t, cx),
            Row::Empty => render_empty(&t, cx),
        };
        let below = match &row {
            Row::Project(_) => PROJECT_GAP,
            Row::Workspace(WorkspaceRow {
                block_end, group, ..
            })
            | Row::ShowMore {
                block_end, group, ..
            } => WORKSPACE_GAP + block_gap(*block_end, group),
            Row::PinnedHeader {
                block_end: true, ..
            } => PINNED_BLOCK_GAP,
            _ => 0.,
        };
        // The list container pads 8 px at the sides, and 16 px under the last row.
        div()
            .w_full()
            .px(px(SPACE_2))
            .pb(px(below + if last { SPACE_4 } else { 0. }))
            .child(body)
            .into_any_element()
    }

    fn render_pinned_header(
        &mut self,
        collapsed: bool,
        t: &Tokens,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let muted = t.foreground_muted;
        let chevron = if collapsed {
            IconName::ChevronRight
        } else {
            IconName::ChevronDown
        };
        div()
            .id("sidebar-pinned-header")
            .role(Role::Button)
            .aria_label("Pinned")
            .aria_expanded(!collapsed)
            .group("pinned-header")
            .flex()
            .flex_row()
            .items_center()
            .self_start()
            .min_h(px(ROW_HEIGHT))
            .px(px(SPACE_2))
            .py(px(SPACE_1))
            .gap(px(SPACE_1))
            .text_size(ui_font_size(FONT_SM, cx))
            .text_color(muted)
            .cursor_pointer()
            .on_click(cx.listener(|this, _, _, cx| this.toggle_pinned(cx)))
            .child("Pinned")
            .child(
                div()
                    .invisible()
                    .group_hover("pinned-header", |style| style.visible())
                    .child(icon(chevron).with_size(px(ICON_XS))),
            )
            .into_any_element()
    }

    fn render_project(
        &mut self,
        ix: usize,
        project: &ProjectRow,
        t: &Tokens,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let group = SharedString::from(format!("project-{ix}"));
        let id = project.id.clone();
        let chevron = if project.collapsed {
            IconName::ChevronRight
        } else {
            IconName::ChevronDown
        };
        let badge =
            (project.collapsed && project.bucket != StatusBucket::Done).then_some(project.bucket);
        let ring = (badge == Some(StatusBucket::Running))
            .then(|| self.ring(format!("p:{}", project.id), Knockout::Sidebar, cx));
        let leading = div()
            .relative()
            .w(px(ICON_MD))
            .h(px(LINE_HEIGHT))
            .flex_shrink_0()
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .group_hover(group.clone(), |style| style.invisible())
                    .child(project_icon(
                        &project.id,
                        &project.name,
                        badge,
                        ring,
                        &group,
                        t,
                        cx,
                    )),
            )
            .child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .invisible()
                    .group_hover(group.clone(), |style| style.visible())
                    // `#9ca3af` in Electron; the closest token.
                    .text_color(t.foreground_extra_muted)
                    .child(icon(chevron).with_size(px(ICON_SM))),
            );
        let toggle_id = id.clone();
        let click = div()
            .id(("project", ix))
            .role(Role::Button)
            .aria_label(project.name.clone())
            .aria_expanded(!project.collapsed)
            .w_full()
            .min_h(px(ROW_HEIGHT))
            .p(px(SPACE_2))
            // The kebab sits on the right rail: 24 px pulled 6 px out, then the row gap.
            .pr(px(SPACE_2 + PROJECT_KEBAB - 6. + SPACE_2))
            .rounded(px(RADIUS_LG))
            .flex()
            .flex_row()
            .items_center()
            .gap(px(SPACE_2))
            .cursor_pointer()
            .hover(|style| style.bg(t.surface_sidebar_hover))
            .active(|style| style.bg(t.surface2))
            .on_click(cx.listener(move |this, _, _, cx| this.toggle_project(&toggle_id, cx)))
            .child(leading)
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_size(ui_font_size(FONT_BASE, cx))
                    .line_height(px(LINE_HEIGHT))
                    .text_color(t.foreground_muted)
                    .child(project.name.clone()),
            );
        let root = project.root.clone();
        let weak = cx.weak_entity();
        let kebab = kebab(
            ("project-kebab", ix),
            "Project actions",
            PROJECT_KEBAB,
            RADIUS_MD,
            group.clone(),
            t,
            move |menu, _, _| {
                menu.min_w(px(220.)).item(menu_item(
                    "Copy path",
                    IconName::Copy,
                    weak.clone(),
                    root.clone(),
                    |this, path, window, cx| this.copy_path(path, window, cx),
                ))
            },
        );
        div()
            .relative()
            .group(group)
            .w_full()
            .child(click)
            .child(
                div()
                    .absolute()
                    .top(px((ROW_HEIGHT - PROJECT_KEBAB) / 2.))
                    .right(px(SPACE_2 - 6.))
                    .child(kebab),
            )
            .into_any_element()
    }

    fn render_workspace(
        &mut self,
        ix: usize,
        row: &WorkspaceRow,
        t: &Tokens,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let group = SharedString::from(format!("workspace-{ix}"));
        let selected = self.selected.as_deref() == Some(row.id.as_str());
        let archiving = self.archiving.contains(&row.id);
        let pinned_section = row.group == Group::Pinned;
        let leading = if pinned_section {
            // A pinned row has no project row above it, so the project icon leads.
            let badge = (row.bucket != StatusBucket::Done).then_some(row.bucket);
            let ring = (badge == Some(StatusBucket::Running))
                .then(|| self.ring(format!("w:{}:icon", row.id), Knockout::Sidebar, cx));
            div()
                .w(px(ICON_MD))
                .h(px(LINE_HEIGHT))
                .flex_shrink_0()
                .flex()
                .items_center()
                .justify_center()
                .child(project_icon(
                    &row.project_id,
                    &row.project_name,
                    badge,
                    ring,
                    &group,
                    t,
                    cx,
                ))
                .into_any_element()
        } else {
            let ring = (row.bucket == StatusBucket::Running)
                .then(|| self.ring(format!("w:{}:dot", row.id), Knockout::None, cx));
            workspace_status(row, ring, t, cx)
        };
        let click_id = row.id.clone();
        let rest = if selected {
            t.surface_sidebar_selected
        } else {
            t.surface_sidebar_hover
        };
        let title = div()
            .flex_1()
            .min_w_0()
            .truncate()
            .text_size(ui_font_size(FONT_BASE, cx))
            .line_height(px(LINE_HEIGHT))
            .text_color(t.foreground.opacity(0.76))
            .group_hover(group.clone(), |style| style.text_color(t.foreground))
            .child(row.name.clone());
        let click = div()
            .id(("workspace", ix))
            .role(Role::Button)
            .aria_label(workspace_label(row, pinned_section))
            .aria_selected(selected)
            .w_full()
            .min_h(px(ROW_HEIGHT))
            .py(px(SPACE_2))
            .pl(px(SPACE_2))
            .pr(px(SPACE_3))
            .rounded(px(RADIUS_LG))
            .flex()
            .flex_row()
            .items_center()
            .gap(px(SPACE_2))
            .cursor_pointer()
            .when(selected, |this| this.bg(t.surface_sidebar_selected))
            .when(!selected, |this| {
                this.hover(|style| style.bg(t.surface_sidebar_hover))
            })
            .active(|style| style.bg(t.surface2))
            .when(!archiving, |this| {
                this.on_click(cx.listener(move |this, _, _, cx| this.open_workspace(&click_id, cx)))
            })
            .child(leading)
            .child(title);

        let weak = cx.weak_entity();
        let menu_row = row.clone();
        let kebab = kebab(
            ("workspace-kebab", ix),
            "Workspace actions",
            WORKSPACE_KEBAB,
            RADIUS_BASE,
            group.clone(),
            t,
            move |menu, _, _| workspace_menu(menu, &weak, &menu_row),
        );
        // The scrim fades the title tail into the row color under the kebab.
        let scrim = div()
            .absolute()
            .top(px(SPACE_2))
            .right(px(SPACE_3))
            .w(px(SCRIM_WIDTH))
            .h(px(LINE_HEIGHT))
            .bg(linear_gradient(
                90.,
                linear_color_stop(rest.opacity(0.), 0.),
                linear_color_stop(rest, SCRIM_SOLID),
            ))
            .invisible()
            .group_hover(group.clone(), |style| style.visible());
        div()
            .relative()
            .group(group)
            .w_full()
            .child(click)
            .when(!archiving, |this| {
                this.child(scrim).child(
                    div()
                        .absolute()
                        .top(px(SPACE_2))
                        .right(px(SPACE_3 - 7.))
                        .child(kebab),
                )
            })
            .when(archiving, |this| {
                this.child(
                    div()
                        .id(("workspace-archiving", ix))
                        .role(Role::Status)
                        .aria_label("Archiving")
                        .absolute()
                        .top_0()
                        .left_0()
                        .size_full()
                        .rounded(px(RADIUS_LG))
                        .bg(t.surface0.opacity(0.8))
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_size(ui_font_size(FONT_SM, cx))
                        .text_color(t.foreground_muted)
                        .child("Archiving..."),
                )
            })
            .into_any_element()
    }

    fn render_show_more(
        &mut self,
        ix: usize,
        group: Group,
        expanded: bool,
        t: &Tokens,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let label = if expanded { "Show less" } else { "Show more" };
        let chevron = if expanded {
            IconName::ChevronUp
        } else {
            IconName::ChevronDown
        };
        div()
            .id(("show-more", ix))
            .role(Role::Button)
            .aria_label(label)
            .w_full()
            .min_h(px(ROW_HEIGHT))
            .py(px(SPACE_2))
            .pl(px(SPACE_2))
            .pr(px(SPACE_3))
            .rounded(px(RADIUS_LG))
            .flex()
            .flex_row()
            .items_center()
            .gap(px(SPACE_2))
            .cursor_pointer()
            .text_size(ui_font_size(FONT_BASE, cx))
            .text_color(t.foreground_muted)
            .hover(|style| style.bg(t.surface_sidebar_hover).text_color(t.foreground))
            .active(|style| style.bg(t.surface2).text_color(t.foreground))
            .on_click(cx.listener(move |this, _, _, cx| this.toggle_group(group.clone(), cx)))
            .child(
                div()
                    .w(px(ICON_MD))
                    .h(px(ICON_MD))
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(icon(chevron).with_size(px(ICON_SM))),
            )
            .child(div().min_w_0().truncate().child(label))
            .into_any_element()
    }

    // ---- chrome ----

    fn render_nav(&self, t: &Tokens, cx: &mut Context<Self>) -> impl IntoElement {
        // `sidebarHeaderGroup` holds the rows. The sidebar keeps the one whose target is in
        // scope: Search, which opens the command center.
        div()
            .flex()
            .flex_col()
            .flex_shrink_0()
            .gap(px(SPACE_0_5))
            .pt(px(SPACE_2))
            .pb(px(SPACE_1_5))
            .border_b_1()
            .border_color(t.border)
            .child(
                div().px(px(SPACE_2)).child(
                    div()
                        .id("sidebar-search")
                        .role(Role::Button)
                        .aria_label("Search")
                        .min_h(px(28.))
                        .px(px(SPACE_2))
                        .py(px(SPACE_1))
                        .rounded(px(RADIUS_LG))
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(SPACE_2))
                        .cursor_pointer()
                        .text_size(ui_font_size(FONT_BASE, cx))
                        .text_color(t.foreground_muted)
                        .hover(|style| style.bg(t.surface_sidebar_hover).text_color(t.foreground))
                        .on_click(
                            cx.listener(|_, _, _, cx| cx.emit(SidebarEvent::OpenCommandCenter)),
                        )
                        .child(icon(IconName::Search).with_size(px(ICON_SM)))
                        .child("Search"),
                ),
            )
    }

    fn render_footer(&self, t: &Tokens, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .flex_shrink_0()
            .justify_end()
            .px(px(SPACE_2))
            .py(px(SPACE_3))
            .border_t_1()
            .border_color(t.border)
            .child(
                div()
                    .id("sidebar-settings")
                    .role(Role::Button)
                    .aria_label("Settings")
                    .w(px(28.))
                    .h(px(28.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .text_color(t.foreground_muted)
                    .hover(|style| style.text_color(t.foreground))
                    .on_click(cx.listener(|_, _, _, cx| cx.emit(SidebarEvent::OpenSettings)))
                    .child(icon(IconName::Settings).with_size(px(ICON_MD))),
            )
    }

    fn render_resize_handle(&self, t: &Tokens) -> impl IntoElement {
        div()
            .id("sidebar-resize-handle")
            .role(Role::Splitter)
            .aria_label("Resize sidebar")
            .aria_numeric_value(f64::from(self.width))
            .group("sidebar-resize")
            .absolute()
            .top_0()
            .bottom_0()
            .right(px(-HANDLE_WIDTH / 2.))
            .w(px(HANDLE_WIDTH))
            .cursor_col_resize()
            .on_drag(ResizeDrag, |_, _, _, cx| cx.new(|_| NoPreview))
            .child(
                // `highlight`: 1 px, foreground at 25%, 5 px in from the handle's left edge.
                div()
                    .absolute()
                    .top_0()
                    .bottom_0()
                    .left(px(HANDLE_WIDTH / 2.))
                    .w(px(1.))
                    .bg(t.foreground.opacity(0.25))
                    .invisible()
                    .group_hover("sidebar-resize", |style| style.visible()),
            )
    }
}

fn block_gap(block_end: bool, group: &Group) -> f32 {
    match (block_end, group) {
        (false, _) => 0.,
        (true, Group::Pinned) => PINNED_BLOCK_GAP,
        (true, Group::Project(_)) => PROJECT_BLOCK_GAP,
    }
}

/// The ring entity key of a row that shows one, so the view drops rings that stop running.
fn ring_key(row: &Row) -> Option<String> {
    match row {
        Row::Workspace(w) if w.bucket == StatusBucket::Running => {
            Some(if w.group == Group::Pinned {
                format!("w:{}:icon", w.id)
            } else {
                format!("w:{}:dot", w.id)
            })
        }
        Row::Project(p) if p.collapsed && p.bucket == StatusBucket::Running => {
            Some(format!("p:{}", p.id))
        }
        _ => None,
    }
}

fn render_section_title(t: &Tokens, cx: &mut Context<Sidebar>) -> AnyElement {
    // `workspacesSectionHeader`: the title with the display menu, which is out of scope.
    div()
        .id("sidebar-workspaces-title")
        .role(Role::Heading)
        .aria_label("Workspaces")
        .flex()
        .flex_row()
        .items_center()
        .min_h(px(SECTION_TITLE_HEIGHT))
        .pl(px(SPACE_2))
        .pr(px(SPACE_1))
        .py(px(SPACE_1))
        .text_size(ui_font_size(FONT_SM, cx))
        .text_color(t.foreground_muted)
        .child("Workspaces")
        .into_any_element()
}

/// The height of the "Workspaces" heading row: the 24 px display button plus its padding.
const SECTION_TITLE_HEIGHT: f32 = 32.;

/// `SidebarProjectEmptyState` without its two buttons (open project and import are out of scope).
fn render_empty(t: &Tokens, cx: &mut Context<Sidebar>) -> AnyElement {
    div()
        .id("sidebar-empty-state")
        .role(Role::Status)
        .aria_label("No projects yet")
        .mx(px(SPACE_2))
        .mt(px(SPACE_4))
        .pt(px(SPACE_6))
        .pb(px(SPACE_4))
        .px(px(SPACE_4))
        .rounded(px(RADIUS_LG))
        .bg(t.surface0)
        .flex()
        .flex_col()
        .items_center()
        .gap(px(SPACE_3))
        .text_size(ui_font_size(FONT_SM, cx))
        .text_center()
        .child(
            div()
                .font_weight(FontWeight(WEIGHT_MEDIUM))
                .text_color(t.foreground)
                .child("No projects yet"),
        )
        .child(
            div()
                .text_color(t.foreground_muted)
                .child("Add a project to get started"),
        )
        .into_any_element()
}

/// A disc of the alert color with a bang, which Electron draws as a filled `CircleAlert`.
fn alert_disc(side: f32, t: &Tokens) -> Div {
    div()
        .w(px(side))
        .h(px(side))
        .flex_shrink_0()
        .rounded_full()
        .bg(t.dot_warning)
        .flex()
        .items_center()
        .justify_center()
        .text_color(t.surface0)
        .text_size(px(side * 0.75))
        .font_weight(FontWeight::BOLD)
        .line_height(px(side))
        .child("!")
}

fn dot(color: Hsla) -> Div {
    div()
        .w(px(DOT))
        .h(px(DOT))
        .flex_shrink_0()
        .rounded_full()
        .bg(color)
}

/// The mark in the leading slot of a workspace row: `WorkspaceStatusIndicator`.
fn workspace_status(
    row: &WorkspaceRow,
    ring: Option<AnyElement>,
    t: &Tokens,
    _cx: &mut Context<Sidebar>,
) -> AnyElement {
    let slot = div()
        .relative()
        .w(px(ICON_MD))
        .h(px(LINE_HEIGHT))
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_center();
    match row.bucket {
        StatusBucket::Running => slot.children(ring).into_any_element(),
        StatusBucket::NeedsInput => slot.child(alert_disc(12., t)).into_any_element(),
        StatusBucket::Attention => slot.child(dot(t.dot_success)).into_any_element(),
        // An idle row keeps a muted dot so the rail has an edge to read against.
        StatusBucket::Done => slot
            .child(dot(t.foreground_extra_muted.opacity(0.3)))
            .into_any_element(),
        StatusBucket::Failed => slot
            .child(
                icon(IconName::Monitor)
                    .with_size(px(ICON_SM))
                    .text_color(t.foreground_muted),
            )
            .child(
                div()
                    .absolute()
                    .right_0()
                    .bottom_0()
                    .w(px(DOT + 2.))
                    .h(px(DOT + 2.))
                    .rounded_full()
                    .border_1()
                    .border_color(t.surface0)
                    .bg(t.dot_danger),
            )
            .into_any_element(),
    }
}

/// `ProjectStatusIndicator`: the lettered project box, with the status mark on its corner.
fn project_icon(
    key: &str,
    name: &str,
    badge: Option<StatusBucket>,
    ring: Option<AnyElement>,
    group: &SharedString,
    t: &Tokens,
    _cx: &mut Context<Sidebar>,
) -> AnyElement {
    let color: Rgba = rgb(identity_color(key));
    // `projectIconRadius`: a quarter of the box.
    let radius = (ICON_MD * 0.25).round();
    let mut icon_box = div().relative().w(px(ICON_MD)).h(px(ICON_MD)).child(
        div()
            .size_full()
            .rounded(px(radius))
            .bg(color)
            .flex()
            .items_center()
            .justify_center()
            .text_color(white())
            .text_size(px(9.))
            .child(project_initial(name)),
    );
    if let Some(bucket) = badge {
        icon_box = if bucket == StatusBucket::Running {
            icon_box.child(
                div()
                    .absolute()
                    .right(px(RING_OFFSET))
                    .bottom(px(RING_OFFSET))
                    .children(ring),
            )
        } else {
            let hover = t.surface_sidebar_hover;
            icon_box.child(
                // The shell is a knockout: it takes the color of the row behind it.
                div()
                    .absolute()
                    .right(px(BADGE_OFFSET))
                    .bottom(px(BADGE_OFFSET))
                    .w(px(BADGE))
                    .h(px(BADGE))
                    .rounded_full()
                    .overflow_hidden()
                    .bg(t.surface_sidebar)
                    .group_hover(group.clone(), move |style| style.bg(hover))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(match bucket {
                        StatusBucket::NeedsInput => alert_disc(BADGE, t),
                        StatusBucket::Failed => dot(t.dot_danger),
                        _ => dot(t.dot_success),
                    }),
            )
        };
    }
    icon_box.into_any_element()
}

/// One menu row: a muted icon, a label, and a handler that gets the view and a string.
fn menu_item(
    label: &'static str,
    name: IconName,
    weak: WeakEntity<Sidebar>,
    value: String,
    run: impl Fn(&mut Sidebar, String, &mut Window, &mut Context<Sidebar>) + 'static,
) -> PopupMenuItem {
    PopupMenuItem::new(label)
        .icon(icon(name).with_size(px(ICON_SM)))
        .on_click(move |_, window, cx| {
            let value = value.clone();
            weak.update(cx, |this, cx| run(this, value, window, cx))
                .ok();
        })
}

/// `SidebarWorkspaceMenuItems`, in the Electron order, for the items whose target is in scope.
fn workspace_menu(menu: PopupMenu, weak: &WeakEntity<Sidebar>, row: &WorkspaceRow) -> PopupMenu {
    let mut menu = menu.min_w(px(260.)).item(menu_item(
        "Copy path",
        IconName::Copy,
        weak.clone(),
        row.cwd.clone(),
        |this, path, window, cx| this.copy_path(path, window, cx),
    ));
    menu = menu.item(menu_item(
        "Rename workspace",
        IconName::Pencil,
        weak.clone(),
        row.id.clone(),
        {
            let current = row.name.clone();
            move |this, id, window, cx| this.rename(id, current.clone(), window, cx)
        },
    ));
    // `canMarkUnread`: a workspace with nothing to look at.
    if row.bucket == StatusBucket::Done {
        menu = menu.item(menu_item(
            "Mark as unread",
            IconName::Circle,
            weak.clone(),
            row.id.clone(),
            |this, id, _, cx| this.mark_unread(&id, cx),
        ));
    }
    let pinned = row.pinned;
    menu = menu.item(menu_item(
        if pinned { "Unpin" } else { "Pin to top" },
        if pinned {
            IconName::PinOff
        } else {
            IconName::Pin
        },
        weak.clone(),
        row.id.clone(),
        move |this, id, _, cx| this.pin(&id, !pinned, cx),
    ));
    let (name, cwd) = (row.name.clone(), row.cwd.clone());
    menu.item(menu_item(
        "Archive",
        IconName::Archive,
        weak.clone(),
        row.id.clone(),
        move |this, id, window, cx| this.archive(id, name.clone(), cwd.clone(), window, cx),
    ))
}

/// A hover kebab with its menu. `group` is the row's group, which shows the button.
fn kebab(
    id: (&'static str, usize),
    label: &'static str,
    side: f32,
    radius: f32,
    group: SharedString,
    t: &Tokens,
    build: impl Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu + 'static,
) -> impl IntoElement {
    let fg = t.foreground;
    let base = div()
        .id(id)
        .role(Role::Button)
        .aria_label(label)
        .w(px(side))
        .h(px(side))
        .rounded(px(radius))
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .text_color(t.foreground_muted)
        .hover(move |style| style.bg(t.surface2).text_color(fg))
        // The row under the kebab must not take the click as its own.
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .child(icon(IconName::MoreVertical).with_size(px(ICON_SM)));
    KebabButton {
        base,
        open: false,
        group,
        fill: t.surface2,
    }
    .dropdown_menu_with_anchor(Anchor::TopRight, build)
}

impl Render for Sidebar {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = tokens(cx);
        let tree = div()
            .id("sidebar-tree")
            .role(Role::Tree)
            .aria_label("Projects and workspaces")
            .flex_1()
            .min_h_0()
            .pt(px(SPACE_0_5))
            .child(list(self.list.clone(), cx.processor(Self::render_row)).size_full());
        div()
            .id("sidebar")
            .role(Role::Navigation)
            .aria_label("Sidebar")
            .relative()
            .flex()
            .flex_col()
            .h_full()
            .w(px(self.width))
            .flex_shrink_0()
            .bg(t.surface_sidebar)
            .border_r_1()
            .border_color(t.border)
            .text_color(t.foreground)
            .on_drag_move::<ResizeDrag>(cx.listener(
                |this, event: &DragMoveEvent<ResizeDrag>, _, cx| {
                    let width =
                        clamp_width(f32::from(event.event.position.x - event.bounds.origin.x));
                    if (width - this.width).abs() >= 0.5 {
                        this.width = width;
                        cx.notify();
                    }
                },
            ))
            .child(self.render_nav(&t, cx))
            .child(tree)
            .child(self.render_footer(&t, cx))
            // The handle hangs 5 px outside the sidebar, so it paints above its neighbor.
            .child(deferred(self.render_resize_handle(&t)).with_priority(1))
    }
}
