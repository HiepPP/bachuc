//! Status groups, counts, titles, and the pill label: `client/dashboard.ts` and
//! `client/pill-label.ts`.

use super::dto::{Board, OverviewTask, Question, Task};
use super::time::round_half_up;
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum TaskGroup {
    Active,
    Blocked,
    Todo,
    Done,
    Unknown,
}

impl TaskGroup {
    /// `groupOrder`.
    pub(crate) const ORDER: [TaskGroup; 5] = [
        TaskGroup::Active,
        TaskGroup::Blocked,
        TaskGroup::Todo,
        TaskGroup::Done,
        TaskGroup::Unknown,
    ];

    /// `groupLabels` of `task-list.tsx`.
    pub(crate) fn label(self) -> &'static str {
        match self {
            TaskGroup::Active => "Active",
            TaskGroup::Blocked => "Blocked",
            TaskGroup::Todo => "Todo",
            TaskGroup::Done => "Done",
            TaskGroup::Unknown => "Unknown",
        }
    }
}

pub(crate) fn task_group(status: &str) -> TaskGroup {
    match status {
        "IN PROGRESS" => TaskGroup::Active,
        "BLOCKED" => TaskGroup::Blocked,
        "DONE" => TaskGroup::Done,
        "TODO" => TaskGroup::Todo,
        _ => TaskGroup::Unknown,
    }
}

/// The fields the rules read from a board task and an overview task.
pub(crate) trait TaskLike: Clone {
    fn id(&self) -> &str;
    fn title(&self) -> &str;
    fn status(&self) -> &str;
    fn set_status(&mut self, status: &str);
}

macro_rules! task_like {
    ($type:ty) => {
        impl TaskLike for $type {
            fn id(&self) -> &str {
                &self.id
            }
            fn title(&self) -> &str {
                &self.title
            }
            fn status(&self) -> &str {
                &self.status
            }
            fn set_status(&mut self, status: &str) {
                self.status = status.to_string();
            }
        }
    };
}

task_like!(Task);
task_like!(OverviewTask);

/// A TASK that a builder agent works on now counts as active, whatever its file says: a rerun of
/// a Blocked TASK runs before the lead rewrites its status. Done stays done.
pub(crate) fn with_live_status<T: TaskLike>(tasks: &[T], live: &BTreeSet<String>) -> Vec<T> {
    tasks
        .iter()
        .map(|task| {
            let mut task = task.clone();
            if live.contains(task.id()) && task.status() != "DONE" {
                task.set_status("IN PROGRESS");
            }
            task
        })
        .collect()
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct GroupCounts {
    pub active: usize,
    pub blocked: usize,
    pub todo: usize,
    pub done: usize,
    pub unknown: usize,
}

impl GroupCounts {
    pub(crate) fn get(&self, group: TaskGroup) -> usize {
        match group {
            TaskGroup::Active => self.active,
            TaskGroup::Blocked => self.blocked,
            TaskGroup::Todo => self.todo,
            TaskGroup::Done => self.done,
            TaskGroup::Unknown => self.unknown,
        }
    }

    fn bump(&mut self, group: TaskGroup) {
        match group {
            TaskGroup::Active => self.active += 1,
            TaskGroup::Blocked => self.blocked += 1,
            TaskGroup::Todo => self.todo += 1,
            TaskGroup::Done => self.done += 1,
            TaskGroup::Unknown => self.unknown += 1,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct DashboardSummary {
    pub counts: GroupCounts,
    pub percentage: i64,
    pub total: usize,
}

pub(crate) fn dashboard_summary<T: TaskLike>(tasks: &[T]) -> DashboardSummary {
    let mut counts = GroupCounts::default();
    for task in tasks {
        counts.bump(task_group(task.status()));
    }
    let percentage = if tasks.is_empty() {
        0
    } else {
        round_half_up(counts.done as f64 / tasks.len() as f64 * 100.)
    };
    DashboardSummary {
        counts,
        percentage,
        total: tasks.len(),
    }
}

/// The tasks of one group, in board order.
pub(crate) fn group_tasks<T: TaskLike>(tasks: &[T], group: TaskGroup) -> Vec<&T> {
    tasks
        .iter()
        .filter(|task| task_group(task.status()) == group)
        .collect()
}

pub(crate) fn questions_for<'a>(task_id: &str, questions: &'a [Question]) -> Vec<&'a Question> {
    questions
        .iter()
        .filter(|question| question.blocks.iter().any(|id| id == task_id))
        .collect()
}

pub(crate) fn plural(count: usize, noun: &str) -> String {
    format!("{count} {noun}{}", if count == 1 { "" } else { "s" })
}

/// The title without its leading TASK ID, or the full title when only the ID remains: JS
/// `title.replace(/^<id>\b[\s:–—-]*/, "").trim() || title`, by hand, because a regex per row
/// costs a compile per row.
pub(crate) fn task_title(id: &str, title: &str) -> String {
    let word = |c: char| c.is_ascii_alphanumeric() || c == '_';
    let rest = match title.strip_prefix(id) {
        // `\b` is the ASCII word boundary of JS; the end of the text counts as a non-word.
        Some(rest)
            if !id.is_empty()
                && id.chars().next_back().is_some_and(word)
                    != rest.chars().next().is_some_and(word) =>
        {
            rest.trim_start_matches(|c: char| {
                c.is_whitespace() || matches!(c, ':' | '–' | '—' | '-')
            })
        }
        _ => title,
    };
    let rest = rest.trim();
    if rest.is_empty() {
        title.to_string()
    } else {
        rest.to_string()
    }
}

/// The pill label. The board RPC returns only OPEN questions that block a task, so each one
/// counts. `live` holds the TASKs whose builder runs now; a done TASK is not counted.
pub(crate) fn pill_label(board: Option<&Board>, live: &BTreeSet<String>) -> String {
    let Some(board) = board.filter(|board| !board.tasks.is_empty()) else {
        return "Watchtower".to_string();
    };
    let done = board
        .tasks
        .iter()
        .filter(|task| task.status == "DONE")
        .count();
    let running = board
        .tasks
        .iter()
        .filter(|task| task.status != "DONE" && live.contains(&task.id))
        .count();
    let mut parts = vec![format!("Watchtower {done}/{}", board.tasks.len())];
    if running > 0 {
        parts.push(format!("{running} running"));
    }
    let questions = board.questions.len();
    if questions > 0 {
        let noun = if questions == 1 {
            "question"
        } else {
            "questions"
        };
        parts.push(format!("{questions} {noun}"));
    }
    parts.join(", ")
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn task(id: &str, status: &str) -> Task {
        Task {
            id: id.into(),
            title: format!("{id} Example task"),
            status: status.into(),
            deps: "-".into(),
            brief: Some("Do the work.".into()),
            ..Task::default()
        }
    }

    fn live(ids: &[&str]) -> BTreeSet<String> {
        ids.iter().map(|id| id.to_string()).collect()
    }

    fn board(statuses: &[&str], questions: &[&str]) -> Board {
        Board {
            tasks: statuses
                .iter()
                .enumerate()
                .map(|(index, status)| task(&format!("TASK-00{}", index + 1), status))
                .collect(),
            questions: questions
                .iter()
                .map(|id| Question {
                    id: id.to_string(),
                    question: "Why?".into(),
                    blocks: vec!["TASK-009".into()],
                })
                .collect(),
            ..Board::default()
        }
    }

    const TEN: [&str; 10] = [
        "DONE",
        "DONE",
        "DONE",
        "DONE",
        "DONE",
        "IN PROGRESS",
        "TODO",
        "TODO",
        "TODO",
        "BLOCKED",
    ];

    // pill.test.ts
    #[test]
    fn no_plan_gives_the_plain_label() {
        assert_eq!(pill_label(None, &live(&[])), "Watchtower");
        assert_eq!(pill_label(Some(&board(&[], &[])), &live(&[])), "Watchtower");
    }

    #[test]
    fn a_plan_shows_done_over_total() {
        assert_eq!(
            pill_label(Some(&board(&TEN, &[])), &live(&[])),
            "Watchtower 5/10"
        );
    }

    #[test]
    fn blocking_questions_add_a_count() {
        assert_eq!(
            pill_label(Some(&board(&TEN, &["Q-008"])), &live(&[])),
            "Watchtower 5/10, 1 question"
        );
        assert_eq!(
            pill_label(Some(&board(&TEN, &["Q-008", "Q-009"])), &live(&[])),
            "Watchtower 5/10, 2 questions"
        );
    }

    #[test]
    fn tasks_with_a_running_builder_add_a_count_before_the_questions() {
        let statuses = ["DONE", "DONE", "BLOCKED", "TODO"];
        assert_eq!(
            pill_label(
                Some(&board(&statuses, &["Q-008"])),
                &live(&["TASK-003", "TASK-004"])
            ),
            "Watchtower 2/4, 2 running, 1 question"
        );
        assert_eq!(
            pill_label(Some(&board(&statuses, &[])), &live(&["TASK-003"])),
            "Watchtower 2/4, 1 running"
        );
    }

    #[test]
    fn a_done_task_and_an_unknown_id_do_not_count_as_running() {
        assert_eq!(
            pill_label(
                Some(&board(&["DONE", "TODO"], &[])),
                &live(&["TASK-001", "TASK-099"])
            ),
            "Watchtower 1/2"
        );
    }

    #[test]
    fn three_of_five_done_with_one_live_and_one_question() {
        let mut plan = board(&["DONE", "DONE", "DONE", "TODO", "TODO"], &["Q-001"]);
        plan.tasks[4].status = "IN PROGRESS".into();
        // TASK-001 is live but done, so it does not count; TASK-004 is live and not done.
        assert_eq!(
            pill_label(Some(&plan), &live(&["TASK-001", "TASK-004"])),
            "Watchtower 3/5, 1 running, 1 question"
        );
    }

    // dashboard.test.ts
    #[test]
    fn summarizes_completion_and_keeps_zero_progress_empty() {
        assert_eq!(dashboard_summary::<Task>(&[]), DashboardSummary::default());
        let summary = dashboard_summary(&[
            task("TASK-001", "DONE"),
            task("TASK-002", "IN PROGRESS"),
            task("TASK-003", "BLOCKED"),
            task("TASK-004", "TODO"),
        ]);
        assert_eq!(
            summary,
            DashboardSummary {
                counts: GroupCounts {
                    active: 1,
                    blocked: 1,
                    todo: 1,
                    done: 1,
                    unknown: 0
                },
                percentage: 25,
                total: 4,
            }
        );
    }

    #[test]
    fn the_percentage_rounds_halves_up() {
        // 1 of 8 is 12.5 percent.
        let mut tasks = vec![task("TASK-001", "DONE")];
        tasks.extend((2..=8).map(|index| task(&format!("TASK-00{index}"), "TODO")));
        assert_eq!(dashboard_summary(&tasks).percentage, 13);
    }

    #[test]
    fn keeps_unknown_statuses_separate_from_todo() {
        let invalid = [task("TASK-005", "MAYBE")];
        let summary = dashboard_summary(&invalid);
        assert_eq!(summary.counts.todo, 0);
        assert_eq!(summary.counts.unknown, 1);
        assert!(group_tasks(&invalid, TaskGroup::Todo).is_empty());
        assert_eq!(group_tasks(&invalid, TaskGroup::Unknown)[0].id, "TASK-005");
    }

    #[test]
    fn task_group_for_each_status() {
        assert_eq!(task_group("IN PROGRESS"), TaskGroup::Active);
        assert_eq!(task_group("BLOCKED"), TaskGroup::Blocked);
        assert_eq!(task_group("TODO"), TaskGroup::Todo);
        assert_eq!(task_group("DONE"), TaskGroup::Done);
        assert_eq!(task_group("done"), TaskGroup::Unknown);
        assert_eq!(task_group(""), TaskGroup::Unknown);
    }

    #[test]
    fn lists_the_open_questions_that_block_a_task() {
        let questions = [
            Question {
                id: "Q-001".into(),
                question: "Which key?".into(),
                blocks: vec!["TASK-002".into(), "TASK-005".into()],
            },
            Question {
                id: "Q-002".into(),
                question: "Which port?".into(),
                blocks: vec!["TASK-003".into()],
            },
        ];
        let ids: Vec<&str> = questions_for("TASK-005", &questions)
            .iter()
            .map(|question| question.id.as_str())
            .collect();
        assert_eq!(ids, ["Q-001"]);
        assert!(questions_for("TASK-009", &questions).is_empty());
        assert_eq!(plural(1, "proposed ADR"), "1 proposed ADR");
        assert_eq!(plural(0, "open question"), "0 open questions");
    }

    #[test]
    fn groups_dashboard_tasks_and_separates_ids_from_titles() {
        let tasks = [
            task("TASK-001", "IN PROGRESS"),
            task("TASK-002", "BLOCKED"),
            task("TASK-003", "TODO"),
            task("TASK-004", "DONE"),
        ];
        let ids = |group| -> Vec<String> {
            group_tasks(&tasks, group)
                .iter()
                .map(|task| task.id.clone())
                .collect()
        };
        assert_eq!(ids(TaskGroup::Active), ["TASK-001"]);
        assert_eq!(ids(TaskGroup::Blocked), ["TASK-002"]);
        assert_eq!(ids(TaskGroup::Todo), ["TASK-003"]);
        assert_eq!(ids(TaskGroup::Done), ["TASK-004"]);
        assert_eq!(task_title(&tasks[0].id, &tasks[0].title), "Example task");
    }

    #[test]
    fn task_title_strips_the_id_and_falls_back_to_the_full_title() {
        assert_eq!(
            task_title("TASK-12", "TASK-12: Fix the pill"),
            "Fix the pill"
        );
        assert_eq!(task_title("TASK-12", "TASK-12 – Fix"), "Fix");
        assert_eq!(task_title("TASK-12", "TASK-12: "), "TASK-12: ");
        assert_eq!(task_title("TASK-12", "TASK-12"), "TASK-12");
        // `TASK-123` is another ID, so the word boundary keeps it.
        assert_eq!(task_title("TASK-12", "TASK-123 Other"), "TASK-123 Other");
        assert_eq!(task_title("TASK-12", "Plain title"), "Plain title");
    }

    #[test]
    fn a_task_with_a_live_builder_counts_as_active_and_a_done_task_stays_done() {
        let tasks = with_live_status(
            &[
                task("TASK-001", "BLOCKED"),
                task("TASK-002", "TODO"),
                task("TASK-003", "DONE"),
            ],
            &live(&["TASK-001", "TASK-003"]),
        );
        let statuses: Vec<&str> = tasks.iter().map(|task| task.status.as_str()).collect();
        assert_eq!(statuses, ["IN PROGRESS", "TODO", "DONE"]);
        let counts = dashboard_summary(&tasks).counts;
        assert_eq!((counts.active, counts.blocked), (1, 0));
        assert_eq!(group_tasks(&tasks, TaskGroup::Active)[0].id, "TASK-001");
    }
}
