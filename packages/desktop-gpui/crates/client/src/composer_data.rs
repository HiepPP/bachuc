//! Wire decoders and request builders for the composer, New workspace, and tab menu data.

use crate::model::{
    ActiveTurn, AgentUsage, BranchSuggestion, CreateWorkspace, CreatedWorkspace, DiffCompare,
    DiffMode, DraftConfig, InitialAgent, SubagentKind, SubagentRow, TaskEntry, TaskStatus,
    TurnMetrics, WorkspaceSource, WorktreeAction,
};
use serde_json::{Map, Value, json};

/// The label that names an agent's parent agent (`packages/protocol/src/agent-labels.ts`).
pub(crate) const PARENT_AGENT_LABEL: &str = "paseo.parent-agent-id";

fn uint(value: &Value) -> Option<u64> {
    value.as_f64().filter(|v| *v >= 0.0).map(|v| v as u64)
}

fn text(value: &Value) -> Option<String> {
    value.as_str().map(str::to_string)
}

pub(crate) fn turn_metrics(value: &Value) -> Option<TurnMetrics> {
    value.is_object().then(|| TurnMetrics {
        ttft_ms: value["ttftMs"].as_f64(),
        output_tokens: uint(&value["outputTokens"]),
        output_tokens_per_second: value["outputTokensPerSecond"].as_f64(),
    })
}

pub(crate) fn active_turn(value: &Value) -> Option<ActiveTurn> {
    Some(ActiveTurn {
        turn_id: text(&value["turnId"])?,
        started_at: text(&value["startedAt"]),
    })
}

pub(crate) fn agent_usage(value: &Value) -> Option<AgentUsage> {
    value.is_object().then(|| AgentUsage {
        input_tokens: uint(&value["inputTokens"]),
        cached_input_tokens: uint(&value["cachedInputTokens"]),
        output_tokens: uint(&value["outputTokens"]),
        total_cost_usd: value["totalCostUsd"].as_f64(),
        context_window_max_tokens: uint(&value["contextWindowMaxTokens"]),
        context_window_used_tokens: uint(&value["contextWindowUsedTokens"]),
    })
}

/// The items of a `todo` timeline item.
pub(crate) fn task_entries(item: &Value) -> Vec<TaskEntry> {
    item["items"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|task| TaskEntry {
            id: text(&task["id"]),
            text: task["text"].as_str().unwrap_or_default().to_string(),
            completed: task["completed"] == true,
            status: match task["status"].as_str() {
                Some("pending") => Some(TaskStatus::Pending),
                Some("in_progress") => Some(TaskStatus::InProgress),
                Some("completed") => Some(TaskStatus::Completed),
                _ => None,
            },
            active_form: text(&task["activeForm"]),
        })
        .collect()
}

/// A provider-owned subagent and whether it nests under another provider subagent.
#[derive(Clone)]
pub(crate) struct ProviderSubagent {
    pub parent_agent_id: String,
    pub nested: bool,
    pub row: SubagentRow,
}

pub(crate) fn provider_subagent(value: &Value) -> Option<ProviderSubagent> {
    let status = value["status"].as_str().unwrap_or_default().to_string();
    Some(ProviderSubagent {
        parent_agent_id: text(&value["parentAgentId"])?,
        nested: value["parentSubagentId"].is_string(),
        row: SubagentRow {
            id: text(&value["id"])?,
            kind: SubagentKind::Provider,
            provider: value["provider"].as_str().unwrap_or_default().to_string(),
            title: text(&value["title"]),
            description: text(&value["description"]),
            subtitle: text(&value["subtitle"]),
            requires_attention: status == "failed",
            status,
            created_at: value["createdAt"].as_str().unwrap_or_default().to_string(),
        },
    })
}

/// A `branch_suggestions_response` payload: `branchDetails` when present, else `branches`.
pub(crate) fn branch_suggestions(payload: &Value) -> Vec<BranchSuggestion> {
    if let Some(details) = payload["branchDetails"].as_array() {
        return details
            .iter()
            .filter_map(|branch| {
                Some(BranchSuggestion {
                    name: text(&branch["name"])?,
                    committer_date: branch["committerDate"].as_f64(),
                    has_local: branch["hasLocal"].as_bool(),
                    has_remote: branch["hasRemote"].as_bool(),
                    local_ahead: uint(&branch["localAhead"]).map(|v| v as u32),
                    local_behind: uint(&branch["localBehind"]).map(|v| v as u32),
                })
            })
            .collect();
    }
    payload["branches"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|name| {
            Some(BranchSuggestion {
                name: text(name)?,
                committer_date: None,
                has_local: None,
                has_remote: None,
                local_ahead: None,
                local_behind: None,
            })
        })
        .collect()
}

/// `{provider, cwd, model?, modeId?, thinkingOptionId?}`, with omitted fields left out.
pub(crate) fn draft_config(config: &DraftConfig) -> Value {
    let mut value = json!({"provider": config.provider, "cwd": config.cwd});
    for (key, field) in [
        ("model", &config.model),
        ("modeId", &config.mode_id),
        ("thinkingOptionId", &config.thinking_option_id),
    ] {
        if let Some(field) = field {
            value[key] = json!(field);
        }
    }
    value
}

/// The agent part of a create request: config with feature values, env, prompt, images,
/// attachments, and labels. Empty optional parts are left out.
pub(crate) fn initial_agent(agent: &InitialAgent) -> Map<String, Value> {
    let mut config = draft_config(&agent.config);
    if !agent.feature_values.is_empty() {
        config["featureValues"] = json!(agent.feature_values);
    }
    let mut value = Map::new();
    value.insert("config".into(), config);
    if !agent.env.is_empty() {
        value.insert("env".into(), json!(agent.env));
    }
    value.insert("initialPrompt".into(), json!(agent.initial_prompt));
    value.insert(
        "clientMessageId".into(),
        json!(uuid::Uuid::new_v4().to_string()),
    );
    if !agent.images.is_empty() {
        value.insert("images".into(), Value::Array(agent.images.clone()));
    }
    if !agent.attachments.is_empty() {
        value.insert(
            "attachments".into(),
            Value::Array(agent.attachments.clone()),
        );
    }
    value.insert("labels".into(), json!(agent.labels));
    value
}

/// A `workspace.create.request` without its request id.
/// `subscribe` asks for `workspace.create.update` progress, keyed by `idempotency_key`.
pub(crate) fn create_workspace(
    request: &CreateWorkspace,
    idempotency_key: Option<&str>,
    subscribe: bool,
) -> Value {
    fn put(value: &mut Value, key: &str, field: &Option<String>) {
        if let Some(field) = field {
            value[key] = json!(field);
        }
    }
    let source = match &request.source {
        WorkspaceSource::Directory { path, project_id } => {
            let mut source = json!({"kind": "directory", "path": path});
            put(&mut source, "projectId", project_id);
            source
        }
        WorkspaceSource::Worktree {
            cwd,
            project_id,
            action,
            ref_name,
            base_branch,
            branch_name,
            worktree_slug,
        } => {
            let mut source = json!({"kind": "worktree", "cwd": cwd});
            put(&mut source, "projectId", project_id);
            if let Some(action) = action {
                source["action"] = json!(match action {
                    WorktreeAction::BranchOff => "branch-off",
                    WorktreeAction::Checkout => "checkout",
                });
            }
            put(&mut source, "refName", ref_name);
            put(&mut source, "baseBranch", base_branch);
            put(&mut source, "branchName", branch_name);
            put(&mut source, "worktreeSlug", worktree_slug);
            source
        }
    };
    let mut message = json!({"type": "workspace.create.request", "source": source});
    if let Some(key) = idempotency_key {
        message["idempotencyKey"] = json!(key);
    }
    if subscribe {
        message["subscribe"] = json!(true);
    }
    put(&mut message, "title", &request.title);
    if let Some(prompt) = request
        .first_agent_prompt
        .as_ref()
        .filter(|prompt| !prompt.is_empty())
    {
        message["firstAgentContext"] = json!({"prompt": prompt});
    }
    if let Some(agent) = &request.agent {
        message["agent"] = Value::Object(initial_agent(agent));
    }
    message
}

/// A `workspace.create.update` whose agent exists. The agent's tab can open from it; the
/// response waits until the provider starts the first turn, which takes seconds.
pub(crate) fn created_workspace_update(payload: &Value) -> Option<CreatedWorkspace> {
    if !matches!(
        payload["phase"].as_str(),
        Some("agent_ready" | "prompt_started" | "completed")
    ) {
        return None;
    }
    Some(CreatedWorkspace {
        workspace_id: payload["workspaceId"].as_str()?.to_string(),
        agent_id: Some(payload["agent"]["id"].as_str()?.to_string()),
        setup_terminal_id: None,
    })
}

/// `{mode, baseRef?, ignoreWhitespace?}`; `ignoreWhitespace` goes only when set.
pub(crate) fn diff_compare(compare: &DiffCompare) -> Value {
    let mode = match compare.mode {
        DiffMode::Uncommitted => "uncommitted",
        DiffMode::Base => "base",
    };
    let mut value = json!({"mode": mode});
    if let Some(base_ref) = &compare.base_ref {
        value["baseRef"] = json!(base_ref);
    }
    if compare.ignore_whitespace {
        value["ignoreWhitespace"] = json!(true);
    }
    value
}
