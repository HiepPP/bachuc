//! `bachuc-plugin-thread-branch`: the native client of the thread-branch plugin
//! (`hiep-plugins/plugins/thread-branch/index.client.tsx`, ADR-0045). It adds the branch, PR,
//! and changes pills of agent composers, the branch pill of New workspace, the turn diff card in
//! the timeline, the Turn diff panel with its command, and the GitHub PR attachment source. The
//! server part stays in TypeScript; this crate calls it only through the `thread-branch.*` RPCs.

mod branch;
mod card;
mod panel;
mod pills;
mod selection;
mod store;
mod turn;

use crate::card::{OpenFile, TurnCardView};
use crate::panel::TurnDiffPanel;
use crate::selection::Selection;
use crate::store::Branches;
use bachuc_icons::IconName;
use bachuc_plugin_host::{
    AttachmentPicker, AttachmentSource, CommandCenterItem, CommandContext, CommandInput,
    Contributions, NativePlugin, PanelContext, PanelLocation, PanelTarget, PluginContext,
    TimelineRenderer, WorkspacePanel,
};
use gpui_kit::{App, AppContext as _};
use std::cell::Cell;
use std::rc::Rc;

const PLUGIN_ID: &str = "thread-branch";
const PANEL_ID: &str = "turn-diff";
const ITEM_KIND: &str = "thread-branch-turn-diff";
const ITEM_VERSION: u32 = 1;
const SEARCH_PRS: &str = "thread-branch.search-prs";

pub fn plugin() -> Box<dyn NativePlugin> {
    Box::new(ThreadBranch)
}

struct ThreadBranch;

impl NativePlugin for ThreadBranch {
    fn id(&self) -> &'static str {
        PLUGIN_ID
    }

    fn contribute(&self, ctx: PluginContext, contributions: &mut Contributions, cx: &mut App) {
        // One branch store and one selection per activation; only these closures hold them, so
        // deactivation drops them with the poll task.
        let store = Branches::new(ctx.clone());
        for pill in pills::pills(&store) {
            contributions.add_composer_pill(pill);
        }

        let selection = Selection::new(cx);
        // The Wrap switch starts on and keeps its last value for the session.
        let wrap = Rc::new(Cell::new(true));

        let (panel_ctx, panel_selection) = (ctx.clone(), selection.clone());
        contributions.add_workspace_panel(WorkspacePanel {
            id: PANEL_ID,
            title: "Turn diff".into(),
            icon: IconName::FileDiff,
            context: PanelContext::Agent,
            locations: vec![PanelLocation::Workspace],
            view: Rc::new(move |input, _window, cx| {
                let (ctx, selection, wrap) =
                    (panel_ctx.clone(), panel_selection.clone(), wrap.clone());
                let agent_id = input.agent_id.unwrap_or_default();
                cx.new(|cx| TurnDiffPanel::new(ctx, selection, wrap, agent_id, cx))
                    .into()
            }),
        });

        // A file press selects the file, then opens the panel for the turn's workspace.
        let (open_ctx, open_selection) = (ctx.clone(), selection.clone());
        let open: OpenFile = Rc::new(move |next, workspace_id, _window, cx| {
            let agent_id = next.agent_id.clone();
            open_selection.set(Some(next), cx);
            open_ctx.open_panel(
                PANEL_ID,
                PanelTarget {
                    workspace_id: workspace_id.to_string(),
                    agent_id: Some(agent_id),
                    location: None,
                },
                cx,
            );
        });
        let card_selection = selection.clone();
        contributions.add_timeline_renderer(TimelineRenderer {
            kind: ITEM_KIND,
            version: ITEM_VERSION,
            view: Rc::new(move |input, _window, cx| {
                let (selection, open) = (card_selection.clone(), open.clone());
                cx.new(|cx| TurnCardView::new(input, selection, open, cx))
                    .into()
            }),
        });

        contributions.add_command_item(CommandCenterItem {
            id: "open-turn-diffs",
            title: "Open turn changes".into(),
            icon: IconName::FileDiff,
            keywords: Vec::new(),
            context: CommandContext::Agent,
            visible: None,
            on_select: Rc::new(move |input, _window, cx| {
                let Some(target) = command_target(input) else {
                    return;
                };
                selection.set(None, cx);
                ctx.open_panel(PANEL_ID, target, cx);
            }),
        });

        contributions.add_attachment_source(AttachmentSource {
            id: "github-pr",
            title: "GitHub PR".into(),
            icon: IconName::GitPullRequest,
            picker: Some(AttachmentPicker {
                title: "Attach an open pull request or its failed CI log".into(),
                search_placeholder: "Search by repo, #number, title, or branch".into(),
                search_method: SEARCH_PRS,
            }),
            on_open: None,
        });
    }
}

/// Where the command opens the panel: the input's workspace and agent. A command with no
/// workspace does nothing.
fn command_target(input: CommandInput) -> Option<PanelTarget> {
    Some(PanelTarget {
        workspace_id: input.workspace_id?,
        agent_id: input.agent_id,
        location: None,
    })
}

#[cfg(test)]
mod tests {
    use super::{CommandInput, command_target};

    #[test]
    fn the_command_opens_the_panel_for_the_inputs_workspace_and_agent() {
        let target = command_target(CommandInput {
            workspace_id: Some("w1".into()),
            agent_id: Some("a1".into()),
        })
        .unwrap();
        assert_eq!(target.workspace_id, "w1");
        assert_eq!(target.agent_id.as_deref(), Some("a1"));
        assert_eq!(target.location, None);
    }

    #[test]
    fn a_command_with_no_workspace_does_nothing() {
        let input = CommandInput {
            workspace_id: None,
            agent_id: Some("a1".into()),
        };
        assert_eq!(command_target(input), None);
    }
}
