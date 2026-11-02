//! `bachuc-plugin-ponytail`: the native client of the ponytail plugin
//! (`hiep-plugins/plugins/ponytail`, ADR-0045, ADR-0079). It adds the Ponytail mode pill to
//! every agent composer and New workspace draft. The server part stays in TypeScript; this
//! crate calls it only through its `ponytail.mode.*` RPCs.

mod mode;

use crate::mode::{
    MODE_READ, MODE_WRITE, Mode, Modes, draft_mode, draft_payload, mode_options, mode_read_input,
    mode_write_input, parse_mode, pill_label,
};
use bachuc_client::{Client, ClientEvent};
use bachuc_plugin_host::{
    ComposerPill, ComposerTarget, Contributions, NativePlugin, PillBehavior, PillPlacement,
    PillState, PluginContext, ShowOnDraft,
};
use gpui_kit::{App, AppContext as _, Context, Entity, Subscription};
use std::cell::RefCell;
use std::rc::Rc;

pub(crate) const PLUGIN_ID: &str = "ponytail";

pub fn plugin() -> Box<dyn NativePlugin> {
    Box::new(Ponytail)
}

struct Ponytail;

impl NativePlugin for Ponytail {
    fn id(&self) -> &'static str {
        PLUGIN_ID
    }

    fn contribute(&self, ctx: PluginContext, contributions: &mut Contributions, cx: &mut App) {
        let shared = Rc::new(Shared {
            ctx,
            modes: RefCell::default(),
        });
        // The pill closure holds the watcher, so deactivation drops its subscription.
        let watcher = {
            let shared = shared.clone();
            cx.new(|cx| Watcher::new(shared, cx))
        };
        contributions.add_composer_pill(ComposerPill {
            id: "ponytail-mode",
            placement: PillPlacement::Toolbar,
            show_on_draft: ShowOnDraft::Yes,
            workspace_id: None,
            agent_id: None,
            state: Rc::new(move |target, cx| {
                let _keep = &watcher;
                shared.pill_state(target, cx)
            }),
        });
    }
}

struct Shared {
    ctx: PluginContext,
    modes: RefCell<Modes>,
}

impl Shared {
    /// The mode of a composer, `None` while unknown. An agent whose mode is missing starts one
    /// read.
    fn current_mode(self: &Rc<Self>, target: &ComposerTarget, cx: &App) -> Option<Mode> {
        if let Some(draft) = &target.draft {
            return Some(draft_mode(draft.read(cx).get(PLUGIN_ID)));
        }
        let agent_id = target.agent_id.as_deref()?;
        if self.modes.borrow_mut().ask(agent_id) {
            self.spawn_read(agent_id.to_string(), cx);
        }
        self.modes.borrow().mode(agent_id)
    }

    /// The pill of a composer, `None` for a target that is neither an agent nor a draft.
    fn pill_state(self: &Rc<Self>, target: &ComposerTarget, cx: &App) -> Option<PillState> {
        if target.draft.is_none() && target.agent_id.is_none() {
            return None;
        }
        let mode = self.current_mode(target, cx);
        let shared = self.clone();
        Some(PillState {
            title: "Ponytail mode".into(),
            label: Some(pill_label(mode).into()),
            icon: None,
            color: None,
            disabled: false,
            behavior: PillBehavior::Select {
                options: mode_options(),
                value: Some(mode.unwrap_or_default().value().into()),
                on_select: Rc::new(move |target, value, cx| shared.pick(target, value, cx)),
            },
        })
    }

    fn spawn_read(self: &Rc<Self>, agent_id: String, cx: &App) {
        let weak = Rc::downgrade(self);
        cx.spawn(async move |cx| {
            let Some(task) = cx.update(|cx| {
                weak.upgrade()
                    .map(|shared| shared.ctx.rpc(MODE_READ, mode_read_input(&agent_id), cx))
            }) else {
                return;
            };
            let result = task.await.and_then(|answer| parse_mode(&answer));
            cx.update(|cx| {
                let Some(shared) = weak.upgrade() else { return };
                let changed = match result {
                    Ok(mode) => shared.modes.borrow_mut().publish(&agent_id, mode),
                    Err(error) => {
                        log::debug!("ponytail.mode.read failed: {error:#}");
                        shared.modes.borrow_mut().read_failed(&agent_id);
                        false
                    }
                };
                if changed {
                    shared.ctx.notify(cx);
                }
            });
        })
        .detach();
    }

    /// Saves a pick: a draft stores it in its value, an agent publishes it at once and sends
    /// `ponytail.mode.write`.
    fn pick(self: &Rc<Self>, target: &ComposerTarget, value: &str, cx: &mut App) {
        let Some(mode) = Mode::parse(value) else {
            return;
        };
        if let Some(draft) = &target.draft {
            draft.update(cx, |draft, cx| {
                draft.set(PLUGIN_ID, draft_payload(mode));
                cx.notify();
            });
            self.ctx.notify(cx);
            return;
        }
        let Some(agent_id) = target.agent_id.clone() else {
            return;
        };
        // Show the pick at once, also while the read has not answered.
        self.modes.borrow_mut().publish(&agent_id, mode);
        self.ctx.notify(cx);
        let answer = self
            .ctx
            .rpc(MODE_WRITE, mode_write_input(&agent_id, mode), cx);
        let weak = Rc::downgrade(self);
        cx.spawn(async move |cx| {
            let result = answer.await.and_then(|answer| parse_mode(&answer));
            cx.update(|cx| {
                let Some(shared) = weak.upgrade() else { return };
                match result {
                    Ok(saved) => {
                        if shared.modes.borrow_mut().publish(&agent_id, saved) {
                            shared.ctx.notify(cx);
                        }
                    }
                    // The optimistic mode stays.
                    Err(error) => log::warn!("ponytail.mode.write failed: {error:#}"),
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

/// The reconnect subscription of one activation. The pill closure holds this entity.
struct Watcher {
    /// Held strong: a subscription holds its emitter weakly.
    _client: Option<Entity<Client>>,
    _connected: Option<Subscription>,
}

impl Watcher {
    fn new(shared: Rc<Shared>, cx: &mut Context<Self>) -> Self {
        let client: Option<Entity<Client>> = shared.ctx.client(cx);
        let connected = client.as_ref().map(|client| {
            cx.subscribe(client, move |_, _, event: &ClientEvent, cx| {
                if matches!(event, ClientEvent::Connected) {
                    shared.reconnected(cx);
                }
            })
        });
        Self {
            _client: client,
            _connected: connected,
        }
    }
}
