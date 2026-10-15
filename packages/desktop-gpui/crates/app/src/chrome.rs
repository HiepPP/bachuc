//! The sidebar, the tab bar, and the status line, each a view of its own. The timeline redraws the
//! window many times a second, so every extra element in these views costs frame time: keep the
//! rows to a few elements each.

use crate::shell::Shell;
use crate::state::{Connection, Tab, TabKind, agent_title, badge_count, group_agents, short_id};
use bachuc_client::AgentSummary;
use gpui_kit::component::{ActiveTheme as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

pub const SIDEBAR_WIDTH: f32 = 272.;
pub const TAB_BAR_HEIGHT: f32 = 36.;
pub const STATUS_HEIGHT: f32 = 24.;

/// The dot color of an agent row.
fn status_color(agent: &AgentSummary, theme: &gpui_kit::component::Theme) -> Hsla {
    if agent.pending_permissions > 0 {
        return theme.warning;
    }
    match agent.status.as_str() {
        "running" => theme.success,
        "error" => theme.danger,
        _ => theme.muted_foreground,
    }
}

/// A text button of one element. The component `Button` builds about ten, and these views redraw
/// with every timeline update. It takes focus and answers the accessibility Click action.
fn lean_button(
    id: &'static str,
    label: &'static str,
    primary: bool,
    theme: &gpui_kit::component::Theme,
) -> Stateful<Div> {
    div()
        .id(id)
        .role(Role::Button)
        .aria_label(label)
        .focusable()
        .cursor_pointer()
        .px_2()
        .py_1()
        .rounded_md()
        .text_sm()
        .border_1()
        .border_color(theme.border)
        .when(primary, |button| {
            button
                .bg(theme.primary)
                .text_color(theme.primary_foreground)
                .border_color(theme.primary)
        })
        .when(!primary, |button| {
            button.hover(|button| button.bg(theme.accent))
        })
        .child(label)
}

// ----- sidebar -----

pub struct Sidebar {
    shell: WeakEntity<Shell>,
}

impl Sidebar {
    pub fn new(shell: WeakEntity<Shell>) -> Self {
        Self { shell }
    }
}

impl Render for Sidebar {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let Some(shell) = self.shell.upgrade() else {
            return div().into_any_element();
        };
        let data = shell.read(cx).sidebar_data(cx);
        let groups = group_agents(&data.agents, &data.workspaces);

        let host_rows = data.hosts_open.then(|| {
            data.hosts
                .iter()
                .enumerate()
                .map(|(ix, (name, url))| {
                    let current = ix == data.host_index;
                    div()
                        .id(("host", ix))
                        .role(Role::MenuItem)
                        .aria_label(format!("Host {name}, {url}"))
                        .aria_selected(current)
                        .cursor_pointer()
                        .px_2()
                        .py_1()
                        .rounded_md()
                        .text_sm()
                        .when(current, |row| row.bg(theme.accent))
                        .hover(|row| row.bg(theme.accent))
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.act(cx, |shell, cx| shell.switch_host(ix, window, cx))
                        }))
                        .child(format!("{name}  {url}"))
                })
                .collect::<Vec<_>>()
        });

        let mut rows = Vec::new();
        for (group_ix, group) in groups.iter().enumerate() {
            rows.push(
                div()
                    .id(("group", group_ix))
                    .role(Role::Heading)
                    .aria_label(format!("Workspace {}", group.name))
                    .px_2()
                    .pt_3()
                    .pb_1()
                    .flex_none()
                    .truncate()
                    .text_xs()
                    .text_color(theme.muted_foreground)
                    .child(group.name.clone())
                    .into_any_element(),
            );
            for agent in &group.agents {
                let selected = data.active_agent.as_deref() == Some(agent.id.as_str());
                let dot = status_color(agent, &theme);
                let title = agent_title(agent);
                let mut label = format!("Agent {title}, {}", agent.status);
                if agent.pending_permissions > 0 {
                    label.push_str(&format!(
                        ", {} permission waiting",
                        agent.pending_permissions
                    ));
                }
                let id = agent.id.clone();
                rows.push(
                    h_flex()
                        .id(SharedString::from(format!("agent-{}", agent.id)))
                        .role(Role::ListItem)
                        .aria_label(label)
                        .aria_selected(selected)
                        .cursor_pointer()
                        .gap_2()
                        .mx_1()
                        .px_2()
                        .py_1()
                        .rounded_md()
                        .items_center()
                        .when(selected, |row| row.bg(theme.accent))
                        .hover(|row| row.bg(theme.accent))
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.act(cx, |shell, cx| shell.open_agent(id.clone(), window, cx))
                        }))
                        .child(div().size(px(8.)).flex_none().rounded_full().bg(dot))
                        .child(div().min_w_0().flex_1().truncate().text_sm().child(title))
                        .when(agent.pending_permissions > 0, |row| {
                            row.child(
                                div()
                                    .px_1p5()
                                    .rounded_full()
                                    .bg(theme.warning)
                                    .text_xs()
                                    .text_color(theme.warning_foreground)
                                    .child(agent.pending_permissions.to_string()),
                            )
                        })
                        .into_any_element(),
                );
            }
        }
        let empty = rows.is_empty();
        let empty_text = if data.connection == Connection::Connected {
            "No agents yet. Press Cmd+N to start one."
        } else {
            "Waiting for the host."
        };
        let host_name = data
            .hosts
            .get(data.host_index)
            .map(|(name, _)| name.clone())
            .unwrap_or_default();

        v_flex()
            .id("sidebar")
            .role(Role::Complementary)
            .aria_label("Sidebar")
            .size_full()
            .border_r_1()
            .border_color(theme.sidebar_border)
            .bg(theme.sidebar)
            .text_color(theme.sidebar_foreground)
            .child(
                v_flex()
                    .gap_2()
                    .p_2()
                    .child(
                        div()
                            .id("host-switcher")
                            // A combo box role maps to a pop-up button, which does not answer the
                            // press action; a button with an expanded state does.
                            .role(Role::Button)
                            .aria_label(format!("Host switcher: {host_name}"))
                            .aria_expanded(data.hosts_open)
                            .cursor_pointer()
                            .px_2()
                            .py_1()
                            .rounded_md()
                            .border_1()
                            .border_color(theme.border)
                            .text_sm()
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.act(cx, |shell, cx| shell.toggle_hosts(cx))
                            }))
                            .child(format!("Host: {host_name}")),
                    )
                    .children(host_rows.into_iter().flatten())
                    .child(
                        h_flex()
                            .gap_1()
                            .child(
                                lean_button("sidebar-new-agent", "New agent", true, &theme)
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.act(cx, |shell, cx| shell.open_new_agent(window, cx))
                                    })),
                            )
                            .child(
                                lean_button("sidebar-new-terminal", "New terminal", false, &theme)
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.act(cx, |shell, cx| shell.open_terminal(window, cx))
                                    })),
                            ),
                    ),
            )
            .child(
                v_flex()
                    .id("sidebar-agents")
                    .role(Role::List)
                    .aria_label("Agents")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .pb_2()
                    .children(rows)
                    .when(empty, |list| {
                        list.child(
                            div()
                                .id("sidebar-empty")
                                .role(Role::Note)
                                .aria_label(empty_text)
                                .p_3()
                                .text_sm()
                                .text_color(theme.muted_foreground)
                                .child(empty_text),
                        )
                    }),
            )
            .into_any_element()
    }
}

impl Sidebar {
    /// Runs `f` on the shell. The shell outlives its views, so a missing shell means the window
    /// is closing.
    fn act(&self, cx: &mut Context<Self>, f: impl FnOnce(&mut Shell, &mut Context<Shell>)) {
        let _ = self.shell.update(cx, f);
    }
}

/// What the sidebar shows, copied out of the shell.
pub struct SidebarData {
    pub agents: Vec<AgentSummary>,
    pub workspaces: Vec<bachuc_client::WorkspaceSummary>,
    pub active_agent: Option<String>,
    pub hosts: Vec<(String, String)>,
    pub host_index: usize,
    pub hosts_open: bool,
    pub connection: Connection,
}

// ----- tab bar -----

pub struct TabBar {
    shell: WeakEntity<Shell>,
}

impl TabBar {
    pub fn new(shell: WeakEntity<Shell>) -> Self {
        Self { shell }
    }

    fn act(&self, cx: &mut Context<Self>, f: impl FnOnce(&mut Shell, &mut Context<Shell>)) {
        let _ = self.shell.update(cx, f);
    }
}

/// The tab label: the terminal name, or the agent title.
pub fn tab_title(tab: &Tab, agents: &[AgentSummary]) -> String {
    match &tab.kind {
        TabKind::Terminal => tab.title.clone(),
        TabKind::Agent(id) => agents
            .iter()
            .find(|agent| agent.id == *id)
            .map(agent_title)
            .unwrap_or_else(|| format!("Agent {}", short_id(id))),
    }
}

impl Render for TabBar {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let tabs: Vec<(u64, String)> = match self.shell.upgrade() {
            Some(shell) => shell.read(cx).tab_labels(cx),
            None => Vec::new(),
        };
        let active = self
            .shell
            .upgrade()
            .and_then(|shell| shell.read(cx).active_tab_id());
        h_flex()
            .id("tab-bar")
            .role(Role::TabList)
            .aria_label("Tabs")
            .size_full()
            .items_center()
            .gap_1()
            .px_1()
            .border_b_1()
            .border_color(theme.border)
            .bg(theme.tab_bar)
            .children(tabs.into_iter().map(|(id, title)| {
                let selected = active == Some(id);
                h_flex()
                    .items_center()
                    .rounded_md()
                    .when(selected, |tab| tab.bg(theme.tab_active))
                    .child(
                        div()
                            .id(("tab", id))
                            .role(Role::Tab)
                            .aria_label(format!("Tab {title}"))
                            .aria_selected(selected)
                            .cursor_pointer()
                            .max_w(px(200.))
                            .truncate()
                            .pl_3()
                            .pr_1()
                            .py_1()
                            .text_sm()
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.act(cx, |shell, cx| shell.select_tab(id, window, cx))
                            }))
                            .child(title.clone()),
                    )
                    .child(
                        div()
                            .id(("tab-close", id))
                            .role(Role::Button)
                            .aria_label(format!("Close {title}"))
                            .focusable()
                            .cursor_pointer()
                            .px_2()
                            .py_1()
                            .rounded_md()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .hover(|button| button.bg(theme.accent))
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.act(cx, |shell, cx| shell.close_tab(id, window, cx))
                            }))
                            .child("x"),
                    )
            }))
    }
}

// ----- status line -----

pub struct StatusLine {
    shell: WeakEntity<Shell>,
}

impl StatusLine {
    pub fn new(shell: WeakEntity<Shell>) -> Self {
        Self { shell }
    }
}

/// The status line text: connection, host, agent count, waiting permissions, last message.
pub fn status_text(
    connection: &Connection,
    host: &str,
    agents: &[AgentSummary],
    message: Option<&str>,
) -> String {
    let mut parts = vec![
        connection.text(),
        host.to_string(),
        format!("{} agents", agents.len()),
    ];
    let permissions = badge_count(agents);
    if permissions > 0 {
        parts.push(format!("{permissions} permission waiting"));
    }
    if let Some(message) = message {
        parts.push(message.to_string());
    }
    parts.join("  |  ")
}

impl Render for StatusLine {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let (text, failed) = match self.shell.upgrade() {
            Some(shell) => shell.read(cx).status(cx),
            None => (String::new(), false),
        };
        div()
            .id("status-line")
            .role(Role::Status)
            .aria_label(text.clone())
            .size_full()
            .flex()
            .items_center()
            .px_2()
            .border_t_1()
            .border_color(theme.border)
            .text_xs()
            .text_color(if failed {
                theme.danger
            } else {
                theme.muted_foreground
            })
            .child(text)
    }
}

#[cfg(test)]
mod tests {
    use super::status_text;
    use crate::state::Connection;
    use bachuc_client::AgentSummary;

    fn agent(permissions: usize) -> AgentSummary {
        AgentSummary {
            id: "a".into(),
            title: "t".into(),
            status: "idle".into(),
            cwd: "/".into(),
            workspace_id: None,
            provider: "mock".into(),
            model: None,
            pending_permissions: permissions,
        }
    }

    #[test]
    fn the_status_line_names_host_agents_and_waiting_permissions() {
        let text = status_text(&Connection::Connected, "live", &[agent(1), agent(0)], None);
        assert_eq!(
            text,
            "Connected  |  live  |  2 agents  |  1 permission waiting"
        );
        let text = status_text(
            &Connection::Disconnected("incorrect_password".into()),
            "macmini",
            &[],
            Some("note"),
        );
        assert_eq!(
            text,
            "Disconnected: incorrect_password, retrying  |  macmini  |  0 agents  |  note"
        );
    }
}
