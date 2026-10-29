//! `BoardPage` of `client/page.tsx` (1694-1828): the view switch (`BoardViewSwitch`,
//! `client/recaps.tsx:21-79`), the size control (`client/size-control.tsx`), and the view.

use crate::card::{CONTROL_RADIUS, CardAction, button, tabular};
use crate::model::settings::{
    BOARD_SIZE_DEFAULT, BOARD_SIZE_MAX, BOARD_SIZE_MIN, BOARD_SIZE_STEP, board_scale,
};
use crate::model::text::{new_title, now_ms};
use crate::recaps::RecapsState;
use crate::runs::RunsState;
use crate::store::BoardStore;
use crate::timeline::TimelineState;
use gpui_kit::component::dialog::DialogButtonProps;
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::component::{WindowExt as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use std::cell::Cell;

/// The page's side gutter and toolbar padding, unscaled as in Electron.
const GUTTER: f32 = 24.;
const TOOLBAR_GAP: f32 = 12.;
/// The shape lock of the toolbar: track 8, segments 6.
const TRACK_RADIUS: f32 = 8.;
const SEGMENT_HEIGHT: f32 = 28.;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BoardView {
    Runs,
    Timeline,
    Recaps,
}

const VIEWS: [(BoardView, &str); 3] = [
    (BoardView::Runs, "Runs"),
    (BoardView::Timeline, "Timeline"),
    (BoardView::Recaps, "Recaps"),
];

pub(crate) struct BoardPage {
    pub(crate) store: WeakEntity<BoardStore>,
    view: BoardView,
    pub(crate) runs: RunsState,
    pub(crate) timeline: TimelineState,
    pub(crate) recaps: RecapsState,
    _observe: Subscription,
}

impl BoardPage {
    pub(crate) fn new(store: &Entity<BoardStore>, cx: &mut Context<Self>) -> Self {
        Self {
            store: store.downgrade(),
            view: BoardView::Runs,
            runs: RunsState::new(),
            timeline: TimelineState::new(),
            recaps: RecapsState::new(),
            _observe: cx.observe(store, |_, _, cx| cx.notify()),
        }
    }

    pub(crate) fn view(&self) -> BoardView {
        self.view
    }

    pub(crate) fn shows_runs(&self) -> bool {
        self.view == BoardView::Runs
    }

    /// A card's press: navigation runs now; star, diffs, and Remove show their state on the
    /// card until the host answers.
    pub(crate) fn card_action(
        &mut self,
        action: CardAction,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(store) = self.store.upgrade() else {
            return;
        };
        match action {
            CardAction::Open(agent_id) => {
                store.update(cx, |store, cx| store.open_agent(&agent_id, cx));
            }
            CardAction::OpenTask(run) => store.update(cx, |store, cx| store.open_task(&run, cx)),
            CardAction::OpenProject(run) => {
                self.runs.page_error = store
                    .update(cx, |store, cx| store.open_project(&run, cx))
                    .err()
                    .map(str::to_string);
            }
            CardAction::Toggle(agent_id) => self.runs.toggle(&agent_id),
            CardAction::Star { run_id, starred } => {
                let ui = self.runs.card(&run_id);
                ui.starring = true;
                ui.star_error = false;
                let task = store.update(cx, |store, cx| store.set_starred(&run_id, starred, cx));
                cx.spawn(async move |page, cx| {
                    let failed = task.await.is_err();
                    page.update(cx, |page, cx| {
                        let ui = page.runs.card(&run_id);
                        ui.starring = false;
                        ui.star_error = failed;
                        cx.notify();
                    })
                    .ok();
                })
                .detach();
            }
            CardAction::Diffs { agent_id, off } => {
                let run_id = self.run_id_of(&store, &agent_id, cx);
                let ui = self.runs.card(&run_id);
                ui.toggling_diffs = true;
                ui.diffs_error = false;
                let task = store.update(cx, |store, cx| store.set_diffs_off(&agent_id, off, cx));
                cx.spawn(async move |page, cx| {
                    let failed = task.await.is_err();
                    page.update(cx, |page, cx| {
                        let ui = page.runs.card(&run_id);
                        ui.toggling_diffs = false;
                        ui.diffs_error = failed;
                        cx.notify();
                    })
                    .ok();
                })
                .detach();
            }
            CardAction::Rename { agent_id, title } => self.ask_title(agent_id, title, _window, cx),
            CardAction::Remove { run_id, agent_id } => {
                let ui = self.runs.card(&run_id);
                ui.removing = true;
                ui.remove_error = false;
                let task = store.update(cx, |store, cx| store.remove(&run_id, cx));
                let weak = self.store.clone();
                cx.spawn(async move |page, cx| {
                    let removed = matches!(task.await, Ok(true));
                    if removed {
                        // The card is gone once the Board refreshes, so a failed close reports
                        // on the page.
                        let close = weak
                            .update(cx, |store, cx| store.close_runtime(&agent_id, cx))
                            .ok();
                        if let Some(close) = close
                            && let Err(error) = close.await
                        {
                            page.update(cx, |page, cx| {
                                page.runs.page_error = Some(error.to_string());
                                cx.notify();
                            })
                            .ok();
                        }
                    }
                    page.update(cx, |page, cx| {
                        let ui = page.runs.card(&run_id);
                        ui.removing = false;
                        ui.remove_error = !removed;
                        cx.notify();
                    })
                    .ok();
                })
                .detach();
            }
        }
        cx.notify();
    }

    /// The rename dialog, as the tab menu's Rename agent. An empty or unchanged title does nothing.
    fn ask_title(
        &mut self,
        agent_id: String,
        current: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let input = cx.new(|cx| InputState::new(window, cx).default_value(current.clone()));
        let page = cx.entity().downgrade();
        let width = bachuc_theme::zoomed(420., cx);
        // The builder runs again whenever the dialog layer is rebuilt; focus once.
        let focus_once = Cell::new(true);
        window.open_dialog(cx, move |dialog, window, cx| {
            if focus_once.replace(false) {
                input.update(cx, |input, cx| input.focus(window, cx));
            }
            let (page, field, agent_id, current) = (
                page.clone(),
                input.clone(),
                agent_id.clone(),
                current.clone(),
            );
            dialog
                .title("Rename thread")
                .w(width)
                .child(Input::new(&input).aria_label("Thread title"))
                .button_props(
                    DialogButtonProps::default()
                        .ok_text("Rename")
                        .show_cancel(true)
                        .on_ok(move |_, _, cx| {
                            if let Some(title) = new_title(&field.read(cx).value(), &current) {
                                page.update(cx, |page, cx| page.rename(&agent_id, title, cx))
                                    .ok();
                            }
                            true
                        }),
                )
        });
    }

    fn rename(&mut self, agent_id: &str, title: String, cx: &mut Context<Self>) {
        let Some(store) = self.store.upgrade() else {
            return;
        };
        let run_id = self.run_id_of(&store, agent_id, cx);
        let ui = self.runs.card(&run_id);
        ui.renaming = true;
        ui.rename_error = false;
        let task = store.update(cx, |store, cx| store.rename(agent_id, &title, cx));
        cx.spawn(async move |page, cx| {
            let failed = task.await.is_err();
            page.update(cx, |page, cx| {
                let ui = page.runs.card(&run_id);
                ui.renaming = false;
                ui.rename_error = failed;
                cx.notify();
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    fn run_id_of(&self, store: &Entity<BoardStore>, agent_id: &str, cx: &App) -> String {
        store
            .read(cx)
            .snapshot()
            .and_then(|snapshot| snapshot.runs.iter().find(|run| run.agent_id == agent_id))
            .map_or_else(|| agent_id.to_string(), |run| run.id.clone())
    }

    fn view_switch(&self, cx: &mut Context<Self>) -> AnyElement {
        let t = bachuc_theme::tokens(cx);
        let z = |value: f32| bachuc_theme::zoomed(value, cx);
        let (segment, segment_pad, segment_radius) = (z(SEGMENT_HEIGHT), z(12.), z(CONTROL_RADIUS));
        let (text, line) = (z(13.), z(16.));
        h_flex()
            .id("board-views")
            .role(Role::TabList)
            .aria_label("Board view")
            .p(z(2.))
            .gap(z(2.))
            .rounded(z(TRACK_RADIUS))
            .bg(t.surface2)
            .children(VIEWS.into_iter().map(|(view, label)| {
                let selected = self.view == view;
                div()
                    .id(SharedString::from(format!("board-view-{label}")))
                    .role(Role::Tab)
                    .aria_label(label)
                    .aria_selected(selected)
                    .focusable()
                    .focus_visible(move |style| style.shadow(crate::card::ring(t.accent, false)))
                    .cursor_pointer()
                    .h(segment)
                    .px(segment_pad)
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(segment_radius)
                    .border_1()
                    .map(|tab| {
                        if selected {
                            tab.border_color(t.border).bg(t.surface0)
                        } else {
                            tab.border_color(transparent_black())
                        }
                    })
                    .active(|style| style.opacity(0.7))
                    .on_click(cx.listener(move |page, _, _, cx| {
                        page.view = view;
                        cx.notify();
                    }))
                    .child(
                        div()
                            .text_size(text)
                            .line_height(line)
                            .font_weight(if selected {
                                FontWeight::SEMIBOLD
                            } else {
                                FontWeight::MEDIUM
                            })
                            .text_color(if selected {
                                t.foreground
                            } else {
                                t.foreground_muted
                            })
                            .child(label),
                    )
            }))
            .into_any_element()
    }

    fn size_control(&self, size: u16, cx: &mut Context<Self>) -> AnyElement {
        let t = bachuc_theme::tokens(cx);
        let z = |value: f32| bachuc_theme::zoomed(value, cx);
        let (segment, segment_radius) = (z(SEGMENT_HEIGHT), z(CONTROL_RADIUS));
        let steps = [
            (
                "Decrease Board UI size".to_string(),
                "−".to_string(),
                i64::from(size) - i64::from(BOARD_SIZE_STEP),
                size <= BOARD_SIZE_MIN,
            ),
            (
                format!("Board UI size {size}%. Reset to {BOARD_SIZE_DEFAULT}%"),
                format!("{size}%"),
                i64::from(BOARD_SIZE_DEFAULT),
                false,
            ),
            (
                "Increase Board UI size".to_string(),
                "+".to_string(),
                i64::from(size) + i64::from(BOARD_SIZE_STEP),
                size >= BOARD_SIZE_MAX,
            ),
        ];
        // A bare stepper: no frame or label, so the toolbar stays quieter than the Board.
        h_flex()
            .id("board-size")
            .role(Role::Group)
            .aria_label("Board UI size")
            .items_center()
            .gap(z(2.))
            .children(
                steps
                    .into_iter()
                    .enumerate()
                    .map(|(ix, (label, text, value, disabled))| {
                        let middle = ix == 1;
                        button(
                            SharedString::from(format!("board-size-{ix}")),
                            label,
                            t.accent,
                            false,
                        )
                        .aria_description(if middle {
                            "Restore default Board size"
                        } else {
                            "Affects only Board"
                        })
                        .flex_none()
                        .w(z(if middle { 48. } else { 28. }))
                        .h(segment)
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(segment_radius)
                        .map(|step| {
                            if disabled {
                                step.opacity(0.35)
                            } else {
                                step.active(|style| style.bg(t.surface2))
                                    .on_click(cx.listener(move |page, _, _, cx| {
                                        if let Some(store) = page.store.upgrade() {
                                            store
                                                .update(cx, |store, cx| store.pick_size(value, cx));
                                        }
                                    }))
                            }
                        })
                        .child(
                            div()
                                .text_size(z(if middle { 12.5 } else { 17. }))
                                .line_height(z(if middle { 17. } else { 20. }))
                                .font_weight(FontWeight::MEDIUM)
                                .font_features(tabular())
                                .text_color(if middle {
                                    t.foreground
                                } else {
                                    t.foreground_muted
                                })
                                .child(text),
                        )
                    }),
            )
            .into_any_element()
    }
}

impl Render for BoardPage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = bachuc_theme::tokens(cx);
        let Some(store) = self.store.upgrade() else {
            // The plugin was turned off while the page stayed open.
            return div()
                .id("board-off")
                .role(Role::Note)
                .aria_label("Board is off on this host.")
                .size_full()
                .p(bachuc_theme::zoomed(GUTTER, cx))
                .bg(t.surface0)
                .text_color(t.foreground_muted)
                .child("Board is off on this host.")
                .into_any_element();
        };
        let size = store.read(cx).size();
        // The Board size multiplies with the whole-UI zoom: every measure below follows both.
        let scale = board_scale(size) * bachuc_theme::zoom(cx);
        let s = |value: f32| px(value * scale);
        let (gutter, toolbar_gap) = (
            bachuc_theme::zoomed(GUTTER, cx),
            bachuc_theme::zoomed(TOOLBAR_GAP, cx),
        );
        let (toolbar_top, toolbar_bottom) =
            (bachuc_theme::zoomed(12., cx), bachuc_theme::zoomed(6., cx));
        let now = now_ms();
        let body = match self.view {
            BoardView::Runs => self.render_runs(&store, scale, now, cx),
            BoardView::Timeline => self.render_timeline(&store, scale, cx),
            BoardView::Recaps => self.render_recaps(&store, scale, cx),
        };
        v_flex()
            .id("board-page")
            .size_full()
            .bg(t.surface0)
            .child(
                h_flex()
                    .flex_none()
                    .flex_wrap()
                    .items_center()
                    .justify_between()
                    .gap(toolbar_gap)
                    .px(gutter)
                    .pt(toolbar_top)
                    .pb(toolbar_bottom)
                    .child(self.view_switch(cx))
                    .child(self.size_control(size, cx)),
            )
            .child(
                v_flex()
                    .flex_1()
                    .min_h_0()
                    .px(gutter)
                    .pt(s(8.))
                    .pb(gutter)
                    .gap(s(28.))
                    .child(body)
                    .when(self.view == BoardView::Runs, |page| {
                        page.child(
                            div()
                                .flex_none()
                                .text_size(s(12.))
                                .line_height(s(16.))
                                .text_color(t.foreground_muted)
                                .child("Each host keeps its last 50 finished conversations."),
                        )
                    }),
            )
            .into_any_element()
    }
}
