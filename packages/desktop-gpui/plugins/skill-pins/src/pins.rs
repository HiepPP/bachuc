//! `Pins`: the skills pinned on each composer, as `client/pins.ts` keeps them.
//!
//! An agent composer reads its pins through `skill-pins.read` and saves through
//! `skill-pins.write`. A New workspace draft keeps them in its plugin value. One `Pins` serves
//! one activation; only the contribution closures hold it, so deactivating the plugin drops it.
//!
//! A pill's `state` closure gets only `&App`, so the store keeps the lists in a `RefCell` and
//! starts a read the first time an agent is asked for. At most one read runs per agent between
//! reconnects, however often the pill draws (`read_once_per_agent`).

use crate::PLUGIN_ID;
use crate::catalog::{
    READ, WRITE, draft_payload, draft_skills, normalize, parse_answer, read_input, saved_skills,
    write_input,
};
use crate::menu::SkillsMenu;
use bachuc_client::{Client, ClientEvent};
use bachuc_plugin_host::{ComposerTarget, PluginContext, PluginSettings, SettingsState};
use gpui_kit::{App, Context, Entity, Subscription, WeakEntity};
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

/// The lists and bookkeeping, without any `App`. The key is an agent id.
#[derive(Default)]
pub(crate) struct State {
    known: BTreeMap<String, Vec<String>>,
    in_flight: BTreeSet<String>,
    /// Agents whose read failed. They wait for a reconnect or a plugin state change (a new
    /// store), so one failure starts no read loop.
    failed: BTreeSet<String>,
    errors: BTreeMap<String, String>,
}

impl State {
    pub(crate) fn list(&self, key: &str) -> Option<&Vec<String>> {
        self.known.get(key)
    }

    pub(crate) fn error(&self, key: &str) -> Option<&str> {
        self.errors.get(key).map(String::as_str)
    }

    /// A pill asks for `key`. True when the caller starts the read: the list is unknown, no read
    /// runs for it, and its last read did not fail.
    pub(crate) fn ask(&mut self, key: &str) -> bool {
        !self.known.contains_key(key)
            && !self.failed.contains(key)
            && self.in_flight.insert(key.to_string())
    }

    /// Keeps an answer of a read or a write. True when the list differs from the last one,
    /// which is when pills redraw (`publish`, `pins.ts:12`).
    pub(crate) fn publish(&mut self, key: &str, skills: Vec<String>) -> bool {
        self.in_flight.remove(key);
        self.errors.remove(key);
        if self.known.get(key) == Some(&skills) {
            return false;
        }
        self.known.insert(key.to_string(), skills);
        true
    }

    /// A failed read clears the in-flight mark and sets the error line. The list stays unknown.
    pub(crate) fn read_failed(&mut self, key: &str, error: String) {
        self.in_flight.remove(key);
        self.failed.insert(key.to_string());
        self.errors.insert(key.to_string(), error);
    }

    /// A failed write keeps the optimistic list and sets the error line.
    pub(crate) fn write_failed(&mut self, key: &str, error: String) {
        self.errors.insert(key.to_string(), error);
    }

    /// The optimistic part of a save: publishes `next` at once. False while the list is unknown
    /// (`save`, `pins.ts:56`), when nothing is saved.
    pub(crate) fn begin_save(&mut self, key: &str, next: Vec<String>) -> bool {
        if !self.known.contains_key(key) {
            return false;
        }
        self.errors.remove(key);
        self.known.insert(key.to_string(), next);
        true
    }

    /// After a reconnect every agent reads again.
    pub(crate) fn clear(&mut self) {
        *self = Self::default();
    }
}

pub(crate) struct Pins {
    ctx: PluginContext,
    settings: Entity<PluginSettings>,
    state: RefCell<State>,
    /// The open menus. They redraw when a list or an error changes.
    menus: RefCell<Vec<WeakEntity<SkillsMenu>>>,
}

impl Pins {
    pub(crate) fn new(ctx: PluginContext, settings: Entity<PluginSettings>) -> Rc<Self> {
        Rc::new(Self {
            ctx,
            settings,
            state: RefCell::default(),
            menus: RefCell::default(),
        })
    }

    /// The settings `defaults`, once the settings are ready.
    fn defaults(&self, cx: &App) -> Option<Vec<String>> {
        match self.settings.read(cx).state() {
            SettingsState::Ready { values, .. } => Some(saved_skills(values.get("defaults"))),
            _ => None,
        }
    }

    /// The pins of a composer, `None` while unknown. An agent whose list is missing starts one
    /// read.
    pub(crate) fn current(
        self: &Rc<Self>,
        target: &ComposerTarget,
        cx: &App,
    ) -> Option<Vec<String>> {
        if let Some(draft) = &target.draft {
            return draft_skills(draft.read(cx).get(PLUGIN_ID), self.defaults(cx));
        }
        let agent_id = target.agent_id.as_deref()?;
        if self.state.borrow_mut().ask(agent_id) {
            self.spawn_read(agent_id.to_string(), cx);
        }
        self.state.borrow().list(agent_id).cloned()
    }

    /// The error line of an agent composer.
    pub(crate) fn error(&self, target: &ComposerTarget) -> Option<String> {
        let agent_id = target.agent_id.as_deref()?;
        self.state.borrow().error(agent_id).map(str::to_string)
    }

    pub(crate) fn track_menu(&self, menu: WeakEntity<SkillsMenu>) {
        let mut menus = self.menus.borrow_mut();
        menus.retain(|known| known.upgrade().is_some());
        menus.push(menu);
    }

    /// Redraws the open menus, and the pills when a list changed.
    fn changed(&self, list_changed: bool, cx: &mut App) {
        if list_changed {
            self.ctx.notify(cx);
        }
        let menus: Vec<_> = self
            .menus
            .borrow()
            .iter()
            .filter_map(WeakEntity::upgrade)
            .collect();
        for menu in menus {
            menu.update(cx, |_, cx| cx.notify());
        }
    }

    fn spawn_read(self: &Rc<Self>, agent_id: String, cx: &App) {
        let weak = Rc::downgrade(self);
        cx.spawn(async move |cx| {
            let Some(task) = cx.update(|cx| {
                weak.upgrade()
                    .map(|pins| pins.ctx.rpc(READ, read_input(&agent_id), cx))
            }) else {
                return;
            };
            let result = task.await.and_then(|answer| parse_answer(&answer));
            cx.update(|cx| {
                let Some(pins) = weak.upgrade() else { return };
                let changed = match result {
                    Ok(skills) => pins.state.borrow_mut().publish(&agent_id, skills),
                    Err(error) => {
                        log::debug!("skill-pins.read failed: {error:#}");
                        pins.state
                            .borrow_mut()
                            .read_failed(&agent_id, format!("{error:#}"));
                        false
                    }
                };
                pins.changed(changed, cx);
            });
        })
        .detach();
    }

    /// Saves a new list: a draft stores it in its value, an agent publishes it at once and
    /// sends `skill-pins.write`.
    pub(crate) fn save(self: &Rc<Self>, target: &ComposerTarget, next: Vec<String>, cx: &mut App) {
        let next = normalize(&next);
        if let Some(draft) = &target.draft {
            draft.update(cx, |draft, cx| {
                draft.set(PLUGIN_ID, draft_payload(&next));
                cx.notify();
            });
            self.changed(true, cx);
            return;
        }
        let Some(agent_id) = target.agent_id.clone() else {
            return;
        };
        if !self.state.borrow_mut().begin_save(&agent_id, next.clone()) {
            return;
        }
        self.changed(true, cx);
        let answer = self.ctx.rpc(WRITE, write_input(&agent_id, &next), cx);
        let weak = Rc::downgrade(self);
        cx.spawn(async move |cx| {
            let result = answer.await.and_then(|answer| parse_answer(&answer));
            cx.update(|cx| {
                let Some(pins) = weak.upgrade() else { return };
                let changed = match result {
                    Ok(skills) => pins.state.borrow_mut().publish(&agent_id, skills),
                    Err(error) => {
                        pins.state
                            .borrow_mut()
                            .write_failed(&agent_id, format!("{error:#}"));
                        false
                    }
                };
                pins.changed(changed, cx);
            });
        })
        .detach();
    }

    /// After `ClientEvent::Connected`: forget every list, so each agent reads again.
    fn reconnected(&self, cx: &mut App) {
        self.state.borrow_mut().clear();
        self.changed(true, cx);
    }
}

/// The subscriptions of one activation. A pill closure holds this entity, so deactivating the
/// plugin drops them.
pub(crate) struct Watcher {
    /// Held strong: a subscription holds its emitter weakly.
    _client: Option<Entity<Client>>,
    _connected: Option<Subscription>,
    _settings: Subscription,
}

impl Watcher {
    pub(crate) fn new(pins: Rc<Pins>, cx: &mut Context<Self>) -> Self {
        let client: Option<Entity<Client>> = pins.ctx.client(cx);
        let reconnect = pins.clone();
        let connected = client.as_ref().map(|client| {
            cx.subscribe(client, move |_, _, event: &ClientEvent, cx| {
                if matches!(event, ClientEvent::Connected) {
                    reconnect.reconnected(cx);
                }
            })
        });
        // The settings load later than the pill draws: redraw when they arrive or change.
        let settings = pins.settings.clone();
        let observe = cx.observe(&settings, move |_, _, cx| pins.changed(true, cx));
        Self {
            _client: client,
            _connected: connected,
            _settings: observe,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::State;

    fn ids(list: &[&str]) -> Vec<String> {
        list.iter().map(|id| id.to_string()).collect()
    }

    #[test]
    fn read_once_per_agent() {
        let mut state = State::default();
        let starts = (0..100).filter(|_| state.ask("a1")).count();
        assert_eq!(starts, 1);
        assert!(state.ask("a2"));
        assert!(!state.ask("a2"));
        // A landed list ends the asks for good.
        assert!(state.publish("a1", ids(&["watchtower"])));
        assert!(!state.ask("a1"));
    }

    #[test]
    fn equal_list_does_not_notify() {
        let mut state = State::default();
        assert!(state.publish("a1", ids(&["watchtower"])));
        assert!(!state.publish("a1", ids(&["watchtower"])));
        assert!(state.publish("a1", ids(&["watchtower", "sequential-thinking"])));
        assert!(state.publish("a1", Vec::new()));
        assert!(!state.publish("a1", Vec::new()));
        // An empty list is a known list, not a missing one.
        assert_eq!(state.list("a1"), Some(&Vec::new()));
    }

    #[test]
    fn reconnect_clears_store() {
        let mut state = State::default();
        assert!(state.ask("a1"));
        state.publish("a1", ids(&["watchtower"]));
        state.read_failed("a2", "offline".into());
        state.clear();
        assert_eq!(state.list("a1"), None);
        assert_eq!(state.error("a2"), None);
        assert!(state.ask("a1"));
        assert!(state.ask("a2"));
    }

    #[test]
    fn failed_read_no_loop() {
        let mut state = State::default();
        assert!(state.ask("a1"));
        state.read_failed("a1", "boom".into());
        // The mark is clear, yet no second read starts however often the pill draws.
        assert_eq!((0..100).filter(|_| state.ask("a1")).count(), 0);
        assert_eq!(state.list("a1"), None);
        assert_eq!(state.error("a1"), Some("boom"));
        // A reconnect lets the agent read again.
        state.clear();
        assert!(state.ask("a1"));
    }

    #[test]
    fn failed_write_keeps_list_and_sets_error() {
        let mut state = State::default();
        state.publish("a1", ids(&["watchtower"]));
        assert!(state.begin_save("a1", ids(&["watchtower", "sequential-thinking"])));
        state.write_failed("a1", "write failed".into());
        assert_eq!(
            state.list("a1"),
            Some(&ids(&["watchtower", "sequential-thinking"]))
        );
        assert_eq!(state.error("a1"), Some("write failed"));
        // The next save clears the error line; an answer clears it too.
        assert!(state.begin_save("a1", Vec::new()));
        assert_eq!(state.error("a1"), None);
    }

    #[test]
    fn a_save_waits_for_a_known_list() {
        let mut state = State::default();
        assert!(!state.begin_save("a1", ids(&["watchtower"])));
        assert_eq!(state.list("a1"), None);
    }

    #[test]
    fn an_unknown_rpc_id_leaves_the_list_unknown_with_an_error() {
        let mut state = State::default();
        assert!(state.ask("a1"));
        let error =
            crate::catalog::parse_answer(&serde_json::json!({ "skills": ["nope"] })).unwrap_err();
        state.read_failed("a1", format!("{error:#}"));
        assert_eq!(state.list("a1"), None);
        assert!(state.error("a1").unwrap().contains("unknown skill id"));
    }
}
