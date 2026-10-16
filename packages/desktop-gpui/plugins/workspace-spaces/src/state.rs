//! `SpacesState`: the one entity of an activation that the filter, the project menu, the strip,
//! and the page share (`client/sidebar-state.ts` and `spaces-sidebar.ts`). It holds the
//! `spaces` settings entity, the catalog, the saving flag, the errors, and the selected Space.
//! The rules live in `model`; this file runs the requests and keeps the order of effects.

use crate::model::{
    ASSIGN_FAILED, Catalog, CatalogProject, Choice, HostSpaces, MoveProject, Preferences,
    SAVE_FAILED, Snapshot, UNAVAILABLE, add_project, add_space, created_space, host_state,
    is_visible, move_catalog_project, move_choices, move_project, move_projects, next_wait_ms,
    refresh_due, removal_selection, remove_space, rename_space, select_direction, selected_space,
    swipe_target, text_or, title, with_host_state,
};
use bachuc_client::{Client, ClientEvent};
use bachuc_plugin_host::{
    PluginContext, PluginSettings, SettingsState, SidebarProject, SwipeDirection,
};
use futures::channel::oneshot;
use gpui_kit::{App, AppContext as _, Context, Entity, SharedString, Subscription, Task};
use serde_json::json;
use std::time::{Duration, Instant};

const CATALOG_RPC: &str = "spaces.catalog";

/// Builds the next Space state of the host from the current one.
pub(crate) type Change = Box<dyn FnOnce(&Snapshot) -> Result<HostSpaces, String>>;

pub(crate) struct SpacesState {
    ctx: PluginContext,
    settings: Entity<PluginSettings>,
    /// Held strong: a subscription holds its emitter weakly.
    client: Option<Entity<Client>>,
    /// The last values that passed the membership rules.
    prefs: Option<Preferences>,
    /// The active client's `server_info().server_id`; `None` until the host is known.
    host: Option<String>,
    spaces: Option<HostSpaces>,
    catalog: Option<Vec<CatalogProject>>,
    /// Both the settings and the catalog are loaded for a known host.
    snapshot: Option<Snapshot>,
    busy: bool,
    save_error: String,
    settings_error: String,
    catalog_error: String,
    selected: Option<String>,
    refreshing: bool,
    last_refresh: Option<Instant>,
    waiters: Vec<oneshot::Sender<()>>,
    _refresh: Option<Task<()>>,
    _save: Option<Task<()>>,
    _poll: Option<Task<()>>,
    _subscriptions: Vec<Subscription>,
}

impl SpacesState {
    pub(crate) fn new(
        ctx: PluginContext,
        settings: Entity<PluginSettings>,
        cx: &mut Context<Self>,
    ) -> Self {
        let client = ctx.client(cx);
        let connected = client.as_ref().is_some_and(|c| c.read(cx).is_connected());
        let mut subscriptions = vec![cx.observe(&settings, |this, _, cx| this.sync_settings(cx))];
        if let Some(client) = &client {
            subscriptions.push(cx.subscribe(
                client,
                |this: &mut Self, _, event: &ClientEvent, cx| {
                    // A request sent before `Connected` is lost: load again after each connect.
                    if matches!(event, ClientEvent::Connected) {
                        this.rebuild(cx);
                        this.changed(None, cx);
                        let _ = this.refresh(cx);
                    }
                },
            ));
        }
        let poll = cx.spawn(async move |this, cx| {
            loop {
                let Ok(wait) = this.update(cx, |this, _| next_wait_ms(this.since_refresh())) else {
                    break;
                };
                cx.background_executor()
                    .timer(Duration::from_millis(wait))
                    .await;
                let alive = this
                    .update(cx, |this, cx| {
                        if this.connected(cx) && refresh_due(this.since_refresh(), this.refreshing)
                        {
                            let _ = this.refresh(cx);
                        }
                    })
                    .is_ok();
                if !alive {
                    break;
                }
            }
        });
        let mut state = Self {
            ctx,
            settings,
            client,
            prefs: None,
            host: None,
            spaces: None,
            catalog: None,
            snapshot: None,
            busy: false,
            save_error: String::new(),
            settings_error: String::new(),
            catalog_error: String::new(),
            selected: None,
            refreshing: false,
            last_refresh: None,
            waiters: Vec::new(),
            _refresh: None,
            _save: None,
            _poll: Some(poll),
            _subscriptions: subscriptions,
        };
        state.apply_settings(cx);
        state.rebuild(cx);
        if connected {
            let _ = state.refresh(cx);
        }
        state
    }

    // ---- reads ----

    pub(crate) fn busy(&self) -> bool {
        self.busy
    }

    pub(crate) fn host(&self) -> Option<&str> {
        self.host.as_deref()
    }

    /// This host's Spaces, once the settings and the host are known.
    pub(crate) fn spaces(&self) -> Option<&HostSpaces> {
        self.spaces.as_ref()
    }

    pub(crate) fn snapshot(&self) -> Option<&Snapshot> {
        self.snapshot.as_ref()
    }

    pub(crate) fn catalog(&self) -> Option<&[CatalogProject]> {
        self.catalog.as_deref()
    }

    pub(crate) fn catalog_error(&self) -> &str {
        &self.catalog_error
    }

    /// The saved selection, which may name a Space that is gone; `selected` resolves it.
    pub(crate) fn selection(&self) -> Option<&str> {
        self.selected.as_deref()
    }

    pub(crate) fn selected(&self) -> Option<&str> {
        self.spaces
            .as_ref()
            .map(|spaces| selected_space(spaces, self.selected.as_deref()))
    }

    pub(crate) fn load_error(&self) -> &str {
        if self.settings_error.is_empty() {
            &self.catalog_error
        } else {
            &self.settings_error
        }
    }

    /// The error the filter reads: a failed save, or a failed load once the Spaces are shown.
    pub(crate) fn error(&self) -> &str {
        if !self.save_error.is_empty() {
            &self.save_error
        } else if self.snapshot.is_some() {
            self.load_error()
        } else {
            ""
        }
    }

    pub(crate) fn save_error(&self) -> &str {
        &self.save_error
    }

    /// The error text for a failed action, or `fallback` when there is none.
    pub(crate) fn error_or(&self, fallback: &str) -> String {
        text_or(self.error(), fallback)
    }

    /// The settings are still on their first read.
    pub(crate) fn loading(&self) -> bool {
        self.prefs.is_none() && self.settings_error.is_empty()
    }

    pub(crate) fn saving(&self, cx: &App) -> bool {
        self.settings.read(cx).saving()
    }

    pub(crate) fn host_label(&self, cx: &App) -> String {
        self.client
            .as_ref()
            .map(|client| client.read(cx).host().name.clone())
            .unwrap_or_default()
    }

    pub(crate) fn is_visible(&self, view_key: &str) -> bool {
        is_visible(
            self.snapshot.as_ref(),
            !self.error().is_empty(),
            self.selected.as_deref(),
            view_key,
        )
    }

    pub(crate) fn title(&self) -> Option<SharedString> {
        title(
            self.snapshot.as_ref(),
            !self.error().is_empty(),
            self.selected.as_deref(),
        )
        .map(SharedString::from)
    }

    pub(crate) fn move_choices(&self, view_key: &str) -> Vec<Choice> {
        move_choices(self.snapshot.as_ref(), self.busy, view_key)
    }

    pub(crate) fn move_projects(&self) -> Vec<MoveProject> {
        move_projects(self.snapshot.as_ref())
    }

    // ---- selection ----

    /// Selects a Space; the sidebar slides toward it.
    pub(crate) fn select(&mut self, id: &str, cx: &mut Context<Self>) {
        let direction = self
            .spaces
            .as_ref()
            .and_then(|spaces| select_direction(spaces, self.selected.as_deref(), id));
        self.selected = Some(id.to_string());
        self.changed(direction, cx);
    }

    /// A trackpad swipe moves to the adjacent Space without wrapping, and waits out a save.
    pub(crate) fn swipe(&mut self, direction: SwipeDirection, cx: &mut Context<Self>) {
        if let Some(to) = swipe_target(
            self.snapshot.as_ref(),
            self.busy,
            self.selected.as_deref(),
            direction,
        ) {
            self.select(&to, cx);
        }
    }

    // ---- loading ----

    fn connected(&self, cx: &App) -> bool {
        self.client
            .as_ref()
            .is_some_and(|client| client.read(cx).is_connected())
    }

    fn since_refresh(&self) -> Option<u64> {
        self.last_refresh
            .map(|started| started.elapsed().as_millis() as u64)
    }

    /// Reads the settings and the catalog again (the page's Refresh button).
    pub(crate) fn reload(&mut self, cx: &mut Context<Self>) {
        self.settings.update(cx, |settings, cx| settings.reload(cx));
        let _ = self.refresh(cx);
    }

    /// Reads the catalog. One request is in flight at most; a second caller waits for it. A
    /// refresh does not run while a save does, and it needs a connection.
    pub(crate) fn refresh(&mut self, cx: &mut Context<Self>) -> oneshot::Receiver<()> {
        let (done, wait) = oneshot::channel();
        if self.busy || !self.connected(cx) {
            done.send(()).ok();
            return wait;
        }
        self.waiters.push(done);
        if !self.refreshing {
            self.refreshing = true;
            self.last_refresh = Some(Instant::now());
            let answer = self.ctx.rpc_as::<_, Catalog>(CATALOG_RPC, &json!({}), cx);
            self._refresh = Some(cx.spawn(async move |this, cx| {
                let result = answer.await;
                this.update(cx, |this, cx| this.finish_refresh(result, cx))
                    .ok();
            }));
        }
        wait
    }

    fn finish_refresh(&mut self, result: anyhow::Result<Catalog>, cx: &mut Context<Self>) {
        self.refreshing = false;
        match result {
            Ok(catalog) => {
                self.catalog_error.clear();
                self.save_error.clear();
                if self.catalog.as_ref() != Some(&catalog.projects) {
                    self.catalog = Some(catalog.projects);
                    self.rebuild(cx);
                }
            }
            Err(error) => self.catalog_error = format!("{error:#}"),
        }
        for waiter in std::mem::take(&mut self.waiters) {
            waiter.send(()).ok();
        }
        self.changed(None, cx);
    }

    /// Takes the settings values when they pass the membership rules; a failed read keeps the
    /// last good values and shows its error.
    fn apply_settings(&mut self, cx: &App) {
        let parsed = match self.settings.read(cx).state() {
            SettingsState::Loading => return,
            SettingsState::Ready { values, .. } => Preferences::parse(values),
            SettingsState::Invalid { error, .. } | SettingsState::Error(error) => {
                Err(error.clone())
            }
        };
        match parsed {
            Ok(prefs) => {
                self.settings_error.clear();
                if self.prefs.as_ref() != Some(&prefs) {
                    self.prefs = Some(prefs);
                    self.rebuild(cx);
                }
            }
            Err(error) => self.settings_error = error,
        }
    }

    fn sync_settings(&mut self, cx: &mut Context<Self>) {
        self.apply_settings(cx);
        self.changed(None, cx);
    }

    /// Computes the host's Spaces and the indexed snapshot from the loaded parts.
    fn rebuild(&mut self, cx: &App) {
        self.host = self.client.as_ref().and_then(|client| {
            client
                .read(cx)
                .server_info()
                .map(|info| info.server_id.clone())
        });
        self.spaces = match (&self.prefs, &self.host) {
            (Some(prefs), Some(host)) => Some(host_state(prefs, host)),
            _ => None,
        };
        self.snapshot = match (&self.spaces, &self.catalog) {
            (Some(spaces), Some(catalog)) => Some(Snapshot::new(spaces.clone(), catalog.clone())),
            _ => None,
        };
    }

    /// Redraws the views and tells the sidebar to filter again, with the slide direction when
    /// the selected Space moved.
    fn changed(&mut self, direction: Option<SwipeDirection>, cx: &mut Context<Self>) {
        cx.notify();
        self.ctx.sidebar_filter_changed(direction, cx);
    }

    // ---- saving ----

    /// Runs `change` against the host's current Spaces and saves the result. It waits for a
    /// refresh in flight, runs one save at a time, and resolves to whether it saved. A failed
    /// save keeps the old values and sets the error.
    pub(crate) fn save(&mut self, change: Change, cx: &mut Context<Self>) -> Task<bool> {
        let refreshing = self.refreshing.then(|| self.refresh(cx));
        cx.spawn(async move |this, cx| {
            if let Some(wait) = refreshing {
                wait.await.ok();
            }
            let Ok(saving) = this.update(cx, |this, cx| this.begin_save(change, cx)) else {
                return false;
            };
            saving.await
        })
    }

    fn fail_save(&mut self, message: &str, cx: &mut Context<Self>) -> Task<bool> {
        self.save_error = message.to_string();
        self.changed(None, cx);
        Task::ready(false)
    }

    fn begin_save(&mut self, change: Change, cx: &mut Context<Self>) -> Task<bool> {
        if self.busy {
            return Task::ready(false);
        }
        let Some(host) = self.host.clone() else {
            return self.fail_save(UNAVAILABLE, cx);
        };
        let SettingsState::Ready { values, revision } = self.settings.read(cx).state().clone()
        else {
            return self.fail_save(UNAVAILABLE, cx);
        };
        let prefs = match Preferences::parse(&values) {
            Ok(prefs) => prefs,
            Err(error) => return self.fail_save(&error, cx),
        };
        self.busy = true;
        self.save_error.clear();
        self.changed(None, cx);
        let snapshot = Snapshot::new(
            host_state(&prefs, &host),
            self.catalog.clone().unwrap_or_default(),
        );
        let next = change(&snapshot).and_then(|state| with_host_state(&prefs, &host, state));
        let next = match next {
            Ok(next) => next,
            Err(error) => {
                self.busy = false;
                return self.fail_save(&error, cx);
            }
        };
        if next == prefs {
            self.busy = false;
            self.changed(None, cx);
            return Task::ready(true);
        }
        let saving = self.settings.update(cx, |settings, cx| {
            settings.save(next.to_value(), revision, cx)
        });
        let (done, saved) = oneshot::channel();
        // The save finishes even when the caller drops its task.
        self._save = Some(cx.spawn(async move |this, cx| {
            let ok = saving.await;
            this.update(cx, |this, cx| this.finish_save(ok, cx)).ok();
            done.send(ok).ok();
        }));
        cx.background_spawn(async move { saved.await.unwrap_or(false) })
    }

    fn finish_save(&mut self, saved: bool, cx: &mut Context<Self>) {
        self.busy = false;
        if saved {
            self.apply_settings(cx);
        } else {
            let reason = self.settings.read(cx).save_error().unwrap_or_default();
            self.save_error = text_or(reason, SAVE_FAILED);
        }
        self.changed(None, cx);
    }

    /// Adds a Space and selects it once the save lands.
    pub(crate) fn create(&mut self, cx: &mut Context<Self>) -> Task<bool> {
        let saved = self.save(Box::new(|snapshot| add_space(&snapshot.state)), cx);
        cx.spawn(async move |this, cx| {
            let ok = saved.await;
            if ok {
                this.update(cx, |this, cx| {
                    let last = this
                        .spaces
                        .as_ref()
                        .and_then(created_space)
                        .map(str::to_string);
                    if let Some(id) = last {
                        this.select(&id, cx);
                    }
                })
                .ok();
            }
            ok
        })
    }

    pub(crate) fn rename(
        &mut self,
        id: String,
        name: String,
        cx: &mut Context<Self>,
    ) -> Task<bool> {
        self.save(
            Box::new(move |snapshot| rename_space(&snapshot.state, &id, &name)),
            cx,
        )
    }

    /// Removes a Space. The removal target is selected before the save, so the list never falls
    /// back to the first Space in between; a failed save restores the selection.
    pub(crate) fn remove(&mut self, id: String, cx: &mut Context<Self>) -> Task<bool> {
        let target = self
            .spaces
            .as_ref()
            .and_then(|spaces| removal_selection(spaces, self.selected.as_deref(), &id));
        let moving = target.is_some();
        if let Some(target) = &target {
            self.select(target, cx);
        }
        let removed = id.clone();
        let saved = self.save(
            Box::new(move |snapshot| remove_space(&snapshot.state, &removed)),
            cx,
        );
        cx.spawn(async move |this, cx| {
            let ok = saved.await;
            if !ok && moving {
                this.update(cx, |this, cx| this.select(&id, cx)).ok();
            }
            ok
        })
    }

    /// Moves a project row by its view key, as the project menu does.
    pub(crate) fn move_view_to(
        &mut self,
        key: String,
        target: String,
        cx: &mut Context<Self>,
    ) -> Task<bool> {
        self.save(
            Box::new(move |snapshot| crate::model::move_view(snapshot, &key, &target)),
            cx,
        )
    }

    /// Moves a catalog project, by its view key when it has one (the Move projects page).
    pub(crate) fn move_catalog_project_to(
        &mut self,
        project_id: String,
        target: String,
        cx: &mut Context<Self>,
    ) -> Task<bool> {
        if self.snapshot.is_none() {
            return Task::ready(false);
        }
        self.save(
            Box::new(move |snapshot| {
                move_catalog_project(snapshot, &project_id, &target)
                    .unwrap_or_else(|| Err("Project unavailable. Refresh first.".into()))
            }),
            cx,
        )
    }

    /// Moves a project by the page's own key, `project_key(<serverId>, <projectId>)`.
    pub(crate) fn move_page_project(
        &mut self,
        project_id: String,
        target: String,
        cx: &mut Context<Self>,
    ) -> Task<bool> {
        let Some(host) = self.host.clone() else {
            return Task::ready(false);
        };
        self.save(
            Box::new(move |snapshot| {
                move_project(
                    &snapshot.state,
                    &crate::model::project_key(&host, &project_id),
                    &target,
                )
            }),
            cx,
        )
    }

    /// Saves a project the sidebar added into the selected Space. It refreshes first, and an
    /// already assigned project stays where it is.
    pub(crate) fn add_project(
        &mut self,
        project: SidebarProject,
        cx: &mut Context<Self>,
    ) -> Task<anyhow::Result<()>> {
        let Some(target) = self
            .snapshot
            .as_ref()
            .and_then(|_| self.selected())
            .map(str::to_string)
        else {
            return Task::ready(Err(anyhow::anyhow!(UNAVAILABLE)));
        };
        let Some(id) = project.project_ids.first().cloned() else {
            let message = self.error_or(ASSIGN_FAILED);
            return Task::ready(Err(anyhow::anyhow!(message)));
        };
        let view_key = (!project.view_key.is_empty()).then(|| project.view_key.clone());
        cx.spawn(async move |this, cx| {
            let wait = this.update(cx, |this, cx| this.refresh(cx))?;
            wait.await.ok();
            let saving = this.update(cx, |this, cx| {
                this.save(
                    Box::new(move |snapshot| {
                        add_project(&snapshot.state, &id, view_key.as_deref(), &target)
                    }),
                    cx,
                )
            })?;
            if saving.await {
                return Ok(());
            }
            let message = this.update(cx, |this, _| this.error_or(ASSIGN_FAILED))?;
            Err(anyhow::anyhow!(message))
        })
    }
}
