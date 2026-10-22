//! The derived data and the text rules of the overview: `client/overview.tsx`, `needs-you.tsx`,
//! `plan-tree-view.tsx`, `overview-cells.tsx`, and `run-cells.tsx`. The views only draw them.

use super::id_map::IdMap;
use super::{
    AgentTarget, BarKind, BuilderState, Deadline, Decision, GroupCounts, History, Loaded,
    NeedsYouKind, NeedsYouRow, NeedsYouTone, Overview, OverviewRun, OverviewTask, Phase, PlanTree,
    PlanTreePace, PlanTreeRow, RunAgent, TaskBuilder, TaskGroup, TimelineInput, ago,
    build_plan_tree, build_plan_tree_and_timeline, build_timeline, clock_at, commit_shas,
    dashboard_summary, is_decision_id, lifecycle_phases, needs_you, task_group, with_live_status,
};
use crate::cell::LIST_LIMIT;
use std::collections::{BTreeMap, BTreeSet, HashMap};

/// From `STACK` up, the page has the rail on the left; below it everything stacks.
pub(crate) const STACK: f32 = 720.;
pub(crate) const RAIL_WIDTH: f32 = 360.;
/// Two columns under the tree from this main width.
pub(crate) const WIDE_GRID: f32 = 900.;
/// Below this main width the tree drops its spine and the cells stack.
pub(crate) const NARROW: f32 = 520.;
/// Below this main width the five lifecycle steps show names only.
pub(crate) const COMPACT_STEPPER: f32 = 720.;

/// How the cells under the plan tree sit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Grid {
    /// Needs you beside a column of checks, decisions, and archived plans.
    Wide,
    /// Two rows of two cells.
    Middle,
    /// One cell per line.
    Narrow,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Layout {
    pub stacked: bool,
    /// The width of the timeline side.
    pub main_width: f32,
    pub grid: Grid,
    pub compact_stepper: bool,
}

impl Layout {
    /// The main column padding: 12 stacked, 24 otherwise.
    pub(crate) fn padding(&self) -> f32 {
        if self.stacked { 12. } else { 24. }
    }

    pub(crate) fn narrow(&self) -> bool {
        self.grid == Grid::Narrow
    }
}

/// `overview.tsx:118-122`. A width of 0 (not measured yet) gives the rail layout.
pub(crate) fn layout_for(width: f32) -> Layout {
    let stacked = width > 0. && width < STACK;
    let main_width = if stacked { width } else { width - RAIL_WIDTH };
    let narrow = main_width > 0. && main_width < NARROW;
    let grid = if main_width >= WIDE_GRID {
        Grid::Wide
    } else if narrow {
        Grid::Narrow
    } else {
        Grid::Middle
    };
    Layout {
        stacked,
        main_width,
        grid,
        compact_stepper: main_width > 0. && main_width < COMPACT_STEPPER,
    }
}

/// A JS-truthy string field.
fn truthy(value: &Option<String>) -> Option<&str> {
    value.as_deref().filter(|value| !value.is_empty())
}

/// The `aside` of the Plan cell, and the lead row's line when no lead agent is found.
pub(crate) fn run_state_text(run: Option<&OverviewRun>) -> String {
    let Some(run) = run else {
        return "No autorun yet".into();
    };
    let started = run.started.as_deref();
    let stopped_at = run
        .stopped
        .then(|| clock_at(started, run.now_minute.unwrap_or(0) as f64))
        .flatten();
    if let Some(at) = stopped_at {
        format!("Run stopped at {at}")
    } else if truthy(&run.finished).is_some() {
        "Run finished".into()
    } else if let Some(since) = clock_at(started, 0.) {
        format!("Autorun since {since}")
    } else {
        "Autorun running".into()
    }
}

/// Needs you rows with ", answer by HH:MM" from the timeline deadlines. A stopped run has no
/// real "now", so its answer-by times would be in the past.
pub(crate) fn needs_you_rows(overview: &Overview, deadlines: &[Deadline]) -> Vec<NeedsYouRow> {
    let run = overview.run.as_ref();
    let started = run.and_then(|run| run.started.as_deref());
    let by_question: HashMap<&str, i64> = if run.is_some_and(|run| run.stopped) {
        HashMap::new()
    } else {
        deadlines
            .iter()
            .map(|entry| (entry.question_id.as_str(), entry.minute))
            .collect()
    };
    needs_you(overview)
        .into_iter()
        .map(|mut row| {
            if let Some(clock) = by_question
                .get(row.id.as_str())
                .and_then(|minute| clock_at(started, *minute as f64))
            {
                row.meta = format!("{}, answer by {clock}", row.meta);
            }
            row
        })
        .collect()
}

/// One number of the pace line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PaceItem {
    pub value: String,
    pub label: &'static str,
}

/// `PaceLine`: a number with no data is left out. A stopped run hides the estimated finish.
pub(crate) fn pace_items(pace: &PlanTreePace, has_average: bool, stopped: bool) -> Vec<PaceItem> {
    let mut items = Vec::new();
    if has_average {
        items.push(PaceItem {
            value: format!("{}m", pace.average_minutes),
            label: "average task",
        });
    }
    if let Some(clock) = pace.finish_clock.as_ref().filter(|_| !stopped) {
        items.push(PaceItem {
            value: format!("~{clock}"),
            label: "estimated finish",
        });
    }
    if pace.max_parallel > 0 {
        items.push(PaceItem {
            value: pace.max_parallel.to_string(),
            label: "builders at once",
        });
    }
    if let Some(slowest) = &pace.slowest {
        items.push(PaceItem {
            value: format!("{} +{}m", slowest.id, slowest.minutes),
            label: "slowest over average",
        });
    }
    items
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RejectPress {
    Reject,
    Cancel,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RejectStep {
    /// Show "Confirm reject" and "Cancel".
    Confirm,
    /// Send the reject prompt.
    Send,
    /// Show "Reject" again.
    Back,
}

/// The two-step ADR reject of `needs-you.tsx:216-232`.
pub(crate) fn reject_step(confirming: bool, press: RejectPress) -> RejectStep {
    match (confirming, press) {
        (false, RejectPress::Reject) => RejectStep::Confirm,
        (true, RejectPress::Reject) => RejectStep::Send,
        (_, RejectPress::Cancel) => RejectStep::Back,
    }
}

/// A button that sends: disabled with no agent, a busy agent, or a send in flight.
pub(crate) fn actions_disabled(target: &AgentTarget, sending: bool) -> bool {
    sending || !matches!(target, AgentTarget::Ready { .. })
}

/// The answer form's Send also waits for text.
pub(crate) fn send_disabled(target: &AgentTarget, sending: bool, draft: &str) -> bool {
    actions_disabled(target, sending) || draft.trim().is_empty()
}

/// What a row's buttons act on, for their accessibility labels.
pub(crate) fn action_subject(row: &NeedsYouRow) -> &str {
    if row.kind == NeedsYouKind::Checks {
        "manual checks"
    } else {
        &row.id
    }
}

/// The note above the Needs you rows, and whether it warns.
pub(crate) fn agent_note(target: &AgentTarget) -> (String, bool) {
    match target {
        AgentTarget::NoAgent => ("No agent in this workspace".into(), true),
        AgentTarget::Busy => ("Agent busy".into(), true),
        AgentTarget::Ready { name, .. } => (format!("Actions go to {name}"), false),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RowState {
    Done,
    Running,
    Waiting,
    Planned,
    Blocked,
}

impl RowState {
    pub(crate) fn label(self) -> &'static str {
        match self {
            RowState::Done => "done",
            RowState::Running => "running",
            RowState::Waiting => "waiting",
            RowState::Planned => "planned",
            RowState::Blocked => "blocked",
        }
    }
}

/// A running row whose builder waits for a permission shows a hand, not a spinner.
pub(crate) fn row_state(row: &PlanTreeRow, builder: Option<&TaskBuilder>) -> RowState {
    match row.kind {
        BarKind::Running
            if builder.is_some_and(|builder| builder.state == BuilderState::Waiting) =>
        {
            RowState::Waiting
        }
        BarKind::Running => RowState::Running,
        BarKind::Done => RowState::Done,
        BarKind::Planned => RowState::Planned,
        BarKind::Blocked => RowState::Blocked,
    }
}

/// The plan tree row's second line (`plan-tree-view.tsx:146-164`).
pub(crate) fn secondary_line(
    row: &PlanTreeRow,
    state: RowState,
    builder: Option<&TaskBuilder>,
    task_class: Option<&str>,
    sha: Option<&str>,
) -> String {
    let group = row.group.as_deref().filter(|group| !group.is_empty());
    let letter = group.map_or_else(|| "Builder".to_string(), |group| format!("Builder {group}"));
    let plain = [
        group.map(|group| format!("Group {group}")),
        task_class.map(str::to_string),
    ]
    .into_iter()
    .flatten()
    .filter(|part| !part.is_empty())
    .collect::<Vec<_>>()
    .join(", ");
    let line = match (state, builder) {
        (RowState::Done, _) => sha.map_or(plain, |sha| format!("committed {sha}")),
        (RowState::Waiting, _) => format!("{letter} asks permission"),
        (RowState::Running, Some(builder)) => {
            let word = match builder.state {
                BuilderState::Failed => Some("failed"),
                BuilderState::Idle => Some("idle"),
                _ => None,
            };
            let last = ago(builder.active_ago).map(|last| format!("active {last}"));
            [Some(letter), word.map(str::to_string), last]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
                .join(", ")
        }
        (RowState::Blocked, _) if plain.is_empty() => "Blocked".into(),
        (RowState::Blocked, _) => format!("Blocked, {plain}"),
        _ => plain,
    };
    let run = builder.map(run_part).unwrap_or_default();
    match (line.is_empty(), run.is_empty()) {
        (_, true) => line,
        (true, false) => run,
        (false, false) => format!("{line} · {run}"),
    }
}

/// `provider, model, effort` of a builder; blank or missing parts are skipped.
fn run_part(builder: &TaskBuilder) -> String {
    [
        Some(builder.provider.as_str()),
        builder.model.as_deref(),
        builder.thinking.as_deref(),
    ]
    .into_iter()
    .flatten()
    .map(str::trim)
    .filter(|part| !part.is_empty())
    .collect::<Vec<_>>()
    .join(", ")
}

/// The lead row's title and line (`plan-tree-view.tsx:313-338`, Q-036 default).
pub(crate) fn lead_text(
    lead: Option<&RunAgent>,
    builder_count: usize,
    plan_title: &str,
    run_state: &str,
) -> (String, String) {
    let Some(lead) = lead else {
        return (plan_title.to_string(), run_state.to_string());
    };
    let state = match lead.state {
        BuilderState::Waiting => "asks permission",
        BuilderState::Running => "running",
        BuilderState::Failed => "failed",
        BuilderState::Idle => "idle",
    };
    let builders = if builder_count > 0 {
        format!(
            "waiting on {builder_count} {}",
            if builder_count == 1 {
                "builder"
            } else {
                "builders"
            }
        )
    } else {
        "no builder running".into()
    };
    let line = [
        lead.title.clone().filter(|title| !title.is_empty()),
        Some(state.to_string()),
        ago(lead.active_ago).map(|last| format!("active {last}")),
        Some(builders),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(", ");
    ("Lead agent, implement".into(), line)
}

/// The distinct builders that run or wait now, for "waiting on N builders".
pub(crate) fn active_builders(by_task: &BTreeMap<String, TaskBuilder>) -> usize {
    by_task
        .values()
        .filter(|builder| matches!(builder.state, BuilderState::Running | BuilderState::Waiting))
        .map(|builder| builder.id.as_str())
        .collect::<BTreeSet<_>>()
        .len()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DecisionRow {
    pub id: String,
    pub title: String,
    pub meta: String,
    /// The icon takes the warning color.
    pub proposed: bool,
    /// Only a real ADR ID opens the decision modal.
    pub pressable: bool,
}

/// `DecisionsCell`: newest ID first. JS `localeCompare` and a byte compare agree on ADR IDs.
pub(crate) fn decision_rows(decisions: &[Decision]) -> Vec<DecisionRow> {
    let mut ordered: Vec<&Decision> = decisions.iter().collect();
    ordered.sort_by(|a, b| b.id.cmp(&a.id));
    ordered
        .into_iter()
        .map(|decision| DecisionRow {
            id: decision.id.clone(),
            title: decision.title.clone(),
            meta: [&decision.id, &decision.status, &decision.date]
                .into_iter()
                .filter(|part| !part.is_empty())
                .cloned()
                .collect::<Vec<_>>()
                .join(", "),
            proposed: decision.status.to_lowercase() == "proposed",
            pressable: is_decision_id(&decision.id),
        })
        .collect()
}

/// `ArchivedPlansCell`'s meta line.
pub(crate) fn archived_meta(plan: &History) -> String {
    let when =
        truthy(&plan.date).map_or_else(|| plan.slug.clone(), |date| format!("Archived {date}"));
    [Some(when), plan.has_learn.then(|| "LEARN.md".to_string())]
        .into_iter()
        .flatten()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(", ")
}

/// The color of the autorun strip's dot and state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StripTone {
    Live,
    Stopped,
    Finished,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct StripModel {
    pub state: &'static str,
    pub tone: StripTone,
    pub items: Vec<(&'static str, String)>,
}

/// `AutorunStrip`; `None` without a run.
pub(crate) fn autorun_strip(run: Option<&OverviewRun>) -> Option<StripModel> {
    let run = run?;
    let finished = truthy(&run.finished);
    let state = if finished.is_some() {
        "Run finished"
    } else if run.stopped {
        "Run stopped"
    } else {
        "Autorun running"
    };
    let tone = if finished.is_none() && !run.stopped {
        StripTone::Live
    } else if run.stopped {
        StripTone::Stopped
    } else {
        StripTone::Finished
    };
    let started = truthy(&run.started);
    let last = run.log.iter().rev().find(|row| row.end_minute.is_some());
    let items = [
        truthy(&run.runner).map(|runner| ("runner", runner.to_string())),
        started.map(|started| ("started", started.to_string())),
        Some(("iterations", run.log.len().to_string())),
        last.map(|last| {
            let clock = clock_at(run.started.as_deref(), last.end_minute.unwrap_or(0) as f64);
            ("last activity", clock.unwrap_or_else(|| last.end.clone()))
        }),
        finished.map(|finished| ("finished", finished.to_string())),
    ]
    .into_iter()
    .flatten()
    .collect();
    Some(StripModel { state, tone, items })
}

/// The stepper's Implement segments.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct StatusCounts {
    pub done: usize,
    pub running: usize,
    pub blocked: usize,
    pub todo: usize,
}

impl From<GroupCounts> for StatusCounts {
    fn from(counts: GroupCounts) -> Self {
        Self {
            done: counts.done,
            running: counts.active,
            blocked: counts.blocked,
            todo: counts.todo,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SegmentKind {
    Done,
    Running,
    Blocked,
    Todo,
}

/// The segments with a count above zero, in order, with their flex weight.
pub(crate) fn segments(counts: StatusCounts) -> Vec<(SegmentKind, usize)> {
    [
        (SegmentKind::Done, counts.done),
        (SegmentKind::Running, counts.running),
        (SegmentKind::Blocked, counts.blocked),
        (SegmentKind::Todo, counts.todo),
    ]
    .into_iter()
    .filter(|(_, value)| *value > 0)
    .collect()
}

/// Everything the overview body draws, computed once per data change (`overview.tsx:134-174`).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Derived {
    pub tree: PlanTree,
    /// The Tracker class per TASK.
    pub classes: IdMap<String, String>,
    pub commits: BTreeMap<String, String>,
    pub rows: Vec<NeedsYouRow>,
    pub phases: Vec<Phase>,
    pub counts: StatusCounts,
    pub run_state: String,
    /// Empty without a run.
    pub pace: Vec<PaceItem>,
    pub strip: Option<StripModel>,
    pub decisions: Vec<DecisionRow>,
}

/// `dashboardSummary(withLiveStatus(tasks, live)).counts`, without copying the tasks.
fn live_counts(tasks: &[OverviewTask], live: &BTreeSet<String>) -> StatusCounts {
    let mut counts = GroupCounts::default();
    for task in tasks {
        let status = if live.contains(&task.id) && task.status != "DONE" {
            "IN PROGRESS"
        } else {
            task.status.as_str()
        };
        match task_group(status) {
            TaskGroup::Active => counts.active += 1,
            TaskGroup::Blocked => counts.blocked += 1,
            TaskGroup::Todo => counts.todo += 1,
            TaskGroup::Done => counts.done += 1,
            TaskGroup::Unknown => counts.unknown += 1,
        }
    }
    counts.into()
}

/// Without RUN.md there are no logged times, so any time text would be a guess from minute 0.
pub(crate) fn without_times(mut tree: PlanTree) -> PlanTree {
    for row in tree.waves.iter_mut().flat_map(|wave| wave.rows.iter_mut()) {
        row.time = None;
        row.over = false;
    }
    tree
}

pub(crate) fn derive_overview(overview: &Overview, live: &BTreeSet<String>) -> Derived {
    let input = TimelineInput {
        tasks: &overview.tasks,
        run: overview.run.as_ref(),
        questions: &overview.questions,
    };
    let run = overview.run.as_ref();
    let started = run.and_then(|run| run.started.as_deref());
    let (built, timeline) = build_plan_tree_and_timeline(input, started, live);
    let tree = if run.is_some() {
        built
    } else {
        without_times(built)
    };
    let has_average = timeline.bars.iter().any(|bar| bar.kind == BarKind::Done);
    let pace = run.map_or_else(Vec::new, |run| {
        pace_items(&tree.pace, has_average, run.stopped)
    });
    Derived {
        classes: overview
            .tasks
            .iter()
            .filter_map(|task| Some((task.id.clone(), task.task_class.clone()?)))
            .collect(),
        commits: commit_shas(run.map_or(&[][..], |run| &run.log)),
        rows: needs_you_rows(overview, &timeline.deadlines),
        phases: lifecycle_phases(overview),
        counts: live_counts(&overview.tasks, live),
        run_state: run_state_text(run),
        pace,
        strip: autorun_strip(run),
        decisions: decision_rows(&overview.decisions),
        tree,
    }
}

/// What the derived data reads: the store revision (overview and board), the live TASK set of
/// the builder match, and the minute.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DerivedKey {
    pub revision: u64,
    pub live: BTreeSet<String>,
    pub minute: i64,
}

/// Keeps the derived data until its key changes, so a redraw never rebuilds it.
#[derive(Default)]
pub(crate) struct DerivedCache {
    key: Option<DerivedKey>,
    value: Option<Derived>,
    builds: usize,
}

impl DerivedCache {
    /// Returns whether the value was rebuilt.
    pub(crate) fn update(
        &mut self,
        key: DerivedKey,
        build: impl FnOnce() -> Option<Derived>,
    ) -> bool {
        if self.key.as_ref() == Some(&key) {
            return false;
        }
        self.key = Some(key);
        self.value = build();
        self.builds += 1;
        true
    }

    pub(crate) fn get(&self) -> Option<&Derived> {
        self.value.as_ref()
    }

    pub(crate) fn clear(&mut self) {
        self.key = None;
        self.value = None;
    }

    #[cfg(test)]
    fn builds(&self) -> usize {
        self.builds
    }
}

/// A cell of the grid under the plan tree.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum CellKind {
    Needs,
    Checks,
    Decisions,
    Archived,
}

/// One list item of the overview body. A cell that spans the main column draws as slices, one
/// item each; cells side by side are one item per row group.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum BodyItem {
    Error(String),
    Skeleton,
    NoPlan(String),
    Stepper,
    Warning(usize),
    Strip,
    PlanTop,
    Wave(usize),
    TaskRow {
        wave: usize,
        row: usize,
        last: bool,
    },
    PlanBottom,
    /// Cells side by side; each side is a column of cells.
    Row(Vec<CellKind>, Vec<CellKind>),
    /// Three cells side by side, for the no-plan state.
    Row3,
    Cell(CellKind),
    NeedsTop,
    NeedsRow(usize),
    NeedsMore,
    NeedsBottom,
}

impl BodyItem {
    /// Whether the item starts a block, so it gets the 14 px gap above it.
    pub(crate) fn starts_block(&self) -> bool {
        !matches!(
            self,
            BodyItem::Wave(_)
                | BodyItem::TaskRow { .. }
                | BodyItem::PlanBottom
                | BodyItem::NeedsRow(_)
                | BodyItem::NeedsMore
                | BodyItem::NeedsBottom
        )
    }
}

/// The body items of `overview.tsx:278-481`. `needs_expanded` is the Needs you Show more state.
pub(crate) fn body_items(
    overview: &Loaded<Overview>,
    derived: Option<&Derived>,
    grid: Grid,
    needs_expanded: bool,
) -> Vec<BodyItem> {
    let data = match overview {
        Loaded::Error(error) => return vec![BodyItem::Error(error.clone())],
        Loaded::Loading => return vec![BodyItem::Skeleton],
        Loaded::Ready(data) => data,
    };
    let Some(derived) = derived else {
        return vec![BodyItem::Skeleton];
    };
    if data.tasks.is_empty()
        && let Some(message) = data.message.clone()
    {
        return vec![BodyItem::NoPlan(message), BodyItem::Row3];
    }
    let mut items = vec![BodyItem::Stepper];
    items.extend((0..data.warnings.len()).map(BodyItem::Warning));
    if derived.strip.is_some() {
        items.push(BodyItem::Strip);
    }
    items.push(BodyItem::PlanTop);
    for (wave_index, wave) in derived.tree.waves.iter().enumerate() {
        items.push(BodyItem::Wave(wave_index));
        items.extend((0..wave.rows.len()).map(|row| BodyItem::TaskRow {
            wave: wave_index,
            row,
            last: row + 1 == wave.rows.len(),
        }));
    }
    items.push(BodyItem::PlanBottom);
    match grid {
        Grid::Wide => items.push(BodyItem::Row(
            vec![CellKind::Needs],
            vec![CellKind::Checks, CellKind::Decisions, CellKind::Archived],
        )),
        Grid::Middle => {
            items.push(BodyItem::Row(vec![CellKind::Needs], vec![CellKind::Checks]));
            items.push(BodyItem::Row(
                vec![CellKind::Decisions],
                vec![CellKind::Archived],
            ));
        }
        Grid::Narrow => {
            let total = derived.rows.len();
            let shown = if needs_expanded {
                total
            } else {
                total.min(LIST_LIMIT)
            };
            items.push(BodyItem::NeedsTop);
            items.extend((0..shown).map(BodyItem::NeedsRow));
            if total > LIST_LIMIT {
                items.push(BodyItem::NeedsMore);
            }
            items.push(BodyItem::NeedsBottom);
            items.extend(
                [CellKind::Checks, CellKind::Decisions, CellKind::Archived].map(BodyItem::Cell),
            );
        }
    }
    items
}

/// The Needs you cell takes the warning tint when a row is in danger.
pub(crate) fn needs_warning(rows: &[NeedsYouRow]) -> bool {
    rows.iter().any(|row| row.tone == NeedsYouTone::Danger)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::timeline::tests::{row, run, task};
    use crate::model::{OverviewQuestion, TaskMinutes};
    use std::time::Instant;

    fn plan_row(kind: BarKind, group: Option<&str>) -> PlanTreeRow {
        PlanTreeRow {
            id: "TASK-7".into(),
            title: "Port the board".into(),
            group: group.map(str::to_string),
            kind,
            time: Some("4m".into()),
            over: false,
        }
    }

    fn builder(state: BuilderState, active_ago: Option<i64>) -> TaskBuilder {
        TaskBuilder {
            id: "agent-1".into(),
            title: "Builder".into(),
            state,
            active_ago,
            provider: "claude".into(),
            model: None,
            thinking: None,
        }
    }

    fn run_builder(
        state: BuilderState,
        model: Option<&str>,
        thinking: Option<&str>,
    ) -> TaskBuilder {
        TaskBuilder {
            model: model.map(str::to_string),
            thinking: thinking.map(str::to_string),
            ..builder(state, Some(5 * 60_000))
        }
    }

    fn question(id: &str, blocks: &[&str]) -> OverviewQuestion {
        OverviewQuestion {
            id: id.into(),
            blocks: blocks.iter().map(|id| id.to_string()).collect(),
            question: "Which one?".into(),
            default: "-".into(),
            status: "OPEN".into(),
            ..OverviewQuestion::default()
        }
    }

    /// TASK-1 done in 20 minutes, TASK-2 waits on it and on Q-1.
    fn planned_with_question(stopped: bool) -> Overview {
        let mut started = run("2026-10-17 09:30", vec![row("TASK-1", 0, Some(20))], 20);
        started.stopped = stopped;
        Overview {
            tasks: vec![
                task("TASK-1", "DONE", &[]),
                task("TASK-2", "TODO", &["TASK-1"]),
            ],
            run: Some(started),
            questions: vec![question("Q-1", &["TASK-2"])],
            ..Overview::default()
        }
    }

    #[test]
    fn run_state_text_follows_the_electron_order() {
        assert_eq!(run_state_text(None), "No autorun yet");
        let mut stopped = run("2026-10-17 14:00", vec![], 5);
        stopped.stopped = true;
        assert_eq!(run_state_text(Some(&stopped)), "Run stopped at 14:05");
        let mut finished = run("2026-10-17 14:00", vec![], 5);
        finished.finished = Some("2026-10-17 15:00".into());
        assert_eq!(run_state_text(Some(&finished)), "Run finished");
        let since = run("2026-10-17 09:30", vec![], 0);
        assert_eq!(run_state_text(Some(&since)), "Autorun since 09:30");
        let no_clock = OverviewRun::default();
        assert_eq!(run_state_text(Some(&no_clock)), "Autorun running");
    }

    #[test]
    fn layout_for_keeps_the_electron_breakpoints() {
        let stacked = layout_for(700.);
        assert!(stacked.stacked);
        assert_eq!(stacked.padding(), 12.);
        assert_eq!(stacked.grid, Grid::Middle);
        assert!(stacked.compact_stepper);
        let middle = layout_for(1100.);
        assert!(!middle.stacked);
        assert_eq!(middle.grid, Grid::Middle);
        assert_eq!(middle.padding(), 24.);
        let wide = layout_for(1300.);
        assert_eq!((wide.stacked, wide.grid), (false, Grid::Wide));
        assert!(!wide.compact_stepper);
        let narrow = layout_for(860.);
        assert_eq!(narrow.main_width, 500.);
        assert_eq!((narrow.stacked, narrow.grid), (false, Grid::Narrow));
        assert!(narrow.narrow() && narrow.compact_stepper);
        let unmeasured = layout_for(0.);
        assert_eq!((unmeasured.stacked, unmeasured.grid), (false, Grid::Middle));
        assert!(!unmeasured.compact_stepper);
    }

    #[test]
    fn needs_you_rows_append_the_answer_by_clock_unless_the_run_stopped() {
        let overview = planned_with_question(false);
        let timeline = build_timeline(TimelineInput {
            tasks: &overview.tasks,
            run: overview.run.as_ref(),
            questions: &overview.questions,
        });
        let deadline = timeline.deadlines[0].minute;
        let clock = clock_at(Some("2026-10-17 09:30"), deadline as f64).unwrap();
        let rows = needs_you_rows(&overview, &timeline.deadlines);
        assert_eq!(
            rows[0].meta,
            format!("Q-1 blocks TASK-2, answer by {clock}")
        );
        assert_eq!(clock, "09:50");

        let stopped = planned_with_question(true);
        let rows = needs_you_rows(&stopped, &timeline.deadlines);
        assert_eq!(rows[0].meta, "Q-1 blocks TASK-2");
    }

    #[test]
    fn pace_items_leave_out_the_average_and_a_stopped_finish() {
        let pace = PlanTreePace {
            average_minutes: 12,
            finish_clock: Some("11:40".into()),
            max_parallel: 2,
            slowest: Some(TaskMinutes {
                id: "TASK-4".into(),
                minutes: 6,
            }),
        };
        let labels = |items: Vec<PaceItem>| items.iter().map(|item| item.label).collect::<Vec<_>>();
        let all = pace_items(&pace, true, false);
        assert_eq!(all[0].value, "12m");
        assert_eq!(all[1].value, "~11:40");
        assert_eq!(all[3].value, "TASK-4 +6m");
        assert_eq!(
            labels(pace_items(&pace, false, false)),
            [
                "estimated finish",
                "builders at once",
                "slowest over average"
            ]
        );
        assert_eq!(
            labels(pace_items(&pace, true, true)),
            ["average task", "builders at once", "slowest over average"]
        );
        let empty = PlanTreePace {
            average_minutes: 0,
            finish_clock: None,
            max_parallel: 0,
            slowest: None,
        };
        assert!(pace_items(&empty, false, false).is_empty());
    }

    #[test]
    fn reject_step_confirms_then_sends_and_cancel_goes_back() {
        assert_eq!(reject_step(false, RejectPress::Reject), RejectStep::Confirm);
        assert_eq!(reject_step(true, RejectPress::Reject), RejectStep::Send);
        assert_eq!(reject_step(true, RejectPress::Cancel), RejectStep::Back);
    }

    #[test]
    fn action_buttons_are_disabled_without_an_idle_agent_or_while_sending() {
        let ready = AgentTarget::Ready {
            id: "a1".into(),
            name: "Lead".into(),
        };
        assert!(actions_disabled(&AgentTarget::NoAgent, false));
        assert!(actions_disabled(&AgentTarget::Busy, false));
        assert!(actions_disabled(&ready, true));
        assert!(!actions_disabled(&ready, false));
        assert!(send_disabled(&ready, false, "  "));
        assert!(!send_disabled(&ready, false, "yes"));
        assert_eq!(
            agent_note(&AgentTarget::NoAgent),
            ("No agent in this workspace".into(), true)
        );
        assert_eq!(agent_note(&AgentTarget::Busy), ("Agent busy".into(), true));
        assert_eq!(agent_note(&ready), ("Actions go to Lead".into(), false));
    }

    #[test]
    fn secondary_line_covers_each_row_state() {
        let done = plan_row(BarKind::Done, Some("A"));
        assert_eq!(
            secondary_line(&done, RowState::Done, None, Some("code"), Some("abc1234")),
            "committed abc1234"
        );
        assert_eq!(
            secondary_line(&done, RowState::Done, None, Some("code"), None),
            "Group A, code"
        );
        let running = plan_row(BarKind::Running, Some("A"));
        let waiting = builder(BuilderState::Waiting, None);
        let state = row_state(&running, Some(&waiting));
        assert_eq!(state, RowState::Waiting);
        assert_eq!(
            secondary_line(&running, state, Some(&waiting), None, None),
            "Builder A asks permission · claude"
        );
        let active = builder(BuilderState::Running, Some(5 * 60_000));
        let state = row_state(&running, Some(&active));
        assert_eq!(state, RowState::Running);
        assert_eq!(
            secondary_line(&running, state, Some(&active), None, None),
            "Builder A, active 5m ago · claude"
        );
        let idle = builder(BuilderState::Idle, None);
        assert_eq!(
            secondary_line(
                &plan_row(BarKind::Running, None),
                RowState::Running,
                Some(&idle),
                None,
                None
            ),
            "Builder, idle · claude"
        );
        let blocked = plan_row(BarKind::Blocked, None);
        assert_eq!(
            secondary_line(&blocked, RowState::Blocked, None, Some("risky"), None),
            "Blocked, risky"
        );
        assert_eq!(
            secondary_line(&blocked, RowState::Blocked, None, None, None),
            "Blocked"
        );
        let planned = plan_row(BarKind::Planned, None);
        assert_eq!(
            secondary_line(&planned, RowState::Planned, None, Some("docs"), None),
            "docs"
        );
        assert_eq!(
            secondary_line(&planned, RowState::Planned, None, None, None),
            ""
        );
    }

    #[test]
    fn secondary_line_appends_the_builder_provider_model_and_effort() {
        let running = plan_row(BarKind::Running, Some("A"));
        let full = run_builder(BuilderState::Running, Some("claude-opus-5-5"), Some("high"));
        assert_eq!(
            secondary_line(&running, RowState::Running, Some(&full), None, None),
            "Builder A, active 5m ago · claude, claude-opus-5-5, high"
        );
        let waiting = run_builder(BuilderState::Waiting, Some("claude-opus-5-5"), Some("high"));
        assert_eq!(
            secondary_line(&running, RowState::Waiting, Some(&waiting), None, None),
            "Builder A asks permission · claude, claude-opus-5-5, high"
        );
        let done = plan_row(BarKind::Done, Some("A"));
        assert_eq!(
            secondary_line(
                &done,
                RowState::Done,
                Some(&full),
                Some("code"),
                Some("abc1234")
            ),
            "committed abc1234 · claude, claude-opus-5-5, high"
        );
    }

    #[test]
    fn secondary_line_skips_missing_run_parts() {
        let running = plan_row(BarKind::Running, Some("A"));
        let bare = TaskBuilder {
            provider: "codex".into(),
            ..run_builder(BuilderState::Running, None, None)
        };
        assert_eq!(
            secondary_line(&running, RowState::Running, Some(&bare), None, None),
            "Builder A, active 5m ago · codex"
        );
        let empty = TaskBuilder {
            provider: String::new(),
            ..run_builder(BuilderState::Running, None, None)
        };
        assert_eq!(
            secondary_line(&running, RowState::Running, Some(&empty), None, None),
            "Builder A, active 5m ago"
        );
        let spaces = TaskBuilder {
            model: Some("  ".into()),
            ..run_builder(BuilderState::Running, None, Some("high"))
        };
        assert_eq!(
            secondary_line(&running, RowState::Running, Some(&spaces), None, None),
            "Builder A, active 5m ago · claude, high"
        );
        let blocked = plan_row(BarKind::Blocked, None);
        assert_eq!(
            secondary_line(
                &blocked,
                RowState::Blocked,
                Some(&spaces),
                Some("risky"),
                None
            ),
            "Blocked, risky · claude, high"
        );
        let planned = plan_row(BarKind::Planned, None);
        assert_eq!(
            secondary_line(&planned, RowState::Planned, Some(&spaces), None, None),
            "claude, high"
        );
    }

    #[test]
    fn the_lead_row_shows_the_plan_and_run_state_without_a_lead() {
        assert_eq!(
            lead_text(None, 0, "Port the board", "Autorun since 09:30"),
            ("Port the board".into(), "Autorun since 09:30".into())
        );
        let lead = RunAgent {
            id: "lead".into(),
            title: Some("Lead".into()),
            state: BuilderState::Running,
            active_ago: Some(12_000),
        };
        assert_eq!(
            lead_text(Some(&lead), 2, "Plan", "x").1,
            "Lead, running, active 12s ago, waiting on 2 builders"
        );
        assert_eq!(
            lead_text(Some(&lead), 0, "Plan", "x").1,
            "Lead, running, active 12s ago, no builder running"
        );
        let mut by_task = BTreeMap::new();
        by_task.insert("TASK-1".into(), builder(BuilderState::Running, None));
        by_task.insert("TASK-2".into(), builder(BuilderState::Running, None));
        assert_eq!(
            active_builders(&by_task),
            1,
            "one Group builder counts once"
        );
    }

    #[test]
    fn decision_rows_sort_by_id_descending_and_only_adr_ids_open() {
        let decision = |id: &str, status: &str| Decision {
            id: id.into(),
            date: "2026-10-16".into(),
            title: format!("{id} title"),
            status: status.into(),
        };
        let rows = decision_rows(&[
            decision("ADR-0043", "Accepted"),
            decision("ADR-0045", "Proposed"),
            decision("D-7", ""),
        ]);
        let ids: Vec<&str> = rows.iter().map(|row| row.id.as_str()).collect();
        assert_eq!(ids, ["D-7", "ADR-0045", "ADR-0043"]);
        assert!(!rows[0].pressable);
        assert_eq!(rows[0].meta, "D-7, 2026-10-16");
        assert!(rows[1].pressable && rows[1].proposed);
        assert_eq!(rows[1].meta, "ADR-0045, Proposed, 2026-10-16");
        assert!(!rows[2].proposed);
    }

    #[test]
    fn archived_meta_and_strip_and_segments_match_electron() {
        let plan = History {
            slug: "20261016-x".into(),
            date: Some("2026-10-16".into()),
            title: "X".into(),
            has_learn: true,
        };
        assert_eq!(archived_meta(&plan), "Archived 2026-10-16, LEARN.md");
        let undated = History {
            date: None,
            has_learn: false,
            ..plan
        };
        assert_eq!(archived_meta(&undated), "20261016-x");

        assert_eq!(autorun_strip(None), None);
        let live = run("2026-10-17 09:30", vec![row("TASK-1", 0, Some(25))], 30);
        let strip = autorun_strip(Some(&live)).unwrap();
        assert_eq!(
            (strip.state, strip.tone),
            ("Autorun running", StripTone::Live)
        );
        assert_eq!(
            strip.items,
            [
                ("runner", "loop".to_string()),
                ("started", "2026-10-17 09:30".to_string()),
                ("iterations", "1".to_string()),
                ("last activity", "09:55".to_string()),
            ]
        );
        let mut stopped = live.clone();
        stopped.stopped = true;
        assert_eq!(
            autorun_strip(Some(&stopped)).map(|strip| (strip.state, strip.tone)),
            Some(("Run stopped", StripTone::Stopped))
        );

        let counts = StatusCounts {
            done: 3,
            running: 0,
            blocked: 1,
            todo: 2,
        };
        assert_eq!(
            segments(counts),
            [
                (SegmentKind::Done, 3),
                (SegmentKind::Blocked, 1),
                (SegmentKind::Todo, 2)
            ]
        );
    }

    #[test]
    fn live_counts_match_the_dashboard_summary_of_the_live_tasks() {
        let tasks = vec![
            task("TASK-1", "DONE", &[]),
            task("TASK-2", "BLOCKED", &[]),
            task("TASK-3", "TODO", &[]),
            task("TASK-4", "DONE", &[]),
        ];
        let live = BTreeSet::from(["TASK-2".to_string(), "TASK-4".to_string()]);
        let expected: StatusCounts = dashboard_summary(&with_live_status(&tasks, &live))
            .counts
            .into();
        assert_eq!(live_counts(&tasks, &live), expected);
        assert_eq!(
            expected,
            StatusCounts {
                done: 2,
                running: 1,
                blocked: 0,
                todo: 1
            }
        );
    }

    #[test]
    fn tree_times_are_none_when_the_overview_has_no_run() {
        let mut overview = planned_with_question(false);
        let with_run = derive_overview(&overview, &BTreeSet::new());
        assert!(
            with_run
                .tree
                .waves
                .iter()
                .flat_map(|wave| &wave.rows)
                .any(|row| row.time.is_some())
        );
        overview.run = None;
        let derived = derive_overview(&overview, &BTreeSet::new());
        assert!(
            derived
                .tree
                .waves
                .iter()
                .flat_map(|wave| &wave.rows)
                .all(|row| row.time.is_none() && !row.over)
        );
        assert!(derived.pace.is_empty());
        assert_eq!(derived.run_state, "No autorun yet");
        assert_eq!(derived.strip, None);
    }

    #[test]
    fn body_items_follow_the_three_grids_and_the_load_states() {
        assert_eq!(
            body_items(&Loaded::Loading, None, Grid::Wide, false),
            [BodyItem::Skeleton]
        );
        assert_eq!(
            body_items(&Loaded::Error("offline".into()), None, Grid::Wide, false),
            [BodyItem::Error("offline".into())]
        );
        let empty = Overview {
            message: Some("No watchtower/NEXT.md in this workspace.".into()),
            ..Overview::default()
        };
        let derived = derive_overview(&empty, &BTreeSet::new());
        assert_eq!(
            body_items(
                &Loaded::Ready(empty.clone()),
                Some(&derived),
                Grid::Wide,
                false
            ),
            [
                BodyItem::NoPlan("No watchtower/NEXT.md in this workspace.".into()),
                BodyItem::Row3
            ]
        );

        let overview = planned_with_question(false);
        let derived = derive_overview(&overview, &BTreeSet::new());
        let ready = Loaded::Ready(overview);
        let wide = body_items(&ready, Some(&derived), Grid::Wide, false);
        assert_eq!(
            wide[..3],
            [BodyItem::Stepper, BodyItem::Strip, BodyItem::PlanTop]
        );
        assert_eq!(
            wide.last(),
            Some(&BodyItem::Row(
                vec![CellKind::Needs],
                vec![CellKind::Checks, CellKind::Decisions, CellKind::Archived]
            ))
        );
        let middle = body_items(&ready, Some(&derived), Grid::Middle, false);
        assert_eq!(
            middle[middle.len() - 2..],
            [
                BodyItem::Row(vec![CellKind::Needs], vec![CellKind::Checks]),
                BodyItem::Row(vec![CellKind::Decisions], vec![CellKind::Archived]),
            ]
        );
        let narrow = body_items(&ready, Some(&derived), Grid::Narrow, false);
        let tail: Vec<&BodyItem> = narrow
            .iter()
            .skip_while(|item| **item != BodyItem::PlanBottom)
            .collect();
        assert_eq!(
            tail,
            [
                &BodyItem::PlanBottom,
                &BodyItem::NeedsTop,
                &BodyItem::NeedsRow(0),
                &BodyItem::NeedsBottom,
                &BodyItem::Cell(CellKind::Checks),
                &BodyItem::Cell(CellKind::Decisions),
                &BodyItem::Cell(CellKind::Archived),
            ]
        );
        assert!(narrow.contains(&BodyItem::TaskRow {
            wave: 1,
            row: 0,
            last: true
        }));
    }

    #[test]
    fn the_derived_cache_rebuilds_only_when_its_key_changes() {
        let overview = planned_with_question(false);
        let key = |revision, minute| DerivedKey {
            revision,
            live: BTreeSet::new(),
            minute,
        };
        let mut cache = DerivedCache::default();
        let build = || Some(derive_overview(&overview, &BTreeSet::new()));
        assert!(cache.update(key(1, 100), build));
        for _ in 0..100 {
            assert!(!cache.update(key(1, 100), build));
        }
        assert_eq!(cache.builds(), 1);
        assert!(cache.update(key(1, 101), build), "the minute changed");
        assert!(cache.update(key(2, 101), build), "the data changed");
        let live = DerivedKey {
            live: BTreeSet::from(["TASK-2".to_string()]),
            ..key(2, 101)
        };
        assert!(cache.update(live, build), "the builder set changed");
        assert_eq!(cache.builds(), 4);
        assert!(cache.get().is_some());
    }

    #[test]
    fn one_thousand_derives_of_a_40_task_plan_take_under_200_ms() {
        let ids: Vec<String> = (1..=40).map(|n| format!("TASK-{n:03}")).collect();
        let tasks = (0..40)
            .map(|index| {
                let deps: Vec<&str> = if index < 4 {
                    vec![]
                } else {
                    vec![ids[index - 4].as_str()]
                };
                let status = if index < 15 {
                    "DONE"
                } else if index < 18 {
                    "IN PROGRESS"
                } else {
                    "TODO"
                };
                let mut task = task(&ids[index], status, &deps);
                task.group = Some(["A", "B", "C", "D"][index % 4].into());
                task.task_class = Some("code".into());
                task
            })
            .collect();
        let log = (0..18)
            .map(|index| {
                let end = (index < 15).then_some(index as i64 * 10 + 9);
                let mut entry = row(&ids[index], index as i64 * 10, end);
                entry.detail = format!("committed abc{index:04}");
                entry
            })
            .collect();
        let overview = Overview {
            tasks,
            run: Some(run("2026-10-17 09:30", log, 180)),
            questions: vec![question("Q-1", &["TASK-030"]), question("Q-2", &[])],
            decisions: (1..=20)
                .map(|n| Decision {
                    id: format!("ADR-{n:04}"),
                    title: "A decision".into(),
                    status: "Accepted".into(),
                    date: "2026-10-16".into(),
                })
                .collect(),
            ..Overview::default()
        };
        let live = BTreeSet::from([ids[15].clone(), ids[16].clone()]);
        // The best of three runs: other tests and builds share the machine, and they only add
        // time.
        let elapsed = (0..3)
            .map(|_| {
                let start = Instant::now();
                for _ in 0..1000 {
                    std::hint::black_box(derive_overview(&overview, &live));
                }
                start.elapsed()
            })
            .min()
            .unwrap();
        eprintln!("1,000 derives of 40 tasks: {} ms", elapsed.as_millis());
        assert!(
            elapsed.as_millis() < 200,
            "1,000 derives took {} ms",
            elapsed.as_millis()
        );
    }
}
