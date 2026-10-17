//! The RPC shapes of `shared/board.ts` and `shared/overview.ts`, in camelCase JSON. Every
//! nullable field decodes to `Option`; a task `status` stays a string, so an unknown one survives.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Task {
    pub id: String,
    pub title: String,
    pub status: String,
    pub deps: String,
    pub notes: String,
    pub spec: String,
    /// The spec's `Class:` line; Watchtower reads a missing class as `risky`.
    pub task_class: Option<String>,
    pub brief: Option<String>,
    pub blocker: Option<String>,
    pub error: Option<String>,
}

/// An OPEN row of QUESTIONS.md whose Blocks cell names at least one TASK.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Question {
    pub id: String,
    pub question: String,
    pub blocks: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RunEntry {
    pub start: String,
    pub end: String,
    pub task: String,
    pub result: String,
    pub detail: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Run {
    pub runner: Option<String>,
    pub schedule: Option<String>,
    pub profile: Option<String>,
    pub started: Option<String>,
    pub finished: Option<String>,
    /// Newest first, at most `RUN_LOG_LIMIT` (5) rows.
    pub log: Vec<RunEntry>,
    pub total: i64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Board {
    pub title: String,
    pub tasks: Vec<Task>,
    pub message: Option<String>,
    pub questions: Vec<Question>,
    pub run: Option<Run>,
    pub proposed_adrs: i64,
    /// Unreadable QUESTIONS.md, RUN.md, or DECISIONS.md; a missing file is not a warning.
    pub warnings: Vec<String>,
}

/// A board task without its brief, with the Deps IDs and the Tracker Group cell.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OverviewTask {
    pub id: String,
    pub title: String,
    pub status: String,
    pub deps: String,
    pub notes: String,
    pub spec: String,
    pub task_class: Option<String>,
    pub blocker: Option<String>,
    pub error: Option<String>,
    pub dep_ids: Vec<String>,
    pub group: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OverviewPlan {
    pub title: String,
    pub slug: Option<String>,
    pub status: Option<String>,
    pub updated: Option<String>,
}

/// A `## Handoff` bullet that starts with `Manual check pending:`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ManualCheck {
    pub text: String,
    pub tasks: Vec<String>,
}

/// Minutes count from the run's `Started:` time; `None` for `now`, `-`, or an unreadable time.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OverviewRunEntry {
    pub task: String,
    pub result: String,
    pub detail: String,
    pub start: String,
    pub end: String,
    pub start_minute: Option<i64>,
    pub end_minute: Option<i64>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OverviewRun {
    pub runner: Option<String>,
    pub started: Option<String>,
    pub finished: Option<String>,
    /// Every log row, oldest first.
    pub log: Vec<OverviewRunEntry>,
    pub now_minute: Option<i64>,
    pub stopped: bool,
}

/// An OPEN or DEFAULTED row of QUESTIONS.md.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OverviewQuestion {
    pub id: String,
    pub tasks: Vec<String>,
    pub blocks: Vec<String>,
    pub question: String,
    pub default: String,
    pub status: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Decision {
    pub id: String,
    pub date: String,
    pub title: String,
    pub status: String,
}

/// One folder of `watchtower/archive/`, newest first.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct History {
    pub slug: String,
    pub date: Option<String>,
    pub title: String,
    pub has_learn: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Overview {
    pub plan: OverviewPlan,
    pub tasks: Vec<OverviewTask>,
    pub plan_verify: Vec<String>,
    pub manual_checks: Vec<ManualCheck>,
    pub run: Option<OverviewRun>,
    pub questions: Vec<OverviewQuestion>,
    pub decisions: Vec<Decision>,
    pub history: Vec<History>,
    pub message: Option<String>,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DecisionDetail {
    pub id: String,
    pub title: String,
    pub status: String,
    /// The ADR file, relative to the workspace root.
    pub file: String,
    pub markdown: String,
}

/// The input of `watchtower.read` and `watchtower.overview.read`.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkspaceInput<'a> {
    pub workspace_id: &'a str,
}

/// The input of `watchtower.project.read`.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProjectInput<'a> {
    pub project_id: &'a str,
}

/// The input of `watchtower.decision.read`.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DecisionInput<'a> {
    pub workspace_id: &'a str,
    pub id: &'a str,
}

/// The board search key that the attachment picker reads (`shared/board.ts:83`).
pub(crate) fn attachment_key(workspace_id: &str, task_id: &str) -> String {
    format!("workspace:{workspace_id} {task_id}")
}

/// The ID of a DECISIONS.md row, `^ADR-\d{1,6}$`. The server matches it against file names.
pub(crate) fn is_decision_id(id: &str) -> bool {
    id.strip_prefix("ADR-").is_some_and(|digits| {
        (1..=6).contains(&digits.len()) && digits.bytes().all(|b| b.is_ascii_digit())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // Payloads from the server part: `readBoard`, `readOverview`, and `readDecision` run on this
    // repo (2026-10-16) and on a copy of the archived `20261013-composer-pill-wheel-select`
    // plan, which has a RUN.md. Long briefs and lists are cut.
    const BOARD: &str = include_str!("fixtures/board.json");
    const BOARD_RUN: &str = include_str!("fixtures/board_run.json");
    const OVERVIEW: &str = include_str!("fixtures/overview.json");
    const OVERVIEW_RUN: &str = include_str!("fixtures/overview_run.json");
    const DECISION: &str = include_str!("fixtures/decision.json");

    #[test]
    fn a_real_watchtower_read_payload_decodes() {
        let board: Board = serde_json::from_str(BOARD).unwrap();
        assert_eq!(board.tasks.len(), 3);
        assert_eq!(board.tasks[0].id, "TASK-157");
        assert_eq!(board.tasks[0].task_class.as_deref(), Some("risky"));
        assert_eq!(board.tasks[0].blocker, None);
        assert_eq!(board.run, None);
        assert_eq!(board.message, None);

        let board: Board = serde_json::from_str(BOARD_RUN).unwrap();
        let run = board.run.unwrap();
        assert_eq!(run.runner.as_deref(), Some("loop"));
        assert_eq!(run.schedule, None);
        assert_eq!(run.finished.as_deref(), Some("2026-10-13 14:41"));
        assert_eq!(run.total, 4);
        assert_eq!(run.log[0].task, "TASK-082");
    }

    #[test]
    fn a_real_watchtower_overview_read_payload_decodes() {
        let overview: Overview = serde_json::from_str(OVERVIEW).unwrap();
        assert_eq!(overview.plan.status.as_deref(), Some("ACTIVE"));
        assert_eq!(overview.tasks[0].group.as_deref(), Some("A"));
        assert!(overview.tasks[0].dep_ids.is_empty());
        assert_eq!(overview.questions[0].status, "DEFAULTED");
        assert!(overview.history[0].has_learn);
        assert_eq!(overview.run, None);

        let overview: Overview = serde_json::from_str(OVERVIEW_RUN).unwrap();
        assert_eq!(overview.tasks[1].dep_ids, ["TASK-079"]);
        let run = overview.run.unwrap();
        assert_eq!(run.now_minute, Some(34));
        assert!(!run.stopped);
        assert_eq!(run.log[1].start_minute, Some(7));
        assert_eq!(run.log[1].end_minute, Some(23));
    }

    #[test]
    fn a_real_watchtower_decision_read_payload_decodes() {
        let detail: DecisionDetail = serde_json::from_str(DECISION).unwrap();
        assert_eq!(detail.id, "ADR-0045");
        assert_eq!(detail.status, "accepted");
        assert!(detail.file.starts_with("watchtower/decisions/ADR-0045-"));
        assert!(detail.markdown.starts_with("# ADR-0045 "));
    }

    #[test]
    fn nulls_decode_to_none_and_an_unknown_status_stays() {
        let task: Task = serde_json::from_value(json!({
            "id": "TASK-1", "title": "TASK-1 X", "status": "MAYBE", "deps": "-", "notes": "",
            "spec": "", "taskClass": null, "brief": null, "blocker": null, "error": null,
        }))
        .unwrap();
        assert_eq!(task.status, "MAYBE");
        assert_eq!((task.task_class, task.brief), (None, None));
        let entry: OverviewRunEntry = serde_json::from_value(json!({
            "task": "TASK-1", "result": "DONE", "detail": "", "start": "now", "end": "-",
            "startMinute": null, "endMinute": null,
        }))
        .unwrap();
        assert_eq!((entry.start_minute, entry.end_minute), (None, None));
    }

    #[test]
    fn inputs_encode_in_camel_case() {
        assert_eq!(
            serde_json::to_value(WorkspaceInput { workspace_id: "w1" }).unwrap(),
            json!({"workspaceId": "w1"})
        );
        assert_eq!(
            serde_json::to_value(ProjectInput { project_id: "p1" }).unwrap(),
            json!({"projectId": "p1"})
        );
        assert_eq!(
            serde_json::to_value(DecisionInput {
                workspace_id: "w1",
                id: "ADR-0003"
            })
            .unwrap(),
            json!({"workspaceId": "w1", "id": "ADR-0003"})
        );
    }

    #[test]
    fn the_attachment_key_names_the_workspace_and_task() {
        assert_eq!(attachment_key("w1", "TASK-3"), "workspace:w1 TASK-3");
    }

    #[test]
    fn decision_ids_have_one_to_six_digits() {
        assert!(is_decision_id("ADR-0045"));
        assert!(is_decision_id("ADR-1"));
        assert!(!is_decision_id("ADR-1234567"));
        assert!(!is_decision_id("ADR-"));
        assert!(!is_decision_id("ADR-12a"));
        assert!(!is_decision_id("../ADR-0001"));
    }
}
