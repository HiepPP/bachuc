//! The project config form model: a port of `packages/app/src/utils/project-config-form.ts`.
//!
//! The daemon's `paseo.json` is edited as a draft of text fields. Applying the draft keeps every
//! key and every value shape the form does not edit.

use serde_json::{Map, Value};

/// How a command or lifecycle value was stored, so saving keeps its shape.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Text,
    Lines,
    Missing,
}

/// The three metadata prompts, in the order of the form.
pub const PROMPT_KEYS: [&str; 3] = ["branchName", "commitMessage", "pullRequest"];

/// `settings.project.metadata.*` titles of the Electron screen.
pub const PROMPT_TITLES: [&str; 3] = ["Branch names", "Commit messages", "Pull requests"];

pub const PROMPT_PLACEHOLDERS: [&str; 3] = [
    "Prefix branches with feat/ or fix/, mb/ for personal branches",
    "Use Conventional Commits with a scope",
    "Lead with a one-paragraph summary, include a Test plan section",
];

#[derive(Clone, Debug, PartialEq)]
pub struct ScriptDraft {
    pub id: usize,
    pub name: String,
    pub command_text: String,
    pub command_kind: Kind,
    /// `service` or empty.
    pub kind: String,
    pub port_text: String,
    /// The entry as stored, for the keys the form does not edit.
    pub raw: Map<String, Value>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ConfigDraft {
    pub setup_text: String,
    pub setup_kind: Kind,
    pub teardown_text: String,
    pub teardown_kind: Kind,
    pub scripts: Vec<ScriptDraft>,
    pub prompts: [String; 3],
    pub next_script_id: usize,
}

impl ConfigDraft {
    /// The empty form of a project with no config.
    pub fn empty() -> Self {
        config_to_draft(&Value::Null)
    }

    /// A blank script row for the Add button.
    pub fn new_script(&mut self) -> ScriptDraft {
        let id = self.next_script_id;
        self.next_script_id += 1;
        ScriptDraft {
            id,
            name: String::new(),
            command_text: String::new(),
            command_kind: Kind::Missing,
            kind: String::new(),
            port_text: String::new(),
            raw: Map::new(),
        }
    }
}

fn lifecycle(value: Option<&Value>) -> (String, Kind) {
    match value {
        Some(Value::String(text)) => (text.clone(), Kind::Text),
        Some(Value::Array(items)) => {
            let lines: Vec<&str> = items.iter().filter_map(Value::as_str).collect();
            (lines.join("\n"), Kind::Lines)
        }
        _ => (String::new(), Kind::Missing),
    }
}

fn lifecycle_from_text(text: &str, kind: Kind) -> Option<Value> {
    let lines: Vec<&str> = text.split('\n').filter(|l| !l.trim().is_empty()).collect();
    if lines.is_empty() {
        return None;
    }
    Some(match kind {
        Kind::Text => Value::String(lines.join("\n")),
        Kind::Lines => lines.into_iter().map(Value::from).collect(),
        Kind::Missing if lines.len() == 1 => Value::from(lines[0]),
        Kind::Missing => lines.into_iter().map(Value::from).collect(),
    })
}

fn port_text(value: Option<&Value>) -> String {
    match value {
        Some(Value::Number(number)) => number.to_string(),
        Some(Value::String(text)) => text.clone(),
        _ => String::new(),
    }
}

fn parse_port(text: &str) -> Option<Value> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    if text.chars().all(|c| c.is_ascii_digit())
        && let Ok(number) = text.parse::<u64>()
    {
        return Some(Value::from(number));
    }
    Some(Value::from(text))
}

/// Reads `paseo.json`. Null or a non-object gives the empty form.
pub fn config_to_draft(config: &Value) -> ConfigDraft {
    let empty = Map::new();
    let root = config.as_object().unwrap_or(&empty);
    let worktree = root.get("worktree").and_then(Value::as_object);
    let (setup_text, setup_kind) = lifecycle(worktree.and_then(|w| w.get("setup")));
    let (teardown_text, teardown_kind) = lifecycle(worktree.and_then(|w| w.get("teardown")));

    let mut scripts = Vec::new();
    if let Some(Value::Object(entries)) = root.get("scripts") {
        for (name, entry) in entries {
            let raw = entry.as_object().cloned().unwrap_or_default();
            let (command_text, command_kind) = lifecycle(raw.get("command"));
            scripts.push(ScriptDraft {
                id: scripts.len(),
                name: name.clone(),
                command_text,
                command_kind,
                kind: raw
                    .get("type")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                port_text: port_text(raw.get("port")),
                raw,
            });
        }
    }

    let generation = root.get("metadataGeneration").and_then(Value::as_object);
    let mut prompts = [String::new(), String::new(), String::new()];
    for (slot, key) in prompts.iter_mut().zip(PROMPT_KEYS) {
        if let Some(text) = generation
            .and_then(|g| g.get(key))
            .and_then(|entry| entry.get("instructions"))
            .and_then(Value::as_str)
        {
            *slot = text.to_string();
        }
    }

    let next_script_id = scripts.len();
    ConfigDraft {
        setup_text,
        setup_kind,
        teardown_text,
        teardown_kind,
        scripts,
        prompts,
        next_script_id,
    }
}

/// Builds the config to write: `base` with the draft's fields applied. Keys the form does not
/// edit stay; a field cleared in the form leaves its key out.
pub fn apply_draft(draft: &ConfigDraft, base: &Value) -> Value {
    let mut result = base.as_object().cloned().unwrap_or_default();

    let mut worktree = result
        .get("worktree")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    for (key, text, kind) in [
        ("setup", &draft.setup_text, draft.setup_kind),
        ("teardown", &draft.teardown_text, draft.teardown_kind),
    ] {
        match lifecycle_from_text(text, kind) {
            Some(value) => worktree.insert(key.to_string(), value),
            None => worktree.remove(key),
        };
    }
    set_or_remove(&mut result, "worktree", worktree);

    let mut scripts = Map::new();
    for row in &draft.scripts {
        let name = row.name.trim();
        if name.is_empty() {
            continue;
        }
        let mut entry = row.raw.clone();
        match lifecycle_from_text(&row.command_text, row.command_kind) {
            Some(value) => entry.insert("command".into(), value),
            None => entry.remove("command"),
        };
        let kind = row.kind.trim();
        if kind.is_empty() {
            entry.remove("type");
        } else {
            entry.insert("type".into(), Value::from(kind));
        }
        match parse_port(&row.port_text) {
            Some(port) => entry.insert("port".into(), port),
            None => entry.remove("port"),
        };
        scripts.insert(name.to_string(), Value::Object(entry));
    }
    set_or_remove(&mut result, "scripts", scripts);

    let mut generation = result
        .get("metadataGeneration")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    for (key, text) in PROMPT_KEYS.into_iter().zip(&draft.prompts) {
        let mut entry = generation
            .get(key)
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        if text.trim().is_empty() {
            entry.remove("instructions");
        } else {
            entry.insert("instructions".into(), Value::from(text.as_str()));
        }
        if entry.is_empty() {
            generation.remove(key);
        } else {
            generation.insert(key.to_string(), Value::Object(entry));
        }
    }
    set_or_remove(&mut result, "metadataGeneration", generation);

    Value::Object(result)
}

fn set_or_remove(root: &mut Map<String, Value>, key: &str, value: Map<String, Value>) {
    if value.is_empty() {
        root.remove(key);
    } else {
        root.insert(key.to_string(), Value::Object(value));
    }
}

/// The inline errors of a script row: name and command are required.
pub fn script_errors(script: &ScriptDraft) -> (Option<&'static str>, Option<&'static str>) {
    (
        script
            .name
            .trim()
            .is_empty()
            .then_some("Script name is required"),
        script
            .command_text
            .trim()
            .is_empty()
            .then_some("Command is required"),
    )
}

/// The hint line of a script row: type, port, and the first command line.
pub fn script_hint(script: &ScriptDraft) -> String {
    let mut pieces = Vec::new();
    if !script.kind.is_empty() {
        pieces.push(script.kind.clone());
    }
    if !script.port_text.is_empty() {
        pieces.push(format!("port {}", script.port_text));
    }
    if let Some(first) = script.command_text.lines().next()
        && !first.is_empty()
    {
        pieces.push(first.to_string());
    }
    pieces.join(" · ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn no_config_is_the_empty_form() {
        let draft = ConfigDraft::empty();
        assert_eq!(draft.setup_text, "");
        assert!(draft.scripts.is_empty());
        assert_eq!(draft.prompts, [String::new(), String::new(), String::new()]);
        // Saving an untouched empty form writes an empty object.
        assert_eq!(apply_draft(&draft, &Value::Null), json!({}));
    }

    #[test]
    fn an_untouched_draft_keeps_the_config() {
        let config = json!({
            "worktree": {"setup": ["npm ci", "npm run build"], "teardown": "docker compose down"},
            "scripts": {
                "dev": {"command": "npm run dev", "type": "service", "port": 3000, "extra": true},
                "lint": {"command": ["npm run lint"]},
            },
            "metadataGeneration": {"branchName": {"instructions": "short", "model": "x"}},
            "other": {"keep": 1},
        });
        let draft = config_to_draft(&config);
        assert_eq!(draft.setup_text, "npm ci\nnpm run build");
        assert_eq!(draft.setup_kind, Kind::Lines);
        assert_eq!(draft.teardown_kind, Kind::Text);
        assert_eq!(draft.scripts.len(), 2);
        assert_eq!(draft.prompts[0], "short");
        assert_eq!(apply_draft(&draft, &config), config);
    }

    #[test]
    fn edits_change_only_their_fields() {
        let config = json!({
            "worktree": {"setup": "npm ci"},
            "scripts": {"dev": {"command": "npm run dev", "port": 3000, "extra": true}},
            "other": 1,
        });
        let mut draft = config_to_draft(&config);
        draft.setup_text = "npm install\npnpm build".into();
        draft.scripts[0].command_text = "npm start".into();
        draft.scripts[0].kind = "service".into();
        draft.prompts[1] = "Use conventional commits".into();
        let out = apply_draft(&draft, &config);
        assert_eq!(out["worktree"]["setup"], "npm install\npnpm build");
        assert_eq!(out["scripts"]["dev"]["command"], "npm start");
        assert_eq!(out["scripts"]["dev"]["type"], "service");
        assert_eq!(out["scripts"]["dev"]["port"], 3000);
        assert_eq!(out["scripts"]["dev"]["extra"], true);
        assert_eq!(
            out["metadataGeneration"]["commitMessage"]["instructions"],
            "Use conventional commits"
        );
        assert_eq!(out["other"], 1);
    }

    #[test]
    fn cleared_fields_drop_their_keys() {
        let config = json!({
            "worktree": {"setup": "npm ci", "teardown": "x"},
            "scripts": {"dev": {"command": "run"}},
            "metadataGeneration": {"branchName": {"instructions": "short"}},
        });
        let mut draft = config_to_draft(&config);
        draft.setup_text.clear();
        draft.teardown_text = "  \n ".into();
        draft.scripts.clear();
        draft.prompts[0].clear();
        let out = apply_draft(&draft, &config);
        assert_eq!(out, json!({}));
    }

    #[test]
    fn a_new_script_is_added_and_a_blank_name_is_skipped() {
        let mut draft = ConfigDraft::empty();
        let mut script = draft.new_script();
        script.name = " dev ".into();
        script.command_text = "npm run dev".into();
        script.port_text = "3000".into();
        draft.scripts.push(script);
        let blank = draft.new_script();
        draft.scripts.push(blank);
        let out = apply_draft(&draft, &Value::Null);
        assert_eq!(
            out,
            json!({"scripts": {"dev": {"command": "npm run dev", "port": 3000}}})
        );
    }

    #[test]
    fn a_single_new_line_is_a_string_and_several_are_an_array() {
        assert_eq!(lifecycle_from_text("a", Kind::Missing), Some(json!("a")));
        assert_eq!(
            lifecycle_from_text("a\nb", Kind::Missing),
            Some(json!(["a", "b"]))
        );
        assert_eq!(
            lifecycle_from_text("a\n\nb", Kind::Text),
            Some(json!("a\nb"))
        );
        assert_eq!(lifecycle_from_text("", Kind::Lines), None);
    }

    #[test]
    fn script_rows_validate_name_and_command() {
        let mut draft = ConfigDraft::empty();
        let mut script = draft.new_script();
        assert_eq!(
            script_errors(&script),
            (Some("Script name is required"), Some("Command is required"))
        );
        script.name = "dev".into();
        script.command_text = "npm run dev".into();
        assert_eq!(script_errors(&script), (None, None));
        script.kind = "service".into();
        script.port_text = "3000".into();
        assert_eq!(script_hint(&script), "service · port 3000 · npm run dev");
    }
}
