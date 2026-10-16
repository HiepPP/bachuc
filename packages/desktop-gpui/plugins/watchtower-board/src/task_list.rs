//! `TaskList` of `client/task-list.tsx`: tasks grouped by status. Selecting a task shows its
//! brief, deps, class, blockers, and attachment search. Each group header and each task row is
//! its own list item.

use crate::model::{
    Question, Task, TaskGroup, attachment_key, dashboard_summary, group_tasks, initial_selection,
    questions_for, task_group, task_title,
};
use crate::rail::RailHost;
use bachuc_icons::{IconName, icon};
use bachuc_theme::metrics::{FONT_BASE, FONT_SM, ICON_XS, RADIUS_MD, SPACE_2, WEIGHT_SEMIBOLD};
use bachuc_theme::{Tokens, tokens, ui_font_size};
use gpui_kit::component::Sizable as _;
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use std::collections::BTreeSet;

/// The group header's `minHeight: 38`, `paddingVertical: 9`, and `gap: 7`.
const GROUP_MIN_HEIGHT: f32 = 38.;
const GROUP_PADDING: f32 = 9.;
const GROUP_GAP: f32 = 7.;
/// The group title's `fontSize: 13`.
const TEXT_13: f32 = 13.;
/// The task row's `paddingVertical: 9`, `paddingHorizontal` 8 dense or 10, `borderRadius: 6`.
const ROW_PADDING_Y: f32 = 9.;
const ROW_PADDING_X_DENSE: f32 = 8.;
const ROW_PADDING_X: f32 = 10.;
/// The class's `fontSize: 11` and `lineHeight: 18`; the title's `lineHeight: 18`.
const TEXT_11: f32 = 11.;
const LINE_18: f32 = 18.;
/// The badge's `paddingHorizontal: 6`, `paddingVertical: 2`, `borderRadius: 5`, `fontSize: 10`.
const BADGE_PADDING_X: f32 = 6.;
const BADGE_PADDING_Y: f32 = 2.;
const BADGE_RADIUS: f32 = 5.;
const TEXT_10: f32 = 10.;
/// The details' `marginTop: 2` and `paddingTop: 10`.
const DETAILS_MARGIN: f32 = 2.;
const DETAILS_PADDING: f32 = 10.;
/// Body text `lineHeight: 20`, blockers `lineHeight: 19`, muted `lineHeight: 17`.
const LINE_20: f32 = 20.;
const LINE_19: f32 = 19.;
const LINE_17: f32 = 17.;
/// The copy button's `paddingHorizontal: 10` and `paddingVertical: 7`.
const COPY_PADDING_X: f32 = 10.;
const COPY_PADDING_Y: f32 = 7.;

pub(crate) const COPIED: &str =
    "Search key copied. Open composer + → Watchtower task, paste the key, then select the task.";

/// One list item of the task list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum TaskListRow {
    Group {
        group: TaskGroup,
        count: usize,
        collapsed: bool,
    },
    Task {
        id: String,
        group: TaskGroup,
        selected: bool,
        /// The selected row's notice; empty on other rows.
        notice: String,
    },
}

/// The selection, the collapsed groups, and the notice of one board source.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TaskList {
    /// Selection resets when the board source changes.
    source_key: String,
    selected: Option<String>,
    collapsed: BTreeSet<TaskGroup>,
    notice: String,
    dense: bool,
}

/// `badgeColor`.
pub(crate) fn badge_color(status: &str, colors: &Tokens) -> Hsla {
    match status {
        "DONE" => colors.status_success,
        "BLOCKED" => colors.status_danger,
        "IN PROGRESS" | "TODO" => colors.status_warning,
        _ => colors.status_danger,
    }
}

/// The task row's label for assistive tech.
pub(crate) fn task_row_label(task: &Task) -> String {
    let group = task_group(&task.status).label();
    let class = task
        .task_class
        .as_ref()
        .map(|class| format!(", class {class}"))
        .unwrap_or_default();
    format!(
        "{}, {}, {group}{class}",
        task.id,
        task_title(&task.id, &task.title)
    )
}

impl TaskList {
    /// The opener's task, when the board has it, starts selected, and its group starts open even
    /// when it is one of `collapsed_groups`.
    pub(crate) fn new(
        source_key: impl Into<String>,
        tasks: &[Task],
        collapsed_groups: &[TaskGroup],
        initial: Option<&str>,
        dense: bool,
    ) -> Self {
        let selected = initial_selection(tasks, initial);
        let open = selected
            .as_ref()
            .and_then(|id| tasks.iter().find(|task| &task.id == id))
            .map(|task| task_group(&task.status));
        Self {
            source_key: source_key.into(),
            selected,
            collapsed: collapsed_groups
                .iter()
                .copied()
                .filter(|group| Some(*group) != open)
                .collect(),
            notice: String::new(),
            dense,
        }
    }

    pub(crate) fn source_key(&self) -> &str {
        &self.source_key
    }

    pub(crate) fn selected(&self) -> Option<&str> {
        self.selected.as_deref()
    }

    /// A pick in the list. Picking the selected task again clears the selection.
    pub(crate) fn select(&mut self, task_id: Option<String>) {
        self.selected = task_id;
        self.notice.clear();
    }

    /// A pick from outside the list, such as a plan tree row: opens its group when it is
    /// collapsed, so the row shows.
    #[allow(dead_code, reason = "the plan tree of TASK-194 picks rows")]
    pub(crate) fn select_from_outside(&mut self, task_id: Option<String>, tasks: &[Task]) {
        if let Some(task) = task_id
            .as_ref()
            .and_then(|id| tasks.iter().find(|task| &task.id == id))
        {
            self.collapsed.remove(&task_group(&task.status));
        }
        self.select(task_id);
    }

    pub(crate) fn toggle_group(&mut self, group: TaskGroup) {
        if !self.collapsed.remove(&group) {
            self.collapsed.insert(group);
        }
    }

    pub(crate) fn set_notice(&mut self, notice: impl Into<String>) {
        self.notice = notice.into();
    }

    /// The group headers and rows to show. Unknown shows only when a task has an unknown status.
    pub(crate) fn rows(&self, tasks: &[Task]) -> Vec<TaskListRow> {
        let unknown = dashboard_summary(tasks).counts.unknown;
        let mut rows = Vec::new();
        for group in TaskGroup::ORDER {
            if group == TaskGroup::Unknown && unknown == 0 {
                continue;
            }
            let members = group_tasks(tasks, group);
            let collapsed = self.collapsed.contains(&group);
            rows.push(TaskListRow::Group {
                group,
                count: members.len(),
                collapsed,
            });
            if collapsed {
                continue;
            }
            for task in members {
                let selected = self.selected.as_deref() == Some(task.id.as_str());
                rows.push(TaskListRow::Task {
                    id: task.id.clone(),
                    group,
                    selected,
                    notice: if selected {
                        self.notice.clone()
                    } else {
                        String::new()
                    },
                });
            }
        }
        rows
    }

    pub(crate) fn render_group<V: RailHost>(
        &self,
        group: TaskGroup,
        count: usize,
        collapsed: bool,
        cx: &mut Context<V>,
    ) -> AnyElement {
        let colors = tokens(cx);
        div()
            .id(SharedString::from(format!("wt-group-{}", group.label())))
            .role(Role::Button)
            .aria_label(SharedString::from(format!(
                "{} tasks, {count}",
                group.label()
            )))
            .aria_expanded(!collapsed)
            .w_full()
            .min_h(px(GROUP_MIN_HEIGHT))
            .py(px(GROUP_PADDING))
            .flex()
            .flex_row()
            .items_center()
            .gap(px(GROUP_GAP))
            .border_t_1()
            .border_color(colors.border)
            .cursor_pointer()
            .on_click(cx.listener(move |view: &mut V, _, _, cx| {
                if let Some(list) = view.task_list() {
                    list.toggle_group(group);
                }
                view.rail_changed(cx);
            }))
            .child(
                div().w(px(ICON_XS)).child(
                    icon(if collapsed {
                        IconName::ChevronRight
                    } else {
                        IconName::ChevronDown
                    })
                    .with_size(px(ICON_XS))
                    .text_color(colors.foreground_muted),
                ),
            )
            .child(
                div()
                    .flex_1()
                    .text_size(ui_font_size(TEXT_13, cx))
                    .font_weight(FontWeight(WEIGHT_SEMIBOLD))
                    .text_color(colors.foreground)
                    .child(group.label()),
            )
            .child(
                div()
                    .text_size(ui_font_size(FONT_SM, cx))
                    .text_color(colors.foreground_muted)
                    .child(count.to_string()),
            )
            .into_any_element()
    }

    /// A task row; the selected row also shows its details. `workspace_id` is `None` on a project
    /// board, which has no attachment search key.
    pub(crate) fn render_task<V: RailHost>(
        &self,
        task: &Task,
        questions: &[Question],
        workspace_id: Option<&str>,
        cx: &mut Context<V>,
    ) -> AnyElement {
        let colors = tokens(cx);
        let selected = self.selected.as_deref() == Some(task.id.as_str());
        let group = task_group(&task.status);
        let badge = badge_color(&task.status, &colors);
        let id = task.id.clone();
        let row = div()
            .id(SharedString::from(format!("wt-task-{}", task.id)))
            .role(Role::Button)
            .aria_label(SharedString::from(task_row_label(task)))
            .aria_description("Shows task details and attachment search")
            .aria_selected(selected)
            .aria_expanded(selected)
            .w_full()
            .flex()
            .flex_row()
            .items_start()
            .gap(px(SPACE_2))
            .cursor_pointer()
            .on_click(cx.listener(move |view: &mut V, _, _, cx| {
                if let Some(list) = view.task_list() {
                    let next = (list.selected() != Some(id.as_str())).then(|| id.clone());
                    list.select(next);
                }
                view.rail_changed(cx);
            }))
            .child(
                div()
                    .flex_shrink_0()
                    .text_size(ui_font_size(FONT_SM, cx))
                    .font_weight(FontWeight(WEIGHT_SEMIBOLD))
                    .text_color(colors.foreground)
                    .child(task.id.clone()),
            )
            .children(task.task_class.clone().map(|class| {
                div()
                    .flex_shrink_0()
                    .truncate()
                    .text_size(ui_font_size(TEXT_11, cx))
                    .line_height(px(LINE_18))
                    .text_color(colors.foreground_muted)
                    .child(class)
            }))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .text_size(ui_font_size(TEXT_13, cx))
                    .line_height(px(LINE_18))
                    .text_color(colors.foreground)
                    .when(!selected, |this| this.truncate())
                    .child(task_title(&task.id, &task.title)),
            )
            .child(
                div()
                    .flex_shrink_0()
                    .px(px(BADGE_PADDING_X))
                    .py(px(BADGE_PADDING_Y))
                    .border_1()
                    .border_color(badge)
                    .rounded(px(BADGE_RADIUS))
                    .text_size(ui_font_size(TEXT_10, cx))
                    .font_weight(FontWeight(WEIGHT_SEMIBOLD))
                    .text_color(badge)
                    .child(group.label().to_uppercase()),
            );
        div()
            .w_full()
            .flex()
            .flex_col()
            .gap(px(SPACE_2))
            .py(px(ROW_PADDING_Y))
            .px(px(if self.dense {
                ROW_PADDING_X_DENSE
            } else {
                ROW_PADDING_X
            }))
            .rounded(px(RADIUS_MD))
            .when(selected, |this| this.bg(colors.surface1))
            .child(row)
            .when(selected, |this| {
                this.child(self.render_details(task, questions, workspace_id, cx))
            })
            .into_any_element()
    }

    fn render_details<V: RailHost>(
        &self,
        task: &Task,
        questions: &[Question],
        workspace_id: Option<&str>,
        cx: &mut Context<V>,
    ) -> AnyElement {
        let colors = tokens(cx);
        let label = |text: &'static str, cx: &App| {
            div()
                .text_size(ui_font_size(TEXT_10, cx))
                .font_weight(FontWeight(WEIGHT_SEMIBOLD))
                .text_color(colors.foreground_muted)
                .child(text.to_uppercase())
        };
        let text = |value: String, cx: &App| {
            div()
                .text_size(ui_font_size(FONT_BASE, cx))
                .line_height(px(LINE_20))
                .text_color(colors.foreground)
                .child(value)
        };
        let danger = |value: String, cx: &App| {
            div()
                .text_size(ui_font_size(TEXT_13, cx))
                .line_height(px(LINE_19))
                .text_color(colors.status_danger)
                .child(value)
        };
        let section = |name: &'static str, body: AnyElement, cx: &App| {
            div().flex().flex_col().child(label(name, cx)).child(body)
        };
        let blocking = questions_for(&task.id, questions);
        let key = workspace_id
            .filter(|_| task.brief.is_some() && task.error.is_none())
            .map(|workspace_id| attachment_key(workspace_id, &task.id));
        div()
            .mt(px(DETAILS_MARGIN))
            .pt(px(DETAILS_PADDING))
            .border_t_1()
            .border_color(colors.border)
            .flex()
            .flex_col()
            .gap(px(SPACE_2))
            .child(section(
                "Brief",
                text(
                    task.brief
                        .clone()
                        .unwrap_or_else(|| "Brief unavailable.".into()),
                    cx,
                )
                .into_any_element(),
                cx,
            ))
            .child(section(
                "Dependencies",
                text(
                    if task.deps.is_empty() {
                        "-".into()
                    } else {
                        task.deps.clone()
                    },
                    cx,
                )
                .into_any_element(),
                cx,
            ))
            .when(task.brief.is_some(), |this| {
                this.child(section(
                    "Class",
                    text(
                        task.task_class
                            .clone()
                            .unwrap_or_else(|| "Missing; Watchtower reads it as risky.".into()),
                        cx,
                    )
                    .into_any_element(),
                    cx,
                ))
            })
            .when(!blocking.is_empty(), |this| {
                this.child(section(
                    "Open questions",
                    div()
                        .flex()
                        .flex_col()
                        .children(blocking.iter().map(|question| {
                            danger(format!("{}: {}", question.id, question.question), cx)
                        }))
                        .into_any_element(),
                    cx,
                ))
            })
            .children(
                task.blocker
                    .clone()
                    .map(|blocker| section("Blocker", danger(blocker, cx).into_any_element(), cx)),
            )
            .children(task.error.clone().map(|error| {
                section(
                    "Error",
                    danger(error.clone(), cx)
                        .id("wt-task-error")
                        .role(Role::Alert)
                        .aria_label(SharedString::from(error))
                        .into_any_element(),
                    cx,
                )
            }))
            .children(key.map(|key| {
                let copied = key.clone();
                let task_id = task.id.clone();
                div()
                    .flex()
                    .flex_col()
                    .gap(px(SPACE_2))
                    .child(
                        div()
                            .text_size(ui_font_size(TEXT_11, cx))
                            .text_color(colors.foreground_muted)
                            .child(key),
                    )
                    .child(
                        div()
                            .id(SharedString::from(format!("wt-copy-{task_id}")))
                            .role(Role::Button)
                            .aria_label(SharedString::from(format!(
                                "Copy attachment search for {task_id}"
                            )))
                            .self_start()
                            .px(px(COPY_PADDING_X))
                            .py(px(COPY_PADDING_Y))
                            .rounded(px(RADIUS_MD))
                            .bg(colors.accent)
                            .cursor_pointer()
                            .text_size(ui_font_size(FONT_SM, cx))
                            .font_weight(FontWeight(WEIGHT_SEMIBOLD))
                            .text_color(colors.accent_foreground)
                            .on_click(cx.listener(move |view: &mut V, _, _, cx| {
                                // GPUI's clipboard write cannot fail, so the Electron failure
                                // notice never shows here.
                                cx.write_to_clipboard(ClipboardItem::new_string(copied.clone()));
                                if let Some(list) = view.task_list() {
                                    list.set_notice(COPIED);
                                }
                                view.rail_changed(cx);
                            }))
                            .child("Copy attachment search"),
                    )
            }))
            .when(!self.notice.is_empty(), |this| {
                this.child(
                    div()
                        .id("wt-task-notice")
                        .role(Role::Status)
                        .aria_label(SharedString::from(self.notice.clone()))
                        .text_size(ui_font_size(FONT_SM, cx))
                        .line_height(px(LINE_17))
                        .text_color(colors.foreground_muted)
                        .child(self.notice.clone()),
                )
            })
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    // Not `super::*`: the gpui glob brings in a `test` macro that shadows `#[test]`.
    use super::{TaskList, TaskListRow, task_row_label};
    use crate::model::{Task, TaskGroup};

    fn task(id: &str, status: &str) -> Task {
        Task {
            id: id.into(),
            title: format!("{id} Example task"),
            status: status.into(),
            ..Task::default()
        }
    }

    fn tasks() -> Vec<Task> {
        vec![
            task("TASK-001", "DONE"),
            task("TASK-002", "IN PROGRESS"),
            task("TASK-003", "TODO"),
            task("TASK-004", "DONE"),
        ]
    }

    fn shape(rows: &[TaskListRow]) -> Vec<String> {
        rows.iter()
            .map(|row| match row {
                TaskListRow::Group {
                    group, collapsed, ..
                } => format!("{}{}", group.label(), if *collapsed { "+" } else { "" }),
                TaskListRow::Task { id, selected, .. } => {
                    format!("{id}{}", if *selected { "*" } else { "" })
                }
            })
            .collect()
    }

    #[test]
    fn the_done_group_starts_collapsed_and_unknown_hides_when_empty() {
        let list = TaskList::new("w1", &tasks(), &[TaskGroup::Done], None, true);
        assert_eq!(
            shape(&list.rows(&tasks())),
            ["Active", "TASK-002", "Blocked", "Todo", "TASK-003", "Done+"]
        );
    }

    #[test]
    fn the_openers_task_starts_selected_with_its_group_open() {
        let list = TaskList::new("w1", &tasks(), &[TaskGroup::Done], Some("TASK-004"), true);
        assert_eq!(list.selected(), Some("TASK-004"));
        assert_eq!(
            shape(&list.rows(&tasks()))[5..],
            ["Done", "TASK-001", "TASK-004*"]
        );
        let unknown = TaskList::new("w1", &tasks(), &[TaskGroup::Done], Some("TASK-099"), true);
        assert_eq!(unknown.selected(), None);
    }

    #[test]
    fn a_second_pick_clears_and_a_pick_from_outside_opens_its_group() {
        let mut list = TaskList::new("w1", &tasks(), &[TaskGroup::Done], None, true);
        list.select(Some("TASK-003".into()));
        list.set_notice("copied");
        assert!(list.rows(&tasks()).contains(&TaskListRow::Task {
            id: "TASK-003".into(),
            group: TaskGroup::Todo,
            selected: true,
            notice: "copied".into(),
        }));
        list.select(None);
        assert_eq!(list.selected(), None);
        list.select_from_outside(Some("TASK-001".into()), &tasks());
        assert!(shape(&list.rows(&tasks())).contains(&"TASK-001*".to_string()));
        list.toggle_group(TaskGroup::Done);
        assert!(shape(&list.rows(&tasks())).contains(&"Done+".to_string()));
    }

    #[test]
    fn an_unknown_status_shows_the_unknown_group() {
        let plan = vec![task("TASK-001", "MAYBE")];
        let list = TaskList::new("w1", &plan, &[], None, true);
        assert_eq!(
            shape(&list.rows(&plan)),
            ["Active", "Blocked", "Todo", "Done", "Unknown", "TASK-001"]
        );
    }

    #[test]
    fn the_row_label_names_the_id_title_group_and_class() {
        let mut item = task("TASK-007", "BLOCKED");
        assert_eq!(task_row_label(&item), "TASK-007, Example task, Blocked");
        item.task_class = Some("risky".into());
        assert_eq!(
            task_row_label(&item),
            "TASK-007, Example task, Blocked, class risky"
        );
    }
}
