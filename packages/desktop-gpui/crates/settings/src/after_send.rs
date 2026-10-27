//! After send: what the client does once a prompt is sent, stored in `after-send.json` next to
//! `agent-defaults.json`.
//!
//! A missing, corrupt, or unknown value reads `Stay`.

use anyhow::{Context as _, Result};
use serde_json::{Value, json};
use std::path::Path;

const FILE: &str = "after-send.json";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AfterSend {
    /// Keep the thread open.
    #[default]
    Stay,
    /// Go back to the Board.
    Board,
    /// Open the next unread thread.
    NextUnread,
}

impl AfterSend {
    fn id(self) -> &'static str {
        match self {
            Self::Stay => "stay",
            Self::Board => "board",
            Self::NextUnread => "next-unread",
        }
    }

    fn from_id(id: &str) -> Option<Self> {
        [Self::Stay, Self::Board, Self::NextUnread]
            .into_iter()
            .find(|mode| mode.id() == id)
    }
}

/// Reads `after-send.json` in `dir`.
pub fn load_from(dir: &Path) -> AfterSend {
    let path = dir.join(FILE);
    let Ok(text) = std::fs::read_to_string(&path) else {
        return AfterSend::Stay;
    };
    let Ok(Value::Object(stored)) = serde_json::from_str::<Value>(&text) else {
        log::warn!("{} is not a JSON object; ignoring it", path.display());
        return AfterSend::Stay;
    };
    stored
        .get("mode")
        .and_then(Value::as_str)
        .and_then(AfterSend::from_id)
        .unwrap_or_default()
}

/// Writes `after-send.json` in `dir` through a temporary file and a rename.
pub fn save_to(dir: &Path, mode: AfterSend) -> Result<()> {
    std::fs::create_dir_all(dir).with_context(|| format!("create {}", dir.display()))?;
    let path = dir.join(FILE);
    let temp = dir.join(format!("{FILE}.tmp"));
    let text = serde_json::to_string_pretty(&json!({ "mode": mode.id() }))?;
    std::fs::write(&temp, format!("{text}\n"))
        .with_context(|| format!("write {}", temp.display()))?;
    std::fs::rename(&temp, &path).with_context(|| format!("replace {}", path.display()))?;
    Ok(())
}

/// The stored mode from `~/Library/Application Support/BachucGpui`.
pub fn load() -> AfterSend {
    load_from(&bachuc_command::data_dir())
}

/// `save_to` the app folder.
pub fn save(mode: AfterSend) -> Result<()> {
    save_to(&bachuc_command::data_dir(), mode)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("after-send-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_missing_file_reads_stay() {
        assert_eq!(load_from(&temp_dir()), AfterSend::Stay);
    }

    #[test]
    fn every_mode_round_trips() {
        let dir = temp_dir();
        for mode in [AfterSend::Stay, AfterSend::Board, AfterSend::NextUnread] {
            save_to(&dir, mode).unwrap();
            assert_eq!(load_from(&dir), mode);
        }
        assert!(!dir.join("after-send.json.tmp").exists());
    }

    #[test]
    fn a_corrupt_file_or_unknown_mode_reads_stay() {
        let dir = temp_dir();
        std::fs::write(dir.join("after-send.json"), "[1]").unwrap();
        assert_eq!(load_from(&dir), AfterSend::Stay);
        std::fs::write(dir.join("after-send.json"), r#"{"mode":"jump"}"#).unwrap();
        assert_eq!(load_from(&dir), AfterSend::Stay);
        std::fs::write(dir.join("after-send.json"), r#"{"mode":"next-unread"}"#).unwrap();
        assert_eq!(load_from(&dir), AfterSend::NextUnread);
    }
}
