//! The run timeline model (`client/timeline.ts`) and `clockAt` (`client/run-cells.tsx`).

use super::dto::{OverviewQuestion, OverviewRun, OverviewTask};
use super::time::round_half_up;
use regex::Regex;
use std::collections::HashMap;
use std::sync::LazyLock;

pub(crate) const DEFAULT_TASK_MINUTES: i64 = 20;
const DAY_MINUTES: i64 = 24 * 60;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BarKind {
    Done,
    Running,
    Planned,
    Blocked,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TimelineBar {
    pub id: String,
    pub kind: BarKind,
    pub start: i64,
    /// Done bars end at their log end; running bars end now; planned bars end at start plus the
    /// average.
    pub end: i64,
    /// Running bars only: start plus the average duration.
    pub expected_end: Option<i64>,
    pub lane: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TimelineEdge {
    pub from: String,
    pub to: String,
    pub from_lane: usize,
    pub to_lane: usize,
    pub from_minute: i64,
    pub to_minute: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TimelineTick {
    pub minute: i64,
    pub label: String,
}

/// A TASK with a number of minutes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TaskMinutes {
    pub id: String,
    pub minutes: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TimelineStats {
    pub average_minutes: i64,
    pub longest: Option<TaskMinutes>,
    /// The running bar furthest over the average, with its minutes over.
    pub over_average: Option<TaskMinutes>,
    pub max_parallel: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TimelineAxis {
    pub start: i64,
    pub end: i64,
    pub ticks: Vec<TimelineTick>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Deadline {
    pub question_id: String,
    pub minute: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Timeline {
    pub bars: Vec<TimelineBar>,
    pub edges: Vec<TimelineEdge>,
    pub lane_count: usize,
    pub axis: TimelineAxis,
    pub stats: TimelineStats,
    pub finish_minute: Option<i64>,
    pub deadlines: Vec<Deadline>,
}

/// The overview fields the timeline and the plan tree read.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TimelineInput<'a> {
    pub tasks: &'a [OverviewTask],
    pub run: Option<&'a OverviewRun>,
    pub questions: &'a [OverviewQuestion],
}

/// The minute a bar stops taking room: a running bar holds its lane until its expected end.
pub(crate) fn bar_end(bar: &TimelineBar) -> i64 {
    bar.end.max(bar.expected_end.unwrap_or(bar.end))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct StartedAt {
    pub year: i64,
    pub month: i64,
    pub day: i64,
    pub minutes: i64,
}

static STARTED: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^([0-9]{4})-([0-9]{2})-([0-9]{2})(?:[ T]([0-9]{2}):([0-9]{2}))?").unwrap()
});
static DATE_ONLY: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[0-9]{4}-[0-9]{2}-[0-9]{2}$").unwrap());

/// `Started:` as `YYYY-MM-DD HH:MM`, or a date alone, which counts from midnight like the server.
pub(crate) fn started_at(started: Option<&str>) -> Option<StartedAt> {
    let parts = STARTED.captures(started?)?;
    let number = |index: usize| -> i64 {
        parts
            .get(index)
            .map_or(0, |part| part.as_str().parse().unwrap_or(0))
    };
    Some(StartedAt {
        year: number(1),
        month: number(2),
        day: number(3),
        minutes: number(4) * 60 + number(5),
    })
}

fn date_only(started: Option<&str>) -> bool {
    DATE_ONLY.is_match(started.unwrap_or_default().trim())
}

/// `HH:MM` of a minute count, wrapped into one day.
pub(crate) fn clock_text(total: i64) -> String {
    let total = total.rem_euclid(DAY_MINUTES);
    format!("{:02}:{:02}", total / 60, total % 60)
}

/// The clock time of a timeline minute, from the run's `Started:` value.
pub(crate) fn clock_at(started: Option<&str>, minute: f64) -> Option<String> {
    let origin = started_at(started)?;
    Some(clock_text(origin.minutes + round_half_up(minute)))
}

fn hour_ticks(started: Option<&str>, end: i64) -> Vec<TimelineTick> {
    let origin = started_at(started).map(|at| at.minutes);
    let first = match origin {
        None => 60,
        Some(origin) => (60 - origin % 60) % 60,
    };
    let mut ticks = Vec::new();
    let mut minute = first;
    while minute <= end {
        let label = match origin {
            None => format!("+{}h", minute / 60),
            Some(origin) => format!("{:02}:00", ((origin + minute) / 60) % 24),
        };
        ticks.push(TimelineTick { minute, label });
        minute += 60;
    }
    ticks
}

fn max_parallel(bars: &[TimelineBar]) -> usize {
    let mut points: Vec<(i64, i64)> = bars
        .iter()
        .flat_map(|bar| [(bar.start, 1), (bar_end(bar), -1)])
        .collect();
    // An end and a start at the same minute do not overlap, so ends sort first.
    points.sort();
    let (mut open, mut max) = (0i64, 0i64);
    for (_, change) in points {
        open += change;
        max = max.max(open);
    }
    max as usize
}

/// Bars go into lanes by start time. A task takes its first dep's lane when that lane is free.
fn pack_lanes(bars: &mut [TimelineBar], deps: &HashMap<&str, &[String]>) -> usize {
    let mut by_id: HashMap<String, usize> = HashMap::new();
    for (index, bar) in bars.iter().enumerate() {
        by_id.insert(bar.id.clone(), index);
    }
    let mut lane_ends: Vec<i64> = Vec::new();
    let mut order: Vec<usize> = (0..bars.len()).collect();
    order.sort_by_key(|&index| bars[index].start);
    for index in order {
        let start = bars[index].start;
        let first = deps
            .get(bars[index].id.as_str())
            .and_then(|ids| ids.iter().find_map(|id| by_id.get(id).copied()));
        let free = |lane: usize| lane_ends.get(lane).is_some_and(|end| *end <= start);
        let lane = match first.map(|dep| bars[dep].lane) {
            Some(lane) if free(lane) => lane,
            _ => (0..lane_ends.len())
                .find(|&lane| free(lane))
                .unwrap_or(lane_ends.len()),
        };
        bars[index].lane = lane;
        let end = bar_end(&bars[index]);
        if lane == lane_ends.len() {
            lane_ends.push(end);
        } else {
            lane_ends[lane] = end;
        }
    }
    lane_ends.len()
}

pub(crate) fn build_timeline(input: TimelineInput<'_>) -> Timeline {
    let TimelineInput {
        tasks,
        run,
        questions,
    } = input;
    let log = run.map_or(&[][..], |run| &run.log[..]);
    let now = run.and_then(|run| run.now_minute);
    let mut last_done: HashMap<&str, (i64, i64)> = HashMap::new();
    for row in log {
        if let (true, Some(start), Some(end)) = (
            row.result.to_uppercase() == "DONE",
            row.start_minute,
            row.end_minute,
        ) {
            last_done.insert(&row.task, (start, end));
        }
    }
    let mut bars: Vec<TimelineBar> = Vec::new();
    for task in tasks {
        if let (true, Some(&(start, end))) =
            (task.status == "DONE", last_done.get(task.id.as_str()))
        {
            bars.push(TimelineBar {
                id: task.id.clone(),
                kind: BarKind::Done,
                start,
                end,
                expected_end: None,
                lane: 0,
            });
        }
    }
    let average = if bars.is_empty() {
        DEFAULT_TASK_MINUTES
    } else {
        let sum: i64 = bars.iter().map(|bar| bar.end - bar.start).sum();
        round_half_up(sum as f64 / bars.len() as f64)
    };

    // A running task starts where the latest log row ended, because the run builds one at a time.
    let last_end = log.iter().rev().find_map(|row| row.end_minute).unwrap_or(0);
    for task in tasks.iter().filter(|task| task.status == "IN PROGRESS") {
        let start = last_end.min(now.unwrap_or(last_end));
        bars.push(TimelineBar {
            id: task.id.clone(),
            kind: BarKind::Running,
            start,
            end: start.max(now.unwrap_or(start)),
            expected_end: Some(start + average),
            lane: 0,
        });
    }

    let mut ends: HashMap<String, i64> = HashMap::new();
    for bar in &bars {
        ends.insert(bar.id.clone(), bar_end(bar));
    }
    let mut cursor = bars
        .iter()
        .filter(|bar| bar.kind == BarKind::Running)
        .map(bar_end)
        .fold(now.unwrap_or(0), i64::max);
    for task in tasks {
        if task.status != "TODO" && task.status != "BLOCKED" {
            continue;
        }
        let dep_end = task
            .dep_ids
            .iter()
            .map(|id| ends.get(id).copied().unwrap_or(0))
            .fold(0, i64::max);
        let start = cursor.max(dep_end);
        let bar = TimelineBar {
            id: task.id.clone(),
            kind: if task.status == "BLOCKED" {
                BarKind::Blocked
            } else {
                BarKind::Planned
            },
            start,
            end: start + average,
            expected_end: None,
            lane: 0,
        };
        ends.insert(bar.id.clone(), bar.end);
        cursor = bar.end;
        bars.push(bar);
    }

    let mut deps: HashMap<&str, &[String]> = HashMap::new();
    for task in tasks {
        deps.insert(&task.id, &task.dep_ids);
    }
    let lane_count = pack_lanes(&mut bars, &deps);
    let mut by_id: HashMap<&str, &TimelineBar> = HashMap::new();
    for bar in &bars {
        by_id.insert(&bar.id, bar);
    }
    let mut edges = Vec::new();
    for bar in &bars {
        for dep_id in deps.get(bar.id.as_str()).copied().unwrap_or_default() {
            let Some(dep) = by_id.get(dep_id.as_str()) else {
                continue;
            };
            edges.push(TimelineEdge {
                from: dep.id.clone(),
                to: bar.id.clone(),
                from_lane: dep.lane,
                to_lane: bar.lane,
                from_minute: bar_end(dep),
                to_minute: bar.start,
            });
        }
    }

    let open: Vec<&TimelineBar> = bars
        .iter()
        .filter(|bar| matches!(bar.kind, BarKind::Planned | BarKind::Blocked))
        .collect();
    let running: Vec<&TimelineBar> = bars
        .iter()
        .filter(|bar| bar.kind == BarKind::Running)
        .collect();
    let finish_minute = if !open.is_empty() {
        open.iter().map(|bar| bar.end).max()
    } else {
        running.iter().map(|bar| bar_end(bar)).max()
    };
    let mut deadlines = Vec::new();
    for question in questions {
        if question.status != "OPEN" || question.blocks.is_empty() {
            continue;
        }
        let start = question
            .blocks
            .iter()
            .filter_map(|id| by_id.get(id.as_str()))
            .filter(|bar| matches!(bar.kind, BarKind::Planned | BarKind::Blocked))
            .map(|bar| bar.start)
            .min();
        if let Some(minute) = start {
            deadlines.push(Deadline {
                question_id: question.id.clone(),
                minute,
            });
        }
    }

    let mut longest: Option<TaskMinutes> = None;
    for bar in bars.iter().filter(|bar| bar.kind == BarKind::Done) {
        let minutes = bar.end - bar.start;
        if longest.as_ref().is_none_or(|best| minutes > best.minutes) {
            longest = Some(TaskMinutes {
                id: bar.id.clone(),
                minutes,
            });
        }
    }
    let mut over_average: Option<TaskMinutes> = None;
    for bar in &running {
        let over = bar.end - bar.start - average;
        if over > 0
            && over_average
                .as_ref()
                .is_none_or(|worst| over > worst.minutes)
        {
            over_average = Some(TaskMinutes {
                id: bar.id.clone(),
                minutes: over,
            });
        }
    }
    let axis_end = bars
        .iter()
        .map(bar_end)
        .fold(now.unwrap_or(0).max(0), i64::max);
    // A row logged a minute before `Started:` has a negative start. A `Started:` date with no time
    // counts from an assumed midnight, so the axis starts at the first task instead.
    let first_start = bars.iter().map(|bar| bar.start).min();
    let started = run.and_then(|run| run.started.as_deref());
    let axis_start = match first_start {
        Some(first) if date_only(started) => first,
        Some(first) => first.min(0),
        None => 0,
    };
    let ticks = hour_ticks(started, axis_end)
        .into_iter()
        .filter(|tick| tick.minute >= axis_start)
        .collect();
    let stats = TimelineStats {
        average_minutes: average,
        longest,
        over_average,
        max_parallel: max_parallel(&bars),
    };
    Timeline {
        bars,
        edges,
        lane_count,
        axis: TimelineAxis {
            start: axis_start,
            end: axis_end,
            ticks,
        },
        stats,
        finish_minute,
        deadlines,
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::model::dto::OverviewRunEntry;

    pub(crate) fn task(id: &str, status: &str, dep_ids: &[&str]) -> OverviewTask {
        OverviewTask {
            id: id.into(),
            title: id.into(),
            status: status.into(),
            deps: if dep_ids.is_empty() {
                "-".into()
            } else {
                dep_ids.join(", ")
            },
            dep_ids: dep_ids.iter().map(|id| id.to_string()).collect(),
            ..OverviewTask::default()
        }
    }

    pub(crate) fn row(task: &str, start: i64, end: Option<i64>) -> OverviewRunEntry {
        OverviewRunEntry {
            task: task.into(),
            result: "DONE".into(),
            start_minute: Some(start),
            end_minute: end,
            ..OverviewRunEntry::default()
        }
    }

    pub(crate) fn run(started: &str, log: Vec<OverviewRunEntry>, now: i64) -> OverviewRun {
        OverviewRun {
            runner: Some("loop".into()),
            started: Some(started.into()),
            finished: None,
            log,
            now_minute: Some(now),
            stopped: false,
        }
    }

    struct Plan {
        tasks: Vec<OverviewTask>,
        run: Option<OverviewRun>,
        questions: Vec<OverviewQuestion>,
    }

    impl Plan {
        fn input(&self) -> TimelineInput<'_> {
            TimelineInput {
                tasks: &self.tasks,
                run: self.run.as_ref(),
                questions: &self.questions,
            }
        }
    }

    // The mid-run state of the t3code plan used in the design mock.
    fn mid_run() -> Plan {
        Plan {
            tasks: vec![
                task("TASK-001", "DONE", &[]),
                task("TASK-002", "DONE", &["TASK-001"]),
                task("TASK-003", "DONE", &["TASK-001"]),
                task("TASK-004", "DONE", &["TASK-003"]),
                task("TASK-005", "DONE", &[]),
                task("TASK-006", "IN PROGRESS", &[]),
                task("TASK-010", "TODO", &["TASK-006"]),
                task("TASK-007", "TODO", &["TASK-006", "TASK-010"]),
                task("TASK-008", "IN PROGRESS", &[]),
                task("TASK-009", "BLOCKED", &["TASK-008"]),
            ],
            run: Some(run(
                "2026-10-06 22:52",
                vec![
                    row("TASK-001", 0, Some(14)),
                    row("TASK-002", 17, Some(36)),
                    row("TASK-003", 39, Some(67)),
                    row("TASK-004", 70, Some(89)),
                    row("TASK-005", 92, Some(100)),
                ],
                140,
            )),
            questions: vec![OverviewQuestion {
                id: "Q-008".into(),
                tasks: vec!["TASK-008".into()],
                blocks: vec!["TASK-009".into()],
                question: "Network access?".into(),
                default: "No network".into(),
                status: "OPEN".into(),
            }],
        }
    }

    fn bar<'a>(timeline: &'a Timeline, id: &str) -> &'a TimelineBar {
        timeline.bars.iter().find(|bar| bar.id == id).unwrap()
    }

    // timeline.test.ts
    #[test]
    fn a_sequential_done_log_packs_into_lane_0_and_running_tasks_fork() {
        let timeline = build_timeline(mid_run().input());
        for id in ["TASK-001", "TASK-002", "TASK-003", "TASK-004", "TASK-005"] {
            assert_eq!(bar(&timeline, id).kind, BarKind::Done);
            assert_eq!(bar(&timeline, id).lane, 0);
        }
        assert_eq!(timeline.stats.average_minutes, 18);
        let view = |id| {
            let bar = bar(&timeline, id);
            (bar.kind, bar.start, bar.end, bar.expected_end, bar.lane)
        };
        assert_eq!(view("TASK-006"), (BarKind::Running, 100, 140, Some(118), 0));
        assert_eq!(view("TASK-008"), (BarKind::Running, 100, 140, Some(118), 1));
        assert_eq!(timeline.lane_count, 2);
        assert_eq!(timeline.stats.max_parallel, 2);
        assert_eq!(
            timeline.stats.over_average,
            Some(TaskMinutes {
                id: "TASK-006".into(),
                minutes: 22
            })
        );
        assert_eq!(
            timeline.stats.longest,
            Some(TaskMinutes {
                id: "TASK-003".into(),
                minutes: 28
            })
        );
    }

    #[test]
    fn planned_tasks_run_one_at_a_time_after_their_deps() {
        let timeline = build_timeline(mid_run().input());
        let view = |id| {
            let bar = bar(&timeline, id);
            (bar.kind, bar.start, bar.end, bar.lane)
        };
        assert_eq!(view("TASK-010"), (BarKind::Planned, 140, 158, 0));
        assert_eq!(view("TASK-007"), (BarKind::Planned, 158, 176, 0));
        assert_eq!(view("TASK-009"), (BarKind::Blocked, 176, 194, 1));
        assert!(bar(&timeline, "TASK-007").start >= bar(&timeline, "TASK-010").end);
        assert_eq!(timeline.finish_minute, Some(194));
        assert_eq!(
            timeline.deadlines,
            [Deadline {
                question_id: "Q-008".into(),
                minute: 176
            }]
        );
        let edge = timeline.edges.iter().find(|edge| edge.to == "TASK-009");
        assert_eq!(
            edge,
            Some(&TimelineEdge {
                from: "TASK-008".into(),
                to: "TASK-009".into(),
                from_lane: 1,
                to_lane: 1,
                from_minute: 140,
                to_minute: 176,
            })
        );
        assert_eq!(
            timeline
                .edges
                .iter()
                .filter(|edge| edge.to == "TASK-007")
                .count(),
            2
        );
    }

    #[test]
    fn the_axis_ticks_fall_on_whole_clock_hours() {
        let timeline = build_timeline(mid_run().input());
        assert_eq!(timeline.axis.end, 194);
        let ticks: Vec<(i64, &str)> = timeline
            .axis
            .ticks
            .iter()
            .map(|tick| (tick.minute, tick.label.as_str()))
            .collect();
        assert_eq!(
            ticks,
            [(8, "23:00"), (68, "00:00"), (128, "01:00"), (188, "02:00")]
        );
    }

    #[test]
    fn a_dep_lane_that_is_busy_sends_the_task_to_another_lane() {
        let mut plan = mid_run();
        plan.tasks = vec![
            task("TASK-001", "IN PROGRESS", &[]),
            task("TASK-002", "IN PROGRESS", &["TASK-001"]),
        ];
        plan.run.as_mut().unwrap().log.clear();
        plan.questions.clear();
        let timeline = build_timeline(plan.input());
        let bars: Vec<(&str, i64, usize)> = timeline
            .bars
            .iter()
            .map(|bar| (bar.id.as_str(), bar.start, bar.lane))
            .collect();
        assert_eq!(bars, [("TASK-001", 0, 0), ("TASK-002", 0, 1)]);
        assert_eq!(timeline.stats.average_minutes, 20);
        let lanes: Vec<(usize, usize)> = timeline
            .edges
            .iter()
            .map(|edge| (edge.from_lane, edge.to_lane))
            .collect();
        assert_eq!(lanes, [(0, 1)]);
    }

    #[test]
    fn no_done_bars_gives_the_20_minute_default_and_no_run_gives_relative_ticks() {
        let plan = Plan {
            tasks: vec![
                task("TASK-001", "TODO", &[]),
                task("TASK-002", "TODO", &["TASK-001"]),
            ],
            run: None,
            questions: vec![],
        };
        let timeline = build_timeline(plan.input());
        assert_eq!(timeline.stats.average_minutes, 20);
        let bars: Vec<(&str, i64, i64)> = timeline
            .bars
            .iter()
            .map(|bar| (bar.id.as_str(), bar.start, bar.end))
            .collect();
        assert_eq!(bars, [("TASK-001", 0, 20), ("TASK-002", 20, 40)]);
        assert_eq!(timeline.finish_minute, Some(40));
        assert!(timeline.axis.ticks.is_empty());
        let mut no_run = mid_run();
        no_run.run = None;
        assert_eq!(build_timeline(no_run.input()).axis.ticks[0].label, "+1h");
    }

    #[test]
    fn every_task_done_gives_no_finish_and_a_done_task_without_a_log_row_has_no_bar() {
        let mut plan = mid_run();
        for task in &mut plan.tasks {
            task.status = "DONE".into();
        }
        let timeline = build_timeline(plan.input());
        assert_eq!(timeline.finish_minute, None);
        assert_eq!(timeline.bars.len(), 5);
        assert!(!timeline.bars.iter().any(|bar| bar.id == "TASK-006"));
        assert!(timeline.deadlines.is_empty());
    }

    #[test]
    fn the_model_stays_cheap_on_200_tasks_and_200_log_rows() {
        let mut tasks = Vec::new();
        let mut log = Vec::new();
        for index in 0..200 {
            let id = format!("TASK-{:03}", index + 1);
            let dep = format!("TASK-{index:03}");
            let deps: Vec<&str> = if index > 0 {
                vec![dep.as_str()]
            } else {
                vec![]
            };
            let status = if index < 160 {
                "DONE"
            } else if index < 165 {
                "IN PROGRESS"
            } else {
                "TODO"
            };
            tasks.push(task(&id, status, &deps));
            log.push(row(&id, index * 20, Some(index * 20 + 15)));
        }
        let plan = Plan {
            tasks,
            run: Some(run("2026-10-06 22:52", log, 4000)),
            questions: vec![],
        };
        let runs = 50;
        let begin = std::time::Instant::now();
        for _ in 0..runs {
            build_timeline(plan.input());
        }
        let average = begin.elapsed().as_secs_f64() * 1000. / runs as f64;
        assert!(average <= 10., "average {average} ms");
    }

    // run-cells.tsx `clockAt`
    #[test]
    fn clock_at_wraps_past_midnight_and_needs_a_readable_start() {
        assert_eq!(
            clock_at(Some("2026-10-14 23:50"), 20.),
            Some("00:10".into())
        );
        assert_eq!(
            clock_at(Some("2026-10-14 10:00"), 17.),
            Some("10:17".into())
        );
        assert_eq!(
            clock_at(Some("2026-10-14 10:00"), -15.),
            Some("09:45".into())
        );
        // `Math.round` rounds the half minute up.
        assert_eq!(
            clock_at(Some("2026-10-14 10:00"), 0.5),
            Some("10:01".into())
        );
        assert_eq!(clock_at(Some("2026-10-14"), 61.), Some("01:01".into()));
        assert_eq!(clock_at(Some("soon"), 20.), None);
        assert_eq!(clock_at(None, 20.), None);
    }
}
