//! `BoardStore`: one per activation. It polls `board.snapshot` for the page and for TASK-193's
//! pills together, keeps the three host settings, and runs the card actions. Only the
//! contribution closures hold it strongly, so deactivating the plugin drops it with its timers
//! and requests; views hold a `WeakEntity`.

use crate::model::pending::{apply_pending, shows_pending};
use crate::model::poll::{Poller, Received};
use crate::model::remove::{
    CloseOutput, RemoveInput, RemoveOutput, close_result, remove_finished_run,
};
use crate::model::run::{board_poll_interval, decode_snapshot};
use crate::model::settings::{
    BOARD_SIZE_ID, ORB_SETTINGS_ID, PROJECT_COLORS_ID, SizeSaver, allocate_colors, decode_hues,
    decode_size, hues_values, orb_settings,
};
use crate::model::text::now_ms;
use crate::model::{BoardSnapshot, ConnectionState, OrbSettings, Run};
use bachuc_client::{Client, ClientEvent, PENDING_TURN_TTL_MS, PendingTurnsChanged};
use bachuc_plugin_host::{
    AgentAnchor, NewWorkspaceRequest, OpenSurfaceOptions, PluginContext, PluginSettings,
    SettingsState, SurfacePresentation,
};
use gpui_kit::{App, AppContext as _, Context, Entity, Subscription, Task};
use serde::Deserialize;
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

const WATCHTOWER_PLUGIN: &str = "watchtower-board";
const WATCHTOWER_SURFACE: &str = "watchtower";

#[derive(Deserialize)]
struct Updated {
    updated: bool,
}

/// A TASK chip links when the active host runs watchtower-board and lists its page.
pub(crate) fn watchtower_open(cx: &App) -> bool {
    bachuc_plugin_host::host(cx).is_some_and(|host| {
        host.read(cx)
            .index()
            .surface(WATCHTOWER_PLUGIN, WATCHTOWER_SURFACE)
            .is_some()
    })
}

pub(crate) struct BoardStore {
    ctx: PluginContext,
    client: Option<Entity<Client>>,
    poller: Poller,
    /// The last decoded `board.snapshot` answer, before the pending-turn overlay.
    raw: Option<BoardSnapshot>,
    /// What the views and the poll cadence read: `raw` with the overlay of the client marks.
    snapshot: Option<BoardSnapshot>,
    /// The agents whose run `snapshot` shows as Running only because of a mark.
    pending: BTreeSet<String>,
    /// Grows when `snapshot` changes, so views rebuild only on new data.
    revision: u64,
    /// The rail state the views last drew.
    shown: ConnectionState,
    palette: Entity<PluginSettings>,
    size_settings: Entity<PluginSettings>,
    orb_settings: Entity<PluginSettings>,
    hues: BTreeMap<String, u16>,
    stored_size: Option<u16>,
    size: SizeSaver,
    orb: OrbSettings,
    palette_saving: bool,
    _poll: Option<Task<()>>,
    _request: Option<Task<()>>,
    _stale: Option<Task<()>>,
    _pending_expiry: Option<Task<()>>,
    _size_save: Option<Task<()>>,
    _palette_save: Option<Task<()>>,
    _subscriptions: Vec<Subscription>,
}

impl BoardStore {
    pub(crate) fn new(ctx: PluginContext, cx: &mut Context<Self>) -> Self {
        let client = ctx.client(cx);
        let palette = ctx.settings(PROJECT_COLORS_ID, cx);
        let size_settings = ctx.settings(BOARD_SIZE_ID, cx);
        let orb = ctx.settings(ORB_SETTINGS_ID, cx);
        let mut subscriptions = vec![
            cx.observe(&palette, |store, _, cx| store.palette_changed(cx)),
            cx.observe(&size_settings, |store, _, cx| store.size_changed(cx)),
            cx.observe(&orb, |store, orb, cx| {
                if let SettingsState::Ready { values, .. } = orb.read(cx).state() {
                    store.orb = orb_settings(values);
                }
                cx.notify();
            }),
        ];
        let mut poller = Poller::default();
        if let Some(client) = &client {
            if client.read(cx).is_connected() {
                poller.connected();
            }
            subscriptions.push(cx.subscribe(client, |store, _, event: &ClientEvent, cx| {
                match event {
                    ClientEvent::Connected => store.poller.connected(),
                    ClientEvent::Disconnected { .. } => store.poller.disconnected(),
                    _ => return,
                }
                store.schedule(cx);
                store.check_connection(cx);
            }));
            subscriptions.push(
                cx.subscribe(client, |store, _, _: &PendingTurnsChanged, cx| {
                    store.pending_changed(cx)
                }),
            );
        }
        let mut store = Self {
            ctx,
            client,
            poller,
            raw: None,
            snapshot: None,
            pending: BTreeSet::new(),
            revision: 0,
            shown: ConnectionState::Connecting,
            palette,
            size_settings,
            orb_settings: orb,
            hues: BTreeMap::new(),
            stored_size: None,
            size: SizeSaver::default(),
            orb: OrbSettings::default(),
            palette_saving: false,
            _poll: None,
            _request: None,
            _stale: None,
            _pending_expiry: None,
            _size_save: None,
            _palette_save: None,
            _subscriptions: subscriptions,
        };
        store.schedule(cx);
        store
    }

    pub(crate) fn snapshot(&self) -> Option<&BoardSnapshot> {
        self.snapshot.as_ref()
    }

    pub(crate) fn revision(&self) -> u64 {
        self.revision
    }

    pub(crate) fn connection(&self, now_ms: i64) -> ConnectionState {
        self.poller.connection(now_ms)
    }

    pub(crate) fn is_connected(&self) -> bool {
        self.poller.is_connected()
    }

    /// A request is in flight, as `isFetching`.
    pub(crate) fn fetching(&self) -> bool {
        self.poller.in_flight()
    }

    pub(crate) fn hues(&self) -> &BTreeMap<String, u16> {
        &self.hues
    }

    pub(crate) fn size(&self) -> u16 {
        self.size.size(self.stored_size)
    }

    /// The saved orb settings; the defaults until they load.
    pub(crate) fn orb(&self) -> OrbSettings {
        self.orb
    }

    pub(crate) fn orb_settings(&self) -> Entity<PluginSettings> {
        self.orb_settings.clone()
    }

    /// The plugin context, for the views' own RPCs and navigation.
    pub(crate) fn context(&self) -> PluginContext {
        self.ctx.clone()
    }

    /// The active host's name, as the lane rail shows it.
    pub(crate) fn host_label(&self, cx: &App) -> String {
        self.client
            .as_ref()
            .map(|client| client.read(cx).host().name.clone())
            .unwrap_or_default()
    }

    /// Loads again now, or as soon as the request in flight ends.
    pub(crate) fn refresh(&mut self, cx: &mut Context<Self>) {
        self.poller.refresh();
        self.schedule(cx);
        cx.notify();
    }

    // ---- polling ----

    /// Reads the shown snapshot, so the fast cadence runs while a mark shows a card as Running.
    fn interval(&self) -> i64 {
        board_poll_interval(
            self.snapshot
                .as_ref()
                .map(|snapshot| snapshot.runs.as_slice()),
        )
    }

    /// Sends when a poll is due, else sleeps until the next one. A dropped store drops the
    /// timer and the request with it.
    fn schedule(&mut self, cx: &mut Context<Self>) {
        let now = now_ms();
        let interval = self.interval();
        if self.poller.should_send(now, interval) {
            self.send(now, cx);
            return;
        }
        self._poll = self.poller.next_wake(now, interval).map(|delay| {
            cx.spawn(async move |this, cx| {
                cx.background_executor()
                    .timer(Duration::from_millis(delay.max(0) as u64))
                    .await;
                this.update(cx, |store, cx| store.schedule(cx)).ok();
            })
        });
    }

    fn send(&mut self, now: i64, cx: &mut Context<Self>) {
        self.poller.begin(now);
        self._poll = None;
        let answer = self.ctx.rpc("board.snapshot", json!({}), cx);
        self._request = Some(cx.spawn(async move |this, cx| {
            let result = answer.await;
            this.update(cx, |store, cx| store.receive(result, cx)).ok();
        }));
    }

    fn receive(&mut self, result: anyhow::Result<serde_json::Value>, cx: &mut Context<Self>) {
        let now = now_ms();
        if let Err(error) = &result {
            log::warn!("board: board.snapshot failed: {error:#}");
        }
        let received = self.poller.finish(result, now);
        if received == Received::Changed {
            match self.poller.last_json().map(decode_snapshot) {
                Some(Ok(snapshot)) => {
                    self.raw = Some(snapshot);
                    self.rebuild(cx);
                }
                Some(Err(error)) => log::warn!("board: {error:#}"),
                None => {}
            }
        }
        if received != Received::Failed {
            self.confirm_pending(cx);
        }
        self.check_connection(cx);
        self._stale = self.poller.stale_at().map(|at| {
            cx.spawn(async move |this, cx| {
                cx.background_executor()
                    .timer(Duration::from_millis((at - now).max(0) as u64))
                    .await;
                this.update(cx, |store, cx| store.check_connection(cx)).ok();
            })
        });
        self.schedule(cx);
    }

    // ---- pending turns ----

    /// Builds the shown snapshot from the raw one and the client marks, and redraws only when it
    /// differs from the last one. Returns true when a card newly shows as Running by a mark.
    fn rebuild(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(raw) = &self.raw else {
            return false;
        };
        let marks: BTreeMap<String, i64> = {
            let client = self.client.as_ref().map(|client| client.read(cx));
            raw.runs
                .iter()
                .filter_map(|run| {
                    let marked_at = client?.pending_turn(&run.agent_id)?;
                    shows_pending(run, marked_at).then(|| (run.agent_id.clone(), marked_at))
                })
                .collect()
        };
        let shown = apply_pending(raw, |id| marks.get(id).copied());
        let added = marks.keys().any(|id| !self.pending.contains(id));
        self.pending = marks.keys().cloned().collect();
        self.arm_pending_expiry(marks.values().min().copied(), cx);
        if self.snapshot.as_ref() == Some(&shown) {
            return false;
        }
        self.snapshot = Some(shown);
        self.revision += 1;
        self.sync_palette(cx);
        cx.notify();
        added
    }

    /// A mark changed. A card that newly shows as Running asks for one early load; a clear and
    /// a change with no card change never do, so a confirm cannot start a poll loop.
    fn pending_changed(&mut self, cx: &mut Context<Self>) {
        if self.rebuild(cx) {
            self.refresh(cx);
        }
    }

    /// Rebuilds once the earliest mark that shows ends.
    fn arm_pending_expiry(&mut self, earliest: Option<i64>, cx: &mut Context<Self>) {
        self._pending_expiry = earliest.map(|marked_at| {
            let delay = (marked_at + PENDING_TURN_TTL_MS + 1 - now_ms()).max(0);
            cx.spawn(async move |this, cx| {
                cx.background_executor()
                    .timer(Duration::from_millis(delay as u64))
                    .await;
                this.update(cx, |store, cx| {
                    store.rebuild(cx);
                })
                .ok();
            })
        });
    }

    /// Clears the mark of each agent whose raw run no longer needs it: it runs, or it ended
    /// after the mark. A mark with no run waits for its run or its limit.
    fn confirm_pending(&mut self, cx: &mut Context<Self>) {
        let (Some(client), Some(raw)) = (&self.client, &self.raw) else {
            return;
        };
        let confirmed: Vec<String> = raw
            .runs
            .iter()
            .filter(|run| {
                client
                    .read(cx)
                    .pending_turn(&run.agent_id)
                    .is_some_and(|marked_at| !shows_pending(run, marked_at))
            })
            .map(|run| run.agent_id.clone())
            .collect();
        for agent_id in confirmed {
            client.update(cx, |client, cx| client.clear_turn_pending(&agent_id, cx));
        }
    }

    /// Redraws only when the rail state changed, so an unchanged poll draws nothing.
    fn check_connection(&mut self, cx: &mut Context<Self>) {
        let state = self.poller.connection(now_ms());
        if state != self.shown {
            self.shown = state;
            cx.notify();
        }
    }

    // ---- settings ----

    fn palette_changed(&mut self, cx: &mut Context<Self>) {
        if let SettingsState::Ready { values, .. } = self.palette.read(cx).state() {
            self.hues = decode_hues(values);
        }
        self.sync_palette(cx);
        cx.notify();
    }

    /// Gives each new project of the snapshot a hue; one save at a time, and none while a
    /// save error shows.
    fn sync_palette(&mut self, cx: &mut Context<Self>) {
        if self.palette_saving || !self.poller.is_connected() {
            return;
        }
        let palette = self.palette.read(cx);
        let SettingsState::Ready { revision, .. } = palette.state() else {
            return;
        };
        if palette.saving() || palette.save_error().is_some() {
            return;
        }
        let revision = revision.clone();
        let ids: BTreeSet<String> = self
            .snapshot
            .iter()
            .flat_map(|snapshot| &snapshot.runs)
            .filter_map(|run| run.project_id.clone())
            .collect();
        let ids: Vec<String> = ids.into_iter().collect();
        let next = allocate_colors(&self.hues, &ids);
        if next == self.hues {
            return;
        }
        self.palette_saving = true;
        let save = self.palette.update(cx, |palette, cx| {
            palette.save(hues_values(&next), revision, cx)
        });
        self._palette_save = Some(cx.spawn(async move |this, cx| {
            save.await;
            this.update(cx, |store, _| store.palette_saving = false)
                .ok();
        }));
    }

    /// "Could not save project colors. Tap to retry."
    pub(crate) fn palette_error(&self, cx: &App) -> bool {
        let palette = self.palette.read(cx);
        matches!(palette.state(), SettingsState::Error(_)) || palette.save_error().is_some()
    }

    pub(crate) fn retry_palette(&mut self, cx: &mut Context<Self>) {
        self.palette.update(cx, |palette, cx| palette.reload(cx));
    }

    fn size_changed(&mut self, cx: &mut Context<Self>) {
        if let SettingsState::Ready { values, .. } = self.size_settings.read(cx).state() {
            self.stored_size = Some(decode_size(values));
        }
        self.save_size(cx);
        cx.notify();
    }

    pub(crate) fn pick_size(&mut self, size: i64, cx: &mut Context<Self>) {
        self.size.pick(size);
        self.save_size(cx);
        cx.notify();
    }

    fn save_size(&mut self, cx: &mut Context<Self>) {
        let settings = self.size_settings.read(cx);
        let (stored, revision) = match settings.state() {
            SettingsState::Ready { values, revision } => {
                (Some(decode_size(values)), revision.clone())
            }
            _ => (None, String::new()),
        };
        let Some(size) = self.size.next_save(stored, settings.saving()) else {
            return;
        };
        let save = self.size_settings.update(cx, |settings, cx| {
            settings.save(json!({ "size": size }), revision, cx)
        });
        self._size_save = Some(cx.spawn(async move |this, cx| {
            let saved = save.await;
            this.update(cx, |store, cx| {
                store.size.saved(size, saved);
                store.save_size(cx);
                cx.notify();
            })
            .ok();
        }));
    }

    // ---- card actions ----

    pub(crate) fn set_starred(
        &mut self,
        run_id: &str,
        starred: bool,
        cx: &mut Context<Self>,
    ) -> Task<anyhow::Result<()>> {
        let Some(snapshot) = self
            .snapshot
            .as_ref()
            .filter(|_| self.poller.is_connected())
        else {
            return Task::ready(Err(anyhow::anyhow!(
                "Host is offline. Reconnect and retry."
            )));
        };
        let input = json!({
            "id": run_id,
            "observingSince": snapshot.observing_since,
            "starred": starred,
        });
        let answer = self
            .ctx
            .rpc_as::<_, Updated>("board.set-starred", &input, cx);
        cx.spawn(async move |this, cx| {
            if !answer.await?.updated {
                anyhow::bail!("Run changed. Refresh and retry.");
            }
            this.update(cx, |store, cx| store.refresh(cx))
        })
    }

    pub(crate) fn set_diffs_off(
        &mut self,
        agent_id: &str,
        off: bool,
        cx: &mut Context<Self>,
    ) -> Task<anyhow::Result<()>> {
        let input = json!({ "agentId": agent_id, "off": off });
        let answer = self.ctx.rpc("board.edit-diffs.set", input, cx);
        cx.spawn(async move |this, cx| {
            answer.await?;
            this.update(cx, |store, cx| store.refresh(cx))
        })
    }

    /// Removes a finished run (`removeFinishedRun`); `false` when the run changed meanwhile.
    pub(crate) fn remove(
        &mut self,
        run_id: &str,
        cx: &mut Context<BoardStore>,
    ) -> Task<anyhow::Result<bool>> {
        let Some(snapshot) = &self.snapshot else {
            return Task::ready(Err(anyhow::anyhow!("Board has not loaded.")));
        };
        let Some(run) = snapshot.runs.iter().find(|run| run.id == run_id) else {
            return Task::ready(Ok(true));
        };
        let (id, ended_at) = (run.id.clone(), run.ended_at.clone());
        let scope = snapshot.observing_since.clone();
        let ctx = self.ctx.clone();
        cx.spawn(async move |this, cx| {
            let removed = remove_finished_run(
                &id,
                ended_at.as_deref(),
                &scope,
                |scope| {
                    let input = RemoveInput {
                        id: &id,
                        observing_since: &scope,
                        ended_at: ended_at.as_deref(),
                    };
                    let answer = cx.update(|cx| {
                        ctx.rpc_as::<_, RemoveOutput>("board.remove-finished", &input, cx)
                    });
                    async move { Ok(answer.await?.removed) }
                },
                || {
                    let answer = cx.update(|cx| ctx.rpc("board.snapshot", json!({}), cx));
                    async move { decode_snapshot(&answer.await?) }
                },
            )
            .await?;
            this.update(cx, |store, cx| store.refresh(cx))?;
            Ok(removed)
        })
    }

    /// Releases the processes of a removed thread and its subagents.
    pub(crate) fn close_runtime(
        &mut self,
        agent_id: &str,
        cx: &mut Context<BoardStore>,
    ) -> Task<anyhow::Result<()>> {
        let answer = self.ctx.rpc_as::<_, CloseOutput>(
            "board.close-runtime",
            &json!({ "agentId": agent_id }),
            cx,
        );
        cx.background_spawn(async move { close_result(&answer.await?) })
    }

    // ---- navigation ----

    /// Opens the thread at its latest prompt, as Electron's `anchor: "latest-prompt"`.
    pub(crate) fn open_agent(&self, agent_id: &str, cx: &mut App) {
        self.ctx
            .open_agent_at(agent_id, AgentAnchor::LatestPrompt, cx);
    }

    /// Opens Watchtower at the run's TASK on its workspace (`hosts.ts:19-26`).
    pub(crate) fn open_task(&self, run: &Run, cx: &mut App) {
        if let Some(workspace_id) = &run.workspace_id {
            self.open_watchtower(workspace_id, run.task_id.as_deref(), cx);
        }
    }

    /// Opens Watchtower on a workspace, at `task` when given.
    pub(crate) fn open_watchtower(&self, workspace_id: &str, task: Option<&str>, cx: &mut App) {
        let mut params = BTreeMap::from([("workspaceId".to_string(), workspace_id.to_string())]);
        if let Some(task) = task {
            params.insert("task".into(), task.to_string());
        }
        self.ctx.open_surface(
            WATCHTOWER_SURFACE,
            OpenSurfaceOptions {
                plugin_id: Some(WATCHTOWER_PLUGIN.into()),
                presentation: SurfacePresentation::Overlay,
                params,
            },
            cx,
        );
    }

    /// The New workspace screen on the run's project, as the sidebar project "+" button.
    pub(crate) fn open_project(&self, run: &Run, cx: &mut App) -> Result<(), &'static str> {
        let Some(cwd) = run.cwd.clone() else {
            return Err("Project directory is unavailable.");
        };
        self.ctx.open_new_workspace(
            NewWorkspaceRequest {
                cwd,
                project_id: run.project_id.clone(),
                name: Some(run.project.clone()),
            },
            cx,
        );
        Ok(())
    }
}
