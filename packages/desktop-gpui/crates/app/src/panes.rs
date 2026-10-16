//! Pane views the app opens in a `Workspace`: the agent pane (timeline over composer) and the
//! Electron new-tab launcher (`panels/new-tab-panel.tsx`), and the focus frame around panes
//! that cannot take focus. Terminal, file, and diff panes are the crate views themselves.

use crate::actions::{AGENT_CONTEXT, FindInTimeline};
use bachuc_client::Client;
use bachuc_command::display_combos;
use bachuc_composer::{Composer, CornerPills};
use bachuc_diagram::DiagramRenderer;
use bachuc_icons::{IconName, icon};
use bachuc_theme::metrics::{
    FONT_BASE, FONT_SM, ICON_MD, RADIUS_LG, RADIUS_MD, SPACE_0_5, SPACE_1, SPACE_2, SPACE_3,
    SPACE_6,
};
use bachuc_theme::{tokens, ui_font_size};
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
/// timeline's top-right corner.
pub struct AgentPane {
    pub timeline: Entity<TimelineView>,
    pub composer: Entity<Composer>,
    corner_pills: Entity<CornerPills>,
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
        let composer = cx.new(|cx| Composer::new(client, agent_id, window, cx));
        AgentPane {
            timeline,
            composer,
            corner_pills,
        }
    }

    pub fn focus(&self, window: &mut Window, cx: &mut App) {
        self.composer
            .update(cx, |composer, cx| composer.focus(window, cx));
    }
}

impl Render for AgentPane {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
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
            .child(self.composer.clone())
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
            .min_h(px(44.))
            .px(px(SPACE_3))
            .flex()
            .flex_row()
            .items_center()
            .gap(px(SPACE_2))
            .rounded(px(RADIUS_LG))
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
                    .with_size(px(ICON_MD))
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
                    .px(px(SPACE_1))
                    .py(px(SPACE_0_5))
                    .rounded(px(RADIUS_MD))
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
            .p(px(SPACE_6))
            .child(
                v_flex()
                    .id("new-tab-rows")
                    .role(Role::List)
                    .aria_label("Open in this tab")
                    .w_full()
                    .max_w(px(LAUNCHER_MAX_WIDTH))
                    .gap(px(SPACE_1))
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
        .w(px(28.))
        .h(px(28.))
        .justify_center()
        .rounded(px(RADIUS_MD))
        .cursor_pointer()
        .text_color(t.foreground_muted)
        .hover(move |style| style.bg(t.surface2).text_color(fg))
        // A press must not also start a window drag in the title bar under it.
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .child(icon(name).with_size(px(ICON_MD)))
}

#[cfg(test)]
mod tests {
    // Not `super::*`: the gpui glob brings a `test` macro that shadows the std one.
    use super::{LaunchItem, launch_hints};
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
}
