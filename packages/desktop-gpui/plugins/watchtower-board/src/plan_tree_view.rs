//! The plan tree of `client/plan-tree-view.tsx`: the Plan cell's top slice with the pace line and
//! the lead row, then one list item per wave header and per TASK row, hanging off one spine.

use crate::cell::{Slice, cell_slice, cell_title};
use crate::cells::pace_line;
use crate::model::{
    PaceItem, PlanTreeRow, PlanTreeWave, RowState, RunAgent, TaskBuilder, lead_text, row_state,
    secondary_line,
};
use bachuc_icons::{IconName, icon};
use bachuc_theme::metrics::{FONT_SM, WEIGHT_BOLD, WEIGHT_MEDIUM, WEIGHT_NORMAL, WEIGHT_SEMIBOLD};
use bachuc_theme::{mono_font_family, tokens, ui_font_size};
use gpui_kit::component::Sizable as _;
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use std::rc::Rc;
use std::time::Duration;

/// The state icon's 18 px box; the spine is 2 wide at its center, inset 8, with a 14 gap and
/// 16 px ticks to each row.
const ICON_BOX: f32 = 18.;
const SPINE_WIDTH: f32 = 2.;
const SPINE_INSET: f32 = ICON_BOX / 2. - SPINE_WIDTH / 2.;
const SPINE_GAP: f32 = 14.;
const TICK: f32 = SPINE_GAP + SPINE_WIDTH;
const STATE_ICON: f32 = 16.;
/// The planned circle: 14 wide, a 1.5 dashed border.
const PLANNED_CIRCLE: f32 = 14.;
const PLANNED_BORDER: f32 = 1.5;
/// One turn per 1,000 ms.
const TURN: Duration = Duration::from_millis(1000);
/// Task rows: padding 7 by 8, radius 8, `gap: 8`; the waiting tint is 8%.
const ROW_PADDING_Y: f32 = 7.;
const ROW_PADDING_X: f32 = 8.;
const ROW_RADIUS: f32 = 8.;
const ROW_GAP: f32 = 8.;
const WAITING_TINT: f32 = 0.08;
const ID_TEXT: f32 = 11.;
const TITLE_TEXT: f32 = 13.;
const SECONDARY_TEXT: f32 = 11.5;
const SECONDARY_GAP: f32 = 1.;
/// A wave's `marginBottom: 10`; its header padding 4 by 8 and title 12.5.
const WAVE_GAP: f32 = 10.;
const WAVE_PADDING_Y: f32 = 4.;
const WAVE_PADDING_X: f32 = 8.;
const WAVE_TITLE: f32 = 12.5;
/// The lead row: `gap: 10`, padding 8 by 10, `marginBottom: 10`, radius 10, a 24 box with
/// radius 6 and a 14 icon.
const LEAD_GAP: f32 = 10.;
const LEAD_PADDING_Y: f32 = 8.;
const LEAD_PADDING_X: f32 = 10.;
const LEAD_RADIUS: f32 = 10.;
const LEAD_BOX: f32 = 24.;
const LEAD_BOX_RADIUS: f32 = 6.;
const LEAD_ICON: f32 = 14.;
/// Open thread: padding 10 by 5, radius 7.
const OPEN_PADDING_X: f32 = 10.;
const OPEN_PADDING_Y: f32 = 5.;
const OPEN_RADIUS: f32 = 7.;

/// A handler that takes a TASK or agent id.
pub(crate) type OnId = Rc<dyn Fn(String, &mut Window, &mut App)>;

fn open_thread(id: SharedString, agent_id: String, on_open: OnId, cx: &App) -> AnyElement {
    let colors = tokens(cx);
    div()
        .id(id)
        .role(Role::Button)
        .aria_label("Open thread")
        .flex_shrink_0()
        .px(bachuc_theme::zoomed(OPEN_PADDING_X, cx))
        .py(bachuc_theme::zoomed(OPEN_PADDING_Y, cx))
        .rounded(bachuc_theme::zoomed(OPEN_RADIUS, cx))
        .border_1()
        .border_color(colors.border)
        .bg(colors.surface0)
        .cursor_pointer()
        .active(|style| style.opacity(0.6))
        .on_click(move |_, window, cx| on_open(agent_id.clone(), window, cx))
        .text_size(ui_font_size(FONT_SM, cx))
        .font_weight(FontWeight(WEIGHT_MEDIUM))
        .text_color(colors.foreground)
        .child("Open thread")
        .into_any_element()
}

/// The Plan cell's top slice: the title with the run state, the pace line, and the lead row.
pub(crate) fn plan_top(
    run_state: &str,
    pace: &[PaceItem],
    lead: Option<&RunAgent>,
    builder_count: usize,
    plan_title: &str,
    on_open_agent: OnId,
    cx: &App,
) -> AnyElement {
    let colors = tokens(cx);
    let (title, line) = lead_text(lead, builder_count, plan_title, run_state);
    let lead_row = div()
        .flex()
        .flex_row()
        .items_center()
        .gap(bachuc_theme::zoomed(LEAD_GAP, cx))
        .py(bachuc_theme::zoomed(LEAD_PADDING_Y, cx))
        .px(bachuc_theme::zoomed(LEAD_PADDING_X, cx))
        .rounded(bachuc_theme::zoomed(LEAD_RADIUS, cx))
        .border_1()
        .border_color(colors.border)
        .bg(colors.surface1)
        .child(
            div()
                .flex_shrink_0()
                .size(bachuc_theme::zoomed(LEAD_BOX, cx))
                .rounded(bachuc_theme::zoomed(LEAD_BOX_RADIUS, cx))
                .flex()
                .items_center()
                .justify_center()
                .bg(colors.surface2)
                .child(
                    icon(IconName::Bot)
                        .with_size(bachuc_theme::zoomed(LEAD_ICON, cx))
                        .text_color(colors.foreground),
                ),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .child(
                    div()
                        .truncate()
                        .text_size(ui_font_size(TITLE_TEXT, cx))
                        .font_weight(FontWeight(WEIGHT_SEMIBOLD))
                        .text_color(colors.foreground)
                        .child(title),
                )
                .child(
                    div()
                        .pt(bachuc_theme::zoomed(SECONDARY_GAP, cx))
                        .truncate()
                        .text_size(ui_font_size(SECONDARY_TEXT, cx))
                        .text_color(colors.foreground_muted)
                        .child(line),
                ),
        )
        .children(
            lead.map(|lead| open_thread("wt-lead-open".into(), lead.id.clone(), on_open_agent, cx)),
        );
    cell_slice(Slice::Top, false, cx)
        .child(cell_title(
            "Plan",
            Some(SharedString::from(run_state.to_string())),
            cx,
        ))
        .children(pace_line(pace, cx))
        .child(div().pb(bachuc_theme::zoomed(WAVE_GAP, cx)).child(lead_row))
        .into_any_element()
}

/// "Wave N" and its header; an empty wave closes with its gap here.
pub(crate) fn wave_header(wave: &PlanTreeWave, cx: &App) -> AnyElement {
    let colors = tokens(cx);
    cell_slice(Slice::Middle, false, cx)
        .child(
            div()
                .id(SharedString::from(format!("wt-wave-{}", wave.number)))
                .role(Role::Heading)
                .aria_label(SharedString::from(format!(
                    "Wave {}, {}",
                    wave.number, wave.header
                )))
                .flex()
                .flex_row()
                .items_baseline()
                .gap(bachuc_theme::zoomed(ROW_GAP, cx))
                .py(bachuc_theme::zoomed(WAVE_PADDING_Y, cx))
                .px(bachuc_theme::zoomed(WAVE_PADDING_X, cx))
                .when(wave.rows.is_empty(), |this| {
                    this.mb(bachuc_theme::zoomed(WAVE_GAP, cx))
                })
                .child(
                    div()
                        .flex_shrink_0()
                        .text_size(ui_font_size(WAVE_TITLE, cx))
                        .font_weight(FontWeight(WEIGHT_SEMIBOLD))
                        .text_color(colors.foreground)
                        .child(format!("Wave {}", wave.number)),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .text_size(ui_font_size(SECONDARY_TEXT, cx))
                        .text_color(colors.foreground_muted)
                        .child(wave.header.clone()),
                ),
        )
        .into_any_element()
}

/// The row's state glyph in its 18 px box. The running spinner animates only while it is
/// painted, so it requests frames only while its row is in view.
fn state_icon(state: RowState, id: &str, cx: &App) -> AnyElement {
    let colors = tokens(cx);
    let glyph = match state {
        RowState::Done => icon(IconName::CircleCheck)
            .with_size(bachuc_theme::zoomed(STATE_ICON, cx))
            .text_color(colors.status_success)
            .into_any_element(),
        RowState::Running => icon(IconName::LoaderCircle)
            .with_size(bachuc_theme::zoomed(STATE_ICON, cx))
            .text_color(colors.accent)
            .with_animation(
                SharedString::from(format!("wt-spin-{id}")),
                Animation::new(TURN).repeat(),
                |icon, delta| icon.transform(Transformation::rotate(percentage(delta))),
            )
            .into_any_element(),
        RowState::Waiting => icon(IconName::Hand)
            .with_size(bachuc_theme::zoomed(STATE_ICON, cx))
            .text_color(colors.status_warning)
            .into_any_element(),
        RowState::Blocked => icon(IconName::CircleAlert)
            .with_size(bachuc_theme::zoomed(STATE_ICON, cx))
            .text_color(colors.foreground)
            .into_any_element(),
        RowState::Planned => div()
            .size(bachuc_theme::zoomed(PLANNED_CIRCLE, cx))
            .rounded_full()
            .border(bachuc_theme::zoomed(PLANNED_BORDER, cx))
            .border_dashed()
            .border_color(colors.foreground_muted)
            .into_any_element(),
    };
    div()
        .flex_shrink_0()
        .size(bachuc_theme::zoomed(ICON_BOX, cx))
        .flex()
        .items_center()
        .justify_center()
        .child(glyph)
        .into_any_element()
}

/// One TASK row. Off the narrow layout it sits right of the spine with a tick to it.
#[allow(clippy::too_many_arguments)]
pub(crate) fn task_row(
    row: &PlanTreeRow,
    builder: Option<&TaskBuilder>,
    task_class: Option<&str>,
    sha: Option<&str>,
    narrow: bool,
    last: bool,
    on_select: OnId,
    on_open_agent: OnId,
    cx: &App,
) -> AnyElement {
    let colors = tokens(cx);
    let state = row_state(row, builder);
    let waiting = state == RowState::Waiting;
    let secondary = secondary_line(row, state, builder, task_class, sha);
    let task_id = row.id.clone();
    let pressable = div()
        .id(SharedString::from(format!("wt-tree-{}", row.id)))
        .role(Role::Button)
        .aria_label(SharedString::from(format!(
            "{}, {}, {}",
            row.id,
            row.title,
            state.label()
        )))
        .flex_1()
        .min_w_0()
        .flex()
        .flex_row()
        .items_center()
        .gap(bachuc_theme::zoomed(ROW_GAP, cx))
        .cursor_pointer()
        .active(|style| style.opacity(0.6))
        .on_click(move |_, window, cx| on_select(task_id.clone(), window, cx))
        .child(state_icon(state, &row.id, cx))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .items_baseline()
                        .gap(bachuc_theme::zoomed(ROW_GAP, cx))
                        .child(
                            div()
                                .flex_shrink_0()
                                .font_family(mono_font_family(cx))
                                .text_size(ui_font_size(ID_TEXT, cx))
                                .text_color(colors.foreground_muted)
                                .child(row.id.clone()),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .truncate()
                                .text_size(ui_font_size(TITLE_TEXT, cx))
                                .font_weight(FontWeight(WEIGHT_MEDIUM))
                                .text_color(if state == RowState::Planned {
                                    colors.foreground_muted
                                } else {
                                    colors.foreground
                                })
                                .child(row.title.clone()),
                        ),
                )
                .when(!secondary.is_empty(), |this| {
                    this.child(
                        div()
                            .pt(bachuc_theme::zoomed(SECONDARY_GAP, cx))
                            .truncate()
                            .text_size(ui_font_size(SECONDARY_TEXT, cx))
                            .text_color(if waiting {
                                colors.status_warning
                            } else {
                                colors.foreground_muted
                            })
                            .child(secondary),
                    )
                }),
        )
        .children(row.time.as_ref().map(|time| {
            div()
                .flex_shrink_0()
                .font_family(mono_font_family(cx))
                .text_size(ui_font_size(ID_TEXT, cx))
                .font_weight(FontWeight(if row.over {
                    WEIGHT_BOLD
                } else {
                    WEIGHT_NORMAL
                }))
                .text_color(if state == RowState::Running {
                    colors.foreground
                } else {
                    colors.foreground_muted
                })
                .child(time.clone())
        }));
    let body = div()
        .relative()
        .flex()
        .flex_row()
        .items_center()
        .gap(bachuc_theme::zoomed(ROW_GAP, cx))
        .py(bachuc_theme::zoomed(ROW_PADDING_Y, cx))
        .px(bachuc_theme::zoomed(ROW_PADDING_X, cx))
        .rounded(bachuc_theme::zoomed(ROW_RADIUS, cx))
        .overflow_hidden()
        .border_1()
        .border_color(if waiting {
            colors.status_warning
        } else {
            transparent_black()
        })
        .when(waiting, |this| {
            this.child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full()
                    .bg(colors.status_warning.opacity(WAITING_TINT)),
            )
        })
        .child(pressable)
        .children(builder.filter(|_| waiting).map(|builder| {
            open_thread(
                SharedString::from(format!("wt-tree-open-{}", row.id)),
                builder.id.clone(),
                on_open_agent,
                cx,
            )
        }));
    let row_box = div().relative().child(body).when(!narrow, |this| {
        this.child(
            div()
                .absolute()
                .left(bachuc_theme::zoomed(-TICK, cx))
                .top(relative(0.5))
                .mt(bachuc_theme::zoomed(-SPINE_WIDTH / 2., cx))
                .w(bachuc_theme::zoomed(TICK, cx))
                .h(bachuc_theme::zoomed(SPINE_WIDTH, cx))
                .bg(colors.border),
        )
    });
    let hung = if narrow {
        row_box
    } else {
        div()
            .ml(bachuc_theme::zoomed(SPINE_INSET, cx))
            .pl(bachuc_theme::zoomed(SPINE_GAP, cx))
            .border_l(bachuc_theme::zoomed(SPINE_WIDTH, cx))
            .border_color(colors.border)
            .child(row_box)
    };
    cell_slice(Slice::Middle, false, cx)
        .child(
            div()
                .when(last, |this| this.pb(bachuc_theme::zoomed(WAVE_GAP, cx)))
                .child(hung),
        )
        .into_any_element()
}
