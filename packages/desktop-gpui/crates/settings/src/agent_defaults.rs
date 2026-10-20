//! New agent defaults: the provider, model, thinking option, and mode a draft composer starts
//! with, stored in `agent-defaults.json` next to `hosts.json`.
//!
//! Each field is optional; an unset field keeps the draft's built-in default. The composer reads
//! the file each time it builds a draft, so a change applies to the next draft without a watch.

use anyhow::{Context as _, Result};
use serde_json::{Map, Value, json};
use std::path::Path;

const FILE: &str = "agent-defaults.json";

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AgentDefaults {
    pub provider: Option<String>,
    pub model: Option<String>,
    pub thinking_option_id: Option<String>,
    pub mode_id: Option<String>,
}

const KEYS: [&str; 4] = ["provider", "model", "thinkingOptionId", "modeId"];

impl AgentDefaults {
    fn fields(&self) -> [&Option<String>; 4] {
        [
            &self.provider,
            &self.model,
            &self.thinking_option_id,
            &self.mode_id,
        ]
    }
}

/// Reads `agent-defaults.json` in `dir`. A missing or corrupt file gives no defaults; a field
/// that is not a non-empty string is unset.
pub fn load_from(dir: &Path) -> AgentDefaults {
    let path = dir.join(FILE);
    let Ok(text) = std::fs::read_to_string(&path) else {
        return AgentDefaults::default();
    };
    let Ok(Value::Object(stored)) = serde_json::from_str::<Value>(&text) else {
        log::warn!("{} is not a JSON object; ignoring it", path.display());
        return AgentDefaults::default();
    };
    let field = |key: &str| {
        stored
            .get(key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(String::from)
    };
    AgentDefaults {
        provider: field("provider"),
        model: field("model"),
        thinking_option_id: field("thinkingOptionId"),
        mode_id: field("modeId"),
    }
}

/// Writes `agent-defaults.json` in `dir` through a temporary file and a rename. Unset fields are
/// left out.
pub fn save_to(dir: &Path, defaults: &AgentDefaults) -> Result<()> {
    std::fs::create_dir_all(dir).with_context(|| format!("create {}", dir.display()))?;
    let mut value = Map::new();
    for (key, field) in KEYS.into_iter().zip(defaults.fields()) {
        if let Some(field) = field {
            value.insert(key.into(), json!(field));
        }
    }
    let path = dir.join(FILE);
    let temp = dir.join(format!("{FILE}.tmp"));
    let text = serde_json::to_string_pretty(&Value::Object(value))?;
    std::fs::write(&temp, format!("{text}\n"))
        .with_context(|| format!("write {}", temp.display()))?;
    std::fs::rename(&temp, &path).with_context(|| format!("replace {}", path.display()))?;
    Ok(())
}

/// The stored defaults from `~/Library/Application Support/BachucGpui`.
pub fn load() -> AgentDefaults {
    load_from(&bachuc_command::data_dir())
}

/// `save_to` the app folder.
pub fn save(defaults: &AgentDefaults) -> Result<()> {
    save_to(&bachuc_command::data_dir(), defaults)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("agent-defaults-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_missing_file_sets_nothing() {
        assert_eq!(load_from(&temp_dir()), AgentDefaults::default());
    }

    #[test]
    fn defaults_round_trip_and_unset_fields_stay_out() {
        let dir = temp_dir();
        let defaults = AgentDefaults {
            provider: Some("claude".into()),
            model: Some("claude-opus-5-5".into()),
            thinking_option_id: Some("high".into()),
            mode_id: None,
        };
        save_to(&dir, &defaults).unwrap();
        assert_eq!(load_from(&dir), defaults);
        let text = std::fs::read_to_string(dir.join(FILE)).unwrap();
        assert!(!text.contains("modeId"));
        assert!(!dir.join("agent-defaults.json.tmp").exists());
    }

    #[test]
    fn a_corrupt_file_or_bad_field_is_ignored() {
        let dir = temp_dir();
        std::fs::write(dir.join(FILE), "[1]").unwrap();
        assert_eq!(load_from(&dir), AgentDefaults::default());
        std::fs::write(
            dir.join(FILE),
            r#"{"provider":"claude","model":5,"thinkingOptionId":" ","modeId":"bypassPermissions"}"#,
        )
        .unwrap();
        assert_eq!(
            load_from(&dir),
            AgentDefaults {
                provider: Some("claude".into()),
                mode_id: Some("bypassPermissions".into()),
                ..AgentDefaults::default()
            }
        );
    }
}
