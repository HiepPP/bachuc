//! `PluginSettings`: one settings record of a plugin, as `SettingsState` (`contracts.ts:520`)
//! and `use-settings.ts` keep it.

use bachuc_client::{Client, ClientEvent, PluginEvent, PluginSettingsRead, PluginSettingsWrite};
use gpui_kit::{AppContext as _, Context, Entity, Subscription, Task};
use serde_json::Value;

#[derive(Clone, Debug, PartialEq)]
pub enum SettingsState {
    Loading,
    Error(String),
    Invalid { error: String, revision: String },
    Ready { values: Value, revision: String },
}

/// The settings rules without GPUI.
#[derive(Debug)]
pub(crate) struct SettingsModel {
    pub(crate) state: SettingsState,
    pub(crate) saving: bool,
    pub(crate) save_error: Option<String>,
    read_generation: u64,
}

impl Default for SettingsModel {
    fn default() -> Self {
        Self {
            state: SettingsState::Loading,
            saving: false,
            save_error: None,
            read_generation: 0,
        }
    }
}

impl SettingsModel {
    /// Starts a read; the shown state stays until the answer. Clears the save error, as
    /// `reload` resets the mutation.
    pub(crate) fn begin_read(&mut self) -> u64 {
        self.save_error = None;
        self.read_generation += 1;
        self.read_generation
    }

    /// Applies a read answer unless a newer read or a save came after it.
    pub(crate) fn apply_read(
        &mut self,
        generation: u64,
        result: anyhow::Result<PluginSettingsRead>,
    ) -> bool {
        if generation != self.read_generation {
            return false;
        }
        self.state = match result {
            Ok(PluginSettingsRead::Ready { revision, values }) => {
                SettingsState::Ready { values, revision }
            }
            Ok(PluginSettingsRead::Invalid { revision, error }) => {
                SettingsState::Invalid { error, revision }
            }
            Err(error) => SettingsState::Error(format!("{error:#}")),
        };
        true
    }

    pub(crate) fn begin_write(&mut self) {
        self.saving = true;
        self.save_error = None;
    }

    /// Applies a write or reset answer. `Saved` replaces the values and revision; any other
    /// answer keeps the shown values and sets `save_error`. Returns whether it saved.
    pub(crate) fn apply_write(&mut self, result: anyhow::Result<PluginSettingsWrite>) -> bool {
        self.saving = false;
        match result {
            Ok(PluginSettingsWrite::Saved { revision, values }) => {
                // A read sent before the save would show the old values.
                self.read_generation += 1;
                self.state = SettingsState::Ready { values, revision };
                true
            }
            Ok(
                PluginSettingsWrite::Conflict { error } | PluginSettingsWrite::Invalid { error },
            ) => {
                self.save_error = Some(error);
                false
            }
            Err(error) => {
                self.save_error = Some(format!("{error:#}"));
                false
            }
        }
    }

    /// The revision a reset writes against.
    pub(crate) fn revision(&self) -> Option<&str> {
        match &self.state {
            SettingsState::Ready { revision, .. } | SettingsState::Invalid { revision, .. } => {
                Some(revision)
            }
            SettingsState::Loading | SettingsState::Error(_) => None,
        }
    }
}

/// One plugin settings record. It loads once the client is connected, and reloads on its
/// own `PluginEvent::SettingsChanged` and after each `ClientEvent::Connected`.
pub struct PluginSettings {
    plugin_id: &'static str,
    settings_id: String,
    client: Option<Entity<Client>>,
    model: SettingsModel,
    _read: Option<Task<()>>,
    _write: Option<Task<()>>,
    _subscriptions: Vec<Subscription>,
}

impl PluginSettings {
    pub(crate) fn new(
        plugin_id: &'static str,
        settings_id: String,
        client: Option<Entity<Client>>,
        cx: &mut Context<PluginSettings>,
    ) -> Self {
        let mut model = SettingsModel::default();
        let mut subscriptions = Vec::new();
        let mut connected = false;
        match &client {
            Some(client) => {
                connected = client.read(cx).is_connected();
                subscriptions.push(cx.subscribe(client, |this, _, event: &ClientEvent, cx| {
                    if matches!(event, ClientEvent::Connected) {
                        this.reload(cx);
                    }
                }));
                subscriptions.push(cx.subscribe(client, |this, _, event: &PluginEvent, cx| {
                    if let PluginEvent::SettingsChanged {
                        plugin_id,
                        settings_id,
                    } = event
                        && *plugin_id == this.plugin_id
                        && *settings_id == this.settings_id
                    {
                        this.reload(cx);
                    }
                }));
            }
            None => model.state = SettingsState::Error("no active host".into()),
        }
        let mut settings = Self {
            plugin_id,
            settings_id,
            client,
            model,
            _read: None,
            _write: None,
            _subscriptions: subscriptions,
        };
        if connected {
            settings.reload(cx);
        }
        settings
    }

    pub fn state(&self) -> &SettingsState {
        &self.model.state
    }

    pub fn saving(&self) -> bool {
        self.model.saving
    }

    pub fn save_error(&self) -> Option<&str> {
        self.model.save_error.as_deref()
    }

    pub fn reload(&mut self, cx: &mut Context<PluginSettings>) {
        let Some(client) = self.client.clone() else {
            return;
        };
        let generation = self.model.begin_read();
        let answer = client.update(cx, |client, cx| {
            client.read_plugin_settings(self.plugin_id, &self.settings_id, cx)
        });
        cx.notify();
        self._read = Some(cx.spawn(async move |this, cx| {
            let result = answer.await;
            this.update(cx, |this, cx| {
                if this.model.apply_read(generation, result) {
                    cx.notify();
                }
            })
            .ok();
        }));
    }

    /// Saves a whole document against the shown `revision`. Never fails: the task resolves to
    /// whether it saved, and `save_error` holds the reason when it did not. The save finishes
    /// even when the caller drops the task.
    pub fn save(
        &mut self,
        values: Value,
        revision: String,
        cx: &mut Context<PluginSettings>,
    ) -> Task<bool> {
        let Some(client) = self.client.clone() else {
            return Task::ready(false);
        };
        let answer = client.update(cx, |client, cx| {
            client.write_plugin_settings(self.plugin_id, &self.settings_id, &revision, values, cx)
        });
        self.write(answer, cx)
    }

    /// Writes the defaults against the shown revision; `false` while nothing is loaded.
    pub fn reset(&mut self, cx: &mut Context<PluginSettings>) -> Task<bool> {
        let (Some(client), Some(revision)) = (self.client.clone(), self.model.revision()) else {
            return Task::ready(false);
        };
        let revision = revision.to_string();
        let answer = client.update(cx, |client, cx| {
            client.reset_plugin_settings(self.plugin_id, &self.settings_id, &revision, cx)
        });
        self.write(answer, cx)
    }

    fn write(
        &mut self,
        answer: Task<anyhow::Result<PluginSettingsWrite>>,
        cx: &mut Context<PluginSettings>,
    ) -> Task<bool> {
        self.model.begin_write();
        cx.notify();
        let (done, saved) = futures::channel::oneshot::channel();
        self._write = Some(cx.spawn(async move |this, cx| {
            let result = answer.await;
            let ok = this
                .update(cx, |this, cx| {
                    let ok = this.model.apply_write(result);
                    cx.notify();
                    ok
                })
                .unwrap_or(false);
            done.send(ok).ok();
        }));
        cx.background_spawn(async move { saved.await.unwrap_or(false) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn ready(model: &mut SettingsModel, values: Value, revision: &str) {
        let generation = model.begin_read();
        model.apply_read(
            generation,
            Ok(PluginSettingsRead::Ready {
                revision: revision.into(),
                values,
            }),
        );
    }

    #[test]
    fn a_new_record_is_loading() {
        assert_eq!(SettingsModel::default().state, SettingsState::Loading);
    }

    #[test]
    fn reads_ready_invalid_and_error() {
        let mut model = SettingsModel::default();
        ready(&mut model, json!({"on": true}), "r1");
        assert_eq!(
            model.state,
            SettingsState::Ready {
                values: json!({"on": true}),
                revision: "r1".into()
            }
        );
        let generation = model.begin_read();
        model.apply_read(
            generation,
            Ok(PluginSettingsRead::Invalid {
                revision: "r2".into(),
                error: "bad".into(),
            }),
        );
        assert_eq!(
            model.state,
            SettingsState::Invalid {
                error: "bad".into(),
                revision: "r2".into()
            }
        );
        assert_eq!(model.revision(), Some("r2"));
        let generation = model.begin_read();
        model.apply_read(generation, Err(anyhow::anyhow!("not connected")));
        assert_eq!(model.state, SettingsState::Error("not connected".into()));
        assert_eq!(model.revision(), None);
    }

    #[test]
    fn an_older_read_answer_is_dropped() {
        let mut model = SettingsModel::default();
        let old = model.begin_read();
        ready(&mut model, json!(2), "r2");
        let applied = model.apply_read(
            old,
            Ok(PluginSettingsRead::Ready {
                revision: "r1".into(),
                values: json!(1),
            }),
        );
        assert!(!applied);
        assert_eq!(model.revision(), Some("r2"));
    }

    #[test]
    fn saved_replaces_values_and_revision() {
        let mut model = SettingsModel::default();
        ready(&mut model, json!({"on": false}), "r1");
        let before_save = model.begin_read();
        model.begin_write();
        assert!(model.saving);
        let saved = model.apply_write(Ok(PluginSettingsWrite::Saved {
            revision: "r2".into(),
            values: json!({"on": true}),
        }));
        assert!(saved);
        assert!(!model.saving);
        assert_eq!(model.save_error, None);
        assert_eq!(
            model.state,
            SettingsState::Ready {
                values: json!({"on": true}),
                revision: "r2".into()
            }
        );
        // The read sent before the save answers with the old values: dropped.
        assert!(!model.apply_read(
            before_save,
            Ok(PluginSettingsRead::Ready {
                revision: "r1".into(),
                values: json!({"on": false}),
            })
        ));
    }

    #[test]
    fn conflict_and_invalid_keep_the_shown_values() {
        let mut model = SettingsModel::default();
        ready(&mut model, json!({"on": false}), "r1");
        let shown = model.state.clone();
        model.begin_write();
        let saved = model.apply_write(Ok(PluginSettingsWrite::Conflict {
            error: "changed elsewhere".into(),
        }));
        assert!(!saved);
        assert!(!model.saving);
        assert_eq!(model.state, shown);
        assert_eq!(model.save_error.as_deref(), Some("changed elsewhere"));
        model.begin_write();
        assert_eq!(model.save_error, None);
        model.apply_write(Ok(PluginSettingsWrite::Invalid {
            error: "bad value".into(),
        }));
        assert_eq!(model.state, shown);
        assert_eq!(model.save_error.as_deref(), Some("bad value"));
        model.begin_write();
        assert!(!model.apply_write(Err(anyhow::anyhow!("not connected"))));
        assert_eq!(model.save_error.as_deref(), Some("not connected"));
        model.begin_read();
        assert_eq!(model.save_error, None);
    }
}
