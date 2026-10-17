//! The data types and every pure rule of the plugin, each with unit tests. The views and the
//! store read them; a few ported rules, such as the timeline axis, have no reader, so they stay
//! exported without one.

#![allow(dead_code, unused_imports)]

mod actions;
mod builders;
mod cache;
mod dashboard;
mod dto;
mod id_map;
mod lifecycle;
mod markdown;
mod overview_view;
mod page;
mod plan_tree;
mod time;
mod timeline;

pub(crate) use overview_view::{
    BodyItem, CellKind, DecisionRow, Derived, DerivedCache, DerivedKey, Grid, Layout, PaceItem,
    RAIL_WIDTH, RejectPress, RejectStep, RowState, SegmentKind, StatusCounts, StripModel,
    StripTone, action_subject, actions_disabled, active_builders, agent_note, archived_meta,
    body_items, derive_overview, layout_for, lead_text, needs_warning, reject_step, row_state,
    secondary_line, segments, send_disabled,
};

pub(crate) use actions::{
    AgentCandidate, AgentTarget, accept_decision_prompt, answer_prompt, choose_agent,
    default_answer_prompt, manual_checks_prompt, reject_decision_prompt,
};
pub(crate) use builders::{
    BuilderMatch, BuilderState, RunAgent, TaskBuilder, live_task_ids, match_builders,
};
pub(crate) use cache::{Cache, STALE_MS, Want, cache_fresh};
pub(crate) use dashboard::{
    DashboardSummary, GroupCounts, TaskGroup, TaskLike, dashboard_summary, group_tasks, pill_label,
    plural, questions_for, task_group, task_title, with_live_status,
};
pub(crate) use dto::{
    Board, Decision, DecisionDetail, DecisionInput, History, ManualCheck, Overview, OverviewPlan,
    OverviewQuestion, OverviewRun, OverviewRunEntry, OverviewTask, ProjectInput, Question, Run,
    RunEntry, Task, WorkspaceInput, attachment_key, is_decision_id,
};
pub(crate) use lifecycle::{
    NeedsYouKind, NeedsYouRow, NeedsYouTone, Phase, PhaseName, PhaseState, lifecycle_phases,
    needs_you,
};
pub(crate) use markdown::{Block, parse_markdown};
pub(crate) use page::{
    PageWorkspace, changed_range, choose_workspace, folder_name, initial_selection, initial_task,
    project_name,
};
pub(crate) use plan_tree::{
    PlanTree, PlanTreePace, PlanTreeRow, PlanTreeWave, ago, build_plan_tree,
    build_plan_tree_and_timeline, commit_shas,
};
pub(crate) use time::{now_ms, parse_iso_ms, round_half_up};
pub(crate) use timeline::{
    BarKind, Deadline, TaskMinutes, Timeline, TimelineBar, TimelineEdge, TimelineInput,
    TimelineStats, TimelineTick, bar_end, build_timeline, clock_at, started_at,
};

/// A value the store loads from the server part.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Loaded<T> {
    Loading,
    Ready(T),
    Error(String),
}

impl<T> Loaded<T> {
    pub(crate) fn ready(&self) -> Option<&T> {
        match self {
            Loaded::Ready(value) => Some(value),
            _ => None,
        }
    }
}
