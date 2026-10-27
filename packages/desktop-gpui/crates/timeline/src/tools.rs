//! What a tool call shows: its badge (icon, name, summary) from `buildToolCallDisplayModel` in
//! `packages/protocol/src/tool-call-display.ts` and `resolveToolCallIconName`, and its detail
//! sections from `buildDetailSections` in `packages/app/src/components/tool-call-details.tsx`.

use bachuc_icons::IconName;
use serde_json::Value;

use crate::diff::{self, DiffLine};

/// Detail views without a fill height stop at this height and scroll, as `maxHeight` 300.
pub const DETAIL_MAX_HEIGHT: f32 = 300.;
/// A permission card's detail stops at 200, as the app's `PermissionRequestCard`.
pub const PERMISSION_DETAIL_MAX_HEIGHT: f32 = 200.;

/// The collapsed badge row of a tool call.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Presentation {
    pub name: String,
    pub summary: Option<String>,
    pub icon: IconName,
    /// A read, edit, or write call shows the open-file button.
    pub file_path: Option<String>,
    pub error: Option<String>,
}

/// One hook that ran on a tool call, from the `hooks` of its item.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HookLine {
    pub name: String,
    pub status: String,
    pub exit_code: Option<i64>,
    pub output: Option<String>,
}

/// One block of a tool detail view.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Section {
    /// A shell command and its output, full bleed: `$ ` in muted text, then the command.
    Shell { command: String, output: String },
    /// Monospace text in a bordered scroll area. `path` names the language for syntax colors
    /// and `start_line` adds a line gutter.
    Code {
        text: String,
        path: Option<String>,
        start_line: Option<u64>,
        full_bleed: bool,
    },
    /// An edit diff.
    Diff(Vec<DiffLine>),
    /// Sub-agent activity: its session, its `[Tool] summary` actions, and the rest of the log.
    SubAgent {
        session: Option<String>,
        actions: Vec<(String, Option<String>)>,
        log: String,
    },
    /// Interface text in a bordered scroll area.
    Plain(String),
    /// A group header such as "Input".
    Header(String),
    /// The error of a failed call.
    Error(String),
}

fn text(value: &Value) -> Option<String> {
    value.as_str().filter(|s| !s.is_empty()).map(str::to_string)
}

fn strip_cwd(path: &str, cwd: &str) -> String {
    let cwd = cwd.trim_end_matches('/');
    if cwd.is_empty() {
        return path.to_string();
    }
    path.strip_prefix(cwd)
        .and_then(|rest| rest.strip_prefix('/'))
        .map_or_else(|| path.to_string(), str::to_string)
}

fn humanize(name: &str) -> String {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return name.to_string();
    }
    if let Some(leaf) = paseo_leaf(trimmed) {
        return humanize(leaf);
    }
    if trimmed.contains([':', '.', '/']) || trimmed.contains("__") {
        return trimmed.to_string();
    }
    let words = trimmed
        .split(['.', '_', '-'])
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase();
    let mut chars = words.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => words,
    }
}

/// The tool name inside `mcp__paseo__<leaf>`.
fn paseo_leaf(name: &str) -> Option<&str> {
    let lower = name.to_lowercase();
    let parts: Vec<&str> = name.split("__").filter(|s| !s.is_empty()).collect();
    let is_paseo = parts.len() >= 3
        && lower.starts_with("mcp__")
        && (parts[1].eq_ignore_ascii_case("paseo")
            || parts[1].to_lowercase().starts_with("paseo_"));
    is_paseo.then(|| *parts.last().unwrap_or(&name))
}

fn error_text(error: &Value) -> Option<String> {
    match error {
        Value::Null => None,
        Value::String(text) => Some(text.clone()),
        Value::Object(map) if map.get("content").is_some_and(Value::is_string) => {
            map["content"].as_str().map(str::to_string)
        }
        other => serde_json::to_string_pretty(other).ok(),
    }
}

fn icon_for(name: &str, detail: &Value) -> IconName {
    let lower = name.trim().to_lowercase();
    let kind = detail["type"].as_str().unwrap_or("");
    if kind == "plain_text" && detail["icon"] == "sparkles" {
        return IconName::Sparkles;
    }
    if lower == "thinking" && (kind.is_empty() || kind == "unknown") {
        return IconName::Brain;
    }
    if lower == "speak" {
        return IconName::MicVocal;
    }
    if lower == "hook" {
        return IconName::Zap;
    }
    if lower == "task" {
        return IconName::Bot;
    }
    match kind {
        "shell" | "worktree_setup" => IconName::SquareTerminal,
        "read" => IconName::Eye,
        "edit" | "write" => IconName::Pencil,
        "search" | "fetch" => IconName::Search,
        "sub_agent" => IconName::Bot,
        "plan" => IconName::Brain,
        _ => IconName::Wrench,
    }
}

/// The badge of a tool call item (`name`, `status`, `error`, `detail`, `metadata`).
pub fn presentation(item: &Value, cwd: &str) -> Presentation {
    let name = item["name"].as_str().unwrap_or("");
    let detail = &item["detail"];
    let lower = name.trim().to_lowercase();
    let path = |display: &str| {
        (
            Some(display.to_string()),
            text(&detail["filePath"]).map(|p| strip_cwd(&p, cwd)),
        )
    };
    let (display, summary) = match detail["type"].as_str().unwrap_or("unknown") {
        // The daemon keeps the description Claude gives a Bash call; the open badge still shows
        // the command.
        "shell" => (
            Some("Shell".into()),
            detail["description"]
                .as_str()
                .map(str::trim)
                .filter(|description| !description.is_empty())
                .map(str::to_string)
                .or_else(|| text(&detail["command"])),
        ),
        "read" => path("Read"),
        "edit" => path("Edit"),
        "write" => path("Write"),
        "search" => (Some("Search".into()), text(&detail["query"])),
        "fetch" => (Some("Fetch".into()), text(&detail["url"])),
        "worktree_setup" => (Some("Worktree setup".into()), text(&detail["branchName"])),
        "sub_agent" => (
            Some(text(&detail["subAgentType"]).unwrap_or_else(|| "Task".into())),
            text(&detail["description"]),
        ),
        "plain_text" => (None, text(&detail["label"])),
        "plan" => (Some("Plan".into()), None),
        _ => (None, None),
    };
    let is_unknown = !detail["type"].is_string() || detail["type"] == "unknown";
    let (display, summary) = if is_unknown && lower == "task" {
        (
            Some("Task".to_string()),
            text(&item["metadata"]["subAgentActivity"]),
        )
    } else if is_unknown && lower == "thinking" {
        (Some("Thinking".to_string()), summary)
    } else if lower == "hook" {
        (Some("Hook".to_string()), text(&detail["label"]))
    } else if lower == "terminal" {
        (
            Some("Terminal".to_string()),
            if detail["type"] == "plain_text" {
                text(&detail["label"])
            } else {
                None
            },
        )
    } else {
        (display, summary)
    };
    let file_path = match detail["type"].as_str() {
        Some("read" | "edit" | "write") => text(&detail["filePath"]),
        _ => None,
    };
    Presentation {
        name: display.unwrap_or_else(|| humanize(name)),
        summary,
        icon: icon_for(name, detail),
        file_path,
        error: if item["status"] == "failed" {
            error_text(&item["error"])
        } else {
            None
        },
    }
}

fn trim_leading_newlines(value: &str) -> &str {
    value.trim_start_matches('\n')
}

/// `parseSubAgentLog`: `[Tool] summary` lines become actions; the other lines stay the log.
pub fn parse_sub_agent_log(log: &str) -> (Vec<(String, Option<String>)>, String) {
    let mut actions = Vec::new();
    let mut rest: Vec<&str> = Vec::new();
    for line in trim_leading_newlines(log).split('\n') {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let action = trimmed.strip_prefix('[').and_then(|after| {
            let close = after.find(']')?;
            let tool = after[..close].trim();
            let summary = after[close + 1..].trim();
            let spaced = after[close + 1..].is_empty()
                || after[close + 1..].starts_with(char::is_whitespace);
            (!tool.is_empty() && spaced).then(|| {
                (
                    format_sub_agent_tool(tool),
                    (!summary.is_empty()).then(|| summary.to_string()),
                )
            })
        });
        match action {
            Some(action) => actions.push(action),
            None => rest.push(line),
        }
    }
    (actions, trim_leading_newlines(&rest.join("\n")).to_string())
}

fn format_sub_agent_tool(tool: &str) -> String {
    tool.split(['.', '_', '-', ' '])
        .filter(|s| !s.is_empty())
        .map(|word| {
            let mut chars = word.chars();
            chars
                .next()
                .map(|first| first.to_uppercase().chain(chars).collect::<String>())
                .unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn serialize(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        other => serde_json::to_string_pretty(other).unwrap_or_default(),
    }
}

fn is_meaningful(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::String(text) => !text.trim().is_empty(),
        Value::Array(items) => !items.is_empty(),
        Value::Object(map) => !map.is_empty(),
        _ => true,
    }
}

/// The detail sections of a tool call: `detail` and the error text of a failed call.
pub fn sections(detail: &Value, error: Option<&str>) -> Vec<Section> {
    let mut out = Vec::new();
    let path = text(&detail["filePath"]);
    match detail["type"].as_str().unwrap_or("") {
        "shell" => out.push(Section::Shell {
            command: detail["command"]
                .as_str()
                .unwrap_or("")
                .trim_end_matches('\n')
                .to_string(),
            output: trim_leading_newlines(detail["output"].as_str().unwrap_or("")).to_string(),
        }),
        "worktree_setup" => {
            let log = trim_leading_newlines(detail["log"].as_str().unwrap_or(""));
            let text = if log.is_empty() {
                format!(
                    "Preparing worktree {} at {}",
                    detail["branchName"].as_str().unwrap_or(""),
                    detail["worktreePath"].as_str().unwrap_or("")
                )
            } else {
                log.to_string()
            };
            out.push(Section::Code {
                text,
                path: None,
                start_line: None,
                full_bleed: true,
            });
        }
        "sub_agent" => {
            let (actions, log) = parse_sub_agent_log(detail["log"].as_str().unwrap_or(""));
            let log = if log.is_empty() && actions.is_empty() {
                match (text(&detail["subAgentType"]), text(&detail["description"])) {
                    (Some(kind), Some(description)) => format!("{kind}: {description}"),
                    (Some(one), None) | (None, Some(one)) => one,
                    (None, None) => "Sub-agent activity".to_string(),
                }
            } else {
                log
            };
            out.push(Section::SubAgent {
                session: text(&detail["childSessionId"]),
                actions,
                log,
            });
        }
        "edit" => out.push(Section::Diff(diff::detail_lines(detail))),
        "write" => {
            if let Some(content) = text(&detail["content"]) {
                out.push(Section::Code {
                    text: content,
                    path,
                    start_line: None,
                    full_bleed: true,
                });
            }
        }
        "read" => {
            if let Some(content) = text(&detail["content"]) {
                out.push(Section::Code {
                    text: content,
                    path,
                    start_line: Some(detail["offset"].as_u64().unwrap_or(1)),
                    full_bleed: false,
                });
            }
        }
        "search" => {
            let code = |text: String| Section::Code {
                text,
                path: None,
                start_line: None,
                full_bleed: false,
            };
            if let Some(content) = text(&detail["content"]) {
                out.push(code(content));
            }
            let strings = |key: &str| -> Vec<String> {
                detail[key]
                    .as_array()
                    .map(|items| {
                        items
                            .iter()
                            .filter_map(|item| item.as_str().map(str::to_string))
                            .collect()
                    })
                    .unwrap_or_default()
            };
            let files = strings("filePaths");
            if !files.is_empty() {
                out.push(code(files.join("\n")));
            }
            let web: Vec<String> = detail["webResults"]
                .as_array()
                .map(|items| {
                    items
                        .iter()
                        .map(|item| {
                            format!(
                                "{}\n{}",
                                item["title"].as_str().unwrap_or(""),
                                item["url"].as_str().unwrap_or("")
                            )
                        })
                        .collect()
                })
                .unwrap_or_default();
            if !web.is_empty() {
                out.push(code(web.join("\n\n")));
            }
            let notes = strings("annotations");
            if !notes.is_empty() {
                out.push(code(notes.join("\n\n")));
            }
        }
        "fetch" => {
            let url = detail["url"].as_str().unwrap_or("");
            out.push(Section::Code {
                text: match text(&detail["result"]) {
                    Some(result) => format!("{url}\n\n{result}"),
                    None => url.to_string(),
                },
                path: None,
                start_line: None,
                full_bleed: false,
            });
        }
        "plain_text" => {
            if let Some(text) = text(&detail["text"]) {
                out.push(Section::Plain(text));
            }
        }
        "plan" => {}
        _ => {
            let (input, output) = (&detail["input"], &detail["output"]);
            if input.is_string() && output.is_null() {
                out.push(Section::Plain(input.as_str().unwrap_or("").to_string()));
            } else {
                for (title, value) in [("Input", input), ("Output", output)] {
                    let serialized = serialize(value);
                    if is_meaningful(value) && !serialized.is_empty() {
                        out.push(Section::Header(title.to_string()));
                        out.push(Section::Code {
                            text: serialized,
                            path: None,
                            start_line: None,
                            full_bleed: false,
                        });
                    }
                }
            }
        }
    }
    if let Some(error) = error.filter(|e| !e.is_empty()) {
        out.push(Section::Error(error.to_string()));
    }
    out
}

/// The hooks that ran on a tool call (`hooks` of its item), in order.
pub fn hook_lines(item: &Value) -> Vec<HookLine> {
    item["hooks"]
        .as_array()
        .map(|hooks| {
            hooks
                .iter()
                .map(|hook| HookLine {
                    name: hook["name"].as_str().unwrap_or("").to_string(),
                    status: hook["status"].as_str().unwrap_or("").to_string(),
                    exit_code: hook["exitCode"].as_i64(),
                    output: text(&hook["output"]),
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Whether the detail has anything to open, as `hasMeaningfulToolCallDetail`. Hooks count too.
pub fn has_details(item: &Value) -> bool {
    let error = error_text(&item["error"]);
    !sections(&item["detail"], error.as_deref()).is_empty() || !hook_lines(item).is_empty()
}

/// The syntax language for a file path, by extension: only the grammars the crate builds.
pub fn language_for_path(path: &str) -> Option<&'static str> {
    let name = path.rsplit('/').next().unwrap_or(path);
    let extension = name.rsplit_once('.')?.1.to_lowercase();
    Some(match extension.as_str() {
        "rs" => "rust",
        "ts" | "mts" | "cts" => "typescript",
        "tsx" => "tsx",
        "js" | "mjs" | "cjs" | "jsx" => "javascript",
        "py" => "python",
        "sh" | "bash" | "zsh" => "bash",
        "json" | "jsonc" => "json",
        _ => return None,
    })
}
