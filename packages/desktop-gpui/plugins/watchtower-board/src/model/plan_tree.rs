//! The plan tree model (`client/plan-tree.ts`), and `commitShas` and `ago` of
//! `client/plan-tree-view.tsx`.

use super::dashboard::task_title;
use super::dto::{OverviewRunEntry, OverviewTask};
use super::id_map::{IdMap as HashMap, IdSet as HashSet};
use super::time::round_half_up;
use super::timeline::{
    BarKind, TaskMinutes, Timeline, TimelineBar, TimelineInput, build_timeline, clock_text,
    started_at,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PlanTreeRow {
    pub id: String,
    pub title: String,
    pub group: Option<String>,
    pub kind: BarKind,
    /// `4m` done, `12m / 9m` running, `~10:24` planned; `None` for blocked rows and rows with no
    /// data.
    pub time: Option<String>,
    /// Running rows only: the elapsed minutes pass the average.
    pub over: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PlanTreeWave {
    /// 1-based position in the list.
    pub number: usize,
    pub header: String,
    pub rows: Vec<PlanTreeRow>,
    /// Every row is done; the view collapses the wave.
    pub done: bool,
    /// The wave holds the TASKs left in a dependency cycle, or waiting on one.
    pub cyclic: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PlanTreePace {
    pub average_minutes: i64,
    /// Estimated finish as `HH:MM`; `None` when the run has no start clock or nothing is left.
    pub finish_clock: Option<String>,
    pub max_parallel: usize,
    /// The running TASK furthest over the average, with its minutes over.
    pub slowest: Option<TaskMinutes>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PlanTree {
    pub waves: Vec<PlanTreeWave>,
    pub pace: PlanTreePace,
}

/// `a`, `a and b`, `a, b and c`.
fn names(ids: &[&str], joiner: &str) -> String {
    match ids {
        [] => String::new(),
        [one] => one.to_string(),
        [rest @ .., last] => format!("{} {joiner} {last}", rest.join(", ")),
    }
}

/// The wave number of each TASK, by position in `tasks`, which holds unique IDs; a TASK that sits
/// in or behind a cycle has none. It works on positions, not IDs: per-ID maps made it the
/// slowest part of the derived data in a debug build.
fn assign_waves(tasks: &[OverviewTask]) -> Vec<Option<usize>> {
    let index: HashMap<&str, usize> = tasks
        .iter()
        .enumerate()
        .map(|(position, item)| (item.id.as_str(), position))
        .collect();
    let mut waiting = vec![0usize; tasks.len()];
    let mut dependents: Vec<Vec<usize>> = vec![Vec::new(); tasks.len()];
    for (position, item) in tasks.iter().enumerate() {
        let mut deps: Vec<usize> = Vec::new();
        for dep in &item.dep_ids {
            if let Some(&dep) = index.get(dep.as_str())
                && !deps.contains(&dep)
            {
                deps.push(dep);
            }
        }
        waiting[position] = deps.len();
        for dep in deps {
            dependents[dep].push(position);
        }
    }
    let mut levels: Vec<Option<usize>> = vec![None; tasks.len()];
    let mut ready: Vec<usize> = (0..tasks.len())
        .filter(|&position| waiting[position] == 0)
        .collect();
    for &position in &ready {
        levels[position] = Some(1);
    }
    while !ready.is_empty() {
        let mut next = Vec::new();
        for position in ready {
            let level = levels[position].unwrap_or(1);
            for &dependent in &dependents[position] {
                let current = levels[dependent].unwrap_or(1);
                levels[dependent] = Some(current.max(level + 1));
                waiting[dependent] = waiting[dependent].saturating_sub(1);
                if waiting[dependent] == 0 {
                    next.push(dependent);
                }
            }
        }
        ready = next;
    }
    for (level, left) in levels.iter_mut().zip(&waiting) {
        if *left > 0 {
            *level = None;
        }
    }
    levels
}

fn row_for(
    item: &OverviewTask,
    bar: Option<&TimelineBar>,
    average: i64,
    origin: Option<i64>,
    live: bool,
) -> PlanTreeRow {
    let kind = if item.status == "DONE" {
        BarKind::Done
    } else if item.status == "IN PROGRESS" || live {
        BarKind::Running
    } else if item.status == "BLOCKED" {
        BarKind::Blocked
    } else {
        BarKind::Planned
    };
    let time = bar.and_then(|bar| match kind {
        BarKind::Done => Some(format!("{}m", bar.end - bar.start)),
        BarKind::Running => Some(format!("{}m / {average}m", bar.end - bar.start)),
        BarKind::Planned => origin.map(|origin| format!("~{}", clock_text(origin + bar.start))),
        BarKind::Blocked => None,
    });
    let over = kind == BarKind::Running && bar.is_some_and(|bar| bar.end - bar.start > average);
    PlanTreeRow {
        id: item.id.clone(),
        title: task_title(&item.id, &item.title),
        group: item.group.clone(),
        kind,
        time,
        over,
    }
}

fn header_for(
    number: usize,
    rows: &[PlanTreeRow],
    wave_tasks: &[&OverviewTask],
    cyclic: bool,
    done_ids: &HashSet<&str>,
    in_plan: &HashSet<&str>,
) -> String {
    if cyclic {
        return "waits on a dependency cycle".into();
    }
    // A finished wave is history: its deps are done too, so "ready to start" would read as pending.
    if !rows.is_empty() && rows.iter().all(|row| row.kind == BarKind::Done) {
        return "done".into();
    }
    let running = rows.iter().any(|row| row.kind == BarKind::Running);
    if number == 1 {
        // A TASK with no group is a builder group of its own.
        let groups: HashSet<(bool, &str)> = rows
            .iter()
            .map(|row| {
                row.group
                    .as_deref()
                    .map_or((false, row.id.as_str()), |group| (true, group))
            })
            .collect();
        let count = groups.len();
        let parallel = format!(
            "{count} {} in parallel",
            if count == 1 { "group" } else { "groups" }
        );
        return if running {
            format!("running, {parallel}")
        } else {
            parallel
        };
    }
    let mut pending: Vec<&str> = Vec::new();
    for item in wave_tasks {
        for dep in &item.dep_ids {
            let dep = dep.as_str();
            if in_plan.contains(dep) && !done_ids.contains(dep) && !pending.contains(&dep) {
                pending.push(dep);
            }
        }
    }
    if pending.is_empty() {
        return if running { "running" } else { "ready to start" }.into();
    }
    let verb = if pending.len() == 1 {
        "finishes"
    } else {
        "finish"
    };
    format!("starts when {} {verb}", names(&pending, "and"))
}

/// Waves are dependency levels of the Tracker. `started` is the run's `Started:` text, which
/// gives planned rows and the finish estimate a clock. `live` holds the TASK IDs a builder agent
/// works on now: a rerun of a Blocked TASK runs before its file says so.
pub(crate) fn build_plan_tree(
    input: TimelineInput<'_>,
    started: Option<&str>,
    live: &BTreeSet<String>,
) -> PlanTree {
    build_plan_tree_and_timeline(input, started, live).0
}

/// `build_plan_tree` and `build_timeline` of the same input. With unique TASK IDs the tree reads
/// that one timeline; a repeated ID builds the tree from the first of each instead.
pub(crate) fn build_plan_tree_and_timeline(
    input: TimelineInput<'_>,
    started: Option<&str>,
    live: &BTreeSet<String>,
) -> (PlanTree, Timeline) {
    let mut seen = HashSet::default();
    if input.tasks.iter().all(|item| seen.insert(item.id.as_str())) {
        let timeline = build_timeline(input);
        let tree = tree_from(input.tasks, &timeline, started, live);
        return (tree, timeline);
    }
    let mut seen = HashSet::default();
    let tasks: Vec<OverviewTask> = input
        .tasks
        .iter()
        .filter(|item| seen.insert(item.id.as_str()))
        .cloned()
        .collect();
    let tree_timeline = build_timeline(TimelineInput {
        tasks: &tasks,
        ..input
    });
    let tree = tree_from(&tasks, &tree_timeline, started, live);
    (tree, build_timeline(input))
}

/// The tree of unique `tasks` and their timeline.
fn tree_from(
    tasks: &[OverviewTask],
    timeline: &Timeline,
    started: Option<&str>,
    live: &BTreeSet<String>,
) -> PlanTree {
    let in_plan: HashSet<&str> = tasks.iter().map(|item| item.id.as_str()).collect();
    let bars: HashMap<&str, &TimelineBar> = timeline
        .bars
        .iter()
        .map(|bar| (bar.id.as_str(), bar))
        .collect();
    let origin = started_at(started).map(|at| at.minutes);
    let average = timeline.stats.average_minutes;

    let levels = assign_waves(tasks);
    let last_level = levels.iter().flatten().copied().max().unwrap_or(0);
    let mut buckets: Vec<(Vec<&OverviewTask>, bool)> =
        (0..last_level).map(|_| (Vec::new(), false)).collect();
    // Tracker order inside a wave comes from walking the tasks in order.
    let mut cycle_tasks = Vec::new();
    for (item, level) in tasks.iter().zip(&levels) {
        match level {
            Some(level) => buckets[level - 1].0.push(item),
            None => cycle_tasks.push(item),
        }
    }
    if !cycle_tasks.is_empty() {
        buckets.push((cycle_tasks, true));
    }

    let done_ids: HashSet<&str> = tasks
        .iter()
        .filter(|item| item.status == "DONE")
        .map(|item| item.id.as_str())
        .collect();
    let waves = buckets
        .iter()
        .enumerate()
        .map(|(index, (wave_tasks, cyclic))| {
            let rows: Vec<PlanTreeRow> = wave_tasks
                .iter()
                .map(|item| {
                    row_for(
                        item,
                        bars.get(item.id.as_str()).copied(),
                        average,
                        origin,
                        live.contains(&item.id),
                    )
                })
                .collect();
            PlanTreeWave {
                number: index + 1,
                header: header_for(index + 1, &rows, wave_tasks, *cyclic, &done_ids, &in_plan),
                done: rows.iter().all(|row| row.kind == BarKind::Done),
                rows,
                cyclic: *cyclic,
            }
        })
        .collect();

    PlanTree {
        waves,
        pace: PlanTreePace {
            average_minutes: average,
            finish_clock: origin
                .zip(timeline.finish_minute)
                .map(|(origin, finish)| clock_text(origin + finish)),
            max_parallel: timeline.stats.max_parallel,
            slowest: timeline.stats.over_average.clone(),
        },
    }
}

/// A whole word of 7 to 40 lowercase hex characters with a digit. Words are runs of ASCII
/// letters, digits, and `_`, the `\b` of a JS regex without the `u` flag.
fn is_sha(word: &str) -> bool {
    (7..=40).contains(&word.len())
        && word.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
        && word.bytes().any(|b| b.is_ascii_digit())
}

/// The short commit hash each TASK's log row names; rows with no hash are left out. A hash is
/// the first word that `/\b(?=[0-9a-f]*\d)[0-9a-f]{7,40}\b/` matches whole. The words are split
/// by hand: in a debug build the regex scan took a tenth of the overview's derived data time.
pub(crate) fn commit_shas(log: &[OverviewRunEntry]) -> BTreeMap<String, String> {
    let first = |text: &str| {
        text.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
            .find(|word| is_sha(word))
            .map(|word| word[..7].to_string())
    };
    let mut shas = BTreeMap::new();
    for entry in log {
        if let Some(sha) = first(&entry.detail).or_else(|| first(&entry.result)) {
            shas.insert(entry.task.clone(), sha);
        }
    }
    shas
}

/// `12s ago`, `4m ago`, or `2h ago`; `None` for an unreadable time.
pub(crate) fn ago(ms: Option<i64>) -> Option<String> {
    let seconds = round_half_up(ms? as f64 / 1000.);
    if seconds < 60 {
        return Some(format!("{seconds}s ago"));
    }
    let minutes = round_half_up(seconds as f64 / 60.);
    Some(if minutes < 60 {
        format!("{minutes}m ago")
    } else {
        format!("{}h ago", round_half_up(minutes as f64 / 60.))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::dto::{OverviewQuestion, OverviewRun};
    use crate::model::timeline::tests::{row, run, task};

    const STARTED: &str = "2026-10-14 10:00";

    fn grouped(id: &str, status: &str, deps: &[&str], group: Option<&str>) -> OverviewTask {
        OverviewTask {
            group: group.map(Into::into),
            ..task(id, status, deps)
        }
    }

    fn tree(tasks: &[OverviewTask], run: Option<&OverviewRun>, started: Option<&str>) -> PlanTree {
        tree_live(tasks, run, started, &[])
    }

    fn tree_live(
        tasks: &[OverviewTask],
        run: Option<&OverviewRun>,
        started: Option<&str>,
        live: &[&str],
    ) -> PlanTree {
        let questions: [OverviewQuestion; 0] = [];
        let live: BTreeSet<String> = live.iter().map(|id| id.to_string()).collect();
        build_plan_tree(
            TimelineInput {
                tasks,
                run,
                questions: &questions,
            },
            started,
            &live,
        )
    }

    fn ids(tree: &PlanTree) -> Vec<Vec<&str>> {
        tree.waves
            .iter()
            .map(|wave| wave.rows.iter().map(|row| row.id.as_str()).collect())
            .collect()
    }

    // The Fleet map plan: 090 waits on 086..089, 091 waits on 090.
    fn fleet_map() -> Vec<OverviewTask> {
        vec![
            grouped("TASK-086", "TODO", &[], Some("A")),
            grouped("TASK-087", "TODO", &[], Some("B")),
            grouped("TASK-088", "TODO", &[], Some("C")),
            grouped("TASK-089", "TODO", &[], Some("D")),
            grouped(
                "TASK-090",
                "TODO",
                &["TASK-086", "TASK-087", "TASK-088", "TASK-089"],
                Some("E"),
            ),
            grouped("TASK-091", "TODO", &["TASK-090"], Some("F")),
        ]
    }

    // plan-tree.test.ts
    #[test]
    fn the_fleet_map_plan_gives_three_waves_of_4_1_and_1_rows() {
        let tree = tree(&fleet_map(), None, Some(STARTED));
        assert_eq!(
            ids(&tree),
            [
                vec!["TASK-086", "TASK-087", "TASK-088", "TASK-089"],
                vec!["TASK-090"],
                vec!["TASK-091"]
            ]
        );
        let numbers: Vec<usize> = tree.waves.iter().map(|wave| wave.number).collect();
        assert_eq!(numbers, [1, 2, 3]);
        assert!(!tree.waves.iter().any(|wave| wave.cyclic));
    }

    #[test]
    fn wave_headers_name_the_groups_in_parallel_and_the_deps_still_to_finish() {
        let tree = tree(&fleet_map(), None, Some(STARTED));
        assert_eq!(tree.waves[0].header, "4 groups in parallel");
        assert_eq!(
            tree.waves[1].header,
            "starts when TASK-086, TASK-087, TASK-088 and TASK-089 finish"
        );
        assert_eq!(tree.waves[2].header, "starts when TASK-090 finishes");
    }

    #[test]
    fn two_tasks_of_one_group_count_as_one_group_and_a_task_with_no_group_counts_alone() {
        let tasks = [
            grouped("TASK-001", "TODO", &[], Some("A")),
            grouped("TASK-002", "TODO", &[], Some("A")),
            grouped("TASK-003", "TODO", &[], None),
            grouped("TASK-004", "TODO", &[], None),
        ];
        assert_eq!(
            tree(&tasks, None, Some(STARTED)).waves[0].header,
            "3 groups in parallel"
        );
        let single = tree(&[task("TASK-001", "TODO", &[])], None, Some(STARTED));
        assert_eq!(single.waves[0].header, "1 group in parallel");
    }

    #[test]
    fn a_later_wave_names_only_the_deps_that_are_not_done() {
        let tasks = [
            task("TASK-001", "DONE", &[]),
            task("TASK-002", "TODO", &[]),
            task("TASK-003", "TODO", &["TASK-001", "TASK-002"]),
        ];
        assert_eq!(
            tree(&tasks, None, Some(STARTED)).waves[1].header,
            "starts when TASK-002 finishes"
        );
        let ready = [
            task("TASK-001", "DONE", &[]),
            task("TASK-003", "TODO", &["TASK-001"]),
        ];
        assert_eq!(
            tree(&ready, None, Some(STARTED)).waves[1].header,
            "ready to start"
        );
    }

    #[test]
    fn a_row_title_drops_the_leading_task_id_as_the_task_list_does() {
        let item = OverviewTask {
            title: "TASK-095 Plan tree model".into(),
            ..task("TASK-095", "DONE", &[])
        };
        assert_eq!(
            tree(&[item], None, Some(STARTED)).waves[0].rows[0].title,
            "Plan tree model"
        );
    }

    #[test]
    fn a_wave_with_a_running_task_says_running() {
        let tasks = [
            task("TASK-001", "IN PROGRESS", &[]),
            task("TASK-002", "DONE", &[]),
            task("TASK-003", "IN PROGRESS", &["TASK-002"]),
        ];
        let tree = tree(&tasks, None, Some(STARTED));
        assert!(tree.waves[0].header.starts_with("running, "));
        assert!(tree.waves[0].header.ends_with(" in parallel"));
        assert_eq!(tree.waves[1].header, "running");
    }

    #[test]
    fn a_blocked_or_todo_task_with_a_live_builder_runs_and_a_done_task_stays_done() {
        let tasks = [
            grouped("TASK-001", "BLOCKED", &[], Some("E")),
            grouped("TASK-002", "TODO", &[], Some("F")),
            grouped("TASK-003", "DONE", &[], Some("E")),
            grouped("TASK-004", "BLOCKED", &[], Some("G")),
        ];
        let tree = tree_live(
            &tasks,
            None,
            Some(STARTED),
            &["TASK-001", "TASK-002", "TASK-003"],
        );
        let kinds: Vec<BarKind> = tree.waves[0].rows.iter().map(|row| row.kind).collect();
        assert_eq!(
            kinds,
            [
                BarKind::Running,
                BarKind::Running,
                BarKind::Done,
                BarKind::Blocked
            ]
        );
        assert!(tree.waves[0].header.starts_with("running, "));
    }

    #[test]
    fn a_finished_wave_says_done_instead_of_a_start_condition() {
        let tasks = [
            task("TASK-001", "DONE", &[]),
            task("TASK-002", "DONE", &["TASK-001"]),
        ];
        let tree = tree(&tasks, None, Some(STARTED));
        assert_eq!(tree.waves[0].header, "done");
        assert_eq!(tree.waves[1].header, "done");
    }

    #[test]
    fn a_dep_outside_the_plan_does_not_count() {
        let tasks = [
            task("TASK-010", "TODO", &["TASK-999"]),
            task("TASK-011", "TODO", &["TASK-010", "TASK-999"]),
        ];
        let tree = tree(&tasks, None, Some(STARTED));
        assert_eq!(ids(&tree), [vec!["TASK-010"], vec!["TASK-011"]]);
        assert_eq!(tree.waves[1].header, "starts when TASK-010 finishes");
    }

    #[test]
    fn a_task_sits_one_wave_after_its_highest_dep_and_keeps_tracker_order_inside_a_wave() {
        let tasks = [
            task("TASK-004", "TODO", &["TASK-002", "TASK-001"]),
            task("TASK-003", "TODO", &[]),
            task("TASK-002", "TODO", &["TASK-001"]),
            task("TASK-001", "TODO", &[]),
        ];
        assert_eq!(
            ids(&tree(&tasks, None, Some(STARTED))),
            [
                vec!["TASK-003", "TASK-001"],
                vec!["TASK-002"],
                vec!["TASK-004"]
            ]
        );
    }

    #[test]
    fn a_cycle_ends_in_one_last_wave_marked_cyclic_and_does_not_panic() {
        let tasks = [
            task("TASK-001", "TODO", &[]),
            task("TASK-002", "TODO", &["TASK-001", "TASK-003"]),
            task("TASK-003", "TODO", &["TASK-002"]),
            task("TASK-004", "TODO", &["TASK-003"]),
            task("TASK-005", "TODO", &["TASK-005"]),
        ];
        let tree = tree(&tasks, None, Some(STARTED));
        assert_eq!(tree.waves.len(), 2);
        assert!(!tree.waves[0].cyclic);
        assert_eq!(
            ids(&tree)[1],
            ["TASK-002", "TASK-003", "TASK-004", "TASK-005"]
        );
        assert!(tree.waves[1].cyclic);
        assert_eq!(tree.waves[1].number, 2);
        assert!(!tree.waves[1].done);
        assert_eq!(tree.waves[1].header, "waits on a dependency cycle");
    }

    #[test]
    fn done_running_planned_and_blocked_rows_get_their_time_text() {
        let tasks = [
            task("TASK-001", "DONE", &[]),
            task("TASK-002", "DONE", &[]),
            task("TASK-003", "IN PROGRESS", &[]),
            task("TASK-004", "TODO", &["TASK-003"]),
            task("TASK-005", "BLOCKED", &[]),
            task("TASK-006", "DONE", &[]),
        ];
        let run = run(
            STARTED,
            vec![row("TASK-001", 0, Some(4)), row("TASK-002", 5, Some(9))],
            17,
        );
        let tree = tree(&tasks, Some(&run), Some(STARTED));
        let view = |id: &str| {
            let row = tree
                .waves
                .iter()
                .flat_map(|wave| &wave.rows)
                .find(|row| row.id == id)
                .unwrap();
            (row.kind, row.time.clone(), row.over)
        };
        assert_eq!(view("TASK-001"), (BarKind::Done, Some("4m".into()), false));
        // Started at minute 9, now 17: 8 minutes against the average of 4.
        assert_eq!(
            view("TASK-003"),
            (BarKind::Running, Some("8m / 4m".into()), true)
        );
        // Expected start is minute 17 after 10:00.
        assert_eq!(
            view("TASK-004"),
            (BarKind::Planned, Some("~10:17".into()), false)
        );
        assert_eq!(view("TASK-005"), (BarKind::Blocked, None, false));
        // Done in the Tracker but with no log row: no data, no time.
        assert_eq!(view("TASK-006"), (BarKind::Done, None, false));
    }

    #[test]
    fn a_running_row_under_the_average_is_not_over() {
        let tasks = [
            task("TASK-001", "DONE", &[]),
            task("TASK-002", "IN PROGRESS", &[]),
        ];
        let run = run(STARTED, vec![row("TASK-001", 0, Some(4))], 6);
        let tree = tree(&tasks, Some(&run), Some(STARTED));
        let item = &tree.waves[0].rows[1];
        assert_eq!((item.time.as_deref(), item.over), (Some("2m / 4m"), false));
    }

    #[test]
    fn a_planned_row_has_no_time_when_started_has_no_clock() {
        let tasks = [task("TASK-001", "TODO", &[])];
        let run = run(STARTED, vec![], 0);
        assert_eq!(tree(&tasks, Some(&run), None).waves[0].rows[0].time, None);
        assert_eq!(
            tree(&tasks, Some(&run), Some("2026-10-14")).waves[0].rows[0].time,
            Some("~00:00".into())
        );
    }

    #[test]
    fn a_wave_is_done_when_every_row_is_done() {
        let tasks = [
            task("TASK-001", "DONE", &[]),
            task("TASK-002", "DONE", &[]),
            task("TASK-003", "DONE", &["TASK-001"]),
            task("TASK-004", "TODO", &["TASK-003"]),
        ];
        let done: Vec<bool> = tree(&tasks, None, Some(STARTED))
            .waves
            .iter()
            .map(|wave| wave.done)
            .collect();
        assert_eq!(done, [true, true, false]);
    }

    #[test]
    fn an_empty_plan_has_no_waves() {
        let tree = tree(&[], None, Some(STARTED));
        assert!(tree.waves.is_empty());
        assert_eq!(
            tree.pace,
            PlanTreePace {
                average_minutes: 20,
                finish_clock: None,
                max_parallel: 0,
                slowest: None,
            }
        );
    }

    #[test]
    fn the_pace_line_matches_the_timeline_stats_for_the_same_input() {
        let tasks = [
            task("TASK-001", "DONE", &[]),
            task("TASK-002", "DONE", &[]),
            task("TASK-003", "IN PROGRESS", &[]),
            task("TASK-004", "TODO", &["TASK-003"]),
            task("TASK-005", "BLOCKED", &[]),
        ];
        let run = run(
            STARTED,
            vec![row("TASK-001", 0, Some(4)), row("TASK-002", 5, Some(9))],
            17,
        );
        let timeline = build_timeline(TimelineInput {
            tasks: &tasks,
            run: Some(&run),
            questions: &[],
        });
        let pace = tree(&tasks, Some(&run), Some(STARTED)).pace;
        assert_eq!(pace.average_minutes, timeline.stats.average_minutes);
        assert_eq!(pace.max_parallel, timeline.stats.max_parallel);
        assert_eq!(pace.slowest, timeline.stats.over_average);
        assert_eq!(
            pace.slowest,
            Some(TaskMinutes {
                id: "TASK-003".into(),
                minutes: 4
            })
        );
        // The last open bar ends at minute 25: 10:25.
        assert_eq!(timeline.finish_minute, Some(25));
        assert_eq!(pace.finish_clock.as_deref(), Some("10:25"));
    }

    #[test]
    fn the_finish_clock_wraps_past_midnight() {
        let tasks = [task("TASK-001", "TODO", &[])];
        let run = run(STARTED, vec![], 0);
        assert_eq!(
            tree(&tasks, Some(&run), Some("2026-10-14 23:50"))
                .pace
                .finish_clock
                .as_deref(),
            Some("00:10")
        );
    }

    #[test]
    fn two_hundred_tasks_with_chained_deps_build_in_under_20_ms() {
        let tasks: Vec<OverviewTask> = (0..200)
            .map(|index| {
                let id = format!("TASK-{:03}", index + 1);
                let dep = format!("TASK-{index:03}");
                let deps: Vec<&str> = if index == 0 {
                    vec![]
                } else {
                    vec![dep.as_str()]
                };
                task(&id, if index < 100 { "DONE" } else { "TODO" }, &deps)
            })
            .collect();
        let log = tasks[..100]
            .iter()
            .enumerate()
            .map(|(index, item)| row(&item.id, index as i64 * 5, Some(index as i64 * 5 + 4)))
            .collect();
        let run = run(STARTED, log, 500);
        tree(&tasks, Some(&run), Some(STARTED));
        let begin = std::time::Instant::now();
        let built = tree(&tasks, Some(&run), Some(STARTED));
        let elapsed = begin.elapsed().as_secs_f64() * 1000.;
        assert_eq!(built.waves.len(), 200);
        assert!(elapsed < 20., "built in {elapsed:.1} ms");
    }

    // plan-tree-view.tsx `commitShas` and `ago`
    #[test]
    fn commit_shas_find_the_first_hex_word_with_a_digit_and_keep_7_chars() {
        let entry = |task: &str, detail: &str, result: &str| OverviewRunEntry {
            task: task.into(),
            detail: detail.into(),
            result: result.into(),
            ..OverviewRunEntry::default()
        };
        let shas = commit_shas(&[
            entry("TASK-001", "committed 5572af32b on main", "DONE"),
            entry(
                "TASK-002",
                "deadbeefcafe has no digit, then abc1234",
                "DONE",
            ),
            entry("TASK-003", "#65 PR, no hash", "DONE"),
            entry(
                "TASK-004",
                "x1234567 and 1234567g are not words of hex",
                "DONE",
            ),
            entry("TASK-005", "", "DONE a6e4620b8"),
            entry("TASK-006", &"a1".repeat(21), "DONE"),
        ]);
        assert_eq!(shas.get("TASK-001").map(String::as_str), Some("5572af3"));
        assert_eq!(shas.get("TASK-002").map(String::as_str), Some("abc1234"));
        assert_eq!(shas.get("TASK-003"), None);
        assert_eq!(shas.get("TASK-004"), None);
        assert_eq!(shas.get("TASK-005").map(String::as_str), Some("a6e4620"));
        // 42 hex characters is longer than a hash.
        assert_eq!(shas.get("TASK-006"), None);
    }

    #[test]
    fn ago_rounds_to_seconds_minutes_or_hours() {
        assert_eq!(ago(None), None);
        assert_eq!(ago(Some(0)), Some("0s ago".into()));
        assert_eq!(ago(Some(59_400)), Some("59s ago".into()));
        assert_eq!(ago(Some(59_500)), Some("1m ago".into()));
        assert_eq!(ago(Some(10 * 60_000)), Some("10m ago".into()));
        assert_eq!(ago(Some(90 * 60_000)), Some("2h ago".into()));
    }
}
