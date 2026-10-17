//! Plugins of the active host: the plugin list, plugin RPC answers, and plugin settings.

use anyhow::anyhow;
use serde_json::Value;

/// A plugin change on the host. `Client` emits these next to `ClientEvent` and
/// `DirectoryEvent`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PluginEvent {
    /// The plugin loaded, reloaded, or unloaded (`status.plugin_catalog_changed`).
    CatalogChanged { plugin_id: String },
    /// One settings record of the plugin changed (`status.plugin_settings_changed`).
    SettingsChanged {
        plugin_id: String,
        settings_id: String,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PluginStatus {
    Running,
    Disabled,
    Failed,
    /// A status this build does not know.
    Unknown,
}

/// One installed plugin, in the daemon's list order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PluginListItem {
    pub id: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub status: PluginStatus,
    pub error: Option<String>,
}

/// The answer of `settings.<id>.read`.
#[derive(Clone, Debug, PartialEq)]
pub enum PluginSettingsRead {
    Ready { revision: String, values: Value },
    Invalid { revision: String, error: String },
}

/// The answer of `settings.<id>.write` and `settings.<id>.reset`.
#[derive(Clone, Debug, PartialEq)]
pub enum PluginSettingsWrite {
    Saved { revision: String, values: Value },
    Conflict { error: String },
    Invalid { error: String },
}

/// The `plugins` of `plugin.list.response`. Each item decodes on its own, so an item this build
/// cannot read is left out and an unknown status is `Unknown`.
pub(crate) fn plugin_list(plugins: &Value) -> Vec<PluginListItem> {
    plugins
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|plugin| {
            Some(PluginListItem {
                id: plugin["id"].as_str()?.to_string(),
                description: plugin["description"].as_str().map(str::to_string),
                enabled: plugin["enabled"] == true,
                status: match plugin["status"].as_str() {
                    Some("running") => PluginStatus::Running,
                    Some("disabled") => PluginStatus::Disabled,
                    Some("failed") => PluginStatus::Failed,
                    _ => PluginStatus::Unknown,
                },
                error: plugin["error"].as_str().map(str::to_string),
            })
        })
        .collect()
}

/// The ids of `plugin.catalog.get.response`; the client bundles are for the JS app.
pub(crate) fn catalog_ids(plugins: &Value) -> Vec<String> {
    plugins
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|plugin| plugin["id"].as_str().map(str::to_string))
        .collect()
}

fn text(output: &Value, field: &str) -> anyhow::Result<String> {
    output[field]
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| anyhow!("plugin settings answer has no {field}"))
}

/// Decodes a read answer by `status`, as `SettingsReadResultSchema` does.
pub(crate) fn settings_read(output: Value) -> anyhow::Result<PluginSettingsRead> {
    match output["status"].as_str() {
        Some("ready") => Ok(PluginSettingsRead::Ready {
            revision: text(&output, "revision")?,
            values: output["values"].clone(),
        }),
        Some("invalid") => Ok(PluginSettingsRead::Invalid {
            revision: text(&output, "revision")?,
            error: text(&output, "error")?,
        }),
        status => Err(anyhow!("unknown plugin settings read status {status:?}")),
    }
}

/// Decodes a write or reset answer by `status`, as `SettingsWriteResultSchema` does.
pub(crate) fn settings_write(output: Value) -> anyhow::Result<PluginSettingsWrite> {
    match output["status"].as_str() {
        Some("saved") => Ok(PluginSettingsWrite::Saved {
            revision: text(&output, "revision")?,
            values: output["values"].clone(),
        }),
        Some("conflict") => Ok(PluginSettingsWrite::Conflict {
            error: text(&output, "error")?,
        }),
        Some("invalid") => Ok(PluginSettingsWrite::Invalid {
            error: text(&output, "error")?,
        }),
        status => Err(anyhow!("unknown plugin settings write status {status:?}")),
    }
}
