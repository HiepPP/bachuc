//! Saved daemon hosts and the client id, both in `~/Library/Application Support/BachucGpui`.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// One daemon host. The URL is a plain `ws://` URL such as `ws://127.0.0.1:6768/ws`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostConfig {
    pub name: String,
    pub url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
}

const HOSTS_FILE: &str = "hosts.json";
const CLIENT_ID_FILE: &str = "client-id";

/// The folder that holds `hosts.json` and `client-id`.
pub fn data_dir() -> PathBuf {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default();
    home.join("Library/Application Support/BachucGpui")
}

/// Reads the saved hosts. A missing or unreadable file is replaced with release and live.
pub fn load_hosts() -> Vec<HostConfig> {
    load_hosts_from(&data_dir())
}

pub(crate) fn default_hosts() -> Vec<HostConfig> {
    vec![
        HostConfig {
            name: "release".into(),
            url: "ws://127.0.0.1:6767/ws".into(),
            password: None,
        },
        HostConfig {
            name: "live".into(),
            url: "ws://127.0.0.1:6768/ws".into(),
            password: None,
        },
    ]
}

pub(crate) fn load_hosts_from(dir: &Path) -> Vec<HostConfig> {
    let path = dir.join(HOSTS_FILE);
    if let Ok(text) = std::fs::read_to_string(&path) {
        match serde_json::from_str::<Vec<HostConfig>>(&text) {
            Ok(hosts) if !hosts.is_empty() => return hosts,
            Ok(_) => {}
            // Keep a hand-edited file with a typo; the user fixes it, the app shows the seeds.
            Err(error) => {
                log::warn!("ignoring {}: {error}", path.display());
                return default_hosts();
            }
        }
    }
    let hosts = default_hosts();
    let text = serde_json::to_string_pretty(&hosts).expect("hosts serialize");
    if let Err(error) = std::fs::create_dir_all(dir).and_then(|_| std::fs::write(&path, text)) {
        log::warn!("cannot write {}: {error}", path.display());
    }
    hosts
}

/// Writes the hosts to `hosts.json`, replacing the file in one rename.
pub fn save_hosts(hosts: &[HostConfig]) -> anyhow::Result<()> {
    save_hosts_to(&data_dir(), hosts)
}

pub(crate) fn save_hosts_to(dir: &Path, hosts: &[HostConfig]) -> anyhow::Result<()> {
    std::fs::create_dir_all(dir)?;
    let path = dir.join(HOSTS_FILE);
    let temp = dir.join(format!("{HOSTS_FILE}.tmp"));
    std::fs::write(&temp, serde_json::to_string_pretty(hosts)?)?;
    std::fs::rename(&temp, &path)?;
    Ok(())
}

/// The UUID this installation sends as `clientId`, created on first use.
pub fn client_id() -> String {
    client_id_in(&data_dir())
}

pub(crate) fn client_id_in(dir: &Path) -> String {
    let path = dir.join(CLIENT_ID_FILE);
    if let Ok(text) = std::fs::read_to_string(&path) {
        let id = text.trim();
        if !id.is_empty() {
            return id.to_string();
        }
    }
    let id = uuid::Uuid::new_v4().to_string();
    if let Err(error) = std::fs::create_dir_all(dir).and_then(|_| std::fs::write(&path, &id)) {
        log::warn!("cannot write {}: {error}", path.display());
    }
    id
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("bachuc-client-{name}-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn missing_hosts_file_is_seeded_with_release_and_live() {
        let dir = temp_dir("hosts");
        let hosts = load_hosts_from(&dir);
        assert_eq!(hosts, default_hosts());
        let saved: Vec<HostConfig> =
            serde_json::from_str(&std::fs::read_to_string(dir.join(HOSTS_FILE)).unwrap()).unwrap();
        assert_eq!(saved, hosts);
        assert_eq!(hosts[1].url, "ws://127.0.0.1:6768/ws");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn saved_hosts_are_read_back_with_passwords() {
        let dir = temp_dir("hosts-saved");
        let text = r#"[{"name":"macmini","url":"ws://127.0.0.1:6769/ws","password":"pw"}]"#;
        std::fs::write(dir.join(HOSTS_FILE), text).unwrap();
        let hosts = load_hosts_from(&dir);
        assert_eq!(hosts.len(), 1);
        assert_eq!(hosts[0].password.as_deref(), Some("pw"));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn saved_hosts_add_edit_and_remove() {
        let dir = temp_dir("hosts-save");
        let mut hosts = load_hosts_from(&dir);
        hosts.push(HostConfig {
            name: "macmini".into(),
            url: "ws://127.0.0.1:6769/ws".into(),
            password: Some("pw".into()),
        });
        save_hosts_to(&dir, &hosts).unwrap();
        assert_eq!(load_hosts_from(&dir), hosts);
        hosts[2].url = "ws://127.0.0.1:6770/ws".into();
        hosts.remove(0);
        save_hosts_to(&dir, &hosts).unwrap();
        let saved = load_hosts_from(&dir);
        assert_eq!(saved, hosts);
        assert_eq!(saved.len(), 2);
        assert!(!dir.join(format!("{HOSTS_FILE}.tmp")).exists());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn client_id_is_kept() {
        let dir = temp_dir("client-id");
        let first = client_id_in(&dir);
        assert_eq!(first.len(), 36);
        assert_eq!(client_id_in(&dir), first);
        let _ = std::fs::remove_dir_all(dir);
    }
}
