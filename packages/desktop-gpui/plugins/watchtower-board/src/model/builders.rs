//! Which agent builds which TASK (`client/builders.ts`). GPUI keeps agents live in the client,
//! so the store passes the workspace's agents in instead of polling the agent list.

use super::actions::{AgentCandidate, PARENT_LABEL, sort_newest_first};
use super::dto::OverviewTask;
use super::time::parse_iso_ms;
use std::collections::BTreeMap;

pub(crate) const TASK_LABEL: &str = "watchtower.task";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BuilderState {
    Running,
    Waiting,
    Failed,
    Idle,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RunAgent {
    pub id: String,
    pub title: Option<String>,
    pub state: BuilderState,
    /// Milliseconds since `agent_updated_at`; `None` when the time is unreadable.
    pub active_ago: Option<i64>,
}

/// A builder under a TASK; its title falls back to the agent id.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TaskBuilder {
    pub id: String,
    pub title: String,
    pub state: BuilderState,
    pub active_ago: Option<i64>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct BuilderMatch {
    /// Keyed by TASK ID. A Group builder appears under every TASK of its group.
    pub by_task: BTreeMap<String, TaskBuilder>,
    /// The parent that most matched builders name, or `None` (Q-036 default).
    pub lead: Option<RunAgent>,
}

fn state_of(agent: &AgentCandidate) -> BuilderState {
    // A permission wait comes first: the agent still reports `running` while it waits.
    if agent.pending_permissions > 0 || agent.permission_attention {
        return BuilderState::Waiting;
    }
    match agent.status.as_str() {
        "error" => BuilderState::Failed,
        "running" => BuilderState::Running,
        _ => BuilderState::Idle,
    }
}

fn active_ago(agent: &AgentCandidate, now_ms: i64) -> Option<i64> {
    let at = agent.updated_at.as_deref().and_then(parse_iso_ms)?;
    Some((now_ms - at).max(0))
}

fn parent_of(agent: &AgentCandidate) -> Option<&str> {
    agent
        .labels
        .get(PARENT_LABEL)
        .map(|parent| parent.trim())
        .filter(|parent| !parent.is_empty())
}

/// Plans write `standalone` in the Group column of a TASK that shares no file, so it is no group.
fn group_of(task: &OverviewTask) -> Option<&str> {
    task.group
        .as_deref()
        .filter(|group| !group.is_empty() && *group != "standalone")
}

fn is_task_id(label: &str) -> bool {
    label
        .strip_prefix("TASK-")
        .is_some_and(|digits| !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()))
}

fn tasks_of(label: &str, tasks: &[OverviewTask]) -> Vec<String> {
    let in_group = |group: &str| -> Vec<String> {
        tasks
            .iter()
            .filter(|task| group_of(task) == Some(group))
            .map(|task| task.id.clone())
            .collect()
    };
    if is_task_id(label) {
        let Some(task) = tasks.iter().find(|task| task.id == label) else {
            return Vec::new();
        };
        // A builder labelled with the first TASK ID of its group works on the whole group.
        return match group_of(task) {
            Some(group) => in_group(group),
            None => vec![task.id.clone()],
        };
    }
    in_group(label)
}

/// IDs of the TASKs whose builder runs or waits for a permission, sorted.
pub(crate) fn live_task_ids(found: &BuilderMatch) -> Vec<String> {
    found
        .by_task
        .iter()
        .filter(|(_, builder)| {
            matches!(builder.state, BuilderState::Running | BuilderState::Waiting)
        })
        .map(|(id, _)| id.clone())
        .collect()
}

/// Matches labelled agents to TASKs. The caller passes one workspace's agents.
pub(crate) fn match_builders(
    agents: &[AgentCandidate],
    tasks: &[OverviewTask],
    now_ms: i64,
) -> BuilderMatch {
    let mut by_task = BTreeMap::new();
    // Agents that claimed a TASK, in claim order.
    let mut winners: Vec<&AgentCandidate> = Vec::new();
    // Newest first, so the first agent to claim a TASK is the one to keep.
    let mut sorted: Vec<&AgentCandidate> = agents
        .iter()
        .filter(|agent| !agent.provider.starts_with("mock") && agent.label(TASK_LABEL).is_some())
        .collect();
    sort_newest_first(&mut sorted);
    for agent in sorted {
        let label = agent.label(TASK_LABEL).unwrap_or_default().trim();
        if label.is_empty() {
            continue;
        }
        for task_id in tasks_of(label, tasks) {
            if by_task.contains_key(&task_id) {
                continue;
            }
            by_task.insert(
                task_id,
                TaskBuilder {
                    id: agent.id.clone(),
                    title: agent.name(),
                    state: state_of(agent),
                    active_ago: active_ago(agent, now_ms),
                },
            );
            if !winners.iter().any(|winner| winner.id == agent.id) {
                winners.push(agent);
            }
        }
    }

    // Each builder agent votes once, however many TASKs it covers. A tie keeps the first parent.
    let mut votes: Vec<(&str, usize)> = Vec::new();
    for agent in &winners {
        if let Some(parent) = parent_of(agent) {
            match votes.iter_mut().find(|(id, _)| *id == parent) {
                Some((_, count)) => *count += 1,
                None => votes.push((parent, 1)),
            }
        }
    }
    let mut lead_id: Option<(&str, usize)> = None;
    for &(parent, count) in &votes {
        if lead_id.is_none_or(|(_, best)| count > best) {
            lead_id = Some((parent, count));
        }
    }
    let lead = lead_id.map(
        |(lead_id, _)| match agents.iter().find(|agent| agent.id == lead_id) {
            Some(found) => RunAgent {
                id: found.id.clone(),
                title: Some(found.title.trim().to_string()).filter(|title| !title.is_empty()),
                state: state_of(found),
                active_ago: active_ago(found, now_ms),
            },
            None => RunAgent {
                id: lead_id.to_string(),
                title: None,
                state: BuilderState::Idle,
                active_ago: None,
            },
        },
    );
    BuilderMatch { by_task, lead }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::time::parse_iso_ms;

    fn now() -> i64 {
        parse_iso_ms("2026-10-14T12:00:00.000Z").unwrap()
    }

    fn task(id: &str, group: Option<&str>) -> OverviewTask {
        OverviewTask {
            id: id.into(),
            title: id.into(),
            status: "TODO".into(),
            group: group.map(Into::into),
            ..OverviewTask::default()
        }
    }

    fn tasks() -> Vec<OverviewTask> {
        vec![
            task("TASK-089", Some("D")),
            task("TASK-090", Some("D")),
            task("TASK-091", Some("E")),
            task("TASK-092", None),
        ]
    }

    fn agent(id: &str) -> AgentCandidate {
        AgentCandidate {
            id: id.into(),
            status: "idle".into(),
            workspace_id: Some("ws-1".into()),
            provider: "claude".into(),
            updated_at: Some("2026-10-14T11:50:00.000Z".into()),
            ..AgentCandidate::default()
        }
    }

    fn builder(id: &str, label: &str) -> AgentCandidate {
        AgentCandidate {
            labels: BTreeMap::from([(TASK_LABEL.to_string(), label.to_string())]),
            ..agent(id)
        }
    }

    fn with_parent(mut agent: AgentCandidate, parent: &str) -> AgentCandidate {
        agent
            .labels
            .insert(PARENT_LABEL.to_string(), parent.to_string());
        agent
    }

    fn keys(found: &BuilderMatch) -> Vec<&str> {
        found.by_task.keys().map(String::as_str).collect()
    }

    // builders.test.ts
    #[test]
    fn a_task_id_label_maps_to_every_task_of_its_group() {
        let found = match_builders(&[builder("b1", "TASK-089")], &tasks(), now());
        assert_eq!(keys(&found), ["TASK-089", "TASK-090"]);
        assert_eq!(found.by_task["TASK-090"].id, "b1");
    }

    #[test]
    fn a_standalone_task_id_label_maps_to_that_task_only() {
        let plan = [
            task("TASK-001", Some("standalone")),
            task("TASK-002", Some("standalone")),
        ];
        assert_eq!(
            keys(&match_builders(&[builder("b1", "TASK-001")], &plan, now())),
            ["TASK-001"]
        );
        assert!(
            match_builders(&[builder("b2", "standalone")], &plan, now())
                .by_task
                .is_empty()
        );
    }

    #[test]
    fn live_tasks_are_those_whose_builder_runs_or_waits_sorted() {
        let found = match_builders(
            &[
                AgentCandidate {
                    status: "running".into(),
                    ..builder("b1", "TASK-091")
                },
                AgentCandidate {
                    status: "running".into(),
                    pending_permissions: 1,
                    ..builder("b2", "TASK-089")
                },
                builder("b3", "TASK-092"),
            ],
            &tasks(),
            now(),
        );
        assert_eq!(live_task_ids(&found), ["TASK-089", "TASK-090", "TASK-091"]);
    }

    #[test]
    fn a_task_id_label_of_a_task_without_a_group_maps_to_that_task_only() {
        let found = match_builders(&[builder("b1", "TASK-092")], &tasks(), now());
        assert_eq!(keys(&found), ["TASK-092"]);
    }

    #[test]
    fn a_group_tag_label_maps_to_every_task_of_that_group() {
        let found = match_builders(&[builder("b1", "E"), builder("b2", "D")], &tasks(), now());
        assert_eq!(found.by_task["TASK-091"].id, "b1");
        assert_eq!(found.by_task["TASK-089"].id, "b2");
        assert_eq!(found.by_task["TASK-090"].id, "b2");
    }

    #[test]
    fn unknown_labels_missing_labels_and_mock_agents_never_match() {
        let agents = [
            builder("unknown-task", "TASK-999"),
            builder("unknown-group", "Z"),
            agent("no-label"),
            AgentCandidate {
                provider: "mock-provider".into(),
                ..builder("mock", "TASK-089")
            },
            builder("blank", "  "),
        ];
        let found = match_builders(&agents, &tasks(), now());
        assert!(found.by_task.is_empty());
        assert_eq!(found.lead, None);
    }

    #[test]
    fn agents_of_another_workspace_still_match_because_the_caller_filters_the_workspace() {
        let found = match_builders(
            &[AgentCandidate {
                workspace_id: Some("ws-2".into()),
                ..builder("b1", "TASK-091")
            }],
            &tasks(),
            now(),
        );
        assert_eq!(found.by_task["TASK-091"].id, "b1");
    }

    #[test]
    fn two_agents_on_one_task_keep_the_newest_by_updated_at() {
        let older = AgentCandidate {
            updated_at: Some("2026-10-14T09:00:00.000Z".into()),
            ..builder("old", "TASK-089")
        };
        let newer = AgentCandidate {
            updated_at: Some("2026-10-14T11:00:00.000Z".into()),
            ..builder("new", "D")
        };
        let pick = |agents: &[AgentCandidate]| {
            match_builders(agents, &tasks(), now()).by_task["TASK-089"]
                .id
                .clone()
        };
        assert_eq!(pick(&[newer.clone(), older.clone()]), "new");
        assert_eq!(pick(&[older, newer]), "new");
    }

    #[test]
    fn an_unreadable_time_sorts_last() {
        let unreadable = AgentCandidate {
            updated_at: Some("not a date".into()),
            ..builder("unreadable", "TASK-091")
        };
        let older = AgentCandidate {
            updated_at: Some("2026-10-01T00:00:00.000Z".into()),
            ..builder("older", "TASK-091")
        };
        let found = match_builders(&[unreadable, older], &tasks(), now());
        assert_eq!(found.by_task["TASK-091"].id, "older");
    }

    #[test]
    fn builder_state_running_waiting_failed_else_idle() {
        let state = |patch: AgentCandidate| {
            let agent = AgentCandidate {
                labels: BTreeMap::from([(TASK_LABEL.to_string(), "TASK-091".to_string())]),
                ..patch
            };
            match_builders(&[agent], &tasks(), now()).by_task["TASK-091"].state
        };
        let with = |status: &str| AgentCandidate {
            status: status.into(),
            ..agent("b")
        };
        assert_eq!(state(with("running")), BuilderState::Running);
        assert_eq!(
            state(AgentCandidate {
                permission_attention: true,
                ..with("running")
            }),
            BuilderState::Waiting
        );
        assert_eq!(
            state(AgentCandidate {
                permission_attention: true,
                ..with("idle")
            }),
            BuilderState::Waiting
        );
        assert_eq!(
            state(AgentCandidate {
                pending_permissions: 1,
                ..with("running")
            }),
            BuilderState::Waiting
        );
        assert_eq!(state(with("error")), BuilderState::Failed);
        assert_eq!(state(with("idle")), BuilderState::Idle);
        assert_eq!(state(with("closed")), BuilderState::Idle);
    }

    #[test]
    fn active_ago_counts_from_updated_at_and_is_none_when_unreadable() {
        let ago = |updated_at: &str| {
            let agent = AgentCandidate {
                updated_at: Some(updated_at.into()),
                ..builder("b", "TASK-091")
            };
            match_builders(&[agent], &tasks(), now()).by_task["TASK-091"].active_ago
        };
        assert_eq!(ago("2026-10-14T11:50:00.000Z"), Some(10 * 60_000));
        assert_eq!(ago("not a date"), None);
        assert_eq!(ago("2026-10-14T12:30:00.000Z"), Some(0));
    }

    #[test]
    fn the_title_falls_back_to_the_agent_id() {
        let found = match_builders(
            &[
                AgentCandidate {
                    title: " Builder E ".into(),
                    ..builder("b1", "TASK-091")
                },
                builder("b2", "TASK-092"),
            ],
            &tasks(),
            now(),
        );
        assert_eq!(found.by_task["TASK-091"].title, "Builder E");
        assert_eq!(found.by_task["TASK-092"].title, "b2");
    }

    #[test]
    fn the_lead_is_the_parent_most_matched_builders_name() {
        let agents = [
            with_parent(builder("b1", "TASK-089"), "lead-a"),
            with_parent(builder("b2", "TASK-091"), "lead-a"),
            with_parent(builder("b3", "TASK-092"), "lead-b"),
            AgentCandidate {
                title: "Run lead".into(),
                status: "running".into(),
                updated_at: Some("2026-10-14T11:58:00.000Z".into()),
                ..agent("lead-a")
            },
            agent("lead-b"),
        ];
        assert_eq!(
            match_builders(&agents, &tasks(), now()).lead,
            Some(RunAgent {
                id: "lead-a".into(),
                title: Some("Run lead".into()),
                state: BuilderState::Running,
                active_ago: Some(2 * 60_000),
            })
        );
    }

    #[test]
    fn a_lead_missing_from_the_list_keeps_its_id_with_no_title() {
        let found = match_builders(
            &[with_parent(builder("b1", "TASK-091"), "gone")],
            &tasks(),
            now(),
        );
        assert_eq!(
            found.lead,
            Some(RunAgent {
                id: "gone".into(),
                title: None,
                state: BuilderState::Idle,
                active_ago: None,
            })
        );
    }

    #[test]
    fn with_no_parent_there_is_no_lead() {
        assert_eq!(
            match_builders(&[builder("b1", "TASK-091")], &tasks(), now()).lead,
            None
        );
    }

    #[test]
    fn builders_replaced_by_a_newer_agent_do_not_vote_for_the_lead() {
        let agents = [
            with_parent(
                AgentCandidate {
                    updated_at: Some("2026-10-14T08:00:00.000Z".into()),
                    ..builder("old", "TASK-091")
                },
                "stale",
            ),
            with_parent(
                AgentCandidate {
                    updated_at: Some("2026-10-14T11:00:00.000Z".into()),
                    ..builder("new", "TASK-091")
                },
                "fresh",
            ),
        ];
        assert_eq!(
            match_builders(&agents, &tasks(), now())
                .lead
                .map(|lead| lead.id),
            Some("fresh".into())
        );
    }
}
