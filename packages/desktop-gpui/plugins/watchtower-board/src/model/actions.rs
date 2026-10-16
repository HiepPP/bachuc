//! The prompts the board sends and the agent that gets them (`client/actions.ts`). The board
//! edits no file; each action sends a prompt to an agent of the workspace, which changes the files
//! through the watchtower skill.

use super::time::parse_iso_ms;
use regex::Regex;
use std::cmp::Reverse;
use std::collections::BTreeMap;
use std::sync::LazyLock;

pub(crate) const PARENT_LABEL: &str = "paseo.parent-agent-id";

static TRAILING_PERIODS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\.+$").unwrap());

/// Answers, defaults, and check texts are sentences. Without their final period, the prompt does
/// not end a clause with two.
fn bare(text: &str) -> String {
    TRAILING_PERIODS.replace(text.trim(), "").into_owned()
}

pub(crate) fn answer_prompt(id: &str, question: &str, answer: &str) -> String {
    format!(
        "Watchtower: the owner answers {id} (\"{question}\"): {}. Use the watchtower skill: write the answer into its watchtower/QUESTIONS.md row, set ANSWERED, add it to the Boundaries of the specs it changes, and unblock the tasks it blocked.",
        bare(answer)
    )
}

pub(crate) fn default_answer_prompt(id: &str, question: &str, default_answer: &str) -> String {
    answer_prompt(id, question, &format!("use the default: {default_answer}"))
}

pub(crate) fn accept_decision_prompt(id: &str, title: &str) -> String {
    format!(
        "Watchtower: the owner accepts the proposed {id} ({title}). Use the watchtower skill decision rules: set Status: accepted in the ADR file and its watchtower/DECISIONS.md row, and add its ## Avoid item to watchtower/MEMORY.md (R4)."
    )
}

pub(crate) fn reject_decision_prompt(id: &str, title: &str) -> String {
    format!(
        "Watchtower: the owner rejects the proposed {id} ({title}). Use the watchtower skill decision rules: delete the draft ADR file and its watchtower/DECISIONS.md index row."
    )
}

pub(crate) fn manual_checks_prompt(checks: &[String]) -> String {
    let checks: Vec<String> = checks.iter().map(|check| bare(check)).collect();
    format!(
        "Watchtower: the owner ran the pending manual checks: {}. Use the watchtower skill: record them as done in the Handoff of watchtower/NEXT.md.",
        checks.join("; ")
    )
}

/// One agent as the target and builder rules see it, read from `Client` state: the summary,
/// `agent_labels`, `agent_attention`, and `agent_updated_at`. Archived agents are already absent.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct AgentCandidate {
    pub id: String,
    pub title: String,
    pub status: String,
    pub workspace_id: Option<String>,
    pub provider: String,
    pub labels: BTreeMap<String, String>,
    pub pending_permissions: usize,
    /// `requiresAttention` with the `permission` reason.
    pub permission_attention: bool,
    pub updated_at: Option<String>,
}

impl AgentCandidate {
    pub(crate) fn name(&self) -> String {
        let title = self.title.trim();
        if title.is_empty() {
            self.id.clone()
        } else {
            title.to_string()
        }
    }

    pub(crate) fn label(&self, key: &str) -> Option<&str> {
        self.labels
            .get(key)
            .map(String::as_str)
            .filter(|value| !value.is_empty())
    }

    fn updated_ms(&self) -> Option<i64> {
        self.updated_at.as_deref().and_then(parse_iso_ms)
    }
}

/// Sorts newest `updated_at` first; an unreadable time sorts last. The sort is stable.
pub(crate) fn sort_newest_first(agents: &mut [&AgentCandidate]) {
    agents.sort_by_key(|agent| Reverse(agent.updated_ms().map_or(i64::MIN, |at| at)));
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum AgentTarget {
    Ready {
        id: String,
        name: String,
    },
    /// Every agent of the workspace has a turn in flight.
    Busy,
    NoAgent,
}

/// Sending to an agent with a turn in flight interrupts that turn (the daemon default), so a
/// prompt goes to the newest agent that is not working. A subagent belongs to its parent's turn,
/// and a mock-provider agent is a test fixture (ADR-0013), so neither is a target.
pub(crate) fn choose_agent(agents: &[AgentCandidate], workspace_id: &str) -> AgentTarget {
    let mut own: Vec<&AgentCandidate> = agents
        .iter()
        .filter(|agent| {
            agent.workspace_id.as_deref() == Some(workspace_id)
                && agent.label(PARENT_LABEL).is_none()
                && !agent.provider.starts_with("mock")
        })
        .collect();
    if own.is_empty() {
        return AgentTarget::NoAgent;
    }
    sort_newest_first(&mut own);
    match own
        .iter()
        .find(|agent| agent.status != "running" && agent.status != "initializing")
    {
        Some(idle) => AgentTarget::Ready {
            id: idle.id.clone(),
            name: idle.name(),
        },
        None => AgentTarget::Busy,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // actions.test.ts
    #[test]
    fn prompts_name_the_watchtower_skill_and_the_file_to_change() {
        assert_eq!(
            answer_prompt("Q-008", "Network access?", "  No network "),
            "Watchtower: the owner answers Q-008 (\"Network access?\"): No network. Use the watchtower skill: write the answer into its watchtower/QUESTIONS.md row, set ANSWERED, add it to the Boundaries of the specs it changes, and unblock the tasks it blocked."
        );
        assert_eq!(
            default_answer_prompt("Q-010", "Native scroll?", "Open the agent"),
            "Watchtower: the owner answers Q-010 (\"Native scroll?\"): use the default: Open the agent. Use the watchtower skill: write the answer into its watchtower/QUESTIONS.md row, set ANSWERED, add it to the Boundaries of the specs it changes, and unblock the tasks it blocked."
        );
        assert_eq!(
            accept_decision_prompt("ADR-0003", "Chip-only sources"),
            "Watchtower: the owner accepts the proposed ADR-0003 (Chip-only sources). Use the watchtower skill decision rules: set Status: accepted in the ADR file and its watchtower/DECISIONS.md row, and add its ## Avoid item to watchtower/MEMORY.md (R4)."
        );
        assert_eq!(
            reject_decision_prompt("ADR-0003", "Chip-only sources"),
            "Watchtower: the owner rejects the proposed ADR-0003 (Chip-only sources). Use the watchtower skill decision rules: delete the draft ADR file and its watchtower/DECISIONS.md index row."
        );
        assert_eq!(
            manual_checks_prompt(&[
                "Stop a parent (TASK-001).".into(),
                "Queued message (TASK-003).".into()
            ]),
            "Watchtower: the owner ran the pending manual checks: Stop a parent (TASK-001); Queued message (TASK-003). Use the watchtower skill: record them as done in the Handoff of watchtower/NEXT.md."
        );
        assert!(
            default_answer_prompt("Q-001", "Read-only?", "Read-only. Copy actions.")
                .contains(": use the default: Read-only. Copy actions. Use the watchtower skill")
        );
    }

    #[test]
    fn trailing_periods_are_stripped_from_answers() {
        assert!(answer_prompt("Q-1", "Why?", "Because...").contains("): Because. Use"));
    }

    fn agent(id: &str, updated_at: &str) -> AgentCandidate {
        AgentCandidate {
            id: id.into(),
            status: "idle".into(),
            workspace_id: Some("ws-1".into()),
            provider: "claude".into(),
            updated_at: Some(updated_at.into()),
            ..AgentCandidate::default()
        }
    }

    const T10: &str = "2026-10-09T10:00:00.000Z";

    #[test]
    fn answers_skip_a_newer_idle_mock_agent_for_an_older_real_one_or_report_none() {
        let real = AgentCandidate {
            title: "Real".into(),
            ..agent("real", "2026-10-09T08:00:00.000Z")
        };
        let mock = AgentCandidate {
            provider: "mock".into(),
            ..agent("mock", "2026-10-09T12:00:00.000Z")
        };
        assert_eq!(
            choose_agent(&[mock.clone(), real], "ws-1"),
            AgentTarget::Ready {
                id: "real".into(),
                name: "Real".into()
            }
        );
        assert_eq!(choose_agent(&[mock], "ws-1"), AgentTarget::NoAgent);
    }

    #[test]
    fn the_target_is_the_newest_idle_agent_of_the_workspace() {
        let target = choose_agent(
            &[
                AgentCandidate {
                    title: "Old".into(),
                    ..agent("old", "2026-10-09T08:00:00.000Z")
                },
                AgentCandidate {
                    title: " Newest ".into(),
                    ..agent("new", "2026-10-09T11:00:00.000Z")
                },
                AgentCandidate {
                    workspace_id: Some("ws-2".into()),
                    ..agent("other", "2026-10-09T12:00:00.000Z")
                },
            ],
            "ws-1",
        );
        assert_eq!(
            target,
            AgentTarget::Ready {
                id: "new".into(),
                name: "Newest".into()
            }
        );
    }

    #[test]
    fn a_running_agent_is_skipped_for_an_older_idle_one_and_the_name_falls_back_to_the_id() {
        let target = choose_agent(
            &[
                AgentCandidate {
                    status: "running".into(),
                    ..agent("busy", "2026-10-09T12:00:00.000Z")
                },
                AgentCandidate {
                    status: "initializing".into(),
                    ..agent("starting", "2026-10-09T11:00:00.000Z")
                },
                AgentCandidate {
                    status: "error".into(),
                    ..agent("idle", "2026-10-09T09:00:00.000Z")
                },
            ],
            "ws-1",
        );
        assert_eq!(
            target,
            AgentTarget::Ready {
                id: "idle".into(),
                name: "idle".into()
            }
        );
    }

    #[test]
    fn only_working_agents_give_busy_and_no_agent_gives_none() {
        let busy = AgentCandidate {
            status: "running".into(),
            ..agent("busy", T10)
        };
        let starting = AgentCandidate {
            status: "initializing".into(),
            ..agent("starting", T10)
        };
        assert_eq!(choose_agent(&[busy, starting], "ws-1"), AgentTarget::Busy);
        assert_eq!(choose_agent(&[], "ws-1"), AgentTarget::NoAgent);
        let elsewhere = AgentCandidate {
            workspace_id: Some("ws-2".into()),
            ..agent("a", T10)
        };
        assert_eq!(choose_agent(&[elsewhere], "ws-1"), AgentTarget::NoAgent);
        let nowhere = AgentCandidate {
            workspace_id: None,
            ..agent("a", T10)
        };
        assert_eq!(choose_agent(&[nowhere], "ws-1"), AgentTarget::NoAgent);
    }

    #[test]
    fn a_subagent_is_never_the_target() {
        let child = AgentCandidate {
            labels: BTreeMap::from([(PARENT_LABEL.to_string(), "parent".to_string())]),
            ..agent("child", T10)
        };
        assert_eq!(choose_agent(&[child.clone()], "ws-1"), AgentTarget::NoAgent);
        assert_eq!(
            choose_agent(
                &[child, agent("parent", "2026-10-09T01:00:00.000Z")],
                "ws-1"
            ),
            AgentTarget::Ready {
                id: "parent".into(),
                name: "parent".into()
            }
        );
    }

    #[test]
    fn an_unreadable_time_sorts_last() {
        let unreadable = agent("unreadable", "not a date");
        let older = agent("older", "2026-10-09T01:00:00.000Z");
        assert_eq!(
            choose_agent(&[unreadable, older], "ws-1"),
            AgentTarget::Ready {
                id: "older".into(),
                name: "older".into()
            }
        );
    }
}
