//! The row data rules: `shared/edit-item.ts`, `shared/thread-mode.ts`, the `editRowSchema` of
//! `shared/contracts.ts`, and `displayPath` and `headerLabel` of `client/edit-row.tsx`.

use crate::diff::EditSource;
use serde::Deserialize;
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::sync::OnceLock;

/// Agent label that turns edit diffs off for one thread. Board sets it too, so keep it stable.
pub(crate) const EDIT_DIFFS_MODE_LABEL: &str = "edit-diffs.mode";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Action {
    #[default]
    Edit,
    Write,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Status {
    Running,
    Completed,
    Failed,
    Canceled,
}

/// `editRowSchema`: the data of an `edit-diff` item.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EditRowData {
    #[serde(default)]
    pub action: Action,
    pub file_path: String,
    pub old_string: Option<String>,
    pub new_string: Option<String>,
    pub unified_diff: Option<String>,
    pub content: Option<String>,
    pub status: Status,
    pub error: Option<String>,
}

impl EditRowData {
    /// `None` when the data does not match the schema; the row then shows the unavailable note.
    pub(crate) fn decode(data: &Value) -> Option<Self> {
        serde_json::from_value(data.clone()).ok()
    }

    pub(crate) fn source(&self) -> EditSource<'_> {
        EditSource {
            old_string: self.old_string.as_deref(),
            new_string: self.new_string.as_deref(),
            unified_diff: self.unified_diff.as_deref(),
            content: self.content.as_deref(),
        }
    }
}

/// `errorText`.
fn error_text(error: &Value) -> Option<String> {
    match error {
        Value::Null => None,
        Value::String(text) => Some(text.clone()),
        Value::Object(object) if object.get("message").is_some_and(Value::is_string) => {
            object["message"].as_str().map(str::to_string)
        }
        other => serde_json::to_string(other).ok(),
    }
}

/// `toEditRow`: the row data of an edit or write tool call; `None` keeps every other call
/// native. Absent fields stay absent.
pub(crate) fn to_edit_row(item: &Value) -> Option<Value> {
    let detail = &item["detail"];
    let action = match detail["type"].as_str() {
        Some("edit") => "edit",
        Some("write") => "write",
        _ => return None,
    };
    let mut data = Map::new();
    data.insert("action".into(), action.into());
    let fields: &[&str] = if action == "write" {
        &["filePath", "content"]
    } else {
        &["filePath", "oldString", "newString", "unifiedDiff"]
    };
    for &field in fields {
        if let Some(value) = detail.get(field) {
            data.insert(field.into(), value.clone());
        }
    }
    if let Some(status) = item.get("status") {
        data.insert("status".into(), status.clone());
    }
    if let Some(error) = item.get("error").and_then(error_text) {
        data.insert("error".into(), error.into());
    }
    Some(Value::Object(data))
}

/// `editDiffsOff`.
pub(crate) fn edit_diffs_off(labels: &BTreeMap<String, String>) -> bool {
    labels.get(EDIT_DIFFS_MODE_LABEL).map(String::as_str) == Some("off")
}

/// The label value the thread toggle sets: `on` when the thread is off, else `off`.
pub(crate) fn toggled_mode(labels: &BTreeMap<String, String>) -> &'static str {
    if edit_diffs_off(labels) { "on" } else { "off" }
}

/// `displayPath`: the path under the agent folder without the folder. macOS reports `/tmp` and
/// `/var` paths through their `/private` real path. A path outside the folder under the home
/// folder shows as `~/...`, which Electron does not do (owner choice, 2026-10-17).
pub(crate) fn display_path(file_path: &str, cwd: Option<&str>) -> String {
    static HOME: OnceLock<Option<String>> = OnceLock::new();
    let home = HOME.get_or_init(|| std::env::var("HOME").ok());
    display_path_in(file_path, cwd, home.as_deref())
}

fn display_path_in(file_path: &str, cwd: Option<&str>, home: Option<&str>) -> String {
    let under = |root: &str, prefixes: &[&str]| {
        let root = root.strip_suffix('/').unwrap_or(root);
        prefixes
            .iter()
            .find_map(|prefix| file_path.strip_prefix(&format!("{prefix}{root}/")))
    };
    if let Some(relative) = cwd
        .filter(|cwd| !cwd.is_empty())
        .and_then(|cwd| under(cwd, &["", "/private"]))
    {
        return relative.to_string();
    }
    match home
        .filter(|home| !home.is_empty() && *home != "/")
        .and_then(|home| under(home, &[""]))
    {
        Some(relative) => format!("~/{relative}"),
        None => file_path.to_string(),
    }
}

/// `headerLabel`.
pub(crate) fn header_label(action: Action, status: Status) -> String {
    let verb = match action {
        Action::Write => "Write",
        Action::Edit => "Edit",
    };
    match status {
        Status::Running => match action {
            Action::Write => "Writing".into(),
            Action::Edit => "Editing".into(),
        },
        Status::Failed => format!("{verb} failed"),
        Status::Canceled => format!("{verb} canceled"),
        Status::Completed => verb.into(),
    }
}

/// The status text after the counts: none for a completed call.
pub(crate) fn shown_status(action: Action, status: Status) -> Option<String> {
    (status != Status::Completed).then(|| header_label(action, status))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn tool_call(detail: Value) -> Value {
        json!({
            "type": "tool_call",
            "callId": "call-1",
            "name": "tool",
            "status": "completed",
            "detail": detail,
            "error": null,
        })
    }

    // edit-item.test.ts

    #[test]
    fn write_calls_become_write_rows_and_edit_calls_stay_edit_rows() {
        assert_eq!(
            to_edit_row(&tool_call(
                json!({"type": "write", "filePath": "/a.ts", "content": "x"})
            )),
            Some(json!({
                "action": "write",
                "filePath": "/a.ts",
                "content": "x",
                "status": "completed",
            }))
        );
        let edit = to_edit_row(&tool_call(
            json!({"type": "edit", "filePath": "/a.ts", "newString": "y"}),
        ))
        .unwrap();
        assert_eq!(edit["action"], "edit");
        // Absent fields stay absent.
        assert_eq!(
            edit,
            json!({"action": "edit", "filePath": "/a.ts", "newString": "y", "status": "completed"})
        );
    }

    #[test]
    fn shell_and_read_calls_keep_their_native_rows() {
        assert_eq!(
            to_edit_row(&tool_call(json!({"type": "shell", "command": "ls"}))),
            None
        );
        assert_eq!(
            to_edit_row(&tool_call(json!({"type": "read", "filePath": "/a.ts"}))),
            None
        );
    }

    #[test]
    fn errors_follow_the_error_text_rule() {
        let failed = |error: Value| {
            let mut item = tool_call(json!({"type": "edit", "filePath": "/a"}));
            item["status"] = "failed".into();
            item["error"] = error;
            to_edit_row(&item).unwrap()
        };
        assert_eq!(failed(json!("boom"))["error"], "boom");
        assert_eq!(failed(json!({"message": "bad"}))["error"], "bad");
        assert_eq!(failed(json!({"code": 1}))["error"], r#"{"code":1}"#);
        assert_eq!(failed(json!(3))["error"], "3");
        assert!(failed(Value::Null).get("error").is_none());
    }

    #[test]
    fn row_data_decodes_with_the_schema_defaults() {
        let data = EditRowData::decode(&json!({"filePath": "/a", "status": "running"})).unwrap();
        assert_eq!(data.action, Action::Edit);
        assert_eq!(data.status, Status::Running);
        assert!(EditRowData::decode(&json!({"filePath": "/a", "status": "executing"})).is_none());
        assert!(EditRowData::decode(&json!({"status": "running"})).is_none());
        assert!(EditRowData::decode(&json!({"filePath": 3, "status": "running"})).is_none());
    }

    #[test]
    fn the_mode_label_gates_and_flips() {
        let off = BTreeMap::from([(EDIT_DIFFS_MODE_LABEL.to_string(), "off".to_string())]);
        let on = BTreeMap::from([(EDIT_DIFFS_MODE_LABEL.to_string(), "on".to_string())]);
        assert!(edit_diffs_off(&off));
        assert!(!edit_diffs_off(&on));
        assert!(!edit_diffs_off(&BTreeMap::new()));
        assert_eq!(toggled_mode(&off), "on");
        assert_eq!(toggled_mode(&on), "off");
        assert_eq!(toggled_mode(&BTreeMap::new()), "off");
    }

    #[test]
    fn display_path_drops_the_agent_folder_also_through_private() {
        assert_eq!(display_path("/tmp/x/new.py", Some("/tmp/x")), "new.py");
        assert_eq!(
            display_path("/private/tmp/x/new.py", Some("/tmp/x")),
            "new.py"
        );
        assert_eq!(
            display_path("/private/tmp/x/new.py", Some("/tmp/x/")),
            "new.py"
        );
        assert_eq!(display_path("/other/a.rs", Some("/tmp/x")), "/other/a.rs");
        assert_eq!(display_path("/tmp/x/a.rs", None), "/tmp/x/a.rs");
        assert_eq!(display_path("/tmp/xy/a.rs", Some("/tmp/x")), "/tmp/xy/a.rs");
    }

    #[test]
    fn display_path_shows_the_home_folder_as_a_tilde_outside_the_agent_folder() {
        let home = Some("/Users/hiep");
        let cwd = Some("/Users/hiep/Projects/bachuc");
        assert_eq!(
            display_path_in("/Users/hiep/Projects/bachuc/a.rs", cwd, home),
            "a.rs"
        );
        assert_eq!(
            display_path_in("/Users/hiep/.claude/x.md", cwd, home),
            "~/.claude/x.md"
        );
        assert_eq!(display_path_in("/etc/hosts", cwd, home), "/etc/hosts");
        assert_eq!(
            display_path_in("/Users/hiepx/a.rs", cwd, home),
            "/Users/hiepx/a.rs"
        );
        assert_eq!(
            display_path_in("/Users/hiep/.claude/x.md", Some(""), home),
            "~/.claude/x.md"
        );
        assert_eq!(
            display_path_in("/Users/hiep/.claude/x.md", cwd, None),
            "/Users/hiep/.claude/x.md"
        );
    }

    #[test]
    fn the_header_label_follows_the_status() {
        use Action::*;
        use Status::*;
        assert_eq!(header_label(Edit, Running), "Editing");
        assert_eq!(header_label(Write, Running), "Writing");
        assert_eq!(header_label(Edit, Failed), "Edit failed");
        assert_eq!(header_label(Write, Canceled), "Write canceled");
        assert_eq!(header_label(Write, Completed), "Write");
        assert_eq!(shown_status(Edit, Completed), None);
        assert_eq!(shown_status(Edit, Running).as_deref(), Some("Editing"));
        assert_eq!(shown_status(Edit, Failed).as_deref(), Some("Edit failed"));
    }
}
