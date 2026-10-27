//! The unread thread switcher: a Cmd+Tab style popover over the window (ADR-0070).
//!
//! The shell opens it on Mod+J with the queue rows, oldest first. The view holds focus, so it
//! sees the release of the modifier that opened it: the release opens the selected row, like
//! Enter. The popover draws only when the modifier is still held after `SHOW_DELAY`, so a quick
//! tap opens the oldest thread with no flash. The view never closes itself; the shell removes it
//! after either event.

use crate::state::step_index;
use bachuc_client::AttentionReason;
use bachuc_theme::metrics::{FONT_BASE, FONT_SM, RADIUS_LG, SPACE_1, SPACE_2, SPACE_3, SPACE_4};
use bachuc_theme::{ShadowSize, shadow, tokens, ui_font_size, zoomed};
use gpui_kit::component::h_flex;
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use std::collections::HashSet;
use std::time::Duration;

/// How long the modifier must stay down before the popover draws.
const SHOW_DELAY: Duration = Duration::from_millis(150);
const PANEL_WIDTH: f32 = 520.;
const PANEL_MAX_WIDTH: f32 = 0.92;
const ROW_HEIGHT: f32 = 40.;

/// One queued thread.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SwitcherRow {
    pub agent_id: String,
    pub title: String,
    pub workspace: String,
    pub reason: AttentionReason,
    /// "now", "5m", "3h", or `None` when the timestamp does not parse.
    pub age: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UnreadSwitcherEvent {
    /// Open this thread: Enter, a click, or the release of the held modifier.
    Open { agent_id: String },
    /// Escape, a click outside the panel, or no rows left.
    Dismissed,
}

pub struct UnreadSwitcher {
    rows: Vec<SwitcherRow>,
    selected: usize,
    /// The Cmd, Ctrl, and Alt that were down at open. Empty means nothing to release.
    hold: Modifiers,
    shown: bool,
    focus: FocusHandle,
}

impl EventEmitter<UnreadSwitcherEvent> for UnreadSwitcher {}

impl Focusable for UnreadSwitcher {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl UnreadSwitcher {
    /// Takes focus and selects the first row. `held` is the modifier state of the opening press.
    pub fn new(
        rows: Vec<SwitcherRow>,
        held: Modifiers,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let hold = Modifiers {
            platform: held.platform,
            control: held.control,
            alt: held.alt,
            ..Modifiers::default()
        };
        let focus = cx.focus_handle();
        window.focus(&focus, cx);
        let shown = !hold.modified();
        if !shown {
            cx.spawn(async move |this, cx| {
                cx.background_executor().timer(SHOW_DELAY).await;
                let _ = this.update(cx, |this, cx| {
                    this.shown = true;
                    cx.notify();
                });
            })
            .detach();
        }
        Self {
            rows,
            selected: 0,
            hold,
            shown,
            focus,
        }
    }

    /// Moves the selection one row, wrapping at the ends.
    pub fn step(&mut self, forward: bool, cx: &mut Context<Self>) {
        if let Some(next) = step_index(self.rows.len(), Some(self.selected), forward) {
            self.selected = next;
            cx.notify();
        }
    }

    /// Drops the rows whose agent is not in `alive`, keeping the selected row if it stays.
    /// Emits `Dismissed` when no row is left.
    pub fn retain(&mut self, alive: &HashSet<String>, cx: &mut Context<Self>) {
        let selected = self.rows.get(self.selected).map(|row| row.agent_id.clone());
        self.rows.retain(|row| alive.contains(&row.agent_id));
        if self.rows.is_empty() {
            cx.emit(UnreadSwitcherEvent::Dismissed);
            return;
        }
        self.selected = selected
            .and_then(|id| self.rows.iter().position(|row| row.agent_id == id))
            .unwrap_or(self.selected.min(self.rows.len() - 1));
        cx.notify();
    }

    fn open_selected(&mut self, cx: &mut Context<Self>) {
        match self.rows.get(self.selected) {
            Some(row) => cx.emit(UnreadSwitcherEvent::Open {
                agent_id: row.agent_id.clone(),
            }),
            None => cx.emit(UnreadSwitcherEvent::Dismissed),
        }
    }

    fn released(&self, now: &Modifiers) -> bool {
        (self.hold.platform && !now.platform)
            || (self.hold.control && !now.control)
            || (self.hold.alt && !now.alt)
    }

    fn render_row(&self, ix: usize, row: &SwitcherRow, cx: &mut Context<Self>) -> Stateful<Div> {
        let colors = tokens(cx);
        let reason = match row.reason {
            AttentionReason::Error => ("Error", colors.status_danger),
            _ => ("Finished", colors.status_success),
        };
        let agent_id = row.agent_id.clone();
        let label = format!("{}, {}, {}", row.title, row.workspace, reason.0);
        div()
            .id(("unread-row", ix))
            .role(Role::Button)
            .aria_label(SharedString::from(label))
            .w_full()
            .h(zoomed(ROW_HEIGHT, cx))
            .px(zoomed(SPACE_4, cx))
            .flex()
            .flex_row()
            .items_center()
            .gap(zoomed(SPACE_3, cx))
            .cursor_pointer()
            .text_size(ui_font_size(FONT_BASE, cx))
            .when(ix == self.selected, |this| this.bg(colors.surface1))
            .hover(|style| style.bg(colors.surface1))
            .on_click(cx.listener({
                let agent_id = agent_id.clone();
                move |_, _, _, cx| {
                    cx.emit(UnreadSwitcherEvent::Open {
                        agent_id: agent_id.clone(),
                    })
                }
            }))
            .on_a11y_action(AccessibleAction::Click, {
                let view = cx.weak_entity();
                move |_, _, cx| {
                    let agent_id = agent_id.clone();
                    view.update(cx, |_, cx| cx.emit(UnreadSwitcherEvent::Open { agent_id }))
                        .ok();
                }
            })
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .text_color(colors.foreground)
                    .child(row.title.clone()),
            )
            .child(
                div()
                    .flex_shrink_0()
                    .max_w(relative(0.3))
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .text_size(ui_font_size(FONT_SM, cx))
                    .text_color(colors.foreground_muted)
                    .child(row.workspace.clone()),
            )
            .child(
                h_flex()
                    .flex_shrink_0()
                    .gap(zoomed(SPACE_2, cx))
                    .text_size(ui_font_size(FONT_SM, cx))
                    .child(div().text_color(reason.1).child(reason.0))
                    .children(
                        row.age
                            .clone()
                            .map(|age| div().text_color(colors.foreground_muted).child(age)),
                    ),
            )
    }
}

impl Render for UnreadSwitcher {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = tokens(cx);
        let rows: Vec<_> = self
            .rows
            .iter()
            .enumerate()
            .map(|(ix, row)| self.render_row(ix, row, cx))
            .collect();
        // The root exists before the popover draws, so the focus and key handlers are live
        // during the delay.
        div()
            .id("unread-switcher")
            .role(Role::Group)
            .aria_label("Unread threads")
            .track_focus(&self.focus)
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                match event.keystroke.key.as_str() {
                    "enter" => this.open_selected(cx),
                    "escape" => cx.emit(UnreadSwitcherEvent::Dismissed),
                    "down" => this.step(true, cx),
                    "up" => this.step(false, cx),
                    _ => return,
                }
                cx.stop_propagation();
            }))
            .on_modifiers_changed(cx.listener(|this, event: &ModifiersChangedEvent, _, cx| {
                if this.released(&event.modifiers) {
                    this.open_selected(cx);
                }
            }))
            .when(self.shown, |root| {
                root.flex()
                    .flex_col()
                    .items_center()
                    .pt(zoomed(SPACE_4 * 8., cx))
                    .child(
                        div()
                            .id("unread-switcher-backdrop")
                            .absolute()
                            .top_0()
                            .left_0()
                            .size_full()
                            .bg(black().opacity(0.5))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|_, _, _, cx| cx.emit(UnreadSwitcherEvent::Dismissed)),
                            ),
                    )
                    .child(
                        div()
                            .id("unread-switcher-panel")
                            .role(Role::Dialog)
                            .aria_label("Unread threads")
                            .occlude()
                            .flex()
                            .flex_col()
                            .w(zoomed(PANEL_WIDTH, cx))
                            .max_w(relative(PANEL_MAX_WIDTH))
                            .max_h(relative(0.6))
                            .overflow_hidden()
                            .py(zoomed(SPACE_1, cx))
                            .border_1()
                            .border_color(colors.border)
                            .rounded(zoomed(RADIUS_LG, cx))
                            .bg(colors.surface0)
                            .text_color(colors.foreground)
                            .shadow(vec![shadow(ShadowSize::Lg, cx)])
                            .children(rows),
                    )
            })
    }
}
