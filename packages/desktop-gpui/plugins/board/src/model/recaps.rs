//! The `board.recaps` wire types, `recapDayMarkdown` (`shared/recaps.ts`), and the flat item
//! list the Recaps view draws (`client/recaps.tsx`).

use super::SEPARATOR;
use anyhow::Context as _;
use serde::Deserialize;
use serde_json::Value;

pub(crate) const RECAPS_POLL_MS: i64 = 30_000;
/// The days `board.recaps` reads.
pub(crate) const RECAP_DAYS: u32 = 7;

/// One recap (`recapEntrySchema`).
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RecapEntry {
    pub agent_id: String,
    pub title: String,
    pub ended_at: String,
    #[serde(default)]
    pub branch: Option<String>,
    #[serde(default)]
    pub did: Option<String>,
    #[serde(default)]
    pub commit_push: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct RecapProject {
    pub key: String,
    pub name: String,
    pub project_id: Option<String>,
    pub entries: Vec<RecapEntry>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct RecapDay {
    pub day: String,
    pub projects: Vec<RecapProject>,
}

fn text<'a>(value: &'a Value, key: &str) -> anyhow::Result<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .with_context(|| format!("no {key}"))
}

fn skipped<T>(what: &str, result: anyhow::Result<T>) -> Option<T> {
    result
        .inspect_err(|error| {
            log::warn!("board: skipped a recap {what} that does not decode: {error:#}")
        })
        .ok()
}

/// Decodes a `board.recaps` answer. Each day, project, and entry decodes on its own: a bad row
/// is skipped and logged.
pub(crate) fn decode_recaps(value: &Value) -> anyhow::Result<Vec<RecapDay>> {
    let days = value
        .get("days")
        .and_then(Value::as_array)
        .context("board.recaps answer has no days")?;
    Ok(days
        .iter()
        .filter_map(|day| skipped("day", decode_day(day)))
        .collect())
}

fn decode_day(value: &Value) -> anyhow::Result<RecapDay> {
    let projects = value
        .get("projects")
        .and_then(Value::as_array)
        .context("no projects")?
        .iter()
        .filter_map(|project| skipped("project", decode_project(project)))
        .collect();
    Ok(RecapDay {
        day: text(value, "day")?.to_string(),
        projects,
    })
}

fn decode_project(value: &Value) -> anyhow::Result<RecapProject> {
    let entries = value
        .get("entries")
        .and_then(Value::as_array)
        .context("no entries")?
        .iter()
        .filter_map(|entry| {
            skipped(
                "entry",
                RecapEntry::deserialize(entry).map_err(anyhow::Error::from),
            )
        })
        .collect();
    Ok(RecapProject {
        key: text(value, "key")?.to_string(),
        name: text(value, "name")?.to_string(),
        project_id: value
            .get("projectId")
            .and_then(Value::as_str)
            .map(str::to_string),
        entries,
    })
}

/// A JS truthy string: present and not empty.
fn truthy(value: &Option<String>) -> Option<&str> {
    value.as_deref().filter(|value| !value.is_empty())
}

/// `recapDayMarkdown`: the day as a markdown section for the Copy button.
pub(crate) fn recap_day_markdown(day: &RecapDay) -> String {
    let mut lines = vec![format!("## {}", day.day)];
    for project in &day.projects {
        lines.push(String::new());
        lines.push(format!("### {}", project.name));
        for entry in &project.entries {
            let branch = truthy(&entry.branch).map(|branch| format!("Branch: {branch}"));
            let commit = truthy(&entry.commit_push).map(|commit| format!("Commit/push: {commit}"));
            let details: Vec<String> = [truthy(&entry.did).map(str::to_string), branch, commit]
                .into_iter()
                .flatten()
                .collect();
            let tail = if details.is_empty() {
                String::new()
            } else {
                format!(" — {}", details.join(SEPARATOR))
            };
            lines.push(format!("- **{}**{tail}", entry.title));
        }
    }
    format!("{}\n", lines.join("\n"))
}

/// The meta line under an entry: branch and commit/push.
pub(crate) fn recap_meta(entry: &RecapEntry) -> String {
    [truthy(&entry.branch), truthy(&entry.commit_push)]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(SEPARATOR)
}

/// One item of the Recaps list: a day header, a project header, or an entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RecapItem {
    Day(usize),
    Project(usize, usize),
    Entry(usize, usize, usize),
}

pub(crate) fn recap_items(days: &[RecapDay]) -> Vec<RecapItem> {
    let mut items = Vec::new();
    for (d, day) in days.iter().enumerate() {
        items.push(RecapItem::Day(d));
        for (p, project) in day.projects.iter().enumerate() {
            items.push(RecapItem::Project(d, p));
            items.extend((0..project.entries.len()).map(|e| RecapItem::Entry(d, p, e)));
        }
    }
    items
}

/// The Copy button of one day.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum CopyState {
    #[default]
    Idle,
    Copied,
    Failed,
}

pub(crate) fn copy_text(state: CopyState) -> &'static str {
    match state {
        CopyState::Idle => "Copy",
        CopyState::Copied => "Copied",
        CopyState::Failed => "Copy failed",
    }
}

/// The message in place of the list: loading shows a spinner; `None` draws the days.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RecapsBody {
    Loading,
    Message(&'static str),
    Days,
}

pub(crate) fn recaps_body(days: Option<&[RecapDay]>, failed: bool) -> RecapsBody {
    match days {
        None if failed => RecapsBody::Message("Could not load recaps. Retrying."),
        None => RecapsBody::Loading,
        Some([]) => RecapsBody::Message("No recaps in the last 7 days."),
        Some(_) => RecapsBody::Days,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn entry(did: Option<&str>, commit: Option<&str>) -> RecapEntry {
        RecapEntry {
            agent_id: "agent-1".into(),
            title: "Fix login".into(),
            ended_at: "2026-09-24T10:00:00.000Z".into(),
            branch: Some("main".into()),
            did: did.map(Into::into),
            commit_push: commit.map(Into::into),
        }
    }

    #[test]
    fn the_day_markdown_matches_the_electron_output() {
        let markdown = recap_day_markdown(&RecapDay {
            day: "2026-09-24".into(),
            projects: vec![RecapProject {
                key: "k".into(),
                name: "alpha".into(),
                project_id: None,
                entries: vec![entry(Some("Fixed it"), Some("none")), entry(None, None)],
            }],
        });
        assert_eq!(
            markdown,
            [
                "## 2026-09-24",
                "",
                "### alpha",
                "- **Fix login** — Fixed it · Branch: main · Commit/push: none",
                "- **Fix login** — Branch: main",
                "",
            ]
            .join("\n")
        );
        let mut bare = entry(Some(""), None);
        bare.branch = None;
        let day = RecapDay {
            day: "d".into(),
            projects: vec![RecapProject {
                key: "k".into(),
                name: "p".into(),
                project_id: None,
                entries: vec![bare],
            }],
        };
        assert_eq!(recap_day_markdown(&day), "## d\n\n### p\n- **Fix login**\n");
    }

    #[test]
    fn the_meta_line_joins_branch_and_commit() {
        assert_eq!(recap_meta(&entry(None, Some("pushed"))), "main · pushed");
        let mut none = entry(None, None);
        none.branch = None;
        assert_eq!(recap_meta(&none), "");
    }

    #[test]
    fn the_items_list_each_day_project_and_entry() {
        let days = vec![
            RecapDay {
                day: "a".into(),
                projects: vec![RecapProject {
                    key: "k".into(),
                    name: "p".into(),
                    project_id: None,
                    entries: vec![entry(None, None), entry(None, None)],
                }],
            },
            RecapDay {
                day: "b".into(),
                projects: Vec::new(),
            },
        ];
        assert_eq!(
            recap_items(&days),
            [
                RecapItem::Day(0),
                RecapItem::Project(0, 0),
                RecapItem::Entry(0, 0, 0),
                RecapItem::Entry(0, 0, 1),
                RecapItem::Day(1),
            ]
        );
    }

    #[test]
    fn the_body_shows_loading_an_error_empty_or_the_days() {
        assert_eq!(recaps_body(None, false), RecapsBody::Loading);
        assert_eq!(
            recaps_body(None, true),
            RecapsBody::Message("Could not load recaps. Retrying.")
        );
        assert_eq!(
            recaps_body(Some(&[]), false),
            RecapsBody::Message("No recaps in the last 7 days.")
        );
        let days = [RecapDay {
            day: "a".into(),
            projects: Vec::new(),
        }];
        assert_eq!(recaps_body(Some(&days), true), RecapsBody::Days);
        assert_eq!(copy_text(CopyState::Idle), "Copy");
        assert_eq!(copy_text(CopyState::Copied), "Copied");
        assert_eq!(copy_text(CopyState::Failed), "Copy failed");
    }

    #[test]
    fn a_real_recaps_payload_decodes_and_skips_a_bad_entry() {
        let days = decode_recaps(&json!({
            "days": [{
                "day": "2026-09-24",
                "projects": [{
                    "key": "project:alpha",
                    "name": "alpha",
                    "projectId": "p1",
                    "entries": [
                        {
                            "agentId": "agent-1",
                            "turnId": "turn-1",
                            "title": "Fix login",
                            "cwd": "/Users/me/Projects/alpha",
                            "workspaceId": "ws-1",
                            "project": "alpha",
                            "projectKey": "project:alpha",
                            "endedAt": "2026-09-24T10:00:00.000Z",
                            "day": "2026-09-24",
                            "branch": "main",
                            "did": "Fixed it",
                            "commitPush": null,
                            "raw": "## Recap"
                        },
                        { "agentId": "agent-2", "title": 7 }
                    ]
                }]
            }]
        }))
        .expect("decodes");
        assert_eq!(days.len(), 1);
        let project = &days[0].projects[0];
        assert_eq!(project.project_id.as_deref(), Some("p1"));
        assert_eq!(project.entries.len(), 1);
        assert_eq!(project.entries[0].did.as_deref(), Some("Fixed it"));
        assert_eq!(project.entries[0].commit_push, None);
        assert!(decode_recaps(&json!({})).is_err());
    }
}
