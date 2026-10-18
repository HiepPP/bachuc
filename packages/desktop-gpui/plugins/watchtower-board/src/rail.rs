//! The rail cards of `client/board-rail.tsx`: `ProgressCard`, `WaitingCard`, `TasksCard`, and
//! `AutorunLogCard`. The Watchtower page shows them in its column, and the New workspace panel
//! shows the first three. They take data from the view's `Rail` and never fetch.

use crate::cell::{
    LINE_HEIGHT, LIST_LIMIT, Slice, cell_slice, cell_title, empty, limit, overview_cell, show_more,
};
use crate::model::{
    Board, DashboardSummary, GroupCounts, Loaded, OverviewRunEntry, Question, Run, Task, TaskGroup,
    dashboard_summary, plural, task_group,
};
use crate::task_list::{TaskList, TaskListRow};
use bachuc_theme::metrics::{FONT_SM, SPACE_1, SPACE_1_5, SPACE_3, WEIGHT_BOLD, WEIGHT_SEMIBOLD};
use bachuc_theme::{mono_font_family, tokens, ui_font_size};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

/// `ProgressCard`: the percentage's `fontSize: 26`, the strip's `height: 6`, `gap: 2`, and segment
/// `borderRadius: 2`, the card's `gap: 10`, and the count line's `fontSize: 11`.
const PERCENT_SIZE: f32 = 26.;
const STRIP_HEIGHT: f32 = 6.;
const STRIP_GAP: f32 = 2.;
const SEGMENT_RADIUS: f32 = 2.;
const SEGMENT_MIN_WIDTH: f32 = 1.;
const PROGRESS_GAP: f32 = 10.;
const MONO_SIZE: f32 = 11.;
/// `WaitingCard`: the question's `fontSize: 13` and `lineHeight: 19`.
const QUESTION_SIZE: f32 = 13.;
const QUESTION_LINE: f32 = 19.;
/// `AutorunLogCard`: `lineHeight: 17`, row `paddingVertical: 4`, columns 40, 40, and 30 wide.
const LOG_LINE: f32 = 17.;
const LOG_ROW_PADDING: f32 = 4.;
const CLOCK_WIDTH: f32 = 40.;
const TASK_WIDTH: f32 = 30.;
/// `TasksCard`'s error `fontSize: 12.5`.
const ERROR_SIZE: f32 = 12.5;

/// A view that shows the rail. Its handlers reach the rail through this.
pub(crate) trait RailHost: Sized + 'static {
    fn rail_mut(&mut self) -> &mut Rail;
    /// The rail state changed; rebuild the list items.
    fn rail_changed(&mut self, cx: &mut Context<Self>);
    fn task_list(&mut self) -> Option<&mut TaskList> {
        self.rail_mut().task_list.as_mut()
    }
}

/// One list item of the rail.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum RailItem {
    Progress,
    Warning(usize),
    Waiting,
    TasksTop,
    /// Loading, error, or empty text inside the Tasks card.
    TasksNote,
    TaskRow(TaskListRow),
    TasksBottom,
    LogTop,
    LogRow(usize),
    LogMore,
    LogBottom,
}

/// What a view's rail draws: a snapshot taken when the store changes, not on every frame.
pub(crate) struct Rail {
    /// The board with live builder status applied.
    pub board: Loaded<Board>,
    /// The page's log rows, newest first; `None` hides the Autorun card (the panel).
    pub log: Option<Vec<OverviewRunEntry>>,
    pub log_expanded: bool,
    /// `None` on a project board, which has no attachment search key.
    pub workspace_id: Option<String>,
    pub task_list: Option<TaskList>,
}

impl Rail {
    pub(crate) fn new(workspace_id: Option<String>, log: bool) -> Self {
        Self {
            board: Loaded::Loading,
            log: log.then(Vec::new),
            log_expanded: false,
            workspace_id,
            task_list: None,
        }
    }

    /// Takes a new board. The task list starts once the board loads, so the opener's task and
    /// its group apply then, and starts again when the source changes.
    pub(crate) fn set_board(
        &mut self,
        board: Loaded<Board>,
        source_key: &str,
        initial: Option<&str>,
    ) {
        if let Loaded::Ready(ready) = &board
            && self
                .task_list
                .as_ref()
                .is_none_or(|list| list.source_key() != source_key)
        {
            self.task_list = Some(TaskList::new(
                source_key,
                &ready.tasks,
                &[TaskGroup::Done],
                initial,
                true,
            ));
        }
        self.board = board;
    }

    pub(crate) fn items(&self) -> Vec<RailItem> {
        let mut items = Vec::new();
        let board = self.board.ready();
        if let Some(board) = board {
            items.push(RailItem::Progress);
            items.extend((0..board.warnings.len()).map(RailItem::Warning));
            if WaitingCard::shows(&board.questions, board.proposed_adrs) {
                items.push(RailItem::Waiting);
            }
        }
        items.push(RailItem::TasksTop);
        match (board, &self.task_list) {
            (Some(board), Some(list)) if !board.tasks.is_empty() => {
                items.extend(list.rows(&board.tasks).into_iter().map(RailItem::TaskRow));
            }
            _ => items.push(RailItem::TasksNote),
        }
        items.push(RailItem::TasksBottom);
        if let (Some(log), Some(_)) = (&self.log, board.and_then(|board| board.run.as_ref())) {
            items.push(RailItem::LogTop);
            items.extend((0..limit(log, self.log_expanded).len()).map(RailItem::LogRow));
            if log.len() > LIST_LIMIT {
                items.push(RailItem::LogMore);
            }
            items.push(RailItem::LogBottom);
        }
        items
    }

    pub(crate) fn render_item<V: RailHost>(
        &self,
        item: &RailItem,
        cx: &mut Context<V>,
    ) -> AnyElement {
        let colors = tokens(cx);
        let board = self.board.ready();
        match item {
            RailItem::Progress => board.map_or_else(
                || div().into_any_element(),
                |board| ProgressCard::render(&board.tasks, cx),
            ),
            RailItem::Warning(index) => div()
                .text_size(ui_font_size(FONT_SM, cx))
                .text_color(colors.status_danger)
                .children(board.and_then(|board| board.warnings.get(*index).cloned()))
                .into_any_element(),
            RailItem::Waiting => board
                .and_then(|board| WaitingCard::render(&board.questions, board.proposed_adrs, cx))
                .unwrap_or_else(|| div().into_any_element()),
            RailItem::TasksTop => TasksCard::top(board, cx),
            RailItem::TasksNote => TasksCard::note(&self.board, cx),
            RailItem::TaskRow(row) => {
                let body = match (row, board, &self.task_list) {
                    (
                        TaskListRow::Group {
                            group,
                            count,
                            collapsed,
                        },
                        _,
                        Some(list),
                    ) => list.render_group(*group, *count, *collapsed, cx),
                    (TaskListRow::Task { id, .. }, Some(board), Some(list)) => {
                        match board.tasks.iter().find(|task| &task.id == id) {
                            Some(task) => list.render_task(
                                task,
                                &board.questions,
                                self.workspace_id.as_deref(),
                                cx,
                            ),
                            None => div().into_any_element(),
                        }
                    }
                    _ => div().into_any_element(),
                };
                cell_slice(Slice::Middle, false, cx)
                    .child(body)
                    .into_any_element()
            }
            RailItem::TasksBottom | RailItem::LogBottom => {
                // The padding under the last row closes the card.
                cell_slice(Slice::Bottom, false, cx).into_any_element()
            }
            RailItem::LogTop => {
                let run = board.and_then(|board| board.run.as_ref());
                let empty_log = self.log.as_ref().is_none_or(Vec::is_empty);
                run.map_or_else(
                    || div().into_any_element(),
                    |run| AutorunLogCard::top(run, empty_log, cx),
                )
            }
            RailItem::LogRow(index) => {
                let row = self.log.as_ref().and_then(|log| log.get(*index));
                cell_slice(Slice::Middle, false, cx)
                    .children(row.map(|row| AutorunLogCard::row(row, cx)))
                    .into_any_element()
            }
            RailItem::LogMore => {
                let total = self.log.as_ref().map_or(0, Vec::len);
                let toggle = cx.listener(|view: &mut V, _, _, cx| {
                    let rail = view.rail_mut();
                    rail.log_expanded = !rail.log_expanded;
                    view.rail_changed(cx);
                });
                cell_slice(Slice::Middle, false, cx)
                    .children(show_more(
                        "wt-log-more",
                        total,
                        self.log_expanded,
                        toggle,
                        cx,
                    ))
                    .into_any_element()
            }
        }
    }
}

/// `5 done · 1 running · 2 waiting · 3 not started`.
pub(crate) fn progress_count_line(counts: &GroupCounts) -> String {
    let mut parts = vec![format!("{} done", counts.done)];
    if counts.active > 0 {
        parts.push(format!("{} running", counts.active));
    }
    if counts.blocked > 0 {
        parts.push(format!("{} waiting", counts.blocked));
    }
    parts.push(format!("{} not started", counts.todo + counts.unknown));
    parts.join(" · ")
}

pub(crate) fn progress_label(summary: &DashboardSummary) -> String {
    format!(
        "{}% complete, {} of {} done",
        summary.percentage, summary.counts.done, summary.total
    )
}

pub(crate) struct ProgressCard;

impl ProgressCard {
    pub(crate) fn render(tasks: &[Task], cx: &App) -> AnyElement {
        let colors = tokens(cx);
        let summary = dashboard_summary(tasks);
        // Running work takes the accent, work that waits on the owner the warning color.
        let segment = |group: TaskGroup| match group {
            TaskGroup::Done => colors.status_success,
            TaskGroup::Active => colors.accent,
            TaskGroup::Blocked => colors.status_warning,
            TaskGroup::Todo | TaskGroup::Unknown => colors.surface2,
        };
        cell_slice(Slice::Whole, false, cx)
            .id("wt-progress")
            .role(Role::Group)
            .aria_label(SharedString::from(progress_label(&summary)))
            .gap(bachuc_theme::zoomed(PROGRESS_GAP, cx))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_baseline()
                    .gap(bachuc_theme::zoomed(SPACE_3, cx))
                    .child(
                        div()
                            .text_size(ui_font_size(PERCENT_SIZE, cx))
                            .font_weight(FontWeight(WEIGHT_BOLD))
                            .text_color(colors.foreground)
                            .child(format!("{}%", summary.percentage)),
                    )
                    .child(
                        div()
                            .ml_auto()
                            .text_size(ui_font_size(FONT_SM, cx))
                            .text_color(colors.foreground_muted)
                            .child(format!("{} of {} done", summary.counts.done, summary.total)),
                    ),
            )
            .child(
                div()
                    .h(bachuc_theme::zoomed(STRIP_HEIGHT, cx))
                    .flex()
                    .flex_row()
                    .gap(bachuc_theme::zoomed(STRIP_GAP, cx))
                    .children(tasks.iter().map(|task| {
                        div()
                            .flex_1()
                            .min_w(bachuc_theme::zoomed(SEGMENT_MIN_WIDTH, cx))
                            .h_full()
                            .rounded(bachuc_theme::zoomed(SEGMENT_RADIUS, cx))
                            .bg(segment(task_group(&task.status)))
                    })),
            )
            .child(
                div()
                    .font_family(mono_font_family(cx))
                    .text_size(ui_font_size(MONO_SIZE, cx))
                    .text_color(colors.foreground_muted)
                    .child(progress_count_line(&summary.counts)),
            )
            .into_any_element()
    }
}

pub(crate) fn waiting_line(questions: usize, proposed_adrs: i64) -> String {
    format!(
        "{} · {}",
        plural(questions, "blocking question"),
        plural(proposed_adrs.max(0) as usize, "proposed ADR")
    )
}

pub(crate) struct WaitingCard;

impl WaitingCard {
    pub(crate) fn shows(questions: &[Question], proposed_adrs: i64) -> bool {
        !questions.is_empty() || proposed_adrs != 0
    }

    pub(crate) fn render(
        questions: &[Question],
        proposed_adrs: i64,
        cx: &App,
    ) -> Option<AnyElement> {
        if !Self::shows(questions, proposed_adrs) {
            return None;
        }
        let colors = tokens(cx);
        let body = div()
            .flex()
            .flex_col()
            .child(
                div()
                    .mb(bachuc_theme::zoomed(SPACE_1_5, cx))
                    .text_size(ui_font_size(FONT_SM, cx))
                    .text_color(colors.foreground_muted)
                    .child(waiting_line(questions.len(), proposed_adrs)),
            )
            .children(questions.iter().map(|question| {
                div()
                    .mt(bachuc_theme::zoomed(SPACE_1, cx))
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .text_size(ui_font_size(FONT_SM, cx))
                            .font_weight(FontWeight(WEIGHT_SEMIBOLD))
                            .text_color(colors.foreground)
                            .child(format!(
                                "{} · blocks {}",
                                question.id,
                                question.blocks.join(", ")
                            )),
                    )
                    .child(
                        div()
                            .text_size(ui_font_size(QUESTION_SIZE, cx))
                            .line_height(bachuc_theme::zoomed(QUESTION_LINE, cx))
                            .text_color(colors.foreground)
                            .child(question.question.clone()),
                    )
            }));
        Some(overview_cell("Waiting on the owner", None, false, body, cx))
    }
}

/// The Tasks card's text when it shows no rows.
pub(crate) fn tasks_note(board: &Loaded<Board>) -> (String, bool) {
    match board {
        Loaded::Error(error) => (format!("Could not load tasks. {error}"), true),
        Loaded::Loading => ("Loading tasks…".into(), false),
        Loaded::Ready(board) => (
            board.message.clone().unwrap_or_else(|| "No tasks.".into()),
            false,
        ),
    }
}

pub(crate) struct TasksCard;

impl TasksCard {
    pub(crate) fn top(board: Option<&Board>, cx: &App) -> AnyElement {
        let aside = board.map(|board| SharedString::from(board.tasks.len().to_string()));
        cell_slice(Slice::Top, false, cx)
            .child(cell_title("Tasks", aside, cx))
            .into_any_element()
    }

    pub(crate) fn note(board: &Loaded<Board>, cx: &App) -> AnyElement {
        let colors = tokens(cx);
        let (text, error) = tasks_note(board);
        let line = if error {
            div()
                .id("wt-tasks-error")
                .role(Role::Alert)
                .aria_label(SharedString::from(text.clone()))
                .text_size(ui_font_size(ERROR_SIZE, cx))
                .text_color(colors.status_danger)
                .child(text)
                .into_any_element()
        } else {
            empty(text, cx).into_any_element()
        };
        cell_slice(Slice::Middle, false, cx)
            .child(line)
            .into_any_element()
    }
}

/// A dated time keeps only its clock part, so it fits its column.
pub(crate) fn log_clock(value: &str) -> String {
    let bytes = value.as_bytes();
    let dated = bytes.len() > 10
        && bytes[..10].iter().enumerate().all(|(index, b)| {
            if index == 4 || index == 7 {
                *b == b'-'
            } else {
                b.is_ascii_digit()
            }
        })
        && matches!(bytes[10], b' ' | b'T');
    if dated {
        value[11..].to_string()
    } else {
        value.to_string()
    }
}

/// `Runner loop · Profile x · Schedule y`, or `Runner not recorded`.
pub(crate) fn runner_line(run: &Run) -> String {
    let parts: Vec<String> = [
        ("Runner", &run.runner),
        ("Profile", &run.profile),
        ("Schedule", &run.schedule),
    ]
    .into_iter()
    .filter_map(|(name, value)| {
        // An empty value is falsy in JS, so it is left out too.
        value
            .as_deref()
            .filter(|value| !value.is_empty())
            .map(|value| format!("{name} {value}"))
    })
    .collect();
    if parts.is_empty() {
        "Runner not recorded".into()
    } else {
        parts.join(" · ")
    }
}

pub(crate) fn started_line(run: &Run) -> String {
    format!(
        "Started {} · {}",
        run.started.as_deref().unwrap_or("-"),
        run.finished.as_ref().map_or_else(
            || "Not finished".to_string(),
            |finished| format!("Finished {finished}")
        )
    )
}

pub(crate) struct AutorunLogCard;

impl AutorunLogCard {
    pub(crate) fn top(run: &Run, empty_log: bool, cx: &App) -> AnyElement {
        let colors = tokens(cx);
        let muted = |text: String, cx: &App| {
            div()
                .text_size(ui_font_size(FONT_SM, cx))
                .line_height(bachuc_theme::zoomed(LOG_LINE, cx))
                .text_color(colors.foreground_muted)
                .child(text)
        };
        cell_slice(Slice::Top, false, cx)
            .child(cell_title(
                "Autorun",
                Some(plural(run.total.max(0) as usize, "iteration").into()),
                cx,
            ))
            .child(muted(runner_line(run), cx))
            .child(muted(started_line(run), cx).mb(bachuc_theme::zoomed(SPACE_1_5, cx)))
            .when(empty_log, |this| {
                this.child(empty("No iterations logged yet.", cx))
            })
            .into_any_element()
    }

    pub(crate) fn row(row: &OverviewRunEntry, cx: &App) -> AnyElement {
        let colors = tokens(cx);
        let mono = |cx: &App| {
            div()
                .font_family(mono_font_family(cx))
                .text_size(ui_font_size(MONO_SIZE, cx))
                .line_height(bachuc_theme::zoomed(LOG_LINE, cx))
                .text_color(colors.foreground_muted)
        };
        div()
            .py(bachuc_theme::zoomed(LOG_ROW_PADDING, cx))
            .border_t_1()
            .border_color(colors.border)
            .flex()
            .flex_col()
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap(bachuc_theme::zoomed(SPACE_1_5, cx))
                    .child(
                        mono(cx)
                            .w(bachuc_theme::zoomed(CLOCK_WIDTH, cx))
                            .child(log_clock(&row.start)),
                    )
                    .child(
                        mono(cx)
                            .w(bachuc_theme::zoomed(CLOCK_WIDTH, cx))
                            .child(log_clock(&row.end)),
                    )
                    .child(
                        mono(cx).w(bachuc_theme::zoomed(TASK_WIDTH, cx)).child(
                            row.task
                                .strip_prefix("TASK-")
                                .unwrap_or(&row.task)
                                .to_string(),
                        ),
                    )
                    .child(
                        mono(cx)
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .text_color(colors.foreground)
                            .child(row.result.to_lowercase()),
                    ),
            )
            .when(!row.detail.is_empty(), |this| {
                this.child(mono(cx).line_clamp(2).child(row.detail.clone()))
            })
            .into_any_element()
    }
}

/// The space between two cards, and around the column, as the Electron `gap: 12`.
pub(crate) const CARD_GAP: f32 = SPACE_3;

/// Whether an item starts a card, so it gets the gap above it.
pub(crate) fn starts_card(item: &RailItem) -> bool {
    matches!(
        item,
        RailItem::Progress
            | RailItem::Warning(_)
            | RailItem::Waiting
            | RailItem::TasksTop
            | RailItem::LogTop
    )
}

/// The base line height of every rail item.
pub(crate) fn item_frame() -> Div {
    div().w_full().line_height(relative(LINE_HEIGHT))
}

#[cfg(test)]
mod tests {
    // Not `super::*`: the gpui glob brings in a `test` macro that shadows `#[test]`.
    use super::{
        Rail, RailItem, log_clock, progress_count_line, progress_label, runner_line, started_line,
        tasks_note, waiting_line,
    };
    use crate::model::{Board, DashboardSummary, GroupCounts, Loaded, OverviewRunEntry, Run, Task};
    use crate::task_list::TaskListRow;

    fn task(id: &str, status: &str) -> Task {
        Task {
            id: id.into(),
            title: id.into(),
            status: status.into(),
            ..Task::default()
        }
    }

    #[test]
    fn the_progress_card_counts_like_electron() {
        let counts = GroupCounts {
            active: 1,
            blocked: 2,
            todo: 3,
            done: 5,
            unknown: 1,
        };
        assert_eq!(
            progress_count_line(&counts),
            "5 done · 1 running · 2 waiting · 4 not started"
        );
        assert_eq!(
            progress_count_line(&GroupCounts::default()),
            "0 done · 0 not started"
        );
        let summary = DashboardSummary {
            counts,
            percentage: 42,
            total: 12,
        };
        assert_eq!(progress_label(&summary), "42% complete, 5 of 12 done");
        assert_eq!(waiting_line(1, 0), "1 blocking question · 0 proposed ADRs");
    }

    #[test]
    fn the_autorun_lines_and_log_clock_match_electron() {
        let run = Run {
            runner: Some("loop".into()),
            started: Some("2026-10-13 14:07".into()),
            ..Run::default()
        };
        assert_eq!(runner_line(&run), "Runner loop");
        assert_eq!(runner_line(&Run::default()), "Runner not recorded");
        assert_eq!(
            started_line(&run),
            "Started 2026-10-13 14:07 · Not finished"
        );
        let finished = Run {
            finished: Some("2026-10-13 14:41".into()),
            ..Run::default()
        };
        assert_eq!(
            started_line(&finished),
            "Started - · Finished 2026-10-13 14:41"
        );
        assert_eq!(log_clock("2026-10-13 14:39"), "14:39");
        assert_eq!(log_clock("2026-10-13T14:39"), "14:39");
        assert_eq!(log_clock("14:39"), "14:39");
        assert_eq!(log_clock("now"), "now");
    }

    #[test]
    fn the_tasks_note_covers_loading_error_and_empty() {
        assert_eq!(
            tasks_note(&Loaded::Loading),
            ("Loading tasks…".into(), false)
        );
        assert_eq!(
            tasks_note(&Loaded::Error("offline".into())),
            ("Could not load tasks. offline".into(), true)
        );
        let empty = Board {
            message: Some("No watchtower/NEXT.md in this workspace.".into()),
            ..Board::default()
        };
        assert_eq!(
            tasks_note(&Loaded::Ready(empty)).0,
            "No watchtower/NEXT.md in this workspace."
        );
        assert_eq!(tasks_note(&Loaded::Ready(Board::default())).0, "No tasks.");
    }

    #[test]
    fn the_panel_rail_has_progress_waiting_and_tasks_with_done_collapsed() {
        let mut rail = Rail::new(None, false);
        assert_eq!(
            rail.items(),
            [
                RailItem::TasksTop,
                RailItem::TasksNote,
                RailItem::TasksBottom
            ]
        );
        let board = Board {
            tasks: vec![task("TASK-001", "DONE"), task("TASK-002", "TODO")],
            warnings: vec!["QUESTIONS.md is unreadable".into()],
            proposed_adrs: 1,
            ..Board::default()
        };
        rail.set_board(Loaded::Ready(board), "project:p1", None);
        let items = rail.items();
        assert_eq!(
            items[..3],
            [RailItem::Progress, RailItem::Warning(0), RailItem::Waiting]
        );
        assert!(items.contains(&RailItem::TaskRow(TaskListRow::Group {
            group: crate::model::TaskGroup::Done,
            count: 1,
            collapsed: true,
        })));
        assert!(
            !items.contains(&RailItem::LogTop),
            "the panel has no Autorun card"
        );
    }

    #[test]
    fn the_page_rail_lists_five_log_rows_until_show_more() {
        let mut rail = Rail::new(Some("w1".into()), true);
        let board = Board {
            tasks: vec![task("TASK-001", "DONE")],
            run: Some(Run::default()),
            ..Board::default()
        };
        rail.set_board(Loaded::Ready(board), "w1", None);
        rail.log = Some(vec![OverviewRunEntry::default(); 7]);
        let rows = |rail: &Rail| {
            rail.items()
                .iter()
                .filter(|item| matches!(item, RailItem::LogRow(_)))
                .count()
        };
        assert_eq!(rows(&rail), 5);
        assert!(rail.items().contains(&RailItem::LogMore));
        rail.log_expanded = true;
        assert_eq!(rows(&rail), 7);
    }
}
