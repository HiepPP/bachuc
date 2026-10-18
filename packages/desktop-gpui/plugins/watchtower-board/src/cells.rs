//! The presentational cells of `client/overview-cells.tsx` and the overview's load states
//! (`client/overview.tsx`): `LifecycleStepper`, `ManualChecksCell`, `DecisionsCell`,
//! `ArchivedPlansCell`, `PaceLine`, the error box, the skeleton, and the no-plan block. They take
//! data and handlers and never fetch.

use crate::cell::{LINE_HEIGHT, empty, limit, overview_cell, show_more};
use crate::model::{
    DecisionRow, History, ManualCheck, PaceItem, Phase, PhaseName, PhaseState, SegmentKind,
    StatusCounts, archived_meta, segments,
};
use bachuc_icons::{IconName, icon};
use bachuc_theme::metrics::{FONT_SM, WEIGHT_BOLD, WEIGHT_MEDIUM, WEIGHT_SEMIBOLD};
use bachuc_theme::{mono_font_family, tokens, ui_font_size};
use gpui_kit::component::Sizable as _;
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use std::rc::Rc;

/// The gap between overview blocks, `GAP = 14`.
pub(crate) const GAP: f32 = 14.;
/// The stepper's `borderRadius: 10`, padding 14 by 11 and 12, or 4 by 8 and 9 when compact.
const STEPPER_RADIUS: f32 = 10.;
const STEP_PADDING_X: f32 = 14.;
const STEP_PADDING_TOP: f32 = 11.;
const STEP_PADDING_BOTTOM: f32 = 12.;
const STEP_PADDING_X_COMPACT: f32 = 4.;
const STEP_PADDING_TOP_COMPACT: f32 = 8.;
const STEP_PADDING_BOTTOM_COMPACT: f32 = 9.;
/// The current step's 2 px accent bar and 6% accent overlay.
const STEP_BAR: f32 = 2.;
const STEP_TINT: f32 = 0.06;
const STEP_ICON: f32 = 14.;
const STEP_NAME: f32 = 13.;
const STEP_META: f32 = 11.5;
const STEP_META_GAP: f32 = 3.;
/// `Segments`: `height: 4`, `gap: 2`, `marginTop: 8`, `borderRadius: 2`, todo at 25%.
const SEGMENT_HEIGHT: f32 = 4.;
const SEGMENT_GAP: f32 = 2.;
const SEGMENT_TOP: f32 = 8.;
const SEGMENT_RADIUS: f32 = 2.;
const TODO_OPACITY: f32 = 0.25;
/// History and check rows: `gap: 8`, `paddingVertical: 7`, text 12.5 and 11, meta `marginTop: 1`.
const ROW_GAP: f32 = 8.;
const ROW_PADDING: f32 = 7.;
const ROW_TEXT: f32 = 12.5;
const ROW_LINE: f32 = 17.;
const META_TEXT: f32 = 11.;
const META_GAP: f32 = 1.;
const ROW_ICON: f32 = 14.;
const ICON_TOP: f32 = 2.;
/// The checkbox: 13 square, radius 4, border 1.5 at 60%.
const CHECKBOX: f32 = 13.;
const CHECKBOX_RADIUS: f32 = 4.;
const CHECKBOX_BORDER: f32 = 1.5;
const CHECKBOX_OPACITY: f32 = 0.6;
/// `PaceLine`: `columnGap: 24`, `rowGap: 8`, `marginBottom: 12`, value mono 15, label 11.
const PACE_COLUMN_GAP: f32 = 24.;
const PACE_ROW_GAP: f32 = 8.;
const PACE_BOTTOM: f32 = 12.;
const PACE_VALUE: f32 = 15.;
/// The error box: `gap: 12`, `padding: 14`, `borderRadius: 10`, text 13; Retry 10 by 6, radius 7.
const ERROR_GAP: f32 = 12.;
const ERROR_PADDING: f32 = 14.;
const ERROR_RADIUS: f32 = 10.;
const ERROR_TEXT: f32 = 13.;
const ERROR_ICON: f32 = 15.;
const BUTTON_PADDING_X: f32 = 10.;
const BUTTON_PADDING_Y: f32 = 6.;
const BUTTON_RADIUS: f32 = 7.;
/// The skeleton: bars 8 high, radius 4, `marginBottom: 10`, at 12% muted, in radius 12 boxes.
const BAR_HEIGHT: f32 = 8.;
const BAR_RADIUS: f32 = 4.;
const BAR_GAP: f32 = 10.;
const BAR_OPACITY: f32 = 0.12;
const PLACEHOLDER_RADIUS: f32 = 12.;
const PLACEHOLDER_PADDING: f32 = 14.;
/// The no-plan block: `paddingVertical: 12`, a 32 box with radius 8 and a 16 icon, the title 15
/// at `marginTop: 12`, the message 12.5 at `marginTop: 4` and `maxWidth: 520`.
const NO_PLAN_PADDING: f32 = 12.;
const NO_PLAN_BOX: f32 = 32.;
const NO_PLAN_RADIUS: f32 = 8.;
const NO_PLAN_ICON: f32 = 16.;
const NO_PLAN_TITLE: f32 = 15.;
const NO_PLAN_TITLE_GAP: f32 = 12.;
const NO_PLAN_TEXT: f32 = 12.5;
const NO_PLAN_TEXT_GAP: f32 = 4.;
const NO_PLAN_TEXT_WIDTH: f32 = 520.;
/// Warnings: `fontSize: 12` in `statusWarning`.
const WARNING_TEXT: f32 = 12.;

/// A click handler that several elements share.
pub(crate) type OnClick = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;

fn phase_icon(name: PhaseName) -> IconName {
    match name {
        PhaseName::Plan => IconName::FileText,
        PhaseName::Implement => IconName::Play,
        PhaseName::Verify => IconName::ShieldCheck,
        PhaseName::Review => IconName::GitPullRequest,
        PhaseName::Archive => IconName::Archive,
    }
}

fn phase_state_label(state: PhaseState) -> &'static str {
    match state {
        PhaseState::Done => "done",
        PhaseState::Current => "current",
        PhaseState::Todo => "todo",
    }
}

fn implement_segments(counts: StatusCounts, cx: &App) -> Div {
    let colors = tokens(cx);
    div()
        .mt(bachuc_theme::zoomed(SEGMENT_TOP, cx))
        .h(bachuc_theme::zoomed(SEGMENT_HEIGHT, cx))
        .flex()
        .flex_row()
        .gap(bachuc_theme::zoomed(SEGMENT_GAP, cx))
        .children(segments(counts).into_iter().map(|(kind, value)| {
            let (color, opacity) = match kind {
                SegmentKind::Done => (colors.status_success, 1.),
                SegmentKind::Running => (colors.accent, 1.),
                SegmentKind::Blocked => (colors.status_danger, 1.),
                SegmentKind::Todo => (colors.foreground_muted, TODO_OPACITY),
            };
            div()
                .flex_grow(value as f32)
                .flex_basis(px(0.))
                .h_full()
                .rounded(bachuc_theme::zoomed(SEGMENT_RADIUS, cx))
                .bg(color.opacity(opacity))
        }))
}

/// `LifecycleStepper`: five steps; a narrow panel shows names only.
pub(crate) fn lifecycle_stepper(
    phases: &[Phase],
    counts: StatusCounts,
    compact: bool,
    cx: &App,
) -> AnyElement {
    let colors = tokens(cx);
    div()
        .id("wt-stepper")
        .role(Role::Group)
        .aria_label("Lifecycle")
        .w_full()
        .flex()
        .flex_row()
        .border_1()
        .border_color(colors.border)
        .rounded(bachuc_theme::zoomed(STEPPER_RADIUS, cx))
        .overflow_hidden()
        .line_height(relative(LINE_HEIGHT))
        .children(phases.iter().enumerate().map(|(index, phase)| {
            let current = phase.state == PhaseState::Current;
            let color = match phase.state {
                PhaseState::Done => colors.status_success,
                PhaseState::Current => colors.accent,
                PhaseState::Todo => colors.foreground_muted,
            };
            let name = phase.name.label();
            let label = format!("{name}, {}, {}", phase_state_label(phase.state), phase.meta);
            div()
                .id(SharedString::from(format!("wt-step-{name}")))
                .role(Role::ListItem)
                .aria_label(SharedString::from(label))
                .relative()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .px(bachuc_theme::zoomed(
                    if compact {
                        STEP_PADDING_X_COMPACT
                    } else {
                        STEP_PADDING_X
                    },
                    cx,
                ))
                .pt(bachuc_theme::zoomed(
                    if compact {
                        STEP_PADDING_TOP_COMPACT
                    } else {
                        STEP_PADDING_TOP
                    },
                    cx,
                ))
                .pb(bachuc_theme::zoomed(
                    if compact {
                        STEP_PADDING_BOTTOM_COMPACT
                    } else {
                        STEP_PADDING_BOTTOM
                    },
                    cx,
                ))
                .when(index > 0, |this| {
                    this.border_l_1().border_color(colors.border)
                })
                .when(current, |this| {
                    this.child(
                        div()
                            .absolute()
                            .top_0()
                            .left_0()
                            .size_full()
                            .bg(colors.accent.opacity(STEP_TINT)),
                    )
                    .child(
                        div()
                            .absolute()
                            .left_0()
                            .right_0()
                            .bottom_0()
                            .h(bachuc_theme::zoomed(STEP_BAR, cx))
                            .bg(colors.accent),
                    )
                })
                .map(|this| {
                    if compact {
                        this.child(
                            div()
                                .truncate()
                                .text_center()
                                .text_size(ui_font_size(STEP_META, cx))
                                .font_weight(FontWeight(if current {
                                    WEIGHT_BOLD
                                } else {
                                    WEIGHT_MEDIUM
                                }))
                                .text_color(if current { colors.foreground } else { color })
                                .child(name),
                        )
                    } else {
                        this.child(
                            div()
                                .flex()
                                .flex_row()
                                .items_center()
                                .gap(bachuc_theme::zoomed(ROW_GAP, cx))
                                .child(
                                    icon(if phase.state == PhaseState::Done {
                                        IconName::CircleCheck
                                    } else {
                                        phase_icon(phase.name)
                                    })
                                    .with_size(bachuc_theme::zoomed(STEP_ICON, cx))
                                    .text_color(color),
                                )
                                .child(
                                    div()
                                        .text_size(ui_font_size(STEP_NAME, cx))
                                        .font_weight(FontWeight(WEIGHT_SEMIBOLD))
                                        .text_color(colors.foreground)
                                        .child(name),
                                ),
                        )
                        .child(
                            div()
                                .pt(bachuc_theme::zoomed(STEP_META_GAP, cx))
                                .truncate()
                                .text_size(ui_font_size(STEP_META, cx))
                                .text_color(colors.foreground_muted)
                                .child(phase.meta.clone()),
                        )
                    }
                })
                .when(
                    current && phase.name == PhaseName::Implement && !compact,
                    |this| this.child(implement_segments(counts, cx)),
                )
        }))
        .into_any_element()
}

/// An overview warning line.
pub(crate) fn warning_line(text: &str, cx: &App) -> AnyElement {
    div()
        .text_size(ui_font_size(WARNING_TEXT, cx))
        .text_color(tokens(cx).status_warning)
        .child(text.to_string())
        .into_any_element()
}

/// The frame of a row in a cell list: the top border except on the first row.
fn list_row(first: bool, cx: &App) -> Div {
    div()
        .w_full()
        .flex()
        .flex_row()
        .items_start()
        .gap(bachuc_theme::zoomed(ROW_GAP, cx))
        .py(bachuc_theme::zoomed(ROW_PADDING, cx))
        .when(!first, |this| {
            this.border_t_1().border_color(tokens(cx).border)
        })
}

/// `ManualChecksCell`.
pub(crate) fn manual_checks_cell(
    checks: &[ManualCheck],
    expanded: bool,
    on_toggle: OnClick,
    cx: &App,
) -> AnyElement {
    let colors = tokens(cx);
    let shown = limit(checks, expanded);
    let aside =
        (!checks.is_empty()).then(|| SharedString::from(format!("{} pending", checks.len())));
    let body = div()
        .flex()
        .flex_col()
        .when(shown.is_empty(), |this| {
            this.child(empty("No manual checks pending.", cx))
        })
        .children(shown.iter().enumerate().map(|(index, check)| {
            list_row(index == 0, cx)
                .child(
                    div()
                        .mt(bachuc_theme::zoomed(ICON_TOP, cx))
                        .flex_shrink_0()
                        .size(bachuc_theme::zoomed(CHECKBOX, cx))
                        .rounded(bachuc_theme::zoomed(CHECKBOX_RADIUS, cx))
                        .border(bachuc_theme::zoomed(CHECKBOX_BORDER, cx))
                        .border_color(colors.foreground_muted.opacity(CHECKBOX_OPACITY)),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .line_clamp(2)
                                .text_size(ui_font_size(ROW_TEXT, cx))
                                .line_height(bachuc_theme::zoomed(ROW_LINE, cx))
                                .text_color(colors.foreground)
                                .child(check.text.clone()),
                        )
                        .when(!check.tasks.is_empty(), |this| {
                            this.child(
                                div()
                                    .pt(bachuc_theme::zoomed(META_GAP, cx))
                                    .text_size(ui_font_size(META_TEXT, cx))
                                    .text_color(colors.foreground_muted)
                                    .child(check.tasks.join(", ")),
                            )
                        }),
                )
        }))
        .children(show_more(
            "wt-checks-more",
            checks.len(),
            expanded,
            move |event, window, cx| on_toggle(event, window, cx),
            cx,
        ));
    overview_cell("Manual checks", aside, false, body, cx)
}

/// `HistoryRow`: a button when `on_press` is set.
#[allow(clippy::too_many_arguments)]
fn history_row(
    id: SharedString,
    glyph: IconName,
    glyph_color: Hsla,
    title: &str,
    meta: &str,
    first: bool,
    press: Option<(SharedString, OnClick)>,
    cx: &App,
) -> AnyElement {
    let colors = tokens(cx);
    let content = list_row(first, cx)
        .child(
            div()
                .mt(bachuc_theme::zoomed(ICON_TOP, cx))
                .flex_shrink_0()
                .child(
                    icon(glyph)
                        .with_size(bachuc_theme::zoomed(ROW_ICON, cx))
                        .text_color(glyph_color),
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
                        .text_size(ui_font_size(ROW_TEXT, cx))
                        .text_color(colors.foreground)
                        .child(title.to_string()),
                )
                .child(
                    div()
                        .pt(bachuc_theme::zoomed(META_GAP, cx))
                        .truncate()
                        .text_size(ui_font_size(META_TEXT, cx))
                        .text_color(colors.foreground_muted)
                        .child(meta.to_string()),
                ),
        );
    match press {
        Some((label, on_press)) => div()
            .id(id)
            .role(Role::Button)
            .aria_label(label)
            .cursor_pointer()
            .active(|style| style.opacity(0.6))
            .on_click(move |event, window, cx| on_press(event, window, cx))
            .child(content)
            .into_any_element(),
        None => content.into_any_element(),
    }
}

/// Opens an ADR by its id and title.
pub(crate) type OnOpenDecision = Rc<dyn Fn(String, String, &mut Window, &mut App)>;

/// `DecisionsCell`.
pub(crate) fn decisions_cell(
    rows: &[DecisionRow],
    expanded: bool,
    on_toggle: OnClick,
    on_open: OnOpenDecision,
    cx: &App,
) -> AnyElement {
    let colors = tokens(cx);
    let shown = limit(rows, expanded);
    let body = div()
        .flex()
        .flex_col()
        .when(shown.is_empty(), |this| {
            this.child(empty("No decisions yet.", cx))
        })
        .children(shown.iter().enumerate().map(|(index, row)| {
            let press = row.pressable.then(|| {
                let on_open = on_open.clone();
                let (id, title) = (row.id.clone(), row.title.clone());
                let handler: OnClick =
                    Rc::new(move |_, window, cx| on_open(id.clone(), title.clone(), window, cx));
                (
                    SharedString::from(format!("Open {} {}", row.id, row.title)),
                    handler,
                )
            });
            history_row(
                SharedString::from(format!("wt-decision-{}", row.id)),
                IconName::Scale,
                if row.proposed {
                    colors.status_warning
                } else {
                    colors.foreground_muted
                },
                &row.title,
                &row.meta,
                index == 0,
                press,
                cx,
            )
        }))
        .children(show_more(
            "wt-decisions-more",
            rows.len(),
            expanded,
            move |event, window, cx| on_toggle(event, window, cx),
            cx,
        ));
    overview_cell("Decisions", None, false, body, cx)
}

/// `ArchivedPlansCell`.
pub(crate) fn archived_plans_cell(
    history: &[History],
    expanded: bool,
    on_toggle: OnClick,
    cx: &App,
) -> AnyElement {
    let colors = tokens(cx);
    let shown = limit(history, expanded);
    let body = div()
        .flex()
        .flex_col()
        .when(shown.is_empty(), |this| {
            this.child(empty("No archived plans yet.", cx))
        })
        .children(shown.iter().enumerate().map(|(index, plan)| {
            history_row(
                SharedString::from(format!("wt-archived-{}", plan.slug)),
                IconName::Archive,
                colors.foreground_muted,
                &plan.title,
                &archived_meta(plan),
                index == 0,
                None,
                cx,
            )
        }))
        .children(show_more(
            "wt-archived-more",
            history.len(),
            expanded,
            move |event, window, cx| on_toggle(event, window, cx),
            cx,
        ));
    overview_cell("Archived plans", None, false, body, cx)
}

/// `PaceLine`; `None` when it has no number.
pub(crate) fn pace_line(items: &[PaceItem], cx: &App) -> Option<AnyElement> {
    if items.is_empty() {
        return None;
    }
    let colors = tokens(cx);
    Some(
        div()
            .id("wt-pace")
            .role(Role::Group)
            .aria_label(SharedString::from(
                items
                    .iter()
                    .map(|item| format!("{} {}", item.value, item.label))
                    .collect::<Vec<_>>()
                    .join(", "),
            ))
            .flex()
            .flex_row()
            .flex_wrap()
            .gap_x(bachuc_theme::zoomed(PACE_COLUMN_GAP, cx))
            .gap_y(bachuc_theme::zoomed(PACE_ROW_GAP, cx))
            .pb(bachuc_theme::zoomed(PACE_BOTTOM, cx))
            .children(items.iter().map(|item| {
                div()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .font_family(mono_font_family(cx))
                            .text_size(ui_font_size(PACE_VALUE, cx))
                            .font_weight(FontWeight(WEIGHT_SEMIBOLD))
                            .text_color(colors.foreground)
                            .child(item.value.clone()),
                    )
                    .child(
                        div()
                            .pt(bachuc_theme::zoomed(META_GAP, cx))
                            .text_size(ui_font_size(META_TEXT, cx))
                            .text_color(colors.foreground_muted)
                            .child(item.label),
                    )
            }))
            .into_any_element(),
    )
}

/// A bordered text button such as Retry.
pub(crate) fn outline_button(
    id: impl Into<ElementId>,
    label: &'static str,
    aria: SharedString,
    on_press: OnClick,
    cx: &App,
) -> AnyElement {
    let colors = tokens(cx);
    div()
        .id(id)
        .role(Role::Button)
        .aria_label(aria)
        .flex_shrink_0()
        .px(bachuc_theme::zoomed(BUTTON_PADDING_X, cx))
        .py(bachuc_theme::zoomed(BUTTON_PADDING_Y, cx))
        .rounded(bachuc_theme::zoomed(BUTTON_RADIUS, cx))
        .border_1()
        .border_color(colors.border)
        .cursor_pointer()
        .active(|style| style.opacity(0.6))
        .on_click(move |event, window, cx| on_press(event, window, cx))
        .text_size(ui_font_size(FONT_SM, cx))
        .font_weight(FontWeight(WEIGHT_MEDIUM))
        .text_color(colors.foreground)
        .child(label)
        .into_any_element()
}

/// "Could not load the overview." with Retry.
pub(crate) fn error_box(message: &str, on_retry: OnClick, cx: &App) -> AnyElement {
    let colors = tokens(cx);
    let text = format!("Could not load the overview. {message}");
    div()
        .id("wt-overview-error")
        .role(Role::Alert)
        .aria_label(SharedString::from(text.clone()))
        .flex()
        .flex_row()
        .items_center()
        .gap(bachuc_theme::zoomed(ERROR_GAP, cx))
        .p(bachuc_theme::zoomed(ERROR_PADDING, cx))
        .rounded(bachuc_theme::zoomed(ERROR_RADIUS, cx))
        .border_1()
        .border_color(colors.status_danger)
        .line_height(relative(LINE_HEIGHT))
        .child(
            icon(IconName::CircleAlert)
                .with_size(bachuc_theme::zoomed(ERROR_ICON, cx))
                .text_color(colors.status_danger),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .text_size(ui_font_size(ERROR_TEXT, cx))
                .text_color(colors.status_danger)
                .child(text),
        )
        .child(outline_button(
            "wt-overview-retry",
            "Retry",
            "Retry loading the overview".into(),
            on_retry,
            cx,
        ))
        .into_any_element()
}

fn placeholder(height: f32, cx: &App) -> Div {
    let colors = tokens(cx);
    div()
        .flex_1()
        .min_w_0()
        .h(bachuc_theme::zoomed(height, cx))
        .rounded(bachuc_theme::zoomed(PLACEHOLDER_RADIUS, cx))
        .border_1()
        .border_color(colors.border)
        .p(bachuc_theme::zoomed(PLACEHOLDER_PADDING, cx))
        .children([0.5, 0.8, 0.65].map(|fraction| {
            div()
                .w(relative(fraction))
                .h(bachuc_theme::zoomed(BAR_HEIGHT, cx))
                .mb(bachuc_theme::zoomed(BAR_GAP, cx))
                .rounded(bachuc_theme::zoomed(BAR_RADIUS, cx))
                .bg(colors.foreground_muted.opacity(BAR_OPACITY))
        }))
}

/// The loading skeleton: the stepper and three rows of two boxes.
pub(crate) fn skeleton(cx: &App) -> AnyElement {
    let row = |left: f32, right: f32, cx: &App| {
        div()
            .flex()
            .flex_row()
            .gap(bachuc_theme::zoomed(GAP, cx))
            .child(placeholder(left, cx))
            .child(placeholder(right, cx))
    };
    div()
        .id("wt-overview-loading")
        .role(Role::Status)
        .aria_label("Loading Watchtower overview")
        .flex()
        .flex_col()
        .gap(bachuc_theme::zoomed(GAP, cx))
        .child(div().flex().child(placeholder(58., cx)))
        .child(row(240., 240., cx))
        .child(row(200., 200., cx))
        .child(row(200., 200., cx))
        .into_any_element()
}

/// "No plan yet" with the server's message.
pub(crate) fn no_plan(message: &str, cx: &App) -> AnyElement {
    let colors = tokens(cx);
    div()
        .id("wt-no-plan")
        .role(Role::Status)
        .aria_label(SharedString::from(format!("No plan yet. {message}")))
        .py(bachuc_theme::zoomed(NO_PLAN_PADDING, cx))
        .flex()
        .flex_col()
        .line_height(relative(LINE_HEIGHT))
        .child(
            div()
                .size(bachuc_theme::zoomed(NO_PLAN_BOX, cx))
                .rounded(bachuc_theme::zoomed(NO_PLAN_RADIUS, cx))
                .flex()
                .items_center()
                .justify_center()
                .bg(colors.surface2)
                .child(
                    icon(IconName::ListChecks)
                        .with_size(bachuc_theme::zoomed(NO_PLAN_ICON, cx))
                        .text_color(colors.foreground_muted),
                ),
        )
        .child(
            div()
                .pt(bachuc_theme::zoomed(NO_PLAN_TITLE_GAP, cx))
                .text_size(ui_font_size(NO_PLAN_TITLE, cx))
                .font_weight(FontWeight(WEIGHT_SEMIBOLD))
                .text_color(colors.foreground)
                .child("No plan yet"),
        )
        .child(
            div()
                .pt(bachuc_theme::zoomed(NO_PLAN_TEXT_GAP, cx))
                .max_w(bachuc_theme::zoomed(NO_PLAN_TEXT_WIDTH, cx))
                .text_size(ui_font_size(NO_PLAN_TEXT, cx))
                .text_color(colors.foreground_muted)
                .child(message.to_string()),
        )
        .into_any_element()
}
