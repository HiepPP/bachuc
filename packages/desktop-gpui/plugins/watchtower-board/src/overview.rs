//! The overview body of `client/overview.tsx`: draws one `BodyItem` of the page's list from the
//! page's snapshot. The item list itself comes from `model::body_items`.

use crate::cell::{Slice, cell_slice};
use crate::cells::{
    GAP, OnClick, archived_plans_cell, decisions_cell, error_box, lifecycle_stepper,
    manual_checks_cell, no_plan, skeleton, warning_line,
};
use crate::model::{BodyItem, CellKind, Derived, Overview, active_builders, needs_warning};
use crate::needs_you::{needs_cell, needs_more, needs_row, needs_slice, needs_top};
use crate::page::WatchtowerPage;
use crate::plan_tree_view::{OnId, plan_top, task_row, wave_header};
use crate::run_cells::autorun_strip;
use gpui_kit::*;
use std::rc::Rc;

fn toggle(kind: CellKind, cx: &mut Context<WatchtowerPage>) -> OnClick {
    let weak = cx.entity().downgrade();
    Rc::new(move |_, _, cx| {
        weak.update(cx, |page, cx| page.toggle_cell(kind, cx)).ok();
    })
}

fn on_open_agent(cx: &mut Context<WatchtowerPage>) -> OnId {
    let weak = cx.entity().downgrade();
    Rc::new(move |agent_id, _, cx| {
        weak.update(cx, |page, cx| page.ctx.open_agent(&agent_id, cx))
            .ok();
    })
}

fn on_select_task(cx: &mut Context<WatchtowerPage>) -> OnId {
    let weak = cx.entity().downgrade();
    Rc::new(move |task_id, _, cx| {
        weak.update(cx, |page, cx| page.select_task(task_id, cx))
            .ok();
    })
}

fn cell(
    page: &WatchtowerPage,
    kind: CellKind,
    overview: &Overview,
    derived: &Derived,
    cx: &mut Context<WatchtowerPage>,
) -> AnyElement {
    let expanded = page.expanded.contains(&kind);
    match kind {
        CellKind::Needs => needs_cell(&derived.rows, &page.needs, &page.target, cx),
        CellKind::Checks => {
            let on_toggle = toggle(kind, cx);
            manual_checks_cell(&overview.manual_checks, expanded, on_toggle, cx)
        }
        CellKind::Decisions => {
            let on_toggle = toggle(kind, cx);
            let weak = cx.entity().downgrade();
            let on_open = Rc::new(
                move |id: String, title: String, window: &mut Window, cx: &mut App| {
                    weak.update(cx, |page, cx| page.open_decision(id, title, window, cx))
                        .ok();
                },
            );
            decisions_cell(&derived.decisions, expanded, on_toggle, on_open, cx)
        }
        CellKind::Archived => {
            let on_toggle = toggle(kind, cx);
            archived_plans_cell(&overview.history, expanded, on_toggle, cx)
        }
    }
}

/// A grid column: its cells stacked with the 14 px gap, filling the row's height.
fn column(
    page: &WatchtowerPage,
    kinds: &[CellKind],
    overview: &Overview,
    derived: &Derived,
    cx: &mut Context<WatchtowerPage>,
) -> Div {
    let cells: Vec<AnyElement> = kinds
        .iter()
        .map(|kind| cell(page, *kind, overview, derived, cx))
        .collect();
    div()
        .flex_1()
        .min_w_0()
        .flex()
        .flex_col()
        .gap(px(GAP))
        .children(cells)
}

fn grid_row(columns: Vec<Div>) -> AnyElement {
    div()
        .w_full()
        .flex()
        .flex_row()
        .items_stretch()
        .gap(px(GAP))
        .children(columns)
        .into_any_element()
}

pub(crate) fn render_body(
    page: &WatchtowerPage,
    item: &BodyItem,
    cx: &mut Context<WatchtowerPage>,
) -> AnyElement {
    let overview = page.overview.ready();
    let derived = page.derived.get();
    match item {
        BodyItem::Error(message) => {
            let weak = cx.entity().downgrade();
            let on_retry: OnClick = Rc::new(move |_, _, cx| {
                weak.update(cx, |page, cx| page.refresh(cx)).ok();
            });
            return error_box(message, on_retry, cx);
        }
        BodyItem::Skeleton => return skeleton(cx),
        BodyItem::NoPlan(message) => return no_plan(message, cx),
        _ => {}
    }
    let (Some(overview), Some(derived)) = (overview, derived) else {
        return div().into_any_element();
    };
    let rows = &derived.rows;
    match item {
        BodyItem::Row3 => {
            let columns = [CellKind::Needs, CellKind::Decisions, CellKind::Archived]
                .map(|kind| column(page, &[kind], overview, derived, cx));
            grid_row(columns.into())
        }
        BodyItem::Stepper => lifecycle_stepper(
            &derived.phases,
            derived.counts,
            page.layout.compact_stepper,
            cx,
        ),
        BodyItem::Warning(index) => overview
            .warnings
            .get(*index)
            .map_or_else(|| div().into_any_element(), |text| warning_line(text, cx)),
        BodyItem::Strip => derived.strip.as_ref().map_or_else(
            || div().into_any_element(),
            |strip| autorun_strip(strip, cx),
        ),
        BodyItem::PlanTop => {
            let open = on_open_agent(cx);
            plan_top(
                &derived.run_state,
                &derived.pace,
                page.builders.lead.as_ref(),
                active_builders(&page.builders.by_task),
                &overview.plan.title,
                open,
                cx,
            )
        }
        BodyItem::Wave(index) => derived
            .tree
            .waves
            .get(*index)
            .map_or_else(|| div().into_any_element(), |wave| wave_header(wave, cx)),
        BodyItem::TaskRow { wave, row, last } => {
            let Some(row) = derived
                .tree
                .waves
                .get(*wave)
                .and_then(|wave| wave.rows.get(*row))
            else {
                return div().into_any_element();
            };
            let select = on_select_task(cx);
            let open = on_open_agent(cx);
            task_row(
                row,
                page.builders.by_task.get(&row.id),
                derived.classes.get(&row.id).map(String::as_str),
                derived.commits.get(&row.id).map(String::as_str),
                page.layout.narrow(),
                *last,
                select,
                open,
                cx,
            )
        }
        BodyItem::PlanBottom => cell_slice(Slice::Bottom, false, cx).into_any_element(),
        BodyItem::Row(left, right) => {
            let columns = vec![
                column(page, left, overview, derived, cx),
                column(page, right, overview, derived, cx),
            ];
            grid_row(columns)
        }
        BodyItem::Cell(kind) => cell(page, *kind, overview, derived, cx),
        BodyItem::NeedsTop => needs_top(rows, &page.target, Slice::Top, cx).into_any_element(),
        BodyItem::NeedsRow(index) => {
            let row = rows
                .get(*index)
                .map(|row| needs_row(*index, row, &page.needs, &page.target, cx));
            needs_slice(rows, row, cx)
        }
        BodyItem::NeedsMore => {
            let more = needs_more(rows, &page.needs, cx);
            needs_slice(rows, more, cx)
        }
        BodyItem::NeedsBottom => {
            cell_slice(Slice::Bottom, needs_warning(rows), cx).into_any_element()
        }
        BodyItem::Error(_) | BodyItem::Skeleton | BodyItem::NoPlan(_) => div().into_any_element(),
    }
}
