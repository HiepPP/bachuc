//! Pane views the app opens in a `Workspace`: the agent pane (timeline over composer) and the
//! Electron new-tab launcher (`panels/new-tab-panel.tsx`), and the focus frame around panes
//! that cannot take focus. Terminal, file, and diff panes are the crate views themselves.

use crate::actions::{AGENT_CONTEXT, FindInTimeline};
use bachuc_client::{Client, ClientEvent};
use bachuc_command::display_combos;
use bachuc_composer::{Composer, CornerPills};
use bachuc_diagram::DiagramRenderer;
use bachuc_icons::{IconName, icon};
use bachuc_theme::metrics::{
    BORDER_WIDTH_1, FONT_BASE, FONT_SM, ICON_MD, RADIUS_2XL, RADIUS_LG, RADIUS_MD, SPACE_0_5,
    SPACE_1, SPACE_1_5, SPACE_2, SPACE_3, SPACE_4, SPACE_6,
};
use bachuc_theme::{tokens, ui_font_size, zoomed};
use bachuc_timeline::TimelineView;
use gpui_kit::component::theme::Theme;
use gpui_kit::component::{Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

// ----- focus frame -----

/// The focus target of a pane that has no focusable element of its own (new tab, loading
/// agent, file, diff). Without it the app would focus the `Workspace` element, which has no
/// role, so VoiceOver would announce the whole window.
pub struct PaneFrame {
    focus: FocusHandle,
    label: SharedString,
    child: AnyView,
}

impl PaneFrame {
    pub fn new(label: SharedString, child: AnyView, cx: &mut Context<PaneFrame>) -> PaneFrame {
        PaneFrame {
            focus: cx.focus_handle(),
            label,
            child,
        }
    }

    pub fn focus(&self, window: &mut Window, cx: &mut App) {
        self.focus.focus(window, cx);
    }
}

impl Render for PaneFrame {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("pane-frame")
            .role(Role::Group)
            .aria_label(self.label.clone())
            .track_focus(&self.focus)
            .size_full()
            .child(self.child.clone())
    }
}

// ----- agent pane -----

/// One agent tab: its timeline over its composer, with the plugin corner pills over the
/// timeline's top-right corner. An archived agent shows the Unarchive callout in place of the
/// composer (`archived-agent-callout.tsx`); the composer returns when the agent is live again.
pub struct AgentPane {
    pub timeline: Entity<TimelineView>,
    pub composer: Entity<Composer>,
    corner_pills: Entity<CornerPills>,
    client: Entity<Client>,
    agent_id: String,
    /// The focus target while the composer is hidden.
    callout_focus: FocusHandle,
    /// Stays set after a successful unarchive until the agent is live, as in Electron.
    unarchiving: bool,
    unarchive_error: Option<SharedString>,
    _subscriptions: Vec<Subscription>,
}

/// `FOOTER_HEIGHT` of `constants/layout.ts`.
const FOOTER_HEIGHT: f32 = 75.;
/// The browser line height (1.2), for text that matches Electron.
const TEXT_LINE_FACTOR: f32 = 1.2;

/// The Unarchive button works only while connected and with no unarchive in flight.
pub(crate) fn can_unarchive(connected: bool, unarchiving: bool) -> bool {
    connected && !unarchiving
}

impl AgentPane {
    pub fn new(
        client: Entity<Client>,
        agent_id: String,
        diagrams: Entity<DiagramRenderer>,
        window: &mut Window,
        cx: &mut Context<AgentPane>,
    ) -> AgentPane {
        let timeline_client = client.clone();
        let timeline_id = agent_id.clone();
        let timeline =
            cx.new(|cx| TimelineView::new(timeline_client, timeline_id, diagrams, window, cx));
        let pills_client = client.clone();
        let pills_id = agent_id.clone();
        let corner_pills = cx.new(|cx| CornerPills::new(pills_client, pills_id, cx));
        let composer_client = client.clone();
        let composer_id = agent_id.clone();
        let composer = cx.new(|cx| Composer::new(composer_client, composer_id, window, cx));
        let subscriptions = vec![
            // The timeline keeps the text under the corner pills clear of them.
            cx.observe(&corner_pills, |this: &mut AgentPane, pills, cx| {
                let size = pills.read(cx).size();
                this.timeline
                    .update(cx, |timeline, cx| timeline.set_corner_reserve(size, cx));
            }),
            cx.subscribe_in(
                &client,
                window,
                |this: &mut AgentPane, client, event: &ClientEvent, window, cx| {
                    // The archived state and the connection change with directory and socket
                    // events only.
                    if matches!(event, ClientEvent::TimelineChanged { .. }) {
                        return;
                    }
                    if this.unarchiving && !client.read(cx).is_archived(&this.agent_id) {
                        this.unarchiving = false;
                        this.unarchive_error = None;
                        if this.callout_focus.is_focused(window) {
                            this.composer
                                .update(cx, |composer, cx| composer.focus(window, cx));
                        }
                    }
                    cx.notify();
                },
            ),
        ];
        AgentPane {
            timeline,
            composer,
            corner_pills,
            client,
            agent_id,
            callout_focus: cx.focus_handle(),
            unarchiving: false,
            unarchive_error: None,
            _subscriptions: subscriptions,
        }
    }

    fn archived(&self, cx: &App) -> bool {
        self.client.read(cx).is_archived(&self.agent_id)
    }

    pub fn focus(&self, window: &mut Window, cx: &mut App) {
        if self.archived(cx) {
            self.callout_focus.focus(window, cx);
        } else {
            self.composer
                .update(cx, |composer, cx| composer.focus(window, cx));
        }
    }

    /// `handleUnarchive`: an error shows under the callout and enables the button again.
    fn unarchive(&mut self, cx: &mut Context<Self>) {
        if !can_unarchive(self.client.read(cx).is_connected(), self.unarchiving) {
            return;
        }
        self.unarchiving = true;
        self.unarchive_error = None;
        let agent_id = self.agent_id.clone();
        let task = self
            .client
            .update(cx, |client, cx| client.unarchive_agent(&agent_id, cx));
        cx.spawn(async move |this, cx| {
            let result = task.await;
            this.update(cx, |this, cx| {
                if let Err(error) = result {
                    this.unarchiving = false;
                    this.unarchive_error = Some(format!("{error:#}").into());
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    /// The callout of `archived-agent-callout.tsx`: the text and a small secondary Unarchive
    /// button in a bordered row, the error in red under it.
    fn archived_callout(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let t = tokens(cx);
        let size = ui_font_size(FONT_BASE, cx);
        let enabled = can_unarchive(self.client.read(cx).is_connected(), self.unarchiving);
        let view = cx.weak_entity();
        let button = div()
            .id("agent-unarchive")
            .role(Role::Button)
            .aria_label("Unarchive")
            .flex_shrink_0()
            .px(zoomed(SPACE_3, cx))
            .py(zoomed(SPACE_1_5, cx))
            .rounded(zoomed(RADIUS_MD, cx))
            .bg(t.surface2)
            .text_size(ui_font_size(FONT_SM, cx))
            .text_color(t.foreground)
            .child("Unarchive")
            .when(!enabled, |button| button.opacity(0.5))
            .when(enabled, |button| {
                button
                    .cursor_pointer()
                    .hover(|style| style.bg(t.surface3))
                    .on_click(cx.listener(|this, _, _, cx| this.unarchive(cx)))
                    .on_a11y_action(AccessibleAction::Click, move |_, _, cx| {
                        view.update(cx, |this, cx| this.unarchive(cx)).ok();
                    })
            });
        div()
            .id("agent-archived")
            .role(Role::Group)
            .aria_label("This agent is archived")
            .track_focus(&self.callout_focus)
            .w_full()
            .min_h(zoomed(FOOTER_HEIGHT, cx))
            .flex()
            .justify_center()
            .p(zoomed(SPACE_4, cx))
            .child(
                v_flex()
                    .w_full()
                    .max_w(zoomed(bachuc_theme::reading_look(cx).content_width, cx))
                    .gap(zoomed(SPACE_2, cx))
                    .child(
                        h_flex()
                            .items_center()
                            .justify_center()
                            .gap(zoomed(SPACE_3, cx))
                            .py(zoomed(SPACE_4, cx))
                            .px(zoomed(SPACE_6, cx))
                            .bg(t.surface1)
                            .border(px(BORDER_WIDTH_1))
                            .border_color(t.border_accent)
                            .rounded(zoomed(RADIUS_2XL, cx))
                            .child(
                                div()
                                    .text_size(size)
                                    .text_color(t.foreground_muted)
                                    .child("This agent is archived"),
                            )
                            .child(button),
                    )
                    .when_some(self.unarchive_error.clone(), |stack, error| {
                        stack.child(
                            div()
                                .id("agent-unarchive-error")
                                .role(Role::Alert)
                                .aria_label(error.clone())
                                .w_full()
                                .text_center()
                                .text_size(size)
                                .text_color(t.status_danger)
                                .child(error),
                        )
                    }),
            )
    }
}

impl Render for AgentPane {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let archived = self.archived(cx);
        v_flex()
            .id("pane-agent")
            .role(Role::Group)
            .aria_label("Agent pane")
            .key_context(AGENT_CONTEXT)
            .size_full()
            .on_action(cx.listener(|this, _: &FindInTimeline, window, cx| {
                this.timeline
                    .update(cx, |timeline, cx| timeline.find(window, cx))
            }))
            .child(
                div()
                    .relative()
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .child(self.timeline.clone())
                    .child(self.corner_pills.clone()),
            )
            .when(archived, |pane| pane.child(self.archived_callout(cx)))
            .when(!archived, |pane| pane.child(self.composer.clone()))
    }
}

// ----- provider subagent pane -----

/// A provider subagent tab (`panels/provider-subagent-panel.tsx`): the child's subtitle over its
/// read-only timeline. It has no composer, so its own step list is the only input.
pub struct SubagentPane {
    pub timeline: Entity<TimelineView>,
    subtitle: Option<SharedString>,
}

impl SubagentPane {
    pub fn new(
        client: Entity<Client>,
        key: String,
        subtitle: Option<String>,
        diagrams: Entity<DiagramRenderer>,
        window: &mut Window,
        cx: &mut Context<SubagentPane>,
    ) -> SubagentPane {
        let timeline = cx.new(|cx| TimelineView::read_only(client, key, diagrams, window, cx));
        SubagentPane {
            timeline,
            subtitle: subtitle.map(Into::into),
        }
    }

    pub fn focus(&self, window: &mut Window, cx: &mut App) {
        self.timeline.update(cx, |timeline, cx| {
            timeline.focus_handle(cx).focus(window, cx)
        });
    }
}

impl Render for SubagentPane {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = tokens(cx);
        let size = ui_font_size(FONT_SM, cx);
        v_flex()
            .id("pane-provider-subagent")
            .role(Role::Group)
            .aria_label("Subagent pane")
            .size_full()
            .children(self.subtitle.clone().map(|subtitle| {
                div()
                    .flex_none()
                    .w_full()
                    .truncate()
                    .px(zoomed(SPACE_3, cx))
                    .py(zoomed(SPACE_1, cx))
                    .border_b(zoomed(BORDER_WIDTH_1, cx))
                    .border_color(t.border)
                    .text_size(size)
                    .line_height(size * TEXT_LINE_FACTOR)
                    .text_color(t.foreground_muted)
                    .child(subtitle)
            }))
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .child(self.timeline.clone()),
            )
    }
}

/// An agent tab that has not shown yet. A workspace opens a tab for every agent, as Electron
/// does, but only a shown tab gets a timeline, so hidden tabs stream nothing.
pub struct LazyAgentPane;

impl Render for LazyAgentPane {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = tokens(cx);
        div()
            .id("pane-agent-loading")
            .role(Role::Status)
            .aria_label("Loading agent")
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .text_size(ui_font_size(FONT_BASE, cx))
            .text_color(t.foreground_muted)
            .child("Loading agent")
    }
}

// ----- new tab -----

/// A row of the new-tab launcher, in the Electron primary order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LaunchItem {
    Agent,
    Terminal,
    Changes,
    Files,
}

impl LaunchItem {
    pub const ALL: [LaunchItem; 4] = [
        LaunchItem::Agent,
        LaunchItem::Terminal,
        LaunchItem::Changes,
        LaunchItem::Files,
    ];

    pub fn label(self) -> &'static str {
        match self {
            LaunchItem::Agent => "Agent",
            LaunchItem::Terminal => "Terminal",
            LaunchItem::Changes => "Changes",
            LaunchItem::Files => "Files",
        }
    }

    fn icon(self) -> IconName {
        match self {
            LaunchItem::Agent => IconName::SquarePen,
            LaunchItem::Terminal => IconName::SquareTerminal,
            LaunchItem::Changes => IconName::GitCompareArrows,
            LaunchItem::Files => IconName::Files,
        }
    }

    /// The shortcut action whose chord the row shows.
    pub fn action(self) -> &'static str {
        match self {
            LaunchItem::Agent => "workspace.tab.target.agent",
            LaunchItem::Terminal => "workspace.terminal.new",
            LaunchItem::Changes => "workspace.tab.target.changes",
            LaunchItem::Files => "workspace.tab.target.files",
        }
    }

    /// Terminal, Changes, and Files need a workspace folder.
    fn needs_workspace(self) -> bool {
        self != LaunchItem::Agent
    }
}

/// A row was picked. The pane's key tells the owner which tab to replace.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Launch {
    pub key: String,
    pub item: LaunchItem,
}

/// The Electron new-tab launcher: one row per item with its shortcut, centered in the pane.
pub struct NewTabPane {
    key: String,
    has_workspace: bool,
    /// The display text of each row's chord, in `LaunchItem::ALL` order.
    hints: Vec<Vec<String>>,
}

impl EventEmitter<Launch> for NewTabPane {}

/// The display text of each launcher row's chord.
pub fn launch_hints(shortcuts: &[bachuc_command::Shortcut]) -> Vec<Vec<String>> {
    LaunchItem::ALL
        .iter()
        .map(|item| {
            shortcuts
                .iter()
                .find(|shortcut| shortcut.action == item.action())
                .and_then(|shortcut| shortcut.chord.as_deref())
                .map(display_combos)
                .unwrap_or_default()
        })
        .collect()
}

impl NewTabPane {
    pub fn new(
        key: String,
        has_workspace: bool,
        hints: Vec<Vec<String>>,
        cx: &mut Context<NewTabPane>,
    ) -> NewTabPane {
        cx.observe_global::<Theme>(|_, cx| cx.notify()).detach();
        NewTabPane {
            key,
            has_workspace,
            hints,
        }
    }

    pub fn set_hints(&mut self, hints: Vec<Vec<String>>, cx: &mut Context<NewTabPane>) {
        if self.hints != hints {
            self.hints = hints;
            cx.notify();
        }
    }

    fn row(&self, index: usize, item: LaunchItem, cx: &mut Context<Self>) -> Stateful<Div> {
        let t = tokens(cx);
        let disabled = item.needs_workspace() && !self.has_workspace;
        let hints = self.hints.get(index).cloned().unwrap_or_default();
        let key = self.key.clone();
        div()
            .id(("new-tab-row", index))
            .role(Role::Button)
            .aria_label(if disabled {
                format!("{}, needs a workspace", item.label())
            } else {
                item.label().to_string()
            })
            .w_full()
            .min_h(zoomed(44., cx))
            .px(zoomed(SPACE_3, cx))
            .flex()
            .flex_row()
            .items_center()
            .gap(zoomed(SPACE_2, cx))
            .rounded(zoomed(RADIUS_LG, cx))
            .border_1()
            .border_color(transparent_black())
            .bg(t.surface1)
            .when(disabled, |row| row.opacity(0.5))
            .when(!disabled, |row| {
                row.cursor_pointer()
                    .hover(|style| style.bg(t.surface2))
                    .on_click(cx.listener(move |_, _, _, cx| {
                        cx.emit(Launch {
                            key: key.clone(),
                            item,
                        })
                    }))
            })
            .child(
                icon(item.icon())
                    .with_size(zoomed(ICON_MD, cx))
                    .text_color(t.foreground_muted),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_size(ui_font_size(FONT_BASE, cx))
                    .text_color(t.foreground)
                    .child(item.label()),
            )
            .children(hints.into_iter().map(|text| {
                div()
                    .flex_shrink_0()
                    .px(zoomed(SPACE_1, cx))
                    .py(zoomed(SPACE_0_5, cx))
                    .rounded(zoomed(RADIUS_MD, cx))
                    .bg(t.surface3.opacity(0.5))
                    .text_size(ui_font_size(FONT_SM, cx))
                    .text_color(t.foreground_muted)
                    .child(text)
            }))
    }
}

/// `LAUNCHER_MAX_WIDTH` of `new-tab-panel.tsx`.
const LAUNCHER_MAX_WIDTH: f32 = 380.;

impl Render for NewTabPane {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let rows: Vec<Stateful<Div>> = LaunchItem::ALL
            .iter()
            .enumerate()
            .map(|(index, item)| self.row(index, *item, cx))
            .collect();
        div()
            .id("new-tab-panel")
            .role(Role::Group)
            .aria_label("New tab")
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .p(zoomed(SPACE_6, cx))
            .child(
                v_flex()
                    .id("new-tab-rows")
                    .role(Role::List)
                    .aria_label("Open in this tab")
                    .w_full()
                    .max_w(zoomed(LAUNCHER_MAX_WIDTH, cx))
                    .gap(zoomed(SPACE_1, cx))
                    .children(rows),
            )
    }
}

/// An icon button of the window chrome: 28 px, muted glyph, surface fill on hover.
pub fn chrome_button(
    id: &'static str,
    label: impl Into<SharedString>,
    name: IconName,
    cx: &App,
) -> Stateful<Div> {
    let t = tokens(cx);
    let fg = t.foreground;
    h_flex()
        .id(id)
        .role(Role::Button)
        .aria_label(label)
        .flex_shrink_0()
        .w(zoomed(28., cx))
        .h(zoomed(28., cx))
        .justify_center()
        .rounded(zoomed(RADIUS_MD, cx))
        .cursor_pointer()
        .text_color(t.foreground_muted)
        .hover(move |style| style.bg(t.surface2).text_color(fg))
        // A press must not also start a window drag in the title bar under it.
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .child(icon(name).with_size(zoomed(ICON_MD, cx)))
}

#[cfg(test)]
mod tests {
    // Not `super::*`: the gpui glob brings a `test` macro that shadows the std one.
    use super::{LaunchItem, can_unarchive, launch_hints};
    use bachuc_command::default_shortcuts;

    #[test]
    fn launcher_rows_show_their_chords() {
        let hints = launch_hints(&default_shortcuts());
        assert_eq!(hints.len(), LaunchItem::ALL.len());
        assert_eq!(hints[0], ["⇧⌘A"]);
        assert_eq!(hints[1], ["⇧⌘T"]);
        let mut shortcuts = default_shortcuts();
        for shortcut in &mut shortcuts {
            shortcut.chord = None;
        }
        assert!(launch_hints(&shortcuts).iter().all(Vec::is_empty));
    }

    #[test]
    fn unarchive_needs_a_connection_and_no_request_in_flight() {
        assert!(can_unarchive(true, false));
        assert!(!can_unarchive(false, false));
        assert!(!can_unarchive(true, true));
    }
}
