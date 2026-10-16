//! The plan phases and the Needs you rows (`client/lifecycle.ts`).

use super::dashboard::plural;
use super::dto::Overview;
use std::collections::HashSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PhaseName {
    Plan,
    Implement,
    Verify,
    Review,
    Archive,
}

impl PhaseName {
    pub(crate) const ALL: [PhaseName; 5] = [
        PhaseName::Plan,
        PhaseName::Implement,
        PhaseName::Verify,
        PhaseName::Review,
        PhaseName::Archive,
    ];

    pub(crate) fn label(self) -> &'static str {
        match self {
            PhaseName::Plan => "Plan",
            PhaseName::Implement => "Implement",
            PhaseName::Verify => "Verify",
            PhaseName::Review => "Review",
            PhaseName::Archive => "Archive",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PhaseState {
    Done,
    Current,
    Todo,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Phase {
    pub name: PhaseName,
    pub state: PhaseState,
    pub meta: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NeedsYouKind {
    Question,
    Adr,
    Checks,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NeedsYouTone {
    Danger,
    Warning,
    Neutral,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct NeedsYouRow {
    pub kind: NeedsYouKind,
    pub id: String,
    pub title: String,
    pub meta: String,
    pub tone: NeedsYouTone,
    /// Question rows: the default answer, or `None` when the question has none.
    pub default_answer: Option<String>,
    /// The checks row: the text of each pending manual check.
    pub checks: Vec<String>,
}

/// The index of the current phase, per the lifecycle rules in `watchtower/CONTEXT.md`.
fn current_phase(overview: &Overview) -> usize {
    if overview
        .plan
        .status
        .as_deref()
        .is_some_and(|status| status.to_uppercase() == "ARCHIVED")
    {
        return 4;
    }
    if overview.tasks.is_empty() {
        return 0;
    }
    if overview
        .run
        .as_ref()
        .is_some_and(|run| run.finished.is_some())
    {
        return 3;
    }
    if overview.tasks.iter().all(|task| task.status == "DONE") {
        return 2;
    }
    1
}

pub(crate) fn lifecycle_phases(overview: &Overview) -> Vec<Phase> {
    let tasks = &overview.tasks;
    let run = overview.run.as_ref();
    let current = current_phase(overview);
    let done = tasks.iter().filter(|task| task.status == "DONE").count();
    let groups: HashSet<&str> = tasks
        .iter()
        .filter_map(|task| task.group.as_deref())
        .collect();
    let live =
        run.is_some_and(|run| run.started.is_some() && run.finished.is_none() && !run.stopped);
    let run_state = if live {
        ", autorun live"
    } else if run.is_some_and(|run| run.stopped) {
        ", run stopped"
    } else {
        ""
    };
    let meta = |name: PhaseName| -> String {
        match name {
            PhaseName::Plan if tasks.is_empty() => "No plan yet".into(),
            PhaseName::Plan if !groups.is_empty() => format!(
                "{} in {}",
                plural(tasks.len(), "task"),
                plural(groups.len(), "group")
            ),
            PhaseName::Plan => plural(tasks.len(), "task"),
            PhaseName::Implement => format!("{done} of {} done{run_state}", tasks.len()),
            PhaseName::Verify => format!(
                "{} and {}",
                plural(overview.plan_verify.len(), "check"),
                plural(overview.manual_checks.len(), "manual check")
            ),
            PhaseName::Review if run.is_some_and(|run| run.finished.is_some()) => {
                "Draft PR open".into()
            }
            PhaseName::Review => "After the run".into(),
            PhaseName::Archive => "Learnings, then archive".into(),
        }
    };
    PhaseName::ALL
        .iter()
        .enumerate()
        .map(|(index, &name)| Phase {
            name,
            state: match index.cmp(&current) {
                std::cmp::Ordering::Less => PhaseState::Done,
                std::cmp::Ordering::Equal => PhaseState::Current,
                std::cmp::Ordering::Greater => PhaseState::Todo,
            },
            meta: meta(name),
        })
        .collect()
}

/// Blocking questions first, then other questions, then proposed ADRs, then manual checks.
pub(crate) fn needs_you(overview: &Overview) -> Vec<NeedsYouRow> {
    let blocking = |status: &str, blocks: &[String]| status == "OPEN" && !blocks.is_empty();
    // An empty cell, `-`, and `none` all mean the question has no default.
    let real_default = |default: &str| -> Option<String> {
        let key = default.trim().to_lowercase();
        (!["", "-", "none"].contains(&key.as_str())).then(|| default.to_string())
    };
    let mut rows = Vec::new();
    for question in overview
        .questions
        .iter()
        .filter(|question| blocking(&question.status, &question.blocks))
    {
        rows.push(NeedsYouRow {
            kind: NeedsYouKind::Question,
            id: question.id.clone(),
            title: question.question.clone(),
            meta: format!("{} blocks {}", question.id, question.blocks.join(", ")),
            tone: NeedsYouTone::Danger,
            default_answer: real_default(&question.default),
            checks: Vec::new(),
        });
    }
    for question in overview
        .questions
        .iter()
        .filter(|question| !blocking(&question.status, &question.blocks))
    {
        let default_answer = real_default(&question.default);
        rows.push(NeedsYouRow {
            kind: NeedsYouKind::Question,
            id: question.id.clone(),
            title: question.question.clone(),
            meta: format!(
                "{}, {}",
                question.id,
                if default_answer.is_some() {
                    "has a default"
                } else {
                    "open"
                }
            ),
            tone: if default_answer.is_some() {
                NeedsYouTone::Warning
            } else {
                NeedsYouTone::Neutral
            },
            default_answer,
            checks: Vec::new(),
        });
    }
    for decision in overview
        .decisions
        .iter()
        .filter(|decision| decision.status.to_lowercase() == "proposed")
    {
        rows.push(NeedsYouRow {
            kind: NeedsYouKind::Adr,
            id: decision.id.clone(),
            title: decision.title.clone(),
            meta: format!("{}, proposed", decision.id),
            tone: NeedsYouTone::Neutral,
            default_answer: None,
            checks: Vec::new(),
        });
    }
    let checks = &overview.manual_checks;
    if !checks.is_empty() {
        let mut tasks: Vec<&str> = Vec::new();
        for task in checks.iter().flat_map(|check| &check.tasks) {
            if !tasks.contains(&task.as_str()) {
                tasks.push(task);
            }
        }
        rows.push(NeedsYouRow {
            kind: NeedsYouKind::Checks,
            id: "manual-checks".into(),
            title: format!("{} pending", plural(checks.len(), "manual check")),
            meta: if tasks.is_empty() {
                "From the Handoff".into()
            } else {
                format!("From {}", tasks.join(", "))
            },
            tone: NeedsYouTone::Neutral,
            default_answer: None,
            checks: checks.iter().map(|check| check.text.clone()).collect(),
        });
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::dto::{
        Decision, ManualCheck, OverviewPlan, OverviewQuestion, OverviewRun, OverviewTask,
    };

    fn task(id: &str, status: &str, group: Option<&str>) -> OverviewTask {
        OverviewTask {
            id: id.into(),
            title: id.into(),
            status: status.into(),
            deps: "-".into(),
            group: group.map(Into::into),
            ..OverviewTask::default()
        }
    }

    fn live_run(started: &str, finished: Option<&str>, now: i64, stopped: bool) -> OverviewRun {
        OverviewRun {
            runner: Some("loop".into()),
            started: Some(started.into()),
            finished: finished.map(Into::into),
            log: vec![],
            now_minute: Some(now),
            stopped,
        }
    }

    fn overview() -> Overview {
        Overview {
            plan: OverviewPlan {
                title: "Plan".into(),
                slug: Some("20261007-plan".into()),
                status: Some("ACTIVE".into()),
                updated: Some("2026-10-07".into()),
            },
            tasks: vec![
                task("TASK-001", "DONE", Some("A")),
                task("TASK-002", "IN PROGRESS", Some("B")),
            ],
            plan_verify: vec!["npm test".into(), "npm run lint".into()],
            run: Some(live_run("2026-10-07 12:32", None, 10, false)),
            ..Overview::default()
        }
    }

    fn states(input: &Overview) -> Vec<String> {
        lifecycle_phases(input)
            .iter()
            .map(|phase| {
                let state = match phase.state {
                    PhaseState::Done => "done",
                    PhaseState::Current => "current",
                    PhaseState::Todo => "todo",
                };
                format!("{}:{state}", phase.name.label())
            })
            .collect()
    }

    fn question(id: &str, blocks: &[&str], default: &str, status: &str) -> OverviewQuestion {
        OverviewQuestion {
            id: id.into(),
            tasks: vec![],
            blocks: blocks.iter().map(|id| id.to_string()).collect(),
            question: "Why?".into(),
            default: default.into(),
            status: status.into(),
        }
    }

    // lifecycle.test.ts
    #[test]
    fn open_tasks_put_the_plan_in_implement() {
        assert_eq!(
            states(&overview()),
            [
                "Plan:done",
                "Implement:current",
                "Verify:todo",
                "Review:todo",
                "Archive:todo"
            ]
        );
        let phases = lifecycle_phases(&overview());
        assert_eq!(phases[0].meta, "2 tasks in 2 groups");
        assert_eq!(phases[1].meta, "1 of 2 done, autorun live");
        assert_eq!(phases[2].meta, "2 checks and 0 manual checks");
        assert_eq!(phases[3].meta, "After the run");
    }

    #[test]
    fn a_stopped_run_is_not_live() {
        let input = Overview {
            run: Some(live_run("2026-10-07 08:00", None, 30, true)),
            ..overview()
        };
        assert_eq!(lifecycle_phases(&input)[1].meta, "1 of 2 done, run stopped");
        assert_eq!(states(&input)[1], "Implement:current");
    }

    #[test]
    fn every_task_done_without_a_finish_puts_the_plan_in_verify() {
        let input = Overview {
            tasks: vec![
                task("TASK-001", "DONE", Some("A")),
                task("TASK-002", "DONE", Some("A")),
            ],
            ..overview()
        };
        assert_eq!(
            states(&input)[..3],
            ["Plan:done", "Implement:done", "Verify:current"]
        );
    }

    #[test]
    fn a_finished_run_puts_the_plan_in_review() {
        let input = Overview {
            run: Some(live_run(
                "2026-10-07 12:32",
                Some("2026-10-07 15:00"),
                148,
                false,
            )),
            ..overview()
        };
        assert_eq!(
            states(&input),
            [
                "Plan:done",
                "Implement:done",
                "Verify:done",
                "Review:current",
                "Archive:todo"
            ]
        );
        assert_eq!(lifecycle_phases(&input)[3].meta, "Draft PR open");
        assert_eq!(lifecycle_phases(&input)[1].meta, "1 of 2 done");
    }

    #[test]
    fn an_archived_plan_puts_the_plan_in_archive() {
        let input = Overview {
            plan: OverviewPlan {
                title: "Plan".into(),
                status: Some("ARCHIVED".into()),
                ..OverviewPlan::default()
            },
            ..overview()
        };
        let states = states(&input);
        assert_eq!(states[4], "Archive:current");
        assert!(states[..4].iter().all(|state| state.ends_with(":done")));
    }

    #[test]
    fn no_plan_keeps_plan_current_with_no_plan_yet() {
        let input = Overview {
            tasks: vec![],
            run: None,
            plan_verify: vec![],
            ..overview()
        };
        assert_eq!(
            states(&input),
            [
                "Plan:current",
                "Implement:todo",
                "Verify:todo",
                "Review:todo",
                "Archive:todo"
            ]
        );
        assert_eq!(lifecycle_phases(&input)[0].meta, "No plan yet");
    }

    #[test]
    fn tasks_without_a_group_column_count_tasks_only() {
        let input = Overview {
            tasks: vec![task("TASK-001", "TODO", None)],
            ..overview()
        };
        assert_eq!(lifecycle_phases(&input)[0].meta, "1 task");
    }

    #[test]
    fn needs_you_orders_blocking_questions_defaults_adrs_then_checks() {
        let input = Overview {
            questions: vec![
                question("Q-010", &[], "Open the agent", "DEFAULTED"),
                question("Q-012", &[], "none", "OPEN"),
                question("Q-008", &["TASK-009"], "No network", "OPEN"),
            ],
            decisions: vec![
                Decision {
                    id: "ADR-0001".into(),
                    date: "2026-09-30".into(),
                    title: "Active host".into(),
                    status: "accepted".into(),
                },
                Decision {
                    id: "ADR-0003".into(),
                    date: "2026-10-07".into(),
                    title: "Chip-only sources".into(),
                    status: "proposed".into(),
                },
            ],
            manual_checks: vec![
                ManualCheck {
                    text: "Stop a parent (TASK-001).".into(),
                    tasks: vec!["TASK-001".into()],
                },
                ManualCheck {
                    text: "Queued message (TASK-003, TASK-004).".into(),
                    tasks: vec!["TASK-003".into(), "TASK-004".into()],
                },
            ],
            ..overview()
        };
        let rows = needs_you(&input);
        let view: Vec<(NeedsYouKind, &str, NeedsYouTone, &str)> = rows
            .iter()
            .map(|row| (row.kind, row.id.as_str(), row.tone, row.meta.as_str()))
            .collect();
        assert_eq!(
            view,
            [
                (
                    NeedsYouKind::Question,
                    "Q-008",
                    NeedsYouTone::Danger,
                    "Q-008 blocks TASK-009"
                ),
                (
                    NeedsYouKind::Question,
                    "Q-010",
                    NeedsYouTone::Warning,
                    "Q-010, has a default"
                ),
                (
                    NeedsYouKind::Question,
                    "Q-012",
                    NeedsYouTone::Neutral,
                    "Q-012, open"
                ),
                (
                    NeedsYouKind::Adr,
                    "ADR-0003",
                    NeedsYouTone::Neutral,
                    "ADR-0003, proposed"
                ),
                (
                    NeedsYouKind::Checks,
                    "manual-checks",
                    NeedsYouTone::Neutral,
                    "From TASK-001, TASK-003, TASK-004"
                ),
            ]
        );
        assert_eq!(rows[4].title, "2 manual checks pending");
        assert!(needs_you(&overview()).is_empty());
    }

    #[test]
    fn needs_you_rows_carry_what_their_actions_send() {
        let input = Overview {
            questions: ["", "-", "None", "Yes."]
                .iter()
                .enumerate()
                .map(|(index, answer)| {
                    let blocks: &[&str] = if index == 3 { &["TASK-001"] } else { &[] };
                    question(&format!("Q-00{index}"), blocks, answer, "OPEN")
                })
                .collect(),
            manual_checks: vec![
                ManualCheck {
                    text: "Stop a parent (TASK-001).".into(),
                    tasks: vec!["TASK-001".into()],
                },
                ManualCheck {
                    text: "Queued message.".into(),
                    tasks: vec![],
                },
            ],
            ..overview()
        };
        let rows = needs_you(&input);
        // A blocking question comes first, and it keeps its default.
        let defaults: Vec<Option<&str>> = rows
            .iter()
            .map(|row| row.default_answer.as_deref())
            .collect();
        assert_eq!(defaults, [Some("Yes."), None, None, None, None]);
        assert_eq!(rows[4].kind, NeedsYouKind::Checks);
        assert_eq!(
            rows[4].checks,
            ["Stop a parent (TASK-001).", "Queued message."]
        );
    }

    #[test]
    fn the_model_stays_cheap_on_a_200_task_plan() {
        let input = Overview {
            tasks: (0..200)
                .map(|index| {
                    let group = format!("G{}", index % 9);
                    task(
                        &format!("TASK-{:03}", index + 1),
                        if index % 3 == 0 { "DONE" } else { "TODO" },
                        Some(&group),
                    )
                })
                .collect(),
            questions: (0..40)
                .map(|index| {
                    let block = format!("TASK-{index}");
                    let blocks: Vec<&str> = if index % 2 == 1 { vec![&block] } else { vec![] };
                    question(&format!("Q-{index}"), &blocks, "Yes", "OPEN")
                })
                .collect(),
            manual_checks: (0..50)
                .map(|index| ManualCheck {
                    text: "Check".into(),
                    tasks: vec![format!("TASK-{index}")],
                })
                .collect(),
            ..overview()
        };
        let runs = 100;
        let begin = std::time::Instant::now();
        for _ in 0..runs {
            lifecycle_phases(&input);
            needs_you(&input);
        }
        let average = begin.elapsed().as_secs_f64() * 1000. / runs as f64;
        assert!(average <= 5., "average {average} ms");
    }
}
