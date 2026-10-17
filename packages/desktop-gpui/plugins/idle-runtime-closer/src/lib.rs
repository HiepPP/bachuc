//! `bachuc-plugin-idle-runtime-closer`: the native client of the idle-runtime-closer plugin
//! (`hiep-plugins/plugins/idle-runtime-closer/index.client.tsx`, ADR-0045). It adds the
//! "Dọn process" composer pill, which closes the provider runtime of an idle agent. Electron
//! hides the pill with `visible: false`, so `SHOW_PILL` keeps it hidden here too.

use bachuc_client::AgentSummary;
use bachuc_icons::IconName;
use bachuc_plugin_host::{
    ComposerPill, ComposerTarget, Contributions, Handler, MessageLevel, NativePlugin, PillBehavior,
    PillPlacement, PillState, PluginContext, ShowOnDraft,
};
use gpui_kit::{App, SharedString};
use std::rc::Rc;

const PLUGIN_ID: &str = "idle-runtime-closer";

/// Board's Remove actions close the runtime, so Electron hides the pill. Set this to `true` to
/// show it again; the press rule below already works.
const SHOW_PILL: bool = false;

const PILL_TITLE: &str =
    "Dọn process; giữ conversation. Công việc nền và trạng thái MCP trong RAM sẽ mất.";
const PILL_LABEL: &str = "Dọn process";
const NOT_FOUND: &str = "Không tìm thấy thread.";
const BUSY: &str = "Chờ thread hoàn thành và xử lý quyền đang chờ trước khi dọn process.";

pub fn plugin() -> Box<dyn NativePlugin> {
    Box::new(IdleRuntimeCloser)
}

struct IdleRuntimeCloser;

impl NativePlugin for IdleRuntimeCloser {
    fn id(&self) -> &'static str {
        PLUGIN_ID
    }

    fn contribute(&self, ctx: PluginContext, contributions: &mut Contributions, _cx: &mut App) {
        let on_press: Handler<ComposerTarget> =
            Rc::new(move |target, _window, cx| close_runtime(&ctx, target, cx));
        contributions.add_composer_pill(close_runtime_pill(SHOW_PILL, on_press));
    }
}

/// The `close-runtime` pill. It shows on agent composers only, as the Electron pill does.
fn close_runtime_pill(show: bool, on_press: Handler<ComposerTarget>) -> ComposerPill {
    ComposerPill {
        id: "close-runtime",
        placement: PillPlacement::Track,
        show_on_draft: ShowOnDraft::No,
        workspace_id: None,
        agent_id: None,
        state: Rc::new(move |_target, _cx| pill_state(show, on_press.clone())),
    }
}

/// `None` hides the pill.
fn pill_state(show: bool, on_press: Handler<ComposerTarget>) -> Option<PillState> {
    show.then(|| PillState {
        title: PILL_TITLE.into(),
        label: Some(PILL_LABEL.into()),
        icon: Some(IconName::Power),
        color: None,
        disabled: false,
        behavior: PillBehavior::Action {
            on_press,
            new_workspace_panel: None,
        },
    })
}

/// Why the runtime of this agent must stay, or `Ok` when it may close. `agent` is the status
/// and the pending permission count of the refreshed agent, `None` for an unknown agent.
fn check_closable(agent: Option<(&str, usize)>) -> Result<(), &'static str> {
    match agent {
        None => Err(NOT_FOUND),
        Some((status, pending)) if matches!(status, "running" | "initializing") || pending > 0 => {
            Err(BUSY)
        }
        Some(_) => Ok(()),
    }
}

fn status_of(agents: &[AgentSummary], agent_id: &str) -> Option<(String, usize)> {
    agents
        .iter()
        .find(|agent| agent.id == agent_id)
        .map(|agent| (agent.status.clone(), agent.pending_permissions))
}

/// Refreshes the agent, refuses a busy one, else closes its runtime. A refusal or a failure
/// shows as an error message, as Electron toasts a failed pill press.
fn close_runtime(ctx: &PluginContext, target: ComposerTarget, cx: &mut App) {
    let (Some(agent_id), Some(client)) = (target.agent_id, ctx.client(cx)) else {
        return;
    };
    let refreshed = client.update(cx, |client, cx| client.refresh_agent(&agent_id, cx));
    let ctx = ctx.clone();
    cx.spawn(async move |cx| {
        let result: anyhow::Result<()> = async {
            refreshed.await?;
            let snapshot = cx.update(|cx| status_of(client.read(cx).agents(), &agent_id));
            check_closable(snapshot.as_ref().map(|(status, n)| (status.as_str(), *n)))
                .map_err(anyhow::Error::msg)?;
            let closed = cx.update(|cx| {
                client.update(cx, |client, cx| client.close_agent_runtime(&agent_id, cx))
            });
            closed.await
        }
        .await;
        if let Err(error) = result {
            let message: SharedString = format!("{error:#}").into();
            cx.update(|cx| ctx.show_message(message, MessageLevel::Error, cx));
        }
    })
    .detach();
}

#[cfg(test)]
mod tests {
    use super::*;
    use bachuc_plugin_host::ContributionIndex;

    fn no_press() -> Handler<ComposerTarget> {
        Rc::new(|_, _, _| {})
    }

    #[test]
    fn pill_stays_hidden() {
        const { assert!(!SHOW_PILL) };
        assert!(pill_state(SHOW_PILL, no_press()).is_none());
    }

    #[test]
    fn pill_shows_for_an_unknown_agent_when_enabled() {
        let state = pill_state(true, no_press()).unwrap();
        assert_eq!(state.title.as_ref(), PILL_TITLE);
        assert_eq!(state.label.as_deref(), Some("Dọn process"));
        assert_eq!(state.icon, Some(IconName::Power));
        assert!(!state.disabled);
        assert!(matches!(state.behavior, PillBehavior::Action { .. }));
    }

    #[test]
    fn the_pill_is_an_unscoped_agent_pill() {
        let mut contributions = Contributions::default();
        contributions.add_composer_pill(close_runtime_pill(true, no_press()));
        let index = ContributionIndex::build([(PLUGIN_ID, &contributions)]);
        let [pill] = index.composer_pills() else {
            panic!("expected one pill");
        };
        assert_eq!(pill.value.id, "close-runtime");
        assert_eq!(pill.value.placement, PillPlacement::Track);
        assert_eq!(pill.value.show_on_draft, ShowOnDraft::No);
        assert!(pill.value.workspace_id.is_none() && pill.value.agent_id.is_none());
    }

    #[test]
    fn close_refused_for_busy_agents() {
        assert_eq!(check_closable(Some(("running", 0))), Err(BUSY));
        assert_eq!(check_closable(Some(("initializing", 0))), Err(BUSY));
        assert_eq!(check_closable(Some(("idle", 1))), Err(BUSY));
        assert_eq!(check_closable(None), Err(NOT_FOUND));
        for status in ["idle", "error", "closed"] {
            assert_eq!(check_closable(Some((status, 0))), Ok(()), "{status}");
        }
    }

    #[test]
    fn status_comes_from_the_agent_list() {
        let agent = |id: &str, status: &str, pending| AgentSummary {
            id: id.into(),
            title: String::new(),
            status: status.into(),
            cwd: String::new(),
            workspace_id: None,
            provider: String::new(),
            model: None,
            pending_permissions: pending,
        };
        let agents = [agent("a1", "idle", 0), agent("a2", "running", 2)];
        assert_eq!(status_of(&agents, "a2"), Some(("running".into(), 2)));
        assert_eq!(status_of(&agents, "missing"), None);
    }
}
