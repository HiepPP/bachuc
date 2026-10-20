//! A one-time read of the sidebar order that the Electron app saved (ADR-0057). Electron keeps it
//! in Chromium's Local Storage, a LevelDB folder under its user data folder. Nothing here writes
//! to that folder: the store is copied to a temp folder without its `LOCK` file, and the copy is
//! the one that gets opened, because `rusty-leveldb` takes a lock and may write logs or compact.

use crate::model::SidebarOrder;
use std::path::{Path, PathBuf};

/// `_paseo://app`, a zero byte, a one byte (Chromium's storage key and "Local Storage" tag),
/// then the name of the zustand store.
const KEY: &[u8] = b"_paseo://app\x00\x01sidebar-project-workspace-order";

/// The prefix byte of a Chromium Local Storage value that holds Latin-1 text.
const LATIN1: u8 = 1;
/// The prefix byte of a value that holds UTF-16LE text.
const UTF16: u8 = 0;

/// Electron's user data folder on macOS, which holds `Local Storage`.
pub fn electron_user_data_dir() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(PathBuf::from(home).join("Library/Application Support/Bachuc"))
}

/// Reads the three orders from a Local Storage value: one prefix byte, then the zustand JSON
/// `{"state":{...},"version":N}`. `None` when the value is empty, is not text, or has no `state`
/// object. A missing or bad field inside `state` gives an empty value.
pub fn parse_order_value(value: &[u8]) -> Option<SidebarOrder> {
    let (&prefix, body) = value.split_first()?;
    let text = match prefix {
        LATIN1 => body
            .iter()
            .map(|&byte| char::from(byte))
            .collect::<String>(),
        UTF16 => {
            if body.len() % 2 != 0 {
                return None;
            }
            let units: Vec<u16> = body
                .chunks_exact(2)
                .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
                .collect();
            String::from_utf16(&units).ok()?
        }
        _ => return None,
    };
    let json: serde_json::Value = serde_json::from_str(&text).ok()?;
    let state = json.get("state")?.as_object()?;
    let list = |value: Option<&serde_json::Value>| -> Vec<String> {
        value
            .and_then(|value| value.as_array())
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| item.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default()
    };
    Some(SidebarOrder {
        project_order: list(state.get("projectOrder")),
        workspace_order_by_project: state
            .get("workspaceOrderByProject")
            .and_then(|value| value.as_object())
            .map(|map| {
                map.iter()
                    .map(|(key, value)| (key.clone(), list(Some(value))))
                    .collect()
            })
            .unwrap_or_default(),
        pinned_workspace_order: list(state.get("pinnedWorkspaceOrder")),
    })
}

/// The temp copy of the store, removed when it drops.
struct Copy(PathBuf);

impl Drop for Copy {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Copies the files of the store into a new temp folder, without `LOCK`.
fn copy_store(store: &Path) -> std::io::Result<Copy> {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_nanos());
    let temp = Copy(std::env::temp_dir().join(format!(
        "bachuc-electron-order-{}-{nanos}",
        std::process::id()
    )));
    std::fs::create_dir_all(&temp.0)?;
    for entry in std::fs::read_dir(store)? {
        let entry = entry?;
        if entry.file_name() == "LOCK" || !entry.file_type()?.is_file() {
            continue;
        }
        std::fs::copy(entry.path(), temp.0.join(entry.file_name()))?;
    }
    Ok(temp)
}

/// Reads Electron's saved order from `<user_data>/Local Storage/leveldb`. `None` when the folder
/// or the key is missing, or the read fails; a failure or a bad value logs a warning. Electron's
/// folder is only read.
pub fn read_electron_order(user_data: &Path) -> Option<SidebarOrder> {
    let store = user_data.join("Local Storage/leveldb");
    if !store.is_dir() {
        return None;
    }
    let copy = match copy_store(&store) {
        Ok(copy) => copy,
        Err(error) => {
            log::warn!("cannot copy Electron's Local Storage: {error}");
            return None;
        }
    };
    let options = rusty_leveldb::Options {
        create_if_missing: false,
        ..rusty_leveldb::Options::default()
    };
    let mut db = match rusty_leveldb::DB::open(&copy.0, options) {
        Ok(db) => db,
        Err(error) => {
            log::warn!("cannot open the copy of Electron's Local Storage: {error}");
            return None;
        }
    };
    let value = db.get(KEY)?;
    let order = parse_order_value(&value);
    if order.is_none() {
        log::warn!("Electron's saved sidebar order is not readable; keeping the GPUI order");
    }
    order
}
