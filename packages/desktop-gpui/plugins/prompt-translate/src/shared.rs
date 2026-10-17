//! What every contribution of one activation shares: the context, the `translate` settings
//! record, the current values, the note cache, and the per-agent modes. Only the contribution
//! closures hold it, so deactivating the plugin drops it.

use crate::note_view::NoteView;
use crate::notes::{Cache, Request};
use crate::prefs::Modes;
use crate::rpc::{lookup_call, nullable_text};
use crate::settings::Settings;
use crate::state::{Current, now_ms};
use bachuc_client::{Client, ClientEvent};
use bachuc_plugin_host::{PluginContext, PluginSettings, SettingsState};
use gpui_kit::{App, Context, Entity, Subscription, WeakEntity};
use std::cell::RefCell;
use std::rc::Rc;

pub(crate) struct Shared {
    pub(crate) ctx: PluginContext,
    pub(crate) settings: Entity<PluginSettings>,
    pub(crate) current: RefCell<Current>,
    pub(crate) notes: RefCell<Cache<WeakEntity<NoteView>>>,
    pub(crate) modes: RefCell<Modes>,
}

impl Shared {
    pub(crate) fn new(ctx: PluginContext, settings: Entity<PluginSettings>) -> Rc<Self> {
        Rc::new(Self {
            ctx,
            settings,
            current: RefCell::new(Current::new(now_ms())),
            notes: RefCell::default(),
            modes: RefCell::default(),
        })
    }

    /// The stored settings once they load.
    pub(crate) fn ready_settings(&self, cx: &App) -> Option<Settings> {
        match self.settings.read(cx).state() {
            SettingsState::Ready { values, .. } => Some(Settings::decode(values)),
            _ => None,
        }
    }

    /// Takes the stored settings as the current values. The transformer reads `translate`, so
    /// the timeline runs it again when that flag changes.
    pub(crate) fn sync_settings(&self, cx: &mut App) {
        if let Some(next) = self.ready_settings(cx) {
            let translate_changed = self.current.borrow_mut().apply(next, now_ms());
            if translate_changed {
                self.ctx.invalidate_timeline(cx);
            }
        }
        self.ctx.notify(cx);
    }

    /// Sends a note lookup and settles the cache with its answer.
    pub(crate) fn start_lookup(self: &Rc<Self>, key: String, request: Request, cx: &mut App) {
        let (method, input, field) = lookup_call(&request);
        let answer = self.ctx.rpc(method, input, cx);
        let weak = Rc::downgrade(self);
        cx.spawn(async move |cx| {
            let answer = answer.await.and_then(|value| nullable_text(&value, field));
            cx.update(|cx| {
                let Some(shared) = weak.upgrade() else { return };
                let (waiters, outcome) = shared.notes.borrow_mut().finish(&key, answer);
                for view in waiters.into_iter().filter_map(|view| view.upgrade()) {
                    view.update(cx, |view, cx| view.settled(&key, &outcome, cx));
                }
            });
        })
        .detach();
    }

    /// After `ClientEvent::Connected`: forget every mode, so each agent reads again.
    fn reconnected(&self, cx: &mut App) {
        self.modes.borrow_mut().clear();
        self.ctx.notify(cx);
    }
}

/// The subscriptions of one activation. A contribution closure holds this entity.
pub(crate) struct Watcher {
    /// Held strong: a subscription holds its emitter weakly.
    _client: Option<Entity<Client>>,
    _connected: Option<Subscription>,
    _settings: Subscription,
}

impl Watcher {
    pub(crate) fn new(shared: Rc<Shared>, cx: &mut Context<Self>) -> Self {
        let client: Option<Entity<Client>> = shared.ctx.client(cx);
        let reconnect = shared.clone();
        let connected = client.as_ref().map(|client| {
            cx.subscribe(client, move |_, _, event: &ClientEvent, cx| {
                if matches!(event, ClientEvent::Connected) {
                    reconnect.reconnected(cx);
                }
            })
        });
        // The settings load after the contributions: follow them when they arrive or change.
        let settings = shared.settings.clone();
        let sync = shared.clone();
        let observe = cx.observe(&settings, move |_, _, cx| sync.sync_settings(cx));
        shared.sync_settings(cx);
        Self {
            _client: client,
            _connected: connected,
            _settings: observe,
        }
    }
}
