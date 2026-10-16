//! `PluginHost`: activates each registered plugin while the active host lists it enabled and
//! running, and keeps the `ContributionIndex` of the active ones.

use crate::NativePlugin;
use crate::composer::ComposerAttachment;
use crate::context::{
    AgentAnchor, HostRequest, NewWorkspaceRequest, PanelTarget, PluginContext, Shared,
    TimelinePassage,
};
use crate::contributions::{Contributions, SurfacePresentation, SwipeDirection};
use crate::index::ContributionIndex;
use bachuc_client::{Client, ClientEvent, PluginEvent, PluginListItem, PluginStatus};
use gpui_kit::{App, AppContext as _, Context, Entity, EventEmitter, Global, Subscription, Task};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

/// What the host tells the app. UI requests from plugins come as events; the app routes them.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum PluginHostEvent {
    /// A plugin became active or inactive; read `PluginHost::index` again.
    ContributionsChanged,
    /// Redraw this plugin's contributions (`PluginContext::notify`).
    PluginStateChanged {
        plugin_id: &'static str,
    },
    /// Run the timeline transformers again (`PluginContext::invalidate_timeline`).
    TimelineTransformsChanged,
    SidebarFilterChanged {
        direction: Option<SwipeDirection>,
    },
    OpenSurface {
        plugin_id: String,
        surface_id: String,
        presentation: SurfacePresentation,
        params: BTreeMap<String, String>,
    },
    OpenPanel {
        plugin_id: &'static str,
        panel_id: String,
        target: PanelTarget,
    },
    OpenSettings {
        plugin_id: &'static str,
        settings_id: String,
    },
    SetComposerText {
        agent_id: String,
        text: String,
    },
    AddComposerAttachment(ComposerAttachment),
    RevealTimelinePassage(TimelinePassage),
    OpenNewWorkspace(NewWorkspaceRequest),
    OpenPluginsPage,
    OpenAgent {
        agent_id: String,
    },
    OpenWorkspace {
        workspace_id: String,
    },
    /// `OpenAgent`, then scroll to `anchor` (`PluginContext::open_agent_at`).
    OpenAgentAt {
        agent_id: String,
        anchor: AgentAnchor,
    },
}

/// The registered plugins that `listed` reports enabled and running, in registration order.
/// A listed plugin that is not registered is ignored.
pub fn active_plugin_ids(
    listed: &[PluginListItem],
    registered: &[&'static str],
) -> Vec<&'static str> {
    registered
        .iter()
        .copied()
        .filter(|id| {
            listed.iter().any(|plugin| {
                plugin.id == *id && plugin.enabled && plugin.status == PluginStatus::Running
            })
        })
        .collect()
}

/// A failed list leaves no plugin active, for example without the `daemon.manage` permission.
pub(crate) fn listed_or_none(result: anyhow::Result<Vec<PluginListItem>>) -> Vec<PluginListItem> {
    result.unwrap_or_else(|error| {
        log::warn!("plugin list failed, no plugin is active: {error:#}");
        Vec::new()
    })
}

/// The host rules without GPUI: which plugins are active, the list generation, and the
/// timeline revision.
pub(crate) struct HostModel {
    registered: Vec<&'static str>,
    /// Active plugins with their contributions, in registration order.
    active: Vec<(&'static str, Contributions)>,
    list_generation: u64,
    timeline_revision: u64,
}

impl HostModel {
    /// Keeps the first plugin of each id.
    pub(crate) fn new(ids: impl IntoIterator<Item = &'static str>) -> Self {
        let mut registered: Vec<&'static str> = Vec::new();
        for id in ids {
            if registered.contains(&id) {
                log::warn!("plugin {id} is registered twice; the second is ignored");
            } else {
                registered.push(id);
            }
        }
        Self {
            registered,
            active: Vec::new(),
            list_generation: 0,
            timeline_revision: 0,
        }
    }

    pub(crate) fn registered(&self) -> &[&'static str] {
        &self.registered
    }

    /// Starts a list request; only its answer counts until the next one.
    pub(crate) fn begin_list(&mut self) -> u64 {
        self.list_generation += 1;
        self.list_generation
    }

    pub(crate) fn accepts(&self, generation: u64) -> bool {
        generation == self.list_generation
    }

    /// A host switch: drops every list answer in flight and deactivates every plugin.
    pub(crate) fn switch_host(&mut self) -> bool {
        self.list_generation += 1;
        self.set_active(&[], |_| Contributions::default())
    }

    pub(crate) fn is_active(&self, plugin_id: &str) -> bool {
        self.active.iter().any(|(id, _)| *id == plugin_id)
    }

    /// Makes `ids` the active set. Only plugins whose state changes run `contribute` or drop
    /// their contributions; the others keep theirs. Returns whether the set changed.
    pub(crate) fn set_active(
        &mut self,
        ids: &[&'static str],
        mut contribute: impl FnMut(&'static str) -> Contributions,
    ) -> bool {
        let mut previous = std::mem::take(&mut self.active);
        let mut changed = false;
        for &id in &self.registered {
            let kept = previous
                .iter()
                .position(|(active, _)| *active == id)
                .map(|at| previous.swap_remove(at));
            match (ids.contains(&id), kept) {
                (true, Some(kept)) => self.active.push(kept),
                (true, None) => {
                    self.active.push((id, contribute(id)));
                    changed = true;
                }
                (false, Some(_)) => changed = true,
                (false, None) => {}
            }
        }
        if changed {
            self.timeline_revision += 1;
        }
        changed
    }

    pub(crate) fn index(&self) -> ContributionIndex {
        ContributionIndex::build(
            self.active
                .iter()
                .map(|(id, contributions)| (*id, contributions)),
        )
    }

    pub(crate) fn timeline_revision(&self) -> u64 {
        self.timeline_revision
    }

    pub(crate) fn request(&mut self, request: HostRequest) -> PluginHostEvent {
        match request {
            HostRequest::Emit(event) => event,
            HostRequest::Notify(plugin_id) => PluginHostEvent::PluginStateChanged { plugin_id },
            HostRequest::InvalidateTimeline => {
                self.timeline_revision += 1;
                PluginHostEvent::TimelineTransformsChanged
            }
        }
    }
}

/// The plugin host entity. One per app, stored as a GPUI global by `init`.
pub struct PluginHost {
    plugins: Vec<Box<dyn NativePlugin>>,
    shared: Rc<Shared>,
    model: HostModel,
    index: ContributionIndex,
    _list: Option<Task<()>>,
    _client_subscriptions: Vec<Subscription>,
}

impl EventEmitter<PluginHostEvent> for PluginHost {}

struct GlobalPluginHost(Entity<PluginHost>);

impl Global for GlobalPluginHost {}

/// Creates the host with every plugin crate the app links, in this order, and stores it as a
/// GPUI global. No plugin is active until `set_client` and a list answer.
pub fn init(plugins: Vec<Box<dyn NativePlugin>>, cx: &mut App) -> Entity<PluginHost> {
    let host = cx.new(|cx| {
        let model = HostModel::new(plugins.iter().map(|plugin| plugin.id()));
        PluginHost {
            plugins,
            shared: Rc::new(Shared {
                host: cx.weak_entity(),
                client: RefCell::new(None),
                workspace_id: RefCell::new(None),
                agent_id: RefCell::new(None),
            }),
            model,
            index: ContributionIndex::default(),
            _list: None,
            _client_subscriptions: Vec::new(),
        }
    });
    cx.set_global(GlobalPluginHost(host.clone()));
    host
}

/// The host that `init` stored.
pub fn host(cx: &App) -> Option<Entity<PluginHost>> {
    cx.try_global::<GlobalPluginHost>()
        .map(|global| global.0.clone())
}

impl PluginHost {
    /// Switches to another host's client: deactivates every plugin, then lists the new host's
    /// plugins once it is connected. The same client only lists again.
    pub fn set_client(&mut self, client: Entity<Client>, cx: &mut Context<PluginHost>) {
        let same = self.shared.client.borrow().as_ref() == Some(&client);
        if !same {
            self._list = None;
            if self.model.switch_host() {
                self.contributions_changed(cx);
            }
            *self.shared.client.borrow_mut() = Some(client.clone());
            self._client_subscriptions = vec![
                cx.subscribe(&client, |host, _, event: &ClientEvent, cx| {
                    // A disconnect keeps the active set, so pills do not flicker.
                    if matches!(event, ClientEvent::Connected) {
                        host.list(cx);
                    }
                }),
                cx.subscribe(&client, |host, _, event: &PluginEvent, cx| {
                    if matches!(event, PluginEvent::CatalogChanged { .. }) {
                        host.list(cx);
                    }
                }),
            ];
        }
        if client.read(cx).is_connected() {
            self.list(cx);
        }
    }

    /// The workspace and agent the user looks at, for `PluginContext::active_*`.
    pub fn set_active(
        &mut self,
        workspace_id: Option<String>,
        agent_id: Option<String>,
        cx: &mut Context<PluginHost>,
    ) {
        *self.shared.workspace_id.borrow_mut() = workspace_id;
        *self.shared.agent_id.borrow_mut() = agent_id;
        cx.notify();
    }

    pub fn is_active(&self, plugin_id: &str) -> bool {
        self.model.is_active(plugin_id)
    }

    pub fn index(&self) -> &ContributionIndex {
        &self.index
    }

    /// Grows on each contribution change and each `PluginContext::invalidate_timeline`. The
    /// timeline caches transformer results by entry and this revision.
    pub fn timeline_revision(&self) -> u64 {
        self.model.timeline_revision()
    }

    fn list(&mut self, cx: &mut Context<PluginHost>) {
        let Some(client) = self.shared.client.borrow().clone() else {
            return;
        };
        let generation = self.model.begin_list();
        let answer = client.update(cx, |client, cx| client.list_plugins(cx));
        self._list = Some(cx.spawn(async move |this, cx| {
            let result = answer.await;
            this.update(cx, |host, cx| {
                if host.model.accepts(generation) {
                    host.activate(&listed_or_none(result), cx);
                }
            })
            .ok();
        }));
    }

    fn activate(&mut self, listed: &[PluginListItem], cx: &mut Context<PluginHost>) {
        let ids = active_plugin_ids(listed, self.model.registered());
        let plugins = &self.plugins;
        let shared = &self.shared;
        let changed = self.model.set_active(&ids, |id| {
            let mut contributions = Contributions::default();
            if let Some(plugin) = plugins.iter().find(|plugin| plugin.id() == id) {
                let ctx = PluginContext::new(id, shared.clone());
                plugin.contribute(ctx, &mut contributions, cx);
            }
            contributions
        });
        if changed {
            self.contributions_changed(cx);
        }
    }

    fn contributions_changed(&mut self, cx: &mut Context<PluginHost>) {
        self.index = self.model.index();
        cx.emit(PluginHostEvent::ContributionsChanged);
        cx.notify();
    }

    pub(crate) fn handle(&mut self, request: HostRequest, cx: &mut Context<PluginHost>) {
        let event = self.model.request(request);
        cx.emit(event);
        cx.notify();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contributions::Surface;
    use std::cell::Cell;

    fn surface(id: &'static str) -> Contributions {
        let mut contributions = Contributions::default();
        contributions.add_surface(Surface {
            id,
            title: None,
            icon: None,
            view: Rc::new(|_, _, _| -> gpui_kit::AnyView { unreachable!("no window in tests") }),
        });
        contributions
    }

    fn surface_ids(model: &HostModel) -> Vec<(&'static str, &'static str)> {
        model
            .index()
            .surfaces()
            .iter()
            .map(|surface| (surface.plugin_id, surface.value.id))
            .collect()
    }

    #[test]
    fn only_changed_plugins_contribute_or_drop() {
        let mut model = HostModel::new(["a", "b", "c"]);
        let calls = Cell::new(0);
        let contribute = |id| {
            calls.set(calls.get() + 1);
            surface(id)
        };
        assert!(model.set_active(&["a", "b"], contribute));
        assert_eq!(calls.get(), 2);
        assert!(model.set_active(&["b", "c"], contribute));
        assert_eq!(
            calls.get(),
            3,
            "only c contributes; b keeps its contributions"
        );
        assert!(!model.is_active("a"));
        assert!(model.is_active("b") && model.is_active("c"));
        assert!(!model.set_active(&["c", "b"], contribute));
        assert_eq!(calls.get(), 3);
        assert_eq!(surface_ids(&model), [("b", "b"), ("c", "c")]);
    }

    #[test]
    fn the_index_keeps_registration_order() {
        let mut model = HostModel::new(["first", "second"]);
        model.set_active(&["second", "first"], surface);
        assert_eq!(
            surface_ids(&model),
            [("first", "first"), ("second", "second")]
        );
    }

    #[test]
    fn a_plugin_registered_twice_counts_once() {
        let model = HostModel::new(["a", "a", "b"]);
        assert_eq!(model.registered(), ["a", "b"]);
    }

    #[test]
    fn a_list_error_leaves_no_plugin_active() {
        let mut model = HostModel::new(["a"]);
        let listed = vec![PluginListItem {
            id: "a".into(),
            description: None,
            enabled: true,
            status: PluginStatus::Running,
            error: None,
        }];
        model.set_active(&active_plugin_ids(&listed, model.registered()), surface);
        assert!(model.is_active("a"));
        let listed = listed_or_none(Err(anyhow::anyhow!("missing daemon.manage")));
        assert!(listed.is_empty());
        let ids = active_plugin_ids(&listed, model.registered());
        assert!(model.set_active(&ids, surface));
        assert!(!model.is_active("a"));
    }

    #[test]
    fn a_list_answer_after_a_newer_request_or_a_host_switch_is_dropped() {
        let mut model = HostModel::new(["a"]);
        let first = model.begin_list();
        let second = model.begin_list();
        assert!(!model.accepts(first));
        assert!(model.accepts(second));
        model.set_active(&["a"], surface);
        assert!(model.switch_host());
        assert!(!model.accepts(second));
        assert!(!model.is_active("a"));
        assert!(!model.switch_host(), "nothing left to deactivate");
    }

    #[test]
    fn the_timeline_revision_grows_on_changes_and_invalidation_only() {
        let mut model = HostModel::new(["a"]);
        assert_eq!(model.timeline_revision(), 0);
        model.set_active(&["a"], surface);
        assert_eq!(model.timeline_revision(), 1);
        model.set_active(&["a"], surface);
        assert_eq!(model.timeline_revision(), 1);
        assert_eq!(
            model.request(HostRequest::Notify("a")),
            PluginHostEvent::PluginStateChanged { plugin_id: "a" }
        );
        assert_eq!(model.timeline_revision(), 1);
        assert_eq!(
            model.request(HostRequest::InvalidateTimeline),
            PluginHostEvent::TimelineTransformsChanged
        );
        assert_eq!(model.timeline_revision(), 2);
        model.request(HostRequest::Emit(PluginHostEvent::OpenPluginsPage));
        assert_eq!(model.timeline_revision(), 2);
        model.switch_host();
        assert_eq!(model.timeline_revision(), 3);
    }
}
