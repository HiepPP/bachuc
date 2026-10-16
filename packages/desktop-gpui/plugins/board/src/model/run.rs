//! The `board.snapshot` wire types and the grouping rules of `shared/board.ts`.

use anyhow::Context as _;
use serde::Deserialize;
use serde_json::Value;
use std::cmp::Ordering;
use std::collections::HashMap;

/// `shared/board.ts:68-69`.
pub(crate) const ACTIVE_BOARD_POLL_MS: i64 = 2_000;
pub(crate) const IDLE_BOARD_POLL_MS: i64 = 15_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum RunStatus {
    Running,
    Completed,
    Failed,
    Cancelled,
    Unknown,
}

/// One card of the Board (`runSchema`, `shared/board.ts:4-30`).
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Run {
    pub id: String,
    pub agent_id: String,
    #[serde(default)]
    pub parent_agent_id: Option<String>,
    pub title: String,
    pub starred: bool,
    #[serde(default)]
    pub needs_input: bool,
    /// The host read state of a finished run; absent from older hosts.
    #[serde(default)]
    pub unread: Option<bool>,
    #[serde(default)]
    pub diffs_off: bool,
    pub project: String,
    pub project_key: String,
    #[serde(default)]
    pub project_id: Option<String>,
    #[serde(default)]
    pub cwd: Option<String>,
    pub provider: String,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub effort: Option<String>,
    pub status: RunStatus,
    #[serde(default)]
    pub started_at: Option<String>,
    #[serde(default)]
    pub ended_at: Option<String>,
    #[serde(default)]
    pub task_id: Option<String>,
    #[serde(default)]
    pub workspace_id: Option<String>,
}

impl Run {
    pub(crate) fn running(&self) -> bool {
        self.status == RunStatus::Running
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct BoardSnapshot {
    pub runs: Vec<Run>,
    pub observing_since: String,
}

/// Decodes a `board.snapshot` answer. Each run decodes on its own: a bad run is skipped and
/// logged, so one malformed row never blanks the Board.
pub(crate) fn decode_snapshot(value: &Value) -> anyhow::Result<BoardSnapshot> {
    let observing_since = value
        .get("observingSince")
        .and_then(Value::as_str)
        .context("board.snapshot answer has no observingSince")?
        .to_string();
    let runs = value
        .get("runs")
        .and_then(Value::as_array)
        .context("board.snapshot answer has no runs")?
        .iter()
        .filter_map(|raw| match Run::deserialize(raw) {
            Ok(run) => Some(run),
            Err(error) => {
                log::warn!("board: skipped a run that does not decode: {error}");
                None
            }
        })
        .collect();
    Ok(BoardSnapshot {
        runs,
        observing_since,
    })
}

/// `boardPollInterval`: fast while any run is running.
pub(crate) fn board_poll_interval(runs: Option<&[Run]>) -> i64 {
    if runs.is_some_and(|runs| runs.iter().any(Run::running)) {
        ACTIVE_BOARD_POLL_MS
    } else {
        IDLE_BOARD_POLL_MS
    }
}

/// `starredFirst`: a stable sort with it keeps the order inside each group.
pub(crate) fn starred_first(left: bool, right: bool) -> Ordering {
    right.cmp(&left)
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ProjectGroup<T> {
    pub key: String,
    pub name: String,
    pub runs: Vec<T>,
}

/// `groupRuns`: groups by `projectKey` in first-seen order, starred runs first in each.
pub(crate) fn group_runs<T: Clone>(
    runs: &[T],
    run: impl Fn(&T) -> &Run,
    starred: impl Fn(&T) -> bool,
) -> Vec<ProjectGroup<T>> {
    let mut groups: Vec<ProjectGroup<T>> = Vec::new();
    let mut index: HashMap<String, usize> = HashMap::new();
    for item in runs {
        let key = &run(item).project_key;
        let at = *index.entry(key.clone()).or_insert_with(|| {
            groups.push(ProjectGroup {
                key: key.clone(),
                name: run(item).project.clone(),
                runs: Vec::new(),
            });
            groups.len() - 1
        });
        groups[at].runs.push(item.clone());
    }
    for group in &mut groups {
        group
            .runs
            .sort_by(|left, right| starred_first(starred(left), starred(right)));
    }
    groups
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use serde_json::json;

    pub(crate) fn card(id: &str, project_key: &str, starred: bool) -> Run {
        Run {
            id: id.into(),
            agent_id: id.into(),
            parent_agent_id: None,
            title: id.into(),
            starred,
            needs_input: false,
            unread: None,
            diffs_off: false,
            project: "Same name".into(),
            project_key: project_key.into(),
            project_id: None,
            cwd: None,
            provider: "codex".into(),
            model: None,
            effort: None,
            status: RunStatus::Running,
            started_at: None,
            ended_at: None,
            task_id: None,
            workspace_id: None,
        }
    }

    fn ids(groups: &[ProjectGroup<Run>]) -> Vec<(String, Vec<String>)> {
        groups
            .iter()
            .map(|group| {
                let runs = group.runs.iter().map(|run| run.id.clone()).collect();
                (group.key.clone(), runs)
            })
            .collect()
    }

    fn group(runs: &[Run]) -> Vec<ProjectGroup<Run>> {
        group_runs(runs, |run| run, |run| run.starred)
    }

    #[test]
    fn groups_keep_first_seen_project_order_and_put_starred_runs_first() {
        let mut runs = vec![
            card("a", "one", false),
            card("b", "two", false),
            card("c", "one", false),
            card("s", "two", true),
        ];
        assert_eq!(
            ids(&group(&runs)),
            vec![
                ("one".into(), vec!["a".into(), "c".into()]),
                ("two".into(), vec!["s".into(), "b".into()]),
            ]
        );
        // The input keeps its order.
        let order: Vec<&str> = runs.iter().map(|run| run.id.as_str()).collect();
        assert_eq!(order, ["a", "b", "c", "s"]);
        runs[3].starred = false;
        assert_eq!(ids(&group(&runs))[1].1, ["b", "s"]);
    }

    #[test]
    fn empty_and_all_starred_columns_have_no_empty_groups() {
        assert!(group(&[]).is_empty());
        let runs = [card("a", "one", true), card("b", "two", true)];
        assert_eq!(
            ids(&group(&runs)),
            vec![
                ("one".into(), vec!["a".into()]),
                ("two".into(), vec!["b".into()]),
            ]
        );
    }

    #[test]
    fn stars_rank_first_and_keep_the_order_inside_each_group() {
        let mut rows = [
            ("first", false),
            ("star-1", true),
            ("second", false),
            ("star-2", true),
        ];
        rows.sort_by(|left, right| starred_first(left.1, right.1));
        let order: Vec<&str> = rows.iter().map(|row| row.0).collect();
        assert_eq!(order, ["star-1", "star-2", "first", "second"]);
    }

    #[test]
    fn polls_fast_only_while_a_run_is_running() {
        let mut done = card("d", "one", false);
        done.status = RunStatus::Completed;
        assert_eq!(board_poll_interval(None), IDLE_BOARD_POLL_MS);
        assert_eq!(board_poll_interval(Some(&[])), IDLE_BOARD_POLL_MS);
        assert_eq!(
            board_poll_interval(Some(std::slice::from_ref(&done))),
            15_000
        );
        assert_eq!(
            board_poll_interval(Some(&[done, card("r", "one", false)])),
            2_000
        );
    }

    #[test]
    fn a_real_snapshot_decodes_and_skips_one_bad_run() {
        let payload = json!({
            "observingSince": "2026-10-16T08:00:00.000Z",
            "runs": [
                {
                    "id": "run-1",
                    "agentId": "agent-1",
                    "parentAgentId": null,
                    "title": "Port the Board",
                    "starred": true,
                    "needsInput": false,
                    "diffsOff": true,
                    "project": "bachuc",
                    "projectKey": "project:bachuc",
                    "projectId": "p-1",
                    "cwd": "/Users/hiep/Projects/bachuc",
                    "provider": "claude",
                    "model": "Opus 5.5",
                    "effort": "High",
                    "status": "running",
                    "startedAt": "2026-10-16T08:01:00.000Z",
                    "endedAt": null,
                    "taskId": "TASK-189",
                    "workspaceId": "w-1"
                },
                {
                    "id": "run-2",
                    "agentId": "agent-2",
                    "parentAgentId": "agent-1",
                    "title": "Read the page",
                    "starred": false,
                    "unread": true,
                    "project": "bachuc",
                    "projectKey": "project:bachuc",
                    "provider": "codex",
                    "status": "completed",
                    "startedAt": "2026-10-16T08:02:00.000Z",
                    "endedAt": "2026-10-16T08:03:00.000Z"
                },
                {"id": "bad", "agentId": "agent-3", "status": "exploded"}
            ]
        });
        let snapshot = decode_snapshot(&payload).unwrap();
        assert_eq!(snapshot.observing_since, "2026-10-16T08:00:00.000Z");
        assert_eq!(snapshot.runs.len(), 2);
        let first = &snapshot.runs[0];
        assert_eq!(first.status, RunStatus::Running);
        assert!(first.diffs_off && first.starred && !first.needs_input);
        assert_eq!(first.task_id.as_deref(), Some("TASK-189"));
        assert_eq!(first.parent_agent_id, None);
        let second = &snapshot.runs[1];
        assert_eq!(second.parent_agent_id.as_deref(), Some("agent-1"));
        assert_eq!(second.unread, Some(true));
        assert_eq!(second.model, None);
        assert!(!second.diffs_off);
        assert!(decode_snapshot(&json!({"runs": []})).is_err());
    }
}
