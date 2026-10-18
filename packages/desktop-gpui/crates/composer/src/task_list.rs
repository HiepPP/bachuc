//! `packages/app/src/composer/task-list/index.tsx`: the "N/M tasks" pill and its panel, one row
//! per task as `components/task-list-row.tsx` draws it.

use crate::tracks::{TrackSegment, track_pill, track_row};
use bachuc_client::{TaskEntry, TaskStatus};
use bachuc_icons::{IconName, icon};
use bachuc_theme::metrics::ICON_MD;
use bachuc_theme::{tokens, zoomed};
use gpui_kit::component::{Sizable as _, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

/// A task counts as done when it says `completed`, by flag or by status.
pub fn is_completed(task: &TaskEntry) -> bool {
    task.completed || task.status == Some(TaskStatus::Completed)
}

/// `message.todo.tasksProgress`.
pub fn task_label(tasks: &[TaskEntry]) -> String {
    let completed = tasks.iter().filter(|task| is_completed(task)).count();
    format!("{completed}/{} tasks", tasks.len())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskMark {
    Completed,
    Running,
    Pending,
}

/// The mark and text of a task row: a running task shows its active form.
pub fn task_row(task: &TaskEntry) -> (TaskMark, &str) {
    if is_completed(task) {
        return (TaskMark::Completed, &task.text);
    }
    if task.status == Some(TaskStatus::InProgress) {
        let text = task.active_form.as_deref().unwrap_or(&task.text);
        return (TaskMark::Running, text);
    }
    (TaskMark::Pending, &task.text)
}

/// The pill; `None` with no task.
pub fn task_pill(tasks: Vec<TaskEntry>) -> Option<AnyElement> {
    if tasks.is_empty() {
        return None;
    }
    let label = task_label(&tasks);
    let segments = vec![TrackSegment {
        bucket: None,
        text: label.clone(),
    }];
    Some(track_pill(
        "agent-task-list-header",
        segments,
        label,
        "Tasks",
        move |_, _, cx| {
            let t = tokens(cx);
            v_flex()
                .children(tasks.iter().enumerate().map(|(index, task)| {
                    let (mark, text) = task_row(task);
                    let (glyph, color, text_color) = match mark {
                        TaskMark::Completed => (
                            IconName::CircleCheck,
                            t.foreground_extra_muted,
                            t.foreground_extra_muted,
                        ),
                        TaskMark::Running => (IconName::CircleDot, t.dot_running, t.foreground),
                        TaskMark::Pending => (
                            IconName::Circle,
                            t.foreground_extra_muted,
                            t.foreground_muted,
                        ),
                    };
                    track_row(("composer-task", index), text.to_string(), false, cx)
                        .child(icon(glyph).with_size(zoomed(ICON_MD, cx)).text_color(color))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .truncate()
                                .text_color(text_color)
                                .when(mark == TaskMark::Completed, |text| text.line_through())
                                .child(text.to_string()),
                        )
                }))
                .into_any_element()
        },
    ))
}

#[cfg(test)]
mod tests {
    // Named imports: `use super::*` would bring in GPUI's `test` macro over `#[test]`.
    use super::{TaskMark, task_label, task_pill, task_row};
    use bachuc_client::{TaskEntry, TaskStatus};

    fn task(text: &str, completed: bool, status: Option<TaskStatus>) -> TaskEntry {
        TaskEntry {
            id: None,
            text: text.into(),
            completed,
            status,
            active_form: Some(format!("{text}ing")),
        }
    }

    #[test]
    fn the_label_counts_the_completed_flag_and_the_completed_status() {
        let mut tasks = vec![
            task("a", true, None),
            task("b", false, Some(TaskStatus::Completed)),
        ];
        tasks.extend((0..5).map(|_| task("c", false, Some(TaskStatus::Pending))));
        assert_eq!(task_label(&tasks), "2/7 tasks");
        assert_eq!(task_label(&[task("a", false, None)]), "0/1 tasks");
    }

    #[test]
    fn a_running_task_shows_its_active_form() {
        assert_eq!(
            task_row(&task("Build", false, Some(TaskStatus::InProgress))),
            (TaskMark::Running, "Building")
        );
        assert_eq!(
            task_row(&task("Build", true, Some(TaskStatus::InProgress))),
            (TaskMark::Completed, "Build")
        );
        assert_eq!(
            task_row(&task("Build", false, None)),
            (TaskMark::Pending, "Build")
        );
    }

    #[test]
    fn no_task_draws_no_pill() {
        assert!(task_pill(Vec::new()).is_none());
    }
}
