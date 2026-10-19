//! The Background card, as `background-process-card.tsx` draws it: a running card with a live
//! clock and Stop, a finished card with each result, and the moved note. One card is one list
//! row, so the timeline list lays it out once.

use bachuc_client::BackgroundProcess;
use bachuc_theme::metrics::{RADIUS_LG, RADIUS_MD, SPACE_1, SPACE_2, SPACE_3};
use bachuc_theme::{Tokens, tokens, ui_font_size, zoomed};
use gpui_kit::component::{h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::background::{self, FinishedResult, ResultTone, TurnSlot};
use crate::{RULE_BOTTOM, RULE_TOP, TimelineView};

/// `background-process-card.tsx` sizes: the 7 px dot and the 58 px kind column.
const DOT: f32 = 7.;
const KIND_WIDTH: f32 = 58.;
const ROW_GAP: f32 = 9.;
const CARD_GAP: f32 = SPACE_2;
const CARD_TOP: f32 = 14.;
const TITLE_SIZE: f32 = 12.5;
const KIND_SIZE: f32 = 9.5;
const COMMAND_SIZE: f32 = 12.5;
const TIME_SIZE: f32 = 12.;
const TAIL_SIZE: f32 = 11.5;
const LOG_SIZE: f32 = 11.;
const LOG_LINE_HEIGHT: f32 = 17.;
const PATH_SIZE: f32 = 11.;
const MOVED_SIZE: f32 = 12.;

impl TimelineView {
    /// One card row: the running part, the finished part, and the moved note.
    pub(crate) fn render_background_card(
        &self,
        slot: &TurnSlot,
        rule: bool,
        row: usize,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let t = tokens(cx);
        let running: Vec<&BackgroundProcess> = slot
            .processes
            .iter()
            .filter(|process| process.status == "running")
            .collect();
        let finished: Vec<&BackgroundProcess> = slot
            .processes
            .iter()
            .filter(|process| process.status != "running")
            .collect();
        let supports_stop = self.client.read(cx).supports_background_stop();
        let now = chrono::Utc::now().timestamp_millis();
        let running_rows: Vec<AnyElement> = running
            .iter()
            .map(|process| self.render_process_row(process, true, now, supports_stop, cx))
            .collect();
        let finished_rows: Vec<AnyElement> = finished
            .iter()
            .map(|process| self.render_process_row(process, false, now, supports_stop, cx))
            .collect();
        let card = |id: &'static str| {
            v_flex()
                .id(id)
                .border_1()
                .border_color(t.border)
                .rounded(zoomed(RADIUS_LG, cx))
                .bg(t.surface0)
                .overflow_hidden()
        };
        let mut body = v_flex()
            .id(("background-process-card", row))
            .role(Role::Group)
            .aria_label("Background processes")
            .gap(zoomed(CARD_GAP, cx))
            .mt(zoomed(CARD_TOP, cx))
            .when(!running.is_empty(), |body| {
                body.child(
                    card("background-running-card").child(
                        h_flex()
                            .gap(zoomed(CARD_GAP, cx))
                            .items_center()
                            .px(zoomed(SPACE_3, cx))
                            .py(zoomed(9., cx))
                            .border_b_1()
                            .border_color(t.border)
                            .child(dot(t.dot_running, cx))
                            .child(
                                div()
                                    .text_size(ui_font_size(TITLE_SIZE, cx))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(t.foreground)
                                    .child("Background"),
                            )
                            .child(
                                div()
                                    .text_size(ui_font_size(TITLE_SIZE, cx))
                                    .text_color(t.foreground_muted)
                                    .child(format!("{} running", running.len())),
                            ),
                    )
                    .children(running_rows),
                )
            })
            .when(!finished.is_empty(), |body| {
                body.child(card("background-finished-card").children(finished_rows))
            })
            .when(slot.moved_count > 0, |body| {
                body.child(
                    h_flex()
                        .gap(zoomed(CARD_GAP, cx))
                        .items_center()
                        .child(dot(t.dot_running, cx))
                        .child(
                            div()
                                .text_size(ui_font_size(MOVED_SIZE, cx))
                                .text_color(t.foreground_muted)
                                .child(format!(
                                    "Background processes started here: {} · shown in the latest turn",
                                    slot.moved_count
                                )),
                        ),
                )
            });
        if rule {
            body = body.child(
                div()
                    .mt(zoomed(RULE_TOP, cx))
                    .mb(zoomed(RULE_BOTTOM, cx))
                    .h(px(1.))
                    .w_full()
                    .bg(t.border),
            );
        }
        body.into_any_element()
    }

    fn render_process_row(
        &self,
        process: &BackgroundProcess,
        running: bool,
        now: i64,
        supports_stop: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let t = tokens(cx);
        let id = process.id.clone();
        let label = background::process_label(process).to_string();
        let expanded = self.expanded_processes.contains(&process.id);
        let kind = background::kind_label(&process.kind);
        let view = cx.weak_entity();
        let click_id = id.clone();
        let a11y_id = id.clone();
        let a11y_label = format!("{kind}, {label}");
        let mut main = h_flex()
            .id(ElementId::Name(format!("process-main-{id}").into()))
            .role(Role::Button)
            .aria_label(a11y_label)
            .aria_expanded(expanded)
            .flex_1()
            .min_w_0()
            .gap(zoomed(ROW_GAP, cx))
            .items_center()
            .cursor_pointer()
            .on_click(
                cx.listener(move |this, _, _, cx| this.toggle_process_log(click_id.clone(), cx)),
            )
            .on_a11y_action(AccessibleAction::Click, move |_, _, cx| {
                view.update(cx, |this, cx| this.toggle_process_log(a11y_id.clone(), cx))
                    .ok();
            });
        if running {
            main = main
                .child(dot(t.dot_running, cx))
                .child(kind_column(kind, cx))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .text_size(ui_font_size(COMMAND_SIZE, cx))
                        .text_color(t.foreground)
                        .child(label),
                )
                .child(
                    div()
                        .text_size(ui_font_size(TIME_SIZE, cx))
                        .text_color(t.foreground)
                        .child(crate::format_elapsed(now - process.started_at_ms)),
                );
        } else {
            let result = background::finished_result(&process.status, process.exit_code);
            let color = result_color(&result, &t);
            main = main
                .child(dot(result_dot(&result, &t), cx))
                .child(kind_column(kind, cx))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .text_size(ui_font_size(COMMAND_SIZE, cx))
                        .text_color(t.foreground)
                        .child(label),
                )
                .child(
                    div()
                        .text_size(ui_font_size(TIME_SIZE, cx))
                        .text_color(color)
                        .child(result.text),
                )
                .children(process.ended_at_ms.map(|ended| {
                    div()
                        .text_size(ui_font_size(TIME_SIZE, cx))
                        .text_color(t.foreground_muted)
                        .child(crate::format_duration(ended - process.started_at_ms))
                }));
        }
        let mut row = h_flex().gap(zoomed(ROW_GAP, cx)).items_center().child(main);
        if running && supports_stop {
            let stop_id = process.id.clone();
            let stop_view = cx.weak_entity();
            let a11y_stop = process.id.clone();
            row = row.child(
                div()
                    .id(ElementId::Name(format!("process-stop-{stop_id}").into()))
                    .role(Role::Button)
                    .aria_label("Stop")
                    .flex_none()
                    .px(zoomed(SPACE_2, cx))
                    .py(zoomed(2., cx))
                    .rounded(zoomed(RADIUS_MD, cx))
                    .border_1()
                    .border_color(t.border)
                    .text_size(ui_font_size(TAIL_SIZE, cx))
                    .text_color(t.foreground)
                    .cursor_pointer()
                    .hover(move |button| button.bg(t.surface2))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.stop_background_process(stop_id.clone(), cx)
                    }))
                    .on_a11y_action(AccessibleAction::Click, move |_, _, cx| {
                        stop_view
                            .update(cx, |this, cx| {
                                this.stop_background_process(a11y_stop.clone(), cx)
                            })
                            .ok();
                    })
                    .child("Stop"),
            );
        }
        let log = if expanded {
            Some(self.render_process_log(process, cx))
        } else if running {
            Some(
                div()
                    .w_full()
                    .mt(zoomed(3., cx))
                    .pl(zoomed(DOT + ROW_GAP + KIND_WIDTH + ROW_GAP, cx))
                    .truncate()
                    .text_size(ui_font_size(TAIL_SIZE, cx))
                    .text_color(t.foreground_muted)
                    .child(
                        process
                            .tail
                            .last()
                            .cloned()
                            .unwrap_or_else(|| "no output yet".to_string()),
                    )
                    .into_any_element(),
            )
        } else {
            None
        };
        v_flex()
            .px(zoomed(SPACE_3, cx))
            .py(zoomed(SPACE_2, cx))
            .child(row)
            .children(log)
            .into_any_element()
    }

    fn render_process_log(
        &self,
        process: &BackgroundProcess,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let t = tokens(cx);
        let lines: Vec<String> = if process.tail.is_empty() {
            vec!["no output yet".to_string()]
        } else {
            process.tail.clone()
        };
        div()
            .w_full()
            .mt(zoomed(SPACE_1 + 2., cx))
            .pl(zoomed(DOT + ROW_GAP + KIND_WIDTH + ROW_GAP, cx))
            .child(
                v_flex()
                    .w_full()
                    .gap(px(1.))
                    .px(zoomed(9., cx))
                    .py(zoomed(7., cx))
                    .rounded(zoomed(RADIUS_MD, cx))
                    .border_1()
                    .border_color(t.border)
                    .bg(t.surface1)
                    .children(lines.into_iter().map(|line| {
                        div()
                            .truncate()
                            .text_size(ui_font_size(LOG_SIZE, cx))
                            .line_height(zoomed(LOG_LINE_HEIGHT, cx))
                            .text_color(t.foreground)
                            .child(line)
                    }))
                    .children(process.output_file.clone().map(|path| {
                        div()
                            .mt(zoomed(SPACE_1, cx))
                            .truncate()
                            .text_size(ui_font_size(PATH_SIZE, cx))
                            .text_color(t.foreground_muted)
                            .child(path)
                    })),
            )
            .into_any_element()
    }
}

fn dot(color: Hsla, cx: &App) -> AnyElement {
    div()
        .size(zoomed(DOT, cx))
        .flex_none()
        .rounded_full()
        .bg(color)
        .into_any_element()
}

fn kind_column(kind: &'static str, cx: &App) -> AnyElement {
    div()
        .w(zoomed(KIND_WIDTH, cx))
        .flex_none()
        .text_size(ui_font_size(KIND_SIZE, cx))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(tokens(cx).foreground_muted)
        .child(kind)
        .into_any_element()
}

fn result_dot(result: &FinishedResult, t: &Tokens) -> Hsla {
    match result.tone {
        ResultTone::Success => t.dot_success,
        ResultTone::Danger => t.dot_danger,
        ResultTone::Muted => t.foreground_muted,
    }
}

fn result_color(result: &FinishedResult, t: &Tokens) -> Hsla {
    match result.tone {
        ResultTone::Success => t.status_success,
        ResultTone::Danger => t.status_danger,
        ResultTone::Muted => t.foreground_muted,
    }
}
