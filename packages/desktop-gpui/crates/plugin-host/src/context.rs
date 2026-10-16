//! `PluginContext`: what a plugin's closures call to reach the host (`PluginClientContext`,
//! `contracts.ts:116`).

use crate::composer::ComposerAttachment;
use crate::contributions::{PanelLocation, SurfacePresentation, SwipeDirection};
use crate::host::{PluginHost, PluginHostEvent};
use crate::settings::PluginSettings;
use bachuc_client::Client;
use gpui_kit::component::WindowExt as _;
use gpui_kit::component::notification::Notification;
use gpui_kit::{App, AppContext as _, Entity, SharedString, Task, WeakEntity};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

/// The host state every context reads. It lives outside the `PluginHost` entity, so a plugin
/// can call its context while the host runs `contribute`.
pub(crate) struct Shared {
    pub(crate) host: WeakEntity<PluginHost>,
    pub(crate) client: RefCell<Option<Entity<Client>>>,
    pub(crate) workspace_id: RefCell<Option<String>>,
    pub(crate) agent_id: RefCell<Option<String>>,
}

/// A request that the host handles after the current update.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum HostRequest {
    Emit(PluginHostEvent),
    Notify(&'static str),
    InvalidateTimeline,
}

/// Where a panel opens.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PanelTarget {
    pub workspace_id: String,
    pub agent_id: Option<String>,
    pub location: Option<PanelLocation>,
}

/// An assistant passage to scroll to and mark.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TimelinePassage {
    pub agent_id: String,
    pub message_id: String,
    pub text: Option<String>,
}

/// Opens the New workspace screen on a folder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NewWorkspaceRequest {
    pub cwd: String,
    pub project_id: Option<String>,
    pub name: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct OpenSurfaceOptions {
    /// Another plugin's surface; `None` opens one of this plugin's.
    pub plugin_id: Option<String>,
    pub presentation: SurfacePresentation,
    pub params: BTreeMap<String, String>,
}

/// Where `open_agent_at` scrolls the agent's timeline.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AgentAnchor {
    Latest,
    LatestPrompt,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MessageLevel {
    Info,
    Error,
}

/// One plugin's handle on the host. Cheap to clone; closures keep a copy.
///
/// UI requests reach the app as `PluginHostEvent`s after the current update, so a plugin may
/// call them from `contribute` or from a view's handler.
#[derive(Clone)]
pub struct PluginContext {
    plugin_id: &'static str,
    shared: Rc<Shared>,
}

impl PluginContext {
    pub(crate) fn new(plugin_id: &'static str, shared: Rc<Shared>) -> Self {
        Self { plugin_id, shared }
    }

    pub fn plugin_id(&self) -> &'static str {
        self.plugin_id
    }

    /// The active host's client. Plugins read agents and workspaces from it directly.
    pub fn client(&self, _cx: &App) -> Option<Entity<Client>> {
        self.shared.client.borrow().clone()
    }

    /// Calls one of this plugin's server RPCs.
    pub fn rpc(
        &self,
        method: &str,
        input: serde_json::Value,
        cx: &mut App,
    ) -> Task<anyhow::Result<serde_json::Value>> {
        match self.client(cx) {
            Some(client) => client.update(cx, |client, cx| {
                client.invoke_plugin_rpc(self.plugin_id, method, input, cx)
            }),
            None => Task::ready(Err(anyhow::anyhow!("no active host"))),
        }
    }

    /// `rpc` with a typed input and output.
    pub fn rpc_as<I: serde::Serialize, O: serde::de::DeserializeOwned + Send + 'static>(
        &self,
        method: &str,
        input: &I,
        cx: &mut App,
    ) -> Task<anyhow::Result<O>> {
        match self.client(cx) {
            Some(client) => client.update(cx, |client, cx| {
                client.plugin_rpc(self.plugin_id, method, input, cx)
            }),
            None => Task::ready(Err(anyhow::anyhow!("no active host"))),
        }
    }

    /// A settings record of this plugin. It loads now and follows the host's changes; keep the
    /// entity while a view shows it.
    pub fn settings(&self, settings_id: &str, cx: &mut App) -> Entity<PluginSettings> {
        let client = self.client(cx);
        let plugin_id = self.plugin_id;
        let settings_id = settings_id.to_string();
        cx.new(|cx| PluginSettings::new(plugin_id, settings_id, client, cx))
    }

    pub fn active_workspace_id(&self, _cx: &App) -> Option<String> {
        self.shared.workspace_id.borrow().clone()
    }

    pub fn active_agent_id(&self, _cx: &App) -> Option<String> {
        self.shared.agent_id.borrow().clone()
    }

    pub fn open_surface(&self, surface_id: &str, options: OpenSurfaceOptions, cx: &mut App) {
        self.emit(
            PluginHostEvent::OpenSurface {
                plugin_id: options
                    .plugin_id
                    .unwrap_or_else(|| self.plugin_id.to_string()),
                surface_id: surface_id.to_string(),
                presentation: options.presentation,
                params: options.params,
            },
            cx,
        );
    }

    pub fn open_panel(&self, panel_id: &str, target: PanelTarget, cx: &mut App) {
        self.emit(
            PluginHostEvent::OpenPanel {
                plugin_id: self.plugin_id,
                panel_id: panel_id.to_string(),
                target,
            },
            cx,
        );
    }

    pub fn open_settings(&self, settings_id: &str, cx: &mut App) {
        self.emit(
            PluginHostEvent::OpenSettings {
                plugin_id: self.plugin_id,
                settings_id: settings_id.to_string(),
            },
            cx,
        );
    }

    pub fn set_composer_text(&self, agent_id: &str, text: String, cx: &mut App) {
        self.emit(
            PluginHostEvent::SetComposerText {
                agent_id: agent_id.to_string(),
                text,
            },
            cx,
        );
    }

    /// Adds a chip to an agent composer, as this plugin's.
    pub fn add_composer_attachment(&self, mut attachment: ComposerAttachment, cx: &mut App) {
        attachment.plugin_id = self.plugin_id.to_string();
        self.emit(PluginHostEvent::AddComposerAttachment(attachment), cx);
    }

    pub fn reveal_timeline_passage(&self, passage: TimelinePassage, cx: &mut App) {
        self.emit(PluginHostEvent::RevealTimelinePassage(passage), cx);
    }

    pub fn open_new_workspace(&self, request: NewWorkspaceRequest, cx: &mut App) {
        self.emit(PluginHostEvent::OpenNewWorkspace(request), cx);
    }

    pub fn open_plugins_page(&self, cx: &mut App) {
        self.emit(PluginHostEvent::OpenPluginsPage, cx);
    }

    pub fn open_agent(&self, agent_id: &str, cx: &mut App) {
        self.emit(
            PluginHostEvent::OpenAgent {
                agent_id: agent_id.to_string(),
            },
            cx,
        );
    }

    /// `open_agent`, then scroll the timeline to `anchor`.
    pub fn open_agent_at(&self, agent_id: &str, anchor: AgentAnchor, cx: &mut App) {
        self.emit(
            PluginHostEvent::OpenAgentAt {
                agent_id: agent_id.to_string(),
                anchor,
            },
            cx,
        );
    }

    pub fn open_workspace(&self, workspace_id: &str, cx: &mut App) {
        self.emit(
            PluginHostEvent::OpenWorkspace {
                workspace_id: workspace_id.to_string(),
            },
            cx,
        );
    }

    /// Shows a gpui-kit notification in the active window, or the first window when the app
    /// is in the background.
    pub fn show_message(&self, message: SharedString, level: MessageLevel, cx: &mut App) {
        let plugin_id = self.plugin_id;
        cx.defer(move |cx| {
            let Some(window) = cx
                .active_window()
                .or_else(|| cx.windows().into_iter().next())
            else {
                log::info!("plugin {plugin_id}: {message}");
                return;
            };
            let note = match level {
                MessageLevel::Info => Notification::info(message),
                MessageLevel::Error => Notification::error(message),
            };
            window
                .update(cx, |_, window, cx| window.push_notification(note, cx))
                .ok();
        });
    }

    /// Redraws this plugin's contributions in the host views (`PluginStateChanged`).
    pub fn notify(&self, cx: &mut App) {
        self.request(HostRequest::Notify(self.plugin_id), cx);
    }

    /// Makes the timeline run its transformers again: bumps `timeline_revision`.
    pub fn invalidate_timeline(&self, cx: &mut App) {
        self.request(HostRequest::InvalidateTimeline, cx);
    }

    /// The plugin's project filter changed its selection.
    pub fn sidebar_filter_changed(&self, direction: Option<SwipeDirection>, cx: &mut App) {
        self.emit(PluginHostEvent::SidebarFilterChanged { direction }, cx);
    }

    fn emit(&self, event: PluginHostEvent, cx: &mut App) {
        self.request(HostRequest::Emit(event), cx);
    }

    fn request(&self, request: HostRequest, cx: &mut App) {
        let host = self.shared.host.clone();
        cx.defer(move |cx| {
            host.update(cx, |host, cx| host.handle(request, cx)).ok();
        });
    }
}
