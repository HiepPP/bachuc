//! The window chrome around the crate views, after the Electron window: the sidebar column's
//! title-bar row and host switch, the workspace header, the host gate, and the drag areas that
//! move the window (`titlebar-drag-region.tsx`). The window has a transparent title bar with the
//! traffic lights at (16, 10), so every top row leaves them `TRAFFIC_LIGHT_WIDTH` of room.

use crate::panes::chrome_button;
use crate::shell::Shell;
use crate::state::{
    EditorTarget, agent_of_key, find_workspace, github_url, host_gate, host_option_label,
    open_args, script_state,
};
use bachuc_client::WorkspaceScript;
use bachuc_icons::{IconName, icon};
use bachuc_theme::metrics::{
    FONT_BASE, FONT_SM, HEADER_HEIGHT, ICON_SM, RADIUS_LG, RADIUS_MD, SETTINGS_SIDEBAR_WIDTH,
    SPACE_1, SPACE_2, SPACE_3, SPACE_6, TRAFFIC_LIGHT_WIDTH, WEIGHT_MEDIUM,
};
use bachuc_theme::{ShadowSize, shadow, tokens, ui_font_size};
use gpui_kit::component::menu::{DropdownMenu, PopupMenu, PopupMenuItem};
use gpui_kit::component::{Selectable, Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use std::rc::Rc;

/// The header menu that shows: the Scripts list or the Open in editor targets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HeaderMenu {
    Scripts,
    Editor,
}

/// `HEADER_CONTROL_HEIGHT` of `control-geometry.ts`, for the split buttons.
const HEADER_CONTROL_HEIGHT: f32 = 26.;
/// `MENU_ITEM_HEIGHT.md` of `menu-geometry.ts`.
const MENU_ITEM_HEIGHT: f32 = 28.;
const OPEN_FAILED: &str = "Failed to open workspace";

type ShellAction = Rc<dyn Fn(&mut Shell, &mut Window, &mut Context<Shell>)>;

/// A shell action that needs no window.
fn act(run: impl Fn(&mut Shell, &mut Context<Shell>) + 'static) -> ShellAction {
    Rc::new(move |shell, _, cx| run(shell, cx))
}

/// Runs `run` on a click and on an accessibility press.
fn pressable(base: Stateful<Div>, run: ShellAction, cx: &mut Context<Shell>) -> Stateful<Div> {
    let click = run.clone();
    let weak = cx.weak_entity();
    base.on_click(cx.listener(move |this, _, window, cx| click(this, window, cx)))
        .on_a11y_action(AccessibleAction::Click, move |_, window, cx| {
            weak.update(cx, |this, cx| run(this, window, cx)).ok();
        })
}

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
    /// workspace name and project, then the tab menu and the Explorer toggle. A plugin header
    /// subtitle replaces the project name; the New workspace screen adds its side panel toggle.
    pub(crate) fn render_header(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let t = tokens(cx);
        let (title, subtitle, subtitle_input) = {
            let client = self.client.read(cx);
            match find_workspace(client.projects(), &self.selected) {
                Some((project, workspace)) => (
                    workspace.name.clone(),
                    Some(project.name.clone()),
                    Some((
                        workspace.id.clone(),
                        project.id.clone(),
                        project.name.clone(),
                        project.root.clone(),
                    )),
                ),
                None => ("New tab".to_string(), None, None),
            }
        };
        let plugin_subtitle = self.plugin_subtitle(subtitle_input, window, cx);
        let subtitle = subtitle.filter(|project| *project != title && plugin_subtitle.is_none());
        let side_panel = self
            .active_new_workspace(cx)
            .filter(|screen| screen.read(cx).side_panel_available(cx))
            .map(|screen| screen.read(cx).side_panel_open())
            .map(|open| {
                chrome_button(
                    "new-workspace-side-panel-toggle",
                    if open {
                        "Hide side panel"
                    } else {
                        "Show side panel"
                    },
                    IconName::PanelRight,
                    cx,
                )
                .aria_expanded(open)
                .when(open, |button| button.bg(t.surface2))
                .on_click(cx.listener(|this, _, _, cx| this.toggle_side_panel(cx)))
            });
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
        // Electron's order: Scripts, Open in editor, git actions, then the Explorer toggle.
        let header_buttons = if has_workspace {
            self.render_header_buttons(cx)
        } else {
            Vec::new()
        };
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
                    })
                    .when_some(plugin_subtitle, |row, view| {
                        row.child(div().min_w_0().overflow_hidden().child(view))
                    }),
            )
            .child(
                h_flex()
                    .flex_shrink_0()
                    .items_center()
                    .gap(px(SPACE_1))
                    .children(side_panel)
                    .child(menu)
                    .children(header_buttons)
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

/// The header buttons of `workspace-screen.tsx` (Scripts, Open in editor, git actions), their
/// menus, and what they run.
impl Shell {
    fn render_header_buttons(&mut self, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let workspace_id = self.selected.clone();
        let scripts = self.client.read(cx).workspace_scripts(&workspace_id);
        let mut buttons = Vec::new();
        if !scripts.is_empty() {
            buttons.push(self.render_scripts_button(&workspace_id, scripts, cx));
        }
        if let Some(cwd) = self.selected_cwd().map(str::to_string) {
            buttons.push(self.render_editor_button(&cwd, cx));
        }
        // No primary action draws a zero-size button, as in Electron.
        let git = self.git_actions(&workspace_id);
        // A press must not also start a window drag in the title bar under it.
        buttons.extend(git.map(|button| {
            div()
                .flex_shrink_0()
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .child(button)
                .into_any_element()
        }));
        buttons
    }

    fn render_scripts_button(
        &mut self,
        workspace_id: &str,
        scripts: Vec<WorkspaceScript>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let t = tokens(cx);
        let open = self.header_menu == Some(HeaderMenu::Scripts);
        let running = scripts.iter().any(|script| script.lifecycle == "running");
        let trigger = h_flex()
            .id("header-scripts")
            .role(Role::Button)
            .aria_label("Workspace scripts")
            .aria_expanded(open)
            .h(px(28.))
            .px(px(SPACE_1))
            .gap(px(2.))
            .items_center()
            .rounded(px(RADIUS_MD))
            .cursor_pointer()
            .when(open, |button| button.bg(t.surface2))
            .hover(|style| style.bg(t.surface2))
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(
                icon(IconName::Play)
                    .with_size(px(ICON_SM))
                    .text_color(if running {
                        t.palette_blue_400
                    } else {
                        t.foreground_muted
                    }),
            )
            .child(
                icon(IconName::ChevronDown)
                    .with_size(px(ICON_SM))
                    .text_color(t.foreground_extra_muted),
            );
        let trigger = pressable(
            trigger,
            act(|this: &mut Shell, cx: &mut Context<Shell>| {
                this.toggle_header_menu(HeaderMenu::Scripts, cx)
            }),
            cx,
        );
        let menu = open.then(|| {
            let rows = scripts
                .into_iter()
                .enumerate()
                .map(|(ix, script)| self.script_row(workspace_id, ix, script, cx))
                .collect();
            header_popover("scripts-menu", "Workspace scripts", rows, cx)
        });
        div()
            .relative()
            .flex_shrink_0()
            .child(trigger)
            .children(menu)
            .into_any_element()
    }

    /// One script, in Electron's order: name and state, then View service, View terminal, and
    /// Restart while it runs, then Stop or Run. A running service with a URL adds a URL row.
    fn script_row(
        &mut self,
        workspace_id: &str,
        ix: usize,
        script: WorkspaceScript,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let t = tokens(cx);
        let running = script.lifecycle == "running";
        let service = script.kind != "script";
        let color = match (running, script.health.as_deref()) {
            (true, Some("healthy")) if service => t.status_success,
            (true, Some("unhealthy")) if service => t.status_danger,
            (true, _) => t.palette_blue_400,
            (false, _) => t.foreground_muted,
        };
        let busy_key = format!("{workspace_id}/{}", script.name);
        let busy = self.scripts_busy.contains(&busy_key);
        let name = script.name.clone();
        let url = script_url(&script).map(str::to_string);
        let mut actions = Vec::new();
        if let Some(url) = url.clone() {
            actions.push(script_action(
                ("script-open", ix),
                format!("View {name} service"),
                IconName::Eye,
                Some(open_url_action(url)),
                cx,
            ));
        }
        if running && let Some(terminal_id) = script.terminal_id.clone() {
            let name = name.clone();
            actions.push(script_action(
                ("script-view", ix),
                format!("View {name} terminal"),
                IconName::SquareTerminal,
                Some(Rc::new(move |this: &mut Shell, window, cx| {
                    this.view_script_terminal(&name, &terminal_id, window, cx)
                })),
                cx,
            ));
        }
        if running {
            let (workspace_id, name) = (workspace_id.to_string(), name.clone());
            let terminal_id = script.terminal_id.clone();
            actions.push(script_action(
                ("script-restart", ix),
                format!("Restart {name}"),
                IconName::RotateCw,
                (!busy).then(|| {
                    act(move |this: &mut Shell, cx: &mut Context<Shell>| {
                        this.restart_script(&workspace_id, &name, terminal_id.as_deref(), cx)
                    })
                }),
                cx,
            ));
        }
        let (label, glyph) = if running {
            (format!("Stop {name}"), IconName::Square)
        } else {
            (format!("Run {name} script"), IconName::Play)
        };
        let lifecycle = (!busy).then(|| {
            let (workspace_id, name) = (workspace_id.to_string(), name.clone());
            let terminal_id = script.terminal_id.clone();
            act(move |this: &mut Shell, cx: &mut Context<Shell>| {
                if running {
                    this.stop_script(&name, terminal_id.as_deref(), cx);
                } else {
                    this.start_script(&workspace_id, &name, cx);
                }
            })
        });
        actions.push(script_action(
            ("script-action", ix),
            label,
            glyph,
            lifecycle,
            cx,
        ));
        let state = script_state(&script);
        let header = h_flex()
            .h(px(MENU_ITEM_HEIGHT))
            .px(px(SPACE_2))
            .gap(px(SPACE_2))
            .items_center()
            .child(
                icon(if service {
                    IconName::Globe
                } else {
                    IconName::SquareTerminal
                })
                .with_size(px(ICON_SM))
                .text_color(color)
                .flex_shrink_0(),
            )
            .child(
                div()
                    .min_w_0()
                    .truncate()
                    .text_color(if running {
                        t.foreground
                    } else {
                        t.foreground_muted
                    })
                    .child(name.clone()),
            )
            .child(
                div()
                    .flex_shrink_0()
                    .text_color(
                        if script.exit_code.is_some_and(|code| code != 0) && !running {
                            t.status_danger
                        } else {
                            t.foreground_extra_muted
                        },
                    )
                    .child(state.clone()),
            )
            .child(div().flex_1())
            .children(actions);
        let url_row = url.map(|url| {
            let label = url_label(&url).to_string();
            let row = h_flex()
                .id(("script-url", ix))
                .role(Role::Button)
                .aria_label(format!("Open {label}"))
                .h(px(MENU_ITEM_HEIGHT))
                .mx(px(SPACE_1))
                .pl(px(SPACE_2 + ICON_SM + SPACE_2 - SPACE_1))
                .pr(px(SPACE_2))
                .items_center()
                .rounded(px(RADIUS_MD))
                .cursor_pointer()
                .text_color(t.foreground_muted)
                .hover(|style| style.bg(t.surface2).text_color(t.foreground))
                .child(div().min_w_0().truncate().child(label));
            pressable(row, open_url_action(url), cx)
        });
        v_flex()
            .id(("script-row", ix))
            .role(Role::Group)
            .aria_label(format!("{name} script, {state}"))
            .text_size(ui_font_size(FONT_SM, cx))
            .line_height(relative(1.3))
            .child(header)
            .children(url_row)
            .into_any_element()
    }

    /// The Open in editor split button: the primary opens the last pick (or the first app),
    /// the caret lists the apps and the GitHub page.
    fn render_editor_button(&mut self, cwd: &str, cx: &mut Context<Self>) -> AnyElement {
        let t = tokens(cx);
        let preferred = self.preferred_editor();
        let open = self.header_menu == Some(HeaderMenu::Editor);
        let primary = h_flex()
            .id("header-open-in-editor")
            .role(Role::Button)
            .aria_label(format!("Open workspace in {}", preferred.label()))
            .h_full()
            .px(px(SPACE_1 + 2.))
            .items_center()
            .cursor_pointer()
            .text_color(t.foreground_muted)
            .hover(|style| style.bg(t.surface2).text_color(t.foreground))
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(icon(preferred.icon()).with_size(px(ICON_SM)));
        let primary = pressable(
            primary,
            act(move |this: &mut Shell, cx: &mut Context<Shell>| {
                this.open_in_editor(preferred, cx)
            }),
            cx,
        );
        let caret = h_flex()
            .id("header-choose-editor")
            .role(Role::Button)
            .aria_label("Choose editor")
            .aria_expanded(open)
            .h_full()
            .px(px(2.))
            .items_center()
            .cursor_pointer()
            .border_l_1()
            .border_color(t.border_accent)
            .when(open, |button| button.bg(t.surface2))
            .hover(|style| style.bg(t.surface2))
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(
                icon(IconName::ChevronDown)
                    .with_size(px(ICON_SM))
                    .text_color(t.foreground_extra_muted),
            );
        let caret = pressable(
            caret,
            act(|this: &mut Shell, cx: &mut Context<Shell>| {
                this.toggle_header_menu(HeaderMenu::Editor, cx)
            }),
            cx,
        );
        let menu = open.then(|| {
            let mut rows: Vec<AnyElement> = self
                .editor_targets
                .clone()
                .into_iter()
                .enumerate()
                .map(|(ix, target)| {
                    let row = menu_row(
                        ("editor-target", ix),
                        target.label().into(),
                        target.icon(),
                        target == preferred,
                        cx,
                    );
                    pressable(
                        row,
                        act(move |this: &mut Shell, cx: &mut Context<Shell>| {
                            this.open_in_editor(target, cx)
                        }),
                        cx,
                    )
                    .into_any_element()
                })
                .collect();
            let github = self
                .editor_github
                .as_ref()
                .filter(|(folder, _)| folder == cwd)
                .map(|(_, url)| url.clone());
            if let Some(url) = github {
                let row = menu_row(
                    "editor-github",
                    "Open on GitHub".into(),
                    IconName::Github,
                    false,
                    cx,
                );
                rows.push(
                    pressable(
                        row,
                        act(move |this: &mut Shell, cx: &mut Context<Shell>| {
                            this.header_menu = None;
                            cx.open_url(&url);
                            cx.notify();
                        }),
                        cx,
                    )
                    .into_any_element(),
                );
            }
            header_popover("open-in-editor-menu", "Choose editor", rows, cx)
        });
        div()
            .relative()
            .flex_shrink_0()
            .child(
                h_flex()
                    .h(px(HEADER_CONTROL_HEIGHT))
                    .items_stretch()
                    .rounded(px(RADIUS_MD))
                    .border_1()
                    .border_color(t.border_accent)
                    .overflow_hidden()
                    .child(primary)
                    .child(caret),
            )
            .children(menu)
            .into_any_element()
    }

    /// The last app picked this session when it is still listed, else the first one.
    fn preferred_editor(&self) -> EditorTarget {
        self.editor_choice
            .filter(|choice| self.editor_targets.contains(choice))
            .or_else(|| self.editor_targets.first().copied())
            .unwrap_or(EditorTarget::Finder)
    }

    fn toggle_header_menu(&mut self, menu: HeaderMenu, cx: &mut Context<Self>) {
        let opens = self.header_menu != Some(menu);
        self.header_menu = opens.then_some(menu);
        if opens && menu == HeaderMenu::Editor {
            self.load_editor_github(cx);
        }
        cx.notify();
    }

    /// Reads the folder's remote for the "Open on GitHub" row, only when the menu opens.
    fn load_editor_github(&mut self, cx: &mut Context<Self>) {
        let Some(cwd) = self.selected_cwd().map(str::to_string) else {
            return;
        };
        let task = self
            .client
            .update(cx, |client, cx| client.git_status(&cwd, cx));
        cx.spawn(async move |this, cx| {
            let url = task.await.ok().and_then(|status| {
                github_url(status.remote_url.as_deref()?, status.branch.as_deref())
            });
            let _ = this.update(cx, |shell, cx| {
                shell.editor_github = url.map(|url| (cwd, url));
                cx.notify();
            });
        })
        .detach();
    }

    /// Opens the workspace folder in `target` with the macOS `open` command and keeps the pick
    /// for the session.
    fn open_in_editor(&mut self, target: EditorTarget, cx: &mut Context<Self>) {
        self.editor_choice = Some(target);
        self.header_menu = None;
        cx.notify();
        let Some(cwd) = self.selected_cwd().map(str::to_string) else {
            return;
        };
        let args = open_args(target, &cwd);
        cx.spawn(async move |this, cx| {
            let opened = cx
                .background_executor()
                .spawn(async move {
                    std::process::Command::new("open")
                        .args(&args)
                        .status()
                        .is_ok_and(|status| status.success())
                })
                .await;
            if !opened {
                let _ = this.update(cx, |shell, cx| {
                    shell.show_toast(false, OPEN_FAILED.into(), cx)
                });
            }
        })
        .detach();
    }

    fn start_script(&mut self, workspace_id: &str, name: &str, cx: &mut Context<Self>) {
        let busy_key = format!("{workspace_id}/{name}");
        if !self.scripts_busy.insert(busy_key.clone()) {
            return;
        }
        let task = self.client.update(cx, |client, cx| {
            client.start_workspace_script(workspace_id, name, cx)
        });
        let name = name.to_string();
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |shell, cx| {
                shell.scripts_busy.remove(&busy_key);
                if let Err(error) = result {
                    let message = format!("{error:#}");
                    let message = if message.trim().is_empty() {
                        format!("Failed to start {name}")
                    } else {
                        message
                    };
                    shell.show_toast(false, message, cx);
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    /// Stops a script by killing its terminal, as Electron does.
    fn stop_script(&mut self, name: &str, terminal_id: Option<&str>, cx: &mut Context<Self>) {
        match terminal_id {
            Some(terminal_id) => self
                .client
                .update(cx, |client, cx| client.kill_terminal(terminal_id, cx)),
            None => self.show_toast(false, format!("Failed to stop {name}"), cx),
        }
    }

    /// Restart, as Electron: stops the script, then starts it once the daemon reports it
    /// stopped (`restart_stopped_scripts`).
    fn restart_script(
        &mut self,
        workspace_id: &str,
        name: &str,
        terminal_id: Option<&str>,
        cx: &mut Context<Self>,
    ) {
        if terminal_id.is_some() {
            self.scripts_restart
                .insert(format!("{workspace_id}/{name}"));
        }
        self.stop_script(name, terminal_id, cx);
    }

    /// Starts each script of `workspace_id` that Restart stopped and the daemon no longer lists
    /// as running.
    pub(crate) fn restart_stopped_scripts(&mut self, workspace_id: &str, cx: &mut Context<Self>) {
        if self.scripts_restart.is_empty() {
            return;
        }
        let stopped: Vec<String> = self
            .client
            .read(cx)
            .workspace_scripts(workspace_id)
            .into_iter()
            .filter(|script| script.lifecycle != "running")
            .map(|script| script.name)
            .filter(|name| {
                self.scripts_restart
                    .remove(&format!("{workspace_id}/{name}"))
            })
            .collect();
        for name in stopped {
            self.start_script(workspace_id, &name, cx);
        }
    }
}

/// The URL a running service opens: `proxy_url` first, as the header Scripts menu asks, then the
/// public and the local proxy URL. A script, or a stopped service, has none.
pub(crate) fn script_url(script: &WorkspaceScript) -> Option<&str> {
    if script.kind == "script" || script.lifecycle != "running" {
        return None;
    }
    script
        .proxy_url
        .as_deref()
        .or(script.public_proxy_url.as_deref())
        .or(script.local_proxy_url.as_deref())
        .filter(|url| !url.is_empty())
}

/// A URL without its `http://` or `https://`, as Electron's route label.
fn url_label(url: &str) -> &str {
    url.strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url)
}

fn open_url_action(url: String) -> ShellAction {
    act(move |this: &mut Shell, cx: &mut Context<Shell>| {
        this.header_menu = None;
        cx.open_url(&url);
        cx.notify();
    })
}

/// A 22 px icon button of a script row; `None` draws it dimmed and inert, as a busy action.
fn script_action(
    id: impl Into<ElementId>,
    label: String,
    glyph: IconName,
    run: Option<ShellAction>,
    cx: &mut Context<Shell>,
) -> AnyElement {
    let t = tokens(cx);
    let button = h_flex()
        .id(id)
        .role(Role::Button)
        .aria_label(label)
        .size(px(22.))
        .flex_shrink_0()
        .justify_center()
        .rounded(px(RADIUS_MD))
        .text_color(t.foreground_muted)
        .child(icon(glyph).with_size(px(ICON_SM)));
    match run {
        Some(run) => pressable(
            button
                .cursor_pointer()
                .hover(|style| style.bg(t.surface2).text_color(t.foreground)),
            run,
            cx,
        )
        .into_any_element(),
        None => button.opacity(0.5).into_any_element(),
    }
}

/// One menu row with a leading icon and a check on the selected item.
fn menu_row(
    id: impl Into<ElementId>,
    label: SharedString,
    glyph: IconName,
    selected: bool,
    cx: &mut Context<Shell>,
) -> Stateful<Div> {
    let t = tokens(cx);
    h_flex()
        .id(id)
        .role(Role::MenuItem)
        .aria_label(label.clone())
        .aria_selected(selected)
        .h(px(MENU_ITEM_HEIGHT))
        .mx(px(SPACE_1))
        .px(px(SPACE_2))
        .gap(px(SPACE_2))
        .items_center()
        .rounded(px(RADIUS_MD))
        .cursor_pointer()
        .text_size(ui_font_size(FONT_SM, cx))
        .line_height(relative(1.3))
        .text_color(t.foreground)
        .hover(|style| style.bg(t.surface2))
        .child(
            icon(glyph)
                .with_size(px(ICON_SM))
                .text_color(t.foreground_muted),
        )
        .child(div().flex_1().min_w_0().truncate().child(label))
        .child(div().w(px(ICON_SM)).flex_shrink_0().when(selected, |slot| {
            slot.child(
                icon(IconName::Check)
                    .with_size(px(ICON_SM))
                    .text_color(t.foreground_muted),
            )
        }))
}

/// A header menu under its trigger, right-aligned, closed by a press outside it.
fn header_popover(
    id: &'static str,
    label: &'static str,
    rows: Vec<AnyElement>,
    cx: &mut Context<Shell>,
) -> AnyElement {
    let t = tokens(cx);
    div()
        .absolute()
        .top(px(28. + SPACE_1))
        .right_0()
        .child(
            deferred(
                anchored()
                    .anchor(Anchor::TopRight)
                    .snap_to_window_with_margin(px(8.))
                    .child(
                        v_flex()
                            .id(id)
                            .role(Role::Menu)
                            .aria_label(label)
                            .min_w(px(200.))
                            .max_w(px(280.))
                            .py(px(SPACE_1))
                            .bg(t.popover)
                            .border_1()
                            .border_color(t.border_accent)
                            .rounded(px(RADIUS_LG))
                            .shadow(vec![shadow(ShadowSize::Md, cx)])
                            .occlude()
                            .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                                this.header_menu = None;
                                cx.notify();
                            }))
                            .children(rows),
                    ),
            )
            .with_priority(gpui_kit::base::POPUP_PRIORITY),
        )
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::{script_url, url_label};
    use bachuc_client::WorkspaceScript;

    #[test]
    fn a_running_service_opens_its_proxy_url_first() {
        let mut script = WorkspaceScript {
            name: "web".into(),
            kind: "service".into(),
            lifecycle: "running".into(),
            health: None,
            terminal_id: Some("t1".into()),
            exit_code: None,
            proxy_url: None,
            local_proxy_url: Some("http://web.localhost:6767".into()),
            public_proxy_url: Some("https://web.example.com".into()),
        };
        assert_eq!(script_url(&script), Some("https://web.example.com"));
        script.proxy_url = Some("http://web--repo.localhost:6767".into());
        assert_eq!(script_url(&script), Some("http://web--repo.localhost:6767"));
        assert_eq!(
            url_label("http://web--repo.localhost:6767"),
            "web--repo.localhost:6767"
        );
        script.lifecycle = "stopped".into();
        assert_eq!(script_url(&script), None);
        script.lifecycle = "running".into();
        script.kind = "script".into();
        assert_eq!(script_url(&script), None);
    }
}
