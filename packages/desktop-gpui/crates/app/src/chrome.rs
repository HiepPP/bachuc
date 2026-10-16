//! The window chrome around the crate views, after the Electron window: the sidebar column's
//! title-bar row and host switch, the workspace header, the host gate, and the drag areas that
//! move the window (`titlebar-drag-region.tsx`). The window has a transparent title bar with the
//! traffic lights at (16, 10), so every top row leaves them `TRAFFIC_LIGHT_WIDTH` of room.

use crate::panes::chrome_button;
use crate::shell::Shell;
use crate::state::{agent_of_key, find_workspace, host_gate, host_option_label};
use bachuc_icons::{IconName, icon};
use bachuc_theme::metrics::{
    FONT_BASE, FONT_SM, HEADER_HEIGHT, ICON_SM, RADIUS_LG, SETTINGS_SIDEBAR_WIDTH, SPACE_1,
    SPACE_2, SPACE_3, SPACE_6, TRAFFIC_LIGHT_WIDTH, WEIGHT_MEDIUM,
};
use bachuc_theme::{tokens, ui_font_size};
use gpui_kit::component::menu::{DropdownMenu, PopupMenu, PopupMenuItem};
use gpui_kit::component::{Selectable, Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

/// An icon button that anchors a gpui-kit dropdown menu and keeps its fill while the menu is
/// open, as the sidebar kebab does.
#[derive(IntoElement)]
pub struct MenuButton {
    base: Stateful<Div>,
    open: bool,
    fill: Hsla,
}

impl Selectable for MenuButton {
    fn selected(mut self, selected: bool) -> Self {
        self.open = selected;
        self
    }

    fn is_selected(&self) -> bool {
        self.open
    }
}

impl Styled for MenuButton {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl InteractiveElement for MenuButton {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}

impl DropdownMenu for MenuButton {}

impl RenderOnce for MenuButton {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let fill = self.fill;
        self.base.when(self.open, |button| button.bg(fill))
    }
}

/// What follows the pointer while a divider is dragged: nothing.
pub struct NoPreview;

impl Render for NoPreview {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

/// The marker of an Explorer resize drag.
#[derive(Clone, Copy)]
pub struct ExplorerResize;

/// The Explorer resize band straddles its left border, 10 px wide (`POINTER_HANDLE_WIDTH`).
pub const HANDLE_WIDTH: f32 = 10.;

fn menu_item(
    label: &'static str,
    name: IconName,
    weak: WeakEntity<Shell>,
    run: impl Fn(&mut Shell, &mut Window, &mut Context<Shell>) + 'static,
) -> PopupMenuItem {
    PopupMenuItem::new(label)
        .icon(icon(name).with_size(px(ICON_SM)))
        .on_click(move |_, window, cx| {
            weak.update(cx, |shell, cx| run(shell, window, cx)).ok();
        })
}

impl Shell {
    /// An area that moves the window when dragged and zooms it on a double click, as the title
    /// bar does. Put it first, so the controls above it take their own clicks.
    pub(crate) fn drag_area(&self, id: &'static str, cx: &mut Context<Self>) -> Stateful<Div> {
        div()
            .id(id)
            .absolute()
            .inset_0()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _, _| this.dragging_window = true),
            )
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _, _, _| this.dragging_window = false),
            )
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(|this, _, _, _| this.dragging_window = false),
            )
            .on_mouse_move(cx.listener(|this, _, window, _| {
                if this.dragging_window {
                    this.dragging_window = false;
                    window.start_window_move();
                }
            }))
            .on_click(|event, window, _| {
                if event.click_count() == 2 {
                    window.titlebar_double_click();
                }
            })
    }

    /// The sidebar column: the title-bar row (`desktopChromeRow`), the host switch, then the
    /// sidebar view.
    pub(crate) fn render_sidebar_column(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let t = tokens(cx);
        let host_name = self.hosts[self.host_index].name.clone();
        let hosts_open = self.hosts_open;
        let options =
            hosts_open.then(|| {
                self.hosts
                    .iter()
                    .enumerate()
                    .map(|(ix, host)| {
                        let current = ix == self.host_index;
                        div()
                            .id(("host-option", ix))
                            .role(Role::Button)
                            .aria_label(host_option_label(host, current))
                            .aria_selected(current)
                            .mx(px(SPACE_2))
                            .px(px(SPACE_2))
                            .py(px(SPACE_1))
                            .rounded(px(RADIUS_LG))
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap(px(SPACE_2))
                            .cursor_pointer()
                            .overflow_hidden()
                            .text_size(ui_font_size(FONT_SM, cx))
                            .text_color(t.foreground)
                            .when(current, |row| row.bg(t.surface_sidebar_selected))
                            .hover(|row| row.bg(t.surface_sidebar_hover))
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.switch_host(ix, window, cx)
                            }))
                            .child(div().w(px(ICON_SM)).flex_shrink_0().when(current, |slot| {
                                slot.child(icon(IconName::Check).with_size(px(ICON_SM)))
                            }))
                            .child(host.name.clone())
                            .child(div().text_color(t.foreground_muted).child(host.url.clone()))
                    })
                    .collect::<Vec<_>>()
            });
        let chrome_row = div()
            .id("sidebar-chrome")
            .relative()
            .flex_shrink_0()
            .h(px(HEADER_HEIGHT))
            .bg(t.surface_sidebar)
            .border_r_1()
            .border_color(t.border)
            .child(self.drag_area("sidebar-drag", cx));
        let host_switch = div()
            .flex_shrink_0()
            .relative()
            .h(px(32.))
            .bg(t.surface_sidebar)
            .border_r_1()
            .border_color(t.border)
            .child(
                // Absolute, so a long host name never widens the column past the sidebar.
                div()
                    .id("host-switch")
                    .role(Role::Button)
                    .aria_label(format!("Host: {host_name}"))
                    .aria_expanded(hosts_open)
                    .absolute()
                    .top_0()
                    .bottom_0()
                    .left(px(SPACE_2))
                    .right(px(SPACE_2))
                    .px(px(SPACE_2))
                    .rounded(px(RADIUS_LG))
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(SPACE_2))
                    .cursor_pointer()
                    .overflow_hidden()
                    .text_size(ui_font_size(FONT_BASE, cx))
                    .text_color(t.foreground_muted)
                    .hover(|style| style.bg(t.surface_sidebar_hover))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.hosts_open = !this.hosts_open;
                        cx.notify();
                    }))
                    .child(icon(IconName::Server).with_size(px(ICON_SM)))
                    .child(div().flex_1().min_w_0().truncate().child(host_name))
                    .child(
                        icon(if hosts_open {
                            IconName::ChevronUp
                        } else {
                            IconName::ChevronDown
                        })
                        .with_size(px(ICON_SM)),
                    ),
            );
        v_flex()
            .id("sidebar-column")
            .flex_none()
            .h_full()
            .child(chrome_row)
            .child(host_switch)
            .when_some(options, |column, options| {
                column.child(
                    v_flex()
                        .id("host-options")
                        .role(Role::List)
                        .aria_label("Hosts")
                        .flex_shrink_0()
                        .relative()
                        .h(px(30. * options.len() as f32 + SPACE_1))
                        .bg(t.surface_sidebar)
                        .border_r_1()
                        .border_color(t.border)
                        .child(v_flex().absolute().inset_0().gap(px(2.)).children(options)),
                )
            })
            .child(div().flex_1().min_h_0().child(self.sidebar.clone()))
            .into_any_element()
    }

    /// The workspace header (`ScreenHeader` with `WorkspaceHeaderTitleBar`): sidebar toggle,
    /// workspace name and project, then the tab menu and the Explorer toggle.
    pub(crate) fn render_header(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let t = tokens(cx);
        let (title, subtitle) = {
            let client = self.client.read(cx);
            match find_workspace(client.projects(), &self.selected) {
                Some((project, workspace)) => (workspace.name.clone(), Some(project.name.clone())),
                None => ("New tab".to_string(), None),
            }
        };
        let subtitle = subtitle.filter(|project| *project != title);
        let has_workspace = self.selected_cwd().is_some();
        let active_agent = self
            .active_key(cx)
            .and_then(|key| self.pane_identity(&key))
            .and_then(|identity| agent_of_key(&identity).map(str::to_string));
        let weak = cx.weak_entity();
        let menu = MenuButton {
            base: chrome_button(
                "header-menu",
                "Workspace actions",
                IconName::MoreHorizontal,
                cx,
            ),
            open: false,
            fill: t.surface2,
        }
        .dropdown_menu_with_anchor(Anchor::TopRight, move |menu: PopupMenu, _, _| {
            let mut menu = menu.min_w(px(220.));
            if let Some(agent_id) = active_agent.clone() {
                let rename_id = agent_id.clone();
                menu = menu
                    .item(menu_item(
                        "Rename agent",
                        IconName::Pencil,
                        weak.clone(),
                        move |shell, window, cx| shell.rename_agent(rename_id.clone(), window, cx),
                    ))
                    .item(menu_item(
                        "Archive agent",
                        IconName::Archive,
                        weak.clone(),
                        move |shell, _, cx| shell.archive_agent(&agent_id, cx),
                    ))
                    .separator();
            }
            menu.item(menu_item(
                "New tab",
                IconName::Plus,
                weak.clone(),
                |shell, window, cx| shell.run_action("workspace.tab.menu.open", window, cx),
            ))
            .item(menu_item(
                "Split pane right",
                IconName::Columns2,
                weak.clone(),
                |shell, window, cx| shell.run_action("workspace.pane.split.right", window, cx),
            ))
            .item(menu_item(
                "Split pane down",
                IconName::Rows2,
                weak.clone(),
                |shell, window, cx| shell.run_action("workspace.pane.split.down", window, cx),
            ))
            .when(has_workspace, |menu| {
                menu.item(menu_item(
                    "Copy path",
                    IconName::Copy,
                    weak.clone(),
                    |shell, window, cx| shell.copy_workspace_path(window, cx),
                ))
            })
        });
        let explorer_open = self.layout.explorer_open;
        h_flex()
            .id("workspace-header")
            .role(Role::Banner)
            .aria_label("Workspace header")
            .relative()
            .flex_shrink_0()
            .h(px(HEADER_HEIGHT))
            .px(px(SPACE_3))
            .when(!self.layout.sidebar_open, |row| {
                row.pl(px(TRAFFIC_LIGHT_WIDTH + SPACE_3))
            })
            .gap(px(SPACE_2))
            .items_center()
            .justify_between()
            .bg(t.surface0)
            .border_b_1()
            .border_color(t.border)
            .child(self.drag_area("header-drag", cx))
            .child(
                h_flex()
                    .flex_1()
                    .min_w_0()
                    .items_center()
                    .gap(px(SPACE_2))
                    .child(
                        chrome_button(
                            "toggle-sidebar",
                            if self.layout.sidebar_open {
                                "Hide sidebar"
                            } else {
                                "Show sidebar"
                            },
                            IconName::PanelLeft,
                            cx,
                        )
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.run_action("sidebar.toggle.left", window, cx)
                        })),
                    )
                    .child(
                        div()
                            .id("workspace-title")
                            .role(Role::Heading)
                            .aria_label(match &subtitle {
                                Some(project) => format!("{title}, {project}"),
                                None => title.clone(),
                            })
                            .min_w_0()
                            .truncate()
                            .text_size(ui_font_size(FONT_BASE, cx))
                            .font_weight(FontWeight(WEIGHT_MEDIUM))
                            .text_color(t.foreground)
                            .child(title),
                    )
                    .when_some(subtitle, |row, project| {
                        row.child(
                            div()
                                .min_w_0()
                                .truncate()
                                .text_size(ui_font_size(FONT_BASE, cx))
                                .text_color(t.foreground_muted)
                                .child(project),
                        )
                    }),
            )
            .child(
                h_flex()
                    .flex_shrink_0()
                    .items_center()
                    .gap(px(SPACE_1))
                    .child(menu)
                    .when(has_workspace, |row| {
                        row.child(
                            chrome_button(
                                "toggle-explorer",
                                if explorer_open {
                                    "Hide Explorer"
                                } else {
                                    "Show Explorer"
                                },
                                IconName::PanelRight,
                                cx,
                            )
                            .aria_expanded(explorer_open)
                            .when(explorer_open, |button| button.bg(t.surface2))
                            .on_click(cx.listener(
                                |this, _, window, cx| {
                                    this.run_action("sidebar.toggle.right", window, cx)
                                },
                            )),
                        )
                    }),
            )
            .into_any_element()
    }

    /// Electron's host gate (`WorkspaceUnreachable`): who is offline, the status, the reason,
    /// Retry and Manage host. The client retries by itself; Retry reconnects at once.
    pub(crate) fn render_gate(&mut self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let host = self.hosts[self.host_index].name.clone();
        let (title, description) = host_gate(&self.connection, &host)?;
        let t = tokens(cx);
        let reason = match &self.connection {
            crate::state::Connection::Disconnected(reason) if !reason.is_empty() => {
                Some(reason.clone())
            }
            _ => None,
        };
        let offline = reason.is_some()
            || matches!(self.connection, crate::state::Connection::Disconnected(_));
        let button = |id: &'static str, label: &'static str, name: IconName, primary: bool| {
            h_flex()
                .id(id)
                .role(Role::Button)
                .aria_label(label)
                .h(px(28.))
                .px(px(SPACE_3))
                .gap(px(SPACE_1))
                .items_center()
                .rounded(px(RADIUS_LG))
                .border_1()
                .cursor_pointer()
                .text_size(ui_font_size(FONT_SM, cx))
                .when(primary, |b| {
                    b.bg(t.accent)
                        .border_color(t.accent)
                        .text_color(t.accent_foreground)
                })
                .when(!primary, |b| {
                    b.border_color(t.border)
                        .text_color(t.foreground)
                        .hover(|style| style.bg(t.surface2))
                })
                .child(icon(name).with_size(px(ICON_SM)))
                .child(label)
        };
        Some(
            v_flex()
                .id("host-gate")
                .role(Role::Status)
                .aria_label(format!("{title}. {description}"))
                .size_full()
                .items_center()
                .justify_center()
                .gap(px(SPACE_3))
                .px(px(SPACE_6))
                .child(
                    v_flex()
                        .items_center()
                        .gap(px(SPACE_2))
                        .max_w(px(520.))
                        .text_size(ui_font_size(FONT_BASE, cx))
                        .child(div().text_color(t.foreground).child(title))
                        .child(div().text_color(t.foreground_muted).child(description))
                        .when_some(reason, |stack, reason| {
                            stack.child(div().text_color(t.destructive).child(reason))
                        }),
                )
                .when(offline, |gate| {
                    gate.child(
                        h_flex()
                            .gap(px(SPACE_2))
                            .child(
                                button("gate-retry", "Retry", IconName::RotateCw, true).on_click(
                                    cx.listener(|this, _, window, cx| {
                                        this.reconnect(this.host_index, window, cx)
                                    }),
                                ),
                            )
                            .child(
                                button("gate-manage", "Manage host", IconName::Settings, false)
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.open_settings(window, cx)
                                    })),
                            ),
                    )
                })
                .into_any_element(),
        )
    }

    /// The settings screen under a title-bar row that keeps the traffic lights clear.
    pub(crate) fn render_settings(
        &mut self,
        settings: AnyView,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let t = tokens(cx);
        v_flex()
            .size_full()
            .child(
                h_flex()
                    .id("settings-chrome")
                    .relative()
                    .flex_shrink_0()
                    .h(px(HEADER_HEIGHT))
                    .bg(t.surface0)
                    .child(
                        div()
                            .w(px(SETTINGS_SIDEBAR_WIDTH))
                            .h_full()
                            .bg(t.surface_sidebar)
                            .border_r_1()
                            .border_color(t.border),
                    )
                    .child(self.drag_area("settings-drag", cx)),
            )
            .child(div().flex_1().min_h_0().child(settings))
            .into_any_element()
    }
}
