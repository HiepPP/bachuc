//! `bachuc-plugin-jev-orchestrator`: the native client of the jev-orchestrator plugin
//! (`hiep-plugins/plugins/jev-orchestrator/index.client.tsx`, ADR-0045). It adds the Delegated
//! work and Direct route panels, the `/delegations` and `/route` commands, the "Open direct Jev
//! routing" command item, and the `job` timeline renderer. The server part stays in TypeScript;
//! this crate calls it only through its `jobs.*` and `direct.*` RPCs.

mod direct_panel;
mod job_item;
mod jobs_panel;
mod model;

use crate::model::{ROUTE_ARGS_ERROR, slash_route_allowed};
use bachuc_icons::IconName;
use bachuc_plugin_host::{
    CommandCenterItem, CommandContext, Contributions, MessageLevel, NativePlugin, PanelContext,
    PanelLocation, PanelTarget, PluginContext, PluginSlashCommand, TimelineRenderer,
    WorkspacePanel,
};
use gpui_kit::App;
use std::rc::Rc;

const PLUGIN_ID: &str = "jev-orchestrator";
const JOBS_PANEL: &str = "jobs";
const DIRECT_PANEL: &str = "direct-route";
const ITEM_KIND: &str = "job";
const ITEM_VERSION: u32 = 1;

pub fn plugin() -> Box<dyn NativePlugin> {
    Box::new(JevOrchestrator)
}

struct JevOrchestrator;

impl NativePlugin for JevOrchestrator {
    fn id(&self) -> &'static str {
        PLUGIN_ID
    }

    fn contribute(&self, ctx: PluginContext, contributions: &mut Contributions, _cx: &mut App) {
        let jobs_ctx = ctx.clone();
        contributions.add_workspace_panel(WorkspacePanel {
            id: JOBS_PANEL,
            title: "Delegated work".into(),
            icon: IconName::GitBranch,
            context: PanelContext::Agent,
            locations: Vec::new(),
            view: Rc::new(move |input, _window, cx| jobs_panel::build(&jobs_ctx, input, cx)),
        });
        let slash_ctx = ctx.clone();
        contributions.add_slash_command(PluginSlashCommand {
            name: "delegations",
            description: "Open autonomous delegation jobs and evidence".into(),
            argument_hint: "".into(),
            context: CommandContext::Agent,
            on_submit: Rc::new(move |input, _window, cx| {
                open_jobs(&slash_ctx, input.workspace_id, input.agent_id, cx);
            }),
        });

        let direct_ctx = ctx.clone();
        contributions.add_workspace_panel(WorkspacePanel {
            id: DIRECT_PANEL,
            title: "Direct route".into(),
            icon: IconName::Route,
            context: PanelContext::Workspace,
            locations: vec![PanelLocation::Workspace, PanelLocation::Explorer],
            view: Rc::new(move |input, window, cx| {
                direct_panel::build(&direct_ctx, input, window, cx)
            }),
        });
        let item_ctx = ctx.clone();
        contributions.add_command_item(CommandCenterItem {
            id: "open-direct-route",
            title: "Open direct Jev routing".into(),
            icon: IconName::Route,
            keywords: vec!["model".into(), "effort".into(), "agent".into()],
            context: CommandContext::Workspace,
            visible: None,
            on_select: Rc::new(move |input, _window, cx| {
                open_direct(&item_ctx, input.workspace_id, cx);
            }),
        });
        let route_ctx = ctx;
        contributions.add_slash_command(PluginSlashCommand {
            name: "route",
            description: "Open direct Jev model and effort routing".into(),
            argument_hint: "".into(),
            context: CommandContext::Workspace,
            on_submit: Rc::new(move |input, _window, cx| {
                if slash_route_allowed(&input.args) {
                    open_direct(&route_ctx, input.workspace_id, cx);
                } else {
                    // Electron throws; its host shows the thrown message.
                    route_ctx.show_message(ROUTE_ARGS_ERROR.into(), MessageLevel::Error, cx);
                }
            }),
        });

        contributions.add_timeline_renderer(TimelineRenderer {
            kind: ITEM_KIND,
            version: ITEM_VERSION,
            view: Rc::new(|input, _window, cx| job_item::build(input, cx)),
        });
    }
}

/// The Delegated work panel of the agent in the command, else of the active agent.
fn open_jobs(
    ctx: &PluginContext,
    workspace_id: Option<String>,
    agent_id: Option<String>,
    cx: &mut App,
) {
    let target = panel_target(
        workspace_id.or_else(|| ctx.active_workspace_id(cx)),
        agent_id.or_else(|| ctx.active_agent_id(cx)),
        true,
    );
    if let Some(target) = target {
        ctx.open_panel(JOBS_PANEL, target, cx);
    }
}

/// The Direct route panel of the workspace in the command, else of the active workspace.
fn open_direct(ctx: &PluginContext, workspace_id: Option<String>, cx: &mut App) {
    let target = panel_target(
        workspace_id.or_else(|| ctx.active_workspace_id(cx)),
        None,
        false,
    );
    if let Some(target) = target {
        ctx.open_panel(DIRECT_PANEL, target, cx);
    }
}

/// Where a panel opens. The agent panel needs both ids; with either missing, nothing opens.
fn panel_target(
    workspace_id: Option<String>,
    agent_id: Option<String>,
    needs_agent: bool,
) -> Option<PanelTarget> {
    if needs_agent && agent_id.is_none() {
        return None;
    }
    Some(PanelTarget {
        workspace_id: workspace_id?,
        agent_id: agent_id.filter(|_| needs_agent),
        location: None,
    })
}

#[cfg(test)]
mod tests {
    use super::{PLUGIN_ID, panel_target, plugin};
    use bachuc_plugin_host::PanelTarget;

    fn some(value: &str) -> Option<String> {
        Some(value.to_string())
    }

    #[test]
    fn the_plugin_id_is_jev_orchestrator() {
        assert_eq!(plugin().id(), PLUGIN_ID);
        assert_eq!(plugin().id(), "jev-orchestrator");
    }

    #[test]
    fn the_jobs_panel_needs_an_agent_and_a_workspace() {
        assert_eq!(
            panel_target(some("w1"), some("a1"), true),
            Some(PanelTarget {
                workspace_id: "w1".into(),
                agent_id: Some("a1".into()),
                location: None,
            })
        );
        assert_eq!(panel_target(some("w1"), None, true), None);
        assert_eq!(panel_target(None, some("a1"), true), None);
        assert_eq!(panel_target(None, None, true), None);
    }

    #[test]
    fn the_direct_route_panel_opens_for_the_workspace_with_no_agent() {
        assert_eq!(
            panel_target(some("w1"), None, false),
            Some(PanelTarget {
                workspace_id: "w1".into(),
                agent_id: None,
                location: None,
            })
        );
        assert_eq!(panel_target(None, None, false), None);
        // An agent id never reaches the workspace panel.
        assert_eq!(
            panel_target(some("w1"), some("a1"), false)
                .unwrap()
                .agent_id,
            None
        );
    }
}
